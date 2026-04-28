//! Worker thread: drains `ExecMessage`s from a channel, runs each `Run`
//! payload's nodes in topological order, consults / fills the cache, and
//! emits `ProgressEvent`s on the progress channel.
//!
//! Per A1's wave-1 decision: **single worker thread**. Parallel `execute()`
//! calls would just contend on the single CudaDevice.

use erigui_nodes::{NodeRegistry, NodeValue};
use erigui_widgets::node_graph::{Field, FieldValue, Graph};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crate::cache::{composed_key, Cache};
use crate::sink::ChannelProgressSink;
use crate::topo::topo_sort;
use crate::{ExecMessage, NodeId, ProgressEvent};

/// Spawn the worker thread. Returns the thread join handle.
///
/// `cancel_flag` is set to `true` by `Executor::cancel()`; the worker
/// observes it between nodes (cooperative cancellation only — we do not
/// preempt a running `execute()`).
pub fn spawn_worker(
    registry: Arc<NodeRegistry>,
    rx: Receiver<ExecMessage>,
    progress_tx: Sender<ProgressEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("erigui-runtime-worker".into())
        .spawn(move || worker_loop(registry, rx, progress_tx, cancel_flag))
        .expect("worker thread spawn")
}

fn worker_loop(
    registry: Arc<NodeRegistry>,
    rx: Receiver<ExecMessage>,
    progress_tx: Sender<ProgressEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    let mut cache = Cache::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ExecMessage::Cancel => {
                // No active job to cancel; clear the flag for the next run.
                cancel_flag.store(false, Ordering::SeqCst);
            }
            ExecMessage::Run { graph, dirty } => {
                cancel_flag.store(false, Ordering::SeqCst);
                run_graph(&registry, &graph, &dirty, &progress_tx, &mut cache, &cancel_flag);
                let _ = progress_tx.send(ProgressEvent::QueueIdle);
            }
        }
    }
}

/// Execute one `Run` payload. Walks topo order, manages the cache, emits
/// `ProgressEvent`s.
fn run_graph(
    registry: &NodeRegistry,
    graph: &Graph,
    dirty: &HashSet<NodeId>,
    progress_tx: &Sender<ProgressEvent>,
    cache: &mut Cache,
    cancel_flag: &AtomicBool,
) {
    // Topo-sort first; if it fails (cycle / missing node) emit one
    // NodeError tagged at the source node id (or u32::MAX for cycle) and
    // bail. Wave 3 callers can decide how to surface this.
    let order = match topo_sort(graph) {
        Ok(o) => o,
        Err(err) => {
            let _ = progress_tx.send(ProgressEvent::NodeError {
                node_id: u32::MAX,
                error: erigui_nodes::NodeError::Other(format!("topo: {err}")),
            });
            return;
        }
    };

    // Index nodes by id for quick lookup during execution.
    let mut by_id: HashMap<NodeId, &erigui_widgets::node_graph::Node> =
        HashMap::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        if let Ok(id) = u32::try_from(node.id) {
            by_id.insert(id, node);
        }
    }

    // Build adjacency for dirty propagation: child_of[parent] = [children].
    let mut child_of: HashMap<NodeId, Vec<NodeId>> = HashMap::with_capacity(graph.nodes.len());
    for edge in &graph.edges {
        if let (Ok(from), Ok(to)) = (u32::try_from(edge.from_node), u32::try_from(edge.to_node)) {
            child_of.entry(from).or_default().push(to);
        }
    }

    // Expand `dirty` to all transitive descendants — re-running an upstream
    // node necessarily invalidates downstream cache keys.
    let dirty_expanded = expand_dirty(dirty, &child_of);

    // Outputs of nodes already executed (or pulled from cache) this run,
    // keyed by NodeId. Each entry maps output port name -> NodeValue.
    // Cloning a NodeValue is cheap (Arc/handle clone); we clone on read.
    let mut outputs_by_node: HashMap<NodeId, HashMap<String, NodeValue>> =
        HashMap::with_capacity(order.len());

    // Build edge map: for each (to_node, to_port_name) -> (from_node, from_port_name).
    let port_map = build_port_map(graph);

    // Pre-compute consumer counts: for each (producer_node, port_name) how
    // many downstream nodes will consume that output. We decrement as we
    // run; when a count hits zero we drop the value from outputs_by_node
    // AND from cache (for heavy GPU handles only — Model/Clip/Vae/Lora),
    // then trim the CUDA mempool. This is the equivalent of klein_lora_infer's
    // explicit `drop(encoder); trim_cuda_mempool(0);` between stages.
    let mut consumers_remaining: HashMap<(NodeId, String), usize> = HashMap::new();
    for (to_key, (from_node, from_port)) in &port_map {
        let _ = to_key; // we only care about producer side
        *consumers_remaining
            .entry((*from_node, from_port.clone()))
            .or_insert(0) += 1;
    }

    for node_id in order {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = progress_tx.send(ProgressEvent::NodeError {
                node_id,
                error: erigui_nodes::NodeError::Aborted,
            });
            return;
        }

        let node = match by_id.get(&node_id) {
            Some(n) => *n,
            None => continue, // narrow() failure or stale id; topo would have caught it
        };

        let type_id = match node.component_type.as_deref() {
            Some(t) => t,
            None => {
                // No type bound on this node — wave-1 widgets sometimes carry
                // pure-UI nodes with no executor mapping. Skip silently;
                // downstream nodes won't see any inputs from this id.
                continue;
            }
        };

        let node_type = match registry.get(type_id) {
            Some(t) => t,
            None => {
                let _ = progress_tx.send(ProgressEvent::NodeError {
                    node_id,
                    error: erigui_nodes::NodeError::Other(format!(
                        "registry: unknown type_id `{type_id}`"
                    )),
                });
                continue;
            }
        };

        let schema = node_type.schema();

        // Collect inputs: for each declared input port on the schema, look
        // up the upstream output via port_map → outputs_by_node.
        // ALSO: track which (producer, port) pairs this node will consume,
        // so we can decrement consumers_remaining after this node runs and
        // free upstream outputs that won't be needed again.
        let mut inputs: HashMap<String, NodeValue> = HashMap::with_capacity(schema.inputs.len());
        let mut consumed_this_step: Vec<(NodeId, String)> =
            Vec::with_capacity(schema.inputs.len());
        for port in &schema.inputs {
            let key = (node_id, port.name.clone());
            if let Some((from_node, from_port)) = port_map.get(&key) {
                if let Some(node_outs) = outputs_by_node.get(from_node) {
                    if let Some(v) = node_outs.get(from_port) {
                        inputs.insert(port.name.clone(), v.clone());
                    }
                }
                consumed_this_step.push((*from_node, from_port.clone()));
            }
            // Missing inputs are fine here; the node will return
            // NodeError::MissingInput if it actually needs the value.
        }

        // Build the field map from the node's stored Field list. Honors
        // wave-1's two `FieldValue` shapes.
        let fields = build_field_map(&node.fields);

        // Cache lookup.
        let key = composed_key(type_id, &inputs, &fields);
        let dirty_for_this = dirty_expanded.contains(&node_id);
        if !dirty_for_this {
            if let Some(entry) = cache.get(node_id) {
                if entry.key == key {
                    // Hit. Re-emit NodeStart/NodeDone using the cached
                    // outputs; do NOT call execute().
                    let outs = entry.outputs.clone();
                    let _ = progress_tx.send(ProgressEvent::NodeStart { node_id });
                    let outs_vec: Vec<(String, NodeValue)> =
                        outs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    let _ = progress_tx.send(ProgressEvent::NodeDone {
                        node_id,
                        outputs: outs_vec,
                    });
                    outputs_by_node.insert(node_id, outs);
                    continue;
                }
            }
        }

        // Cache miss (or dirty). Execute.
        let _ = progress_tx.send(ProgressEvent::NodeStart { node_id });
        let sink = ChannelProgressSink::new(node_id, progress_tx.clone());
        let result = node_type.execute(&inputs, &fields, Some(&sink));
        match result {
            Ok(outs) => {
                let outs_vec: Vec<(String, NodeValue)> =
                    outs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                cache.insert(node_id, key, outs.clone());
                let _ = progress_tx.send(ProgressEvent::NodeDone {
                    node_id,
                    outputs: outs_vec,
                });
                outputs_by_node.insert(node_id, outs);
            }
            Err(error) => {
                // Don't poison the cache with a failed run.
                cache.invalidate(node_id);
                let _ = progress_tx.send(ProgressEvent::NodeError { node_id, error });
                // Continue executing the remaining topo order; downstream
                // nodes that depend on this one will simply see missing
                // inputs and likely fail with MissingInput. That's the
                // wave-3 spec's behavior.
            }
        }

        // After this node ran (or hit cache), decrement consumer counts for
        // every (producer, port) we just consumed. Drop any that hit zero.
        // For heavy GPU handles (Model/Clip/Vae/Lora) we also invalidate the
        // cache entry and trim the CUDA mempool — these handles are large
        // (multi-GB) and would otherwise keep the weights resident in VRAM
        // even when no remaining downstream node needs them. Mirrors the
        // klein_lora_infer explicit `drop(encoder); trim_cuda_mempool(0);`
        // pattern between stages.
        let mut freed_heavy = false;
        for (producer, port) in consumed_this_step {
            let key = (producer, port.clone());
            if let Some(c) = consumers_remaining.get_mut(&key) {
                if *c > 0 {
                    *c -= 1;
                }
                if *c == 0 {
                    let mut is_heavy = false;
                    if let Some(outs) = outputs_by_node.get_mut(&producer) {
                        if let Some(v) = outs.remove(&port) {
                            is_heavy = matches!(
                                v,
                                NodeValue::Model { .. }
                                    | NodeValue::Clip { .. }
                                    | NodeValue::Vae { .. }
                                    | NodeValue::Lora { .. }
                            );
                            // v dropped at end of scope
                        }
                        if outs.is_empty() {
                            outputs_by_node.remove(&producer);
                        }
                    }
                    if is_heavy {
                        // Cache also holds a clone — must invalidate so the
                        // Arc refcount drops and the underlying GPU handle
                        // actually frees.
                        cache.invalidate(producer);
                        freed_heavy = true;
                    }
                }
            }
        }
        if freed_heavy {
            flame_core::trim_cuda_mempool(0);
        }
    }
}

/// BFS over `child_of` starting from each user-marked dirty id.
fn expand_dirty(
    seed: &HashSet<NodeId>,
    child_of: &HashMap<NodeId, Vec<NodeId>>,
) -> HashSet<NodeId> {
    let mut out: HashSet<NodeId> = seed.clone();
    let mut stack: Vec<NodeId> = seed.iter().copied().collect();
    while let Some(id) = stack.pop() {
        if let Some(children) = child_of.get(&id) {
            for &c in children {
                if out.insert(c) {
                    stack.push(c);
                }
            }
        }
    }
    out
}

/// Build a (downstream_node_id, downstream_port_name) -> (upstream_node_id,
/// upstream_port_name) lookup. This bridges the widget Edge's port-id ints
/// to the schema's port-name strings.
fn build_port_map(
    graph: &Graph,
) -> HashMap<(NodeId, String), (NodeId, String)> {
    let mut out = HashMap::with_capacity(graph.edges.len());

    // Index node ports by (node_id, port_id) -> port_label.
    let mut input_label: HashMap<(NodeId, usize), String> = HashMap::new();
    let mut output_label: HashMap<(NodeId, usize), String> = HashMap::new();
    for node in &graph.nodes {
        let nid = match u32::try_from(node.id) {
            Ok(n) => n,
            Err(_) => continue,
        };
        for p in &node.inputs {
            input_label.insert((nid, p.id), p.label.clone());
        }
        for p in &node.outputs {
            output_label.insert((nid, p.id), p.label.clone());
        }
    }

    for edge in &graph.edges {
        let from = match u32::try_from(edge.from_node) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let to = match u32::try_from(edge.to_node) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let from_port_label = output_label
            .get(&(from, edge.from_port))
            .cloned()
            .unwrap_or_default();
        let to_port_label = input_label
            .get(&(to, edge.to_port))
            .cloned()
            .unwrap_or_default();
        out.insert((to, to_port_label), (from, from_port_label));
    }

    out
}

fn build_field_map(fields: &[Field]) -> HashMap<String, FieldValue> {
    let mut m = HashMap::with_capacity(fields.len());
    for f in fields {
        // Prefer `name` (programmatic key, matches FieldSpec.name in
        // erigui-nodes). Fall back to `label` for back-compat with
        // workflows saved before `Field.name` existed.
        let key = if f.name.is_empty() { &f.label } else { &f.name };
        m.insert(key.clone(), f.value.clone());
    }
    m
}

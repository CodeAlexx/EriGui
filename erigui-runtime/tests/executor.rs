//! Integration tests for `erigui-runtime` (C1).
//!
//! All tests use FAKE node implementations carrying scalars only — no
//! `flame_core::Tensor`, no GPU, no `inference-flame`. Each fake node has
//! an internal `AtomicUsize` so the cache-hit tests can assert "execute
//! was / wasn't called".
//!
//! Per the wave-3 seed spec ("C1: Executor — Tests" section).

use erigui_nodes::{
    FieldKind, FieldSpec, NodeError, NodeRegistry, NodeSchema, NodeType, NodeValue,
    NodeValueType, PortSpec, ProgressSink,
};
use erigui_runtime::{
    composed_key, topo_sort, ExecError, Executor, NodeId, ProgressEvent,
};
use erigui_widgets::node_graph::{
    Edge, Field, FieldKind as WidgetFieldKind, FieldValue, Graph, Node, Port,
};
use erigui_core::{Point, Size};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// -------- Fake node ---------------------------------------------------------

/// Minimal `NodeType` for tests: takes 0+ Number inputs, emits 1 Number
/// output. Increments a counter every time `execute()` is called so we can
/// assert cache behavior.
struct CountingNode {
    type_id_str: &'static str,
    inputs: Vec<&'static str>,
    output_name: &'static str,
    output_value: f32,
    counter: Arc<AtomicUsize>,
    fields: Vec<&'static str>,
}

impl NodeType for CountingNode {
    fn type_id(&self) -> &'static str {
        self.type_id_str
    }

    fn schema(&self) -> NodeSchema {
        let inputs: Vec<PortSpec> = self
            .inputs
            .iter()
            .map(|n| PortSpec {
                name: (*n).to_string(),
                value_type: NodeValueType::Number,
            })
            .collect();
        let outputs = vec![PortSpec {
            name: self.output_name.to_string(),
            value_type: NodeValueType::Number,
        }];
        let fields: Vec<FieldSpec> = self
            .fields
            .iter()
            .map(|n| FieldSpec {
                name: (*n).to_string(),
                label: (*n).to_string(),
                kind: FieldKind::Number {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                },
                default: FieldValue::Number(0.0),
            })
            .collect();
        NodeSchema {
            display_name: "Counting",
            category: "Test",
            inputs,
            outputs,
            fields,
        }
    }

    fn execute(
        &self,
        _inputs: &HashMap<String, NodeValue>,
        _fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        let mut out = HashMap::new();
        out.insert(
            self.output_name.to_string(),
            NodeValue::Number(self.output_value),
        );
        Ok(out)
    }
}

// -------- helpers -----------------------------------------------------------

fn point(x: i32, y: i32) -> Point {
    Point::new(x, y)
}
fn size(w: i32, h: i32) -> Size {
    Size::new(w, h)
}

fn make_node(id: usize, type_id: &str, inputs: &[&str], outputs: &[&str]) -> Node {
    let inputs_p: Vec<Port> = inputs
        .iter()
        .enumerate()
        .map(|(i, lbl)| Port {
            id: i,
            label: (*lbl).to_string(),
            is_input: true,
        })
        .collect();
    let outputs_p: Vec<Port> = outputs
        .iter()
        .enumerate()
        .map(|(i, lbl)| Port {
            id: i,
            label: (*lbl).to_string(),
            is_input: false,
        })
        .collect();
    Node {
        id,
        title: type_id.to_string(),
        position: point(0, 0),
        size: size(120, 60),
        inputs: inputs_p,
        outputs: outputs_p,
        fields: Vec::new(),
        component_type: Some(type_id.to_string()),
    }
}

fn add_field(node: &mut Node, name: &str, value: FieldValue) {
    node.fields.push(Field {
        id: node.fields.len(),
        name: name.to_string(),
        label: name.to_string(),
        kind: WidgetFieldKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
        },
        value,
    });
}

fn edge(from_node: usize, from_port: usize, to_node: usize, to_port: usize) -> Edge {
    Edge {
        from_node,
        from_port,
        to_node,
        to_port,
    }
}

/// Drain progress events, blocking until `QueueIdle` arrives or `timeout`
/// elapses. Tests use this so we don't race the worker.
fn drain_until_idle(exec: &Executor, timeout: Duration) -> Vec<ProgressEvent> {
    let start = Instant::now();
    let mut all: Vec<ProgressEvent> = Vec::new();
    loop {
        let batch = exec.poll_progress();
        let saw_idle = batch
            .iter()
            .any(|e| matches!(e, ProgressEvent::QueueIdle));
        all.extend(batch);
        if saw_idle {
            return all;
        }
        if start.elapsed() > timeout {
            panic!(
                "drain_until_idle timed out after {:?}; collected {} events: {:?}",
                timeout,
                all.len(),
                all.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn count_node_done(events: &[ProgressEvent], node_id: NodeId) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ProgressEvent::NodeDone { node_id: nid, .. } if *nid == node_id))
        .count()
}

// =============================================================================
// 1. Topo: linear A -> B -> C
// =============================================================================
#[test]
fn topo_sort_linear() {
    // Three nodes with two edges A->B->C.
    let a = make_node(1, "test/a", &[], &["out"]);
    let b = make_node(2, "test/b", &["in"], &["out"]);
    let c = make_node(3, "test/c", &["in"], &["out"]);
    let graph = Graph {
        nodes: vec![a, b, c],
        edges: vec![edge(1, 0, 2, 0), edge(2, 0, 3, 0)],
    };
    let order = topo_sort(&graph).expect("linear graph should sort");
    assert_eq!(order, vec![1, 2, 3]);
}

// =============================================================================
// 2. Topo: diamond A->B, A->C, B->D, C->D
// =============================================================================
#[test]
fn topo_sort_diamond() {
    let a = make_node(1, "test/a", &[], &["out"]);
    let b = make_node(2, "test/b", &["in"], &["out"]);
    let c = make_node(3, "test/c", &["in"], &["out"]);
    let d = make_node(4, "test/d", &["in1", "in2"], &["out"]);
    let graph = Graph {
        nodes: vec![a, b, c, d],
        edges: vec![
            edge(1, 0, 2, 0),
            edge(1, 0, 3, 0),
            edge(2, 0, 4, 0),
            edge(3, 0, 4, 1),
        ],
    };
    let order = topo_sort(&graph).expect("diamond should sort");
    assert_eq!(order.len(), 4);
    assert_eq!(order[0], 1, "A must come first");
    assert_eq!(order[3], 4, "D must come last");
    // B and C must each appear before D and after A.
    let pos = |id: NodeId| order.iter().position(|x| *x == id).unwrap();
    assert!(pos(2) < pos(4));
    assert!(pos(3) < pos(4));
    assert!(pos(1) < pos(2));
    assert!(pos(1) < pos(3));
}

// =============================================================================
// 3. Topo: cycle detection A -> B -> A
// =============================================================================
#[test]
fn cycle_detection() {
    let a = make_node(1, "test/a", &["in"], &["out"]);
    let b = make_node(2, "test/b", &["in"], &["out"]);
    let graph = Graph {
        nodes: vec![a, b],
        edges: vec![edge(1, 0, 2, 0), edge(2, 0, 1, 0)],
    };
    let err = topo_sort(&graph).expect_err("cycle must be detected");
    assert!(matches!(err, ExecError::Cycle), "got {err:?}");
}

// =============================================================================
// 4. Cache hit skips execute
// =============================================================================
#[test]
fn cache_hit_skips_execute() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut registry = NodeRegistry::new();
    registry.register(Box::new(CountingNode {
        type_id_str: "test/counting",
        inputs: vec![],
        output_name: "out",
        output_value: 1.0,
        counter: Arc::clone(&counter),
        fields: vec!["k"],
    }));
    let registry = Arc::new(registry);

    let exec = Executor::new(registry);

    // Single counting node, no edges.
    let mut node = make_node(1, "test/counting", &[], &["out"]);
    add_field(&mut node, "k", FieldValue::Number(7.0));
    let graph = Graph {
        nodes: vec![node],
        edges: vec![],
    };

    // First run: cache empty, execute() called once.
    exec.enqueue(graph.clone(), HashSet::new()).unwrap();
    let _ = drain_until_idle(&exec, Duration::from_secs(2));
    assert_eq!(counter.load(Ordering::SeqCst), 1, "first run must execute");

    // Second run with identical graph + empty dirty: cache hit, no execute.
    exec.enqueue(graph, HashSet::new()).unwrap();
    let events = drain_until_idle(&exec, Duration::from_secs(2));
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "second run must hit cache, not execute"
    );
    // We still expect a NodeDone event (re-emitted from cache).
    assert_eq!(count_node_done(&events, 1), 1);
}

// =============================================================================
// 5. Field change invalidates cache
// =============================================================================
#[test]
fn field_change_invalidates_cache() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut registry = NodeRegistry::new();
    registry.register(Box::new(CountingNode {
        type_id_str: "test/counting",
        inputs: vec![],
        output_name: "out",
        output_value: 1.0,
        counter: Arc::clone(&counter),
        fields: vec!["k"],
    }));
    let registry = Arc::new(registry);

    let exec = Executor::new(registry);

    let mut node = make_node(1, "test/counting", &[], &["out"]);
    add_field(&mut node, "k", FieldValue::Number(7.0));
    let graph = Graph {
        nodes: vec![node],
        edges: vec![],
    };

    exec.enqueue(graph, HashSet::new()).unwrap();
    let _ = drain_until_idle(&exec, Duration::from_secs(2));
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Mutate the field — same node id, different field value -> different
    // composed_key -> cache miss -> execute called again.
    let mut node2 = make_node(1, "test/counting", &[], &["out"]);
    add_field(&mut node2, "k", FieldValue::Number(8.0));
    let graph2 = Graph {
        nodes: vec![node2],
        edges: vec![],
    };

    exec.enqueue(graph2, HashSet::new()).unwrap();
    let _ = drain_until_idle(&exec, Duration::from_secs(2));
    assert_eq!(
        counter.load(Ordering::SeqCst),
        2,
        "field change must invalidate cache"
    );
}

// =============================================================================
// Bonus: composed_key sanity check (doesn't depend on the worker thread)
// =============================================================================
#[test]
fn composed_key_changes_with_field() {
    let mut fields_a: HashMap<String, FieldValue> = HashMap::new();
    fields_a.insert("k".into(), FieldValue::Number(1.0));
    let mut fields_b: HashMap<String, FieldValue> = HashMap::new();
    fields_b.insert("k".into(), FieldValue::Number(2.0));
    let inputs: HashMap<String, NodeValue> = HashMap::new();
    let key_a = composed_key("test/x", &inputs, &fields_a);
    let key_b = composed_key("test/x", &inputs, &fields_b);
    assert_ne!(key_a, key_b);
}

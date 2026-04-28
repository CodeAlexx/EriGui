//! erigui-runtime — node-graph executor for the EriGui diffusion canvas.
//!
//! Wave 3 / C1 scope (`docs/wave3_seed_spec.md`): orchestrate execution of
//! an `erigui_widgets::node_graph::Graph` on a single worker thread, using
//! a content-hash cache to skip nodes whose `(type_id, fields, inputs)`
//! are bit-identical to the prior run, and emit a stable `ProgressEvent`
//! stream the UI side polls per frame.
//!
//! ## Coordination contract for C2 / C3 / C4
//!
//! - `NodeId = u32`. The wave-1 workflow JSON envelope uses `u32` (per
//!   A1's wave-1 spec, "Decisions" #8); the in-memory `Graph` from
//!   `erigui-widgets::node_graph` still uses `usize`. We narrow at the
//!   boundary inside this crate. Any `usize` node id that exceeds
//!   `u32::MAX` produces an `ExecError::MissingNode { node_id: u32::MAX }`
//!   sentinel — it can't appear in a workflow that round-trips to JSON.
//!
//! - The `ProgressEvent` enum and the field shapes in each variant are
//!   **stable**. New variants may be added (non-breaking for receivers
//!   matching all known variants); existing variants do not change.
//!
//! - The crate is **pure CPU**. It calls `NodeType::execute()` on real
//!   nodes (which DO run GPU code), but contains no GPU code itself, no
//!   `inference-flame` import.

use erigui_nodes::{NodeError, NodeRegistry, NodeValue};
use erigui_widgets::node_graph::Graph;
use flame_core::Tensor;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub mod cache;
pub mod sink;
pub mod topo;
pub mod worker;

pub use cache::{composed_key, Cache, CacheKey, CachedOutputs};
pub use sink::ChannelProgressSink;
pub use topo::topo_sort;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Wave-1 node identifier. `Graph::Node::id` in `erigui-widgets` is `usize`
/// for historical reasons; the workflow JSON envelope uses `u32`. We use
/// `u32` everywhere in the runtime and narrow at the `Graph` boundary.
pub type NodeId = u32;

/// Inbound message from the UI thread to the worker.
pub enum ExecMessage {
    /// Re-run the graph. `dirty` is the set of node ids the user explicitly
    /// dirtied via field edits; the executor expands transitively to
    /// downstream descendants. Pass an empty set on the very first run to
    /// force everything to execute (cache is empty anyway).
    Run {
        graph: Graph,
        dirty: HashSet<NodeId>,
    },
    /// Cooperative cancellation. Observed between nodes in the topo order;
    /// does NOT preempt a running `execute()`.
    Cancel,
}

/// Outbound progress event emitted by the worker; UI polls via
/// [`Executor::poll_progress`].
#[derive(Debug)]
pub enum ProgressEvent {
    /// Node is about to run (or be served from cache).
    NodeStart { node_id: NodeId },
    /// Mid-execute progress tick (e.g. KSampler step 3/30).
    NodeStep {
        node_id: NodeId,
        current: u32,
        total: u32,
        label: String,
    },
    /// Optional intermediate preview image (e.g. denoising preview).
    NodePreview { node_id: NodeId, image: Tensor },
    /// Node finished successfully. `outputs` is a Vec of (port_name, value)
    /// — Vec, not HashMap, so receivers don't need to drag in HashMap
    /// hashing semantics.
    NodeDone {
        node_id: NodeId,
        outputs: Vec<(String, NodeValue)>,
    },
    /// Node failed. The executor continues with the rest of the topo order;
    /// downstream nodes that needed this output will fail with `MissingInput`.
    NodeError {
        node_id: NodeId,
        error: NodeError,
    },
    /// All nodes in the queued `Run` have finished (success or error).
    QueueIdle,
}

/// Errors raised at the executor (graph) level rather than per-node. Per-
/// node failures travel via `ProgressEvent::NodeError` instead.
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    /// The graph contains a directed cycle. Topo-sort can't proceed.
    #[error("graph contains a cycle")]
    Cycle,

    /// An edge referenced a node id that wasn't declared in `graph.nodes`.
    #[error("unknown node id `{node_id}` referenced by edge")]
    MissingNode { node_id: NodeId },

    /// The worker channel was closed (worker thread panicked / dropped).
    #[error("worker channel closed")]
    EnqueueFailed,
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/// Top-level runtime. Owns the worker thread + channels.
///
/// Drop semantics: dropping `Executor` closes the inbound channel; the
/// worker drains any pending message and exits cleanly. We don't `join()`
/// the thread on drop (would block); the worker is short-lived per run
/// and parks on `recv()` when idle.
pub struct Executor {
    registry: Arc<NodeRegistry>,
    worker_tx: Sender<ExecMessage>,
    progress_rx: Mutex<Receiver<ProgressEvent>>,
    cancel_flag: Arc<AtomicBool>,
    _worker: JoinHandle<()>,
}

impl Executor {
    /// Spawn the worker thread and return the executor.
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        let (worker_tx, worker_rx) = channel::<ExecMessage>();
        let (progress_tx, progress_rx) = channel::<ProgressEvent>();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let worker = worker::spawn_worker(
            Arc::clone(&registry),
            worker_rx,
            progress_tx,
            Arc::clone(&cancel_flag),
        );
        Self {
            registry,
            worker_tx,
            progress_rx: Mutex::new(progress_rx),
            cancel_flag,
            _worker: worker,
        }
    }

    /// Borrow the registry — useful for the UI side to resolve schemas
    /// without keeping its own `Arc`.
    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }

    /// Enqueue a graph for execution. Returns immediately; progress events
    /// arrive via [`poll_progress`](Self::poll_progress).
    pub fn enqueue(&self, graph: Graph, dirty: HashSet<NodeId>) -> Result<(), ExecError> {
        self.worker_tx
            .send(ExecMessage::Run { graph, dirty })
            .map_err(|_| ExecError::EnqueueFailed)
    }

    /// Drain any pending progress events. Non-blocking — returns immediately
    /// with whatever is currently in the channel.
    pub fn poll_progress(&self) -> Vec<ProgressEvent> {
        let rx = self.progress_rx.lock().expect("progress_rx mutex poisoned");
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    /// Signal the worker to abort the current job at the next inter-node
    /// boundary. Does NOT preempt a running `execute()`.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        // Best-effort: also nudge the worker out of any pending Run.
        let _ = self.worker_tx.send(ExecMessage::Cancel);
    }
}

//! `ProgressSink` adapter that forwards step / preview / message calls
//! into the executor's `ProgressEvent` channel, tagged with the running
//! node's id.
//!
//! The sink is constructed per-node by the worker right before calling
//! `NodeType::execute()` so the `node_id` field is fixed for the lifetime
//! of one execute call.

use erigui_nodes::{ProgressLevel, ProgressSink};
use flame_core::Tensor;
use std::sync::mpsc::Sender;

use crate::{NodeId, ProgressEvent};

/// `ProgressSink` impl that emits [`ProgressEvent`]s on a channel.
///
/// Sender is `Send + Sync` (mpsc::Sender requires only `Send`, but cloning
/// the sender across threads still works — and we wrap each call in &self
/// borrow so the trait's `Send + Sync` bound is satisfied).
pub struct ChannelProgressSink {
    pub node_id: NodeId,
    /// `Sender<T>` is `Send` when `T: Send`. `ProgressEvent` carries
    /// `NodeValue` and `Tensor`, both of which are `Send` (Arc-wrapped
    /// payloads). Wrapping in `Mutex` would let us be `Sync` too, but
    /// the trait only requires `Send`-callable methods (`&self`), and
    /// `mpsc::Sender::send` takes `&self`, so we're fine.
    pub tx: Sender<ProgressEvent>,
}

impl ChannelProgressSink {
    pub fn new(node_id: NodeId, tx: Sender<ProgressEvent>) -> Self {
        Self { node_id, tx }
    }
}

impl ProgressSink for ChannelProgressSink {
    fn step(&self, current: u32, total: u32, label: &str) {
        // Drop sends on receiver-hung-up: the executor is shutting down.
        let _ = self.tx.send(ProgressEvent::NodeStep {
            node_id: self.node_id,
            current,
            total,
            label: label.to_string(),
        });
    }

    fn preview(&self, image: &Tensor) {
        let _ = self.tx.send(ProgressEvent::NodePreview {
            node_id: self.node_id,
            image: image.clone(),
        });
    }

    fn message(&self, _level: ProgressLevel, _msg: &str) {
        // The wave-3 spec's stable `ProgressEvent` set has no NodeMessage
        // variant. Until C2/C3/C4 ship and surface a need for it, we drop
        // info/warn/error messages on the floor — adding the variant later
        // is non-breaking for receivers who match all known variants.
    }
}

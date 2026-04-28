//! Concrete value payloads carried by some [`NodeValue`] variants.
//!
//! Wave 2 collapses the per-handle stub newtypes (`ModelHandle`, `VaeHandle`,
//! ...) into `Arc<dyn Any + Send + Sync>` discriminated by an
//! [`ArchTag`](crate::ArchTag) — see `lib.rs`. The real concrete handle
//! types (e.g. `KleinTransformer`, `Qwen3EncoderHandle`, `KleinVae`) live
//! in `crate::handles` and inside `inference-flame`.
//!
//! `ConditioningPack` is the one variant payload that's wide enough to
//! deserve its own struct: a prompt-encoder output is a triple
//! (embeds, optional pooled, optional mask) plus an architecture tag.

use flame_core::Tensor;

use crate::ArchTag;

/// Encoder output bundle. Klein uses `embeds` + `mask`; SDXL/SD3 add
/// `pooled`. Always includes an `arch` tag so downstream nodes can fail
/// fast on architecture mismatch.
#[derive(Clone, Debug)]
pub struct ConditioningPack {
    pub embeds: Tensor,
    pub pooled: Option<Tensor>,
    pub mask: Option<Tensor>,
    pub arch: ArchTag,
}

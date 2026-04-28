//! Concrete handle types stored inside `Arc<dyn Any + Send + Sync>` for
//! the model/clip/vae/lora variants of [`crate::NodeValue`].
//!
//! Wave-2 scope: Klein only. When other architectures land, sibling
//! sub-modules (e.g. `handles::zimage`) will define their own wrappers
//! and the `ArchTag` discriminant on `NodeValue::Clip { arch, .. }` etc.
//! decides which downcast to attempt.
//!
//! The Klein chat-template constants and tokenizer-padding parameters
//! also live here so `core/encode_prompt` (B3) and `core/load_checkpoint`
//! (B1) share one source of truth — both files were copy-pasting
//! literals from `klein_lora_infer.rs`.

use inference_flame::models::qwen3_encoder::Qwen3Encoder;
use inference_flame::vae::klein_vae::{KleinVaeDecoder, KleinVaeEncoder};
use tokenizers::Tokenizer;

/// Re-export so existing `use crate::handles::ArchTag` imports keep
/// resolving. The canonical home is `crate::ArchTag` (per `nodes_spec.md`
/// §"NodeValue (refined)") but B3 already pinned the path to `handles`,
/// and we don't want to churn that.
pub use crate::ArchTag;

// ---------------------------------------------------------------------------
// Klein tokenizer constants (verbatim from
// inference-flame/src/bin/klein_lora_infer.rs:41-46).
// ---------------------------------------------------------------------------

pub const KLEIN_TEMPLATE_PRE: &str = "<|im_start|>user\n";

pub const KLEIN_TEMPLATE_POST: &str =
    "<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n";

/// Klein text-encoder padding length.
pub const KLEIN_TXT_PAD_LEN: usize = 512;

/// Klein Qwen3 pad-token id.
pub const KLEIN_PAD_ID: i32 = 151643;

// ---------------------------------------------------------------------------
// Klein concrete handle types
// ---------------------------------------------------------------------------

/// Klein text-conditioning handle: Qwen3 encoder + matching tokenizer.
///
/// Klein 4B uses a Qwen3-4B encoder; Klein 9B uses Qwen3-8B. The
/// tokenizer is loaded from the path given in the checkpoint sidecar.
pub struct Qwen3EncoderHandle {
    pub encoder: Qwen3Encoder,
    pub tokenizer: Tokenizer,
}

/// Klein VAE bundle. Decoder is mandatory (used by `core/vae_decode`);
/// encoder is loaded eagerly so a future `core/vae_encode` node can
/// share the same handle without reloading the safetensors. Wave 2
/// only consumes `decoder`; `encoder` lives unused for now (it's a
/// few hundred MB of weights, but reusing one VAE-load avoids the
/// alloc-thrash a graph with both encode + decode would otherwise
/// cause).
pub struct KleinVae {
    pub decoder: KleinVaeDecoder,
    pub encoder: Option<KleinVaeEncoder>,
}

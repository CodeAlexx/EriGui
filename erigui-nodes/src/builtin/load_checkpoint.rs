//! `core/load_checkpoint` — wave 2 implementation (Klein only).
//!
//! Carves the "load DiT + encoder + VAE" stages out of
//! `inference-flame/src/bin/klein_lora_infer.rs` (Stage 1 + Stage 3 +
//! Stage 6) into a single `NodeType::execute()`.
//!
//! # Sidecar JSON format
//!
//! The `path` field on this node points to a small JSON sidecar that
//! names the three (four) safetensors files Klein needs:
//!
//! ```jsonc
//! {
//!     "arch": "klein",
//!     "dit":       "/path/to/flux-2-klein-base-4b.safetensors",
//!     "encoder":   "/path/to/qwen_3_4b.safetensors",
//!     "vae":       "/path/to/flux2-vae.safetensors",
//!     "tokenizer": "/path/to/tokenizer.json"
//! }
//! ```
//!
//! `arch` MUST equal `"klein"` for wave 2; everything else returns
//! `NodeError::Other("only klein supported in wave 2")`.
//!
//! `tokenizer` is required for Klein (the `Qwen3Encoder` only handles
//! token ids — string→id is a separate `tokenizers::Tokenizer` step
//! per `klein_lora_infer.rs:188-205`). The sidecar carries it so
//! downstream `core/encode_prompt` doesn't need a second file picker.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use inference_flame::models::klein::KleinTransformer;
use inference_flame::models::qwen3_encoder::Qwen3Encoder;
use inference_flame::vae::klein_vae::KleinVaeDecoder;

use crate::handles::{KleinVae, Qwen3EncoderHandle};
use crate::{
    ArchTag, FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType, NodeValue,
    NodeValueType, PortSpec, ProgressSink,
};

pub struct LoadCheckpoint;

#[derive(serde::Deserialize)]
struct Sidecar {
    arch: String,
    dit: PathBuf,
    encoder: PathBuf,
    vae: PathBuf,
    tokenizer: PathBuf,
}

impl NodeType for LoadCheckpoint {
    fn type_id(&self) -> &'static str {
        "core/load_checkpoint"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "Load Checkpoint",
            category: "Loaders",
            inputs: vec![],
            outputs: vec![
                PortSpec {
                    name: "model".into(),
                    value_type: NodeValueType::Model,
                },
                PortSpec {
                    name: "clip".into(),
                    value_type: NodeValueType::Clip,
                },
                PortSpec {
                    name: "vae".into(),
                    value_type: NodeValueType::Vae,
                },
            ],
            fields: vec![FieldSpec {
                name: "path".into(),
                label: "Checkpoint sidecar".into(),
                kind: FieldKind::FilePath {
                    extensions: vec!["json".into()],
                },
                default: FieldValue::FilePath(PathBuf::new()),
            }],
        }
    }

    fn execute(
        &self,
        _inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        // ── parse the sidecar JSON ──────────────────────────────────
        let path: &PathBuf = match fields
            .get("path")
            .ok_or_else(|| NodeError::MissingField { name: "path".into() })?
        {
            FieldValue::FilePath(p) => p,
            _ => {
                return Err(NodeError::FieldKindMismatch {
                    name: "path".into(),
                    expected: "FilePath",
                });
            }
        };

        let sidecar_bytes = std::fs::read(path)?;
        let sidecar: Sidecar = serde_json::from_slice(&sidecar_bytes).map_err(|e| {
            NodeError::Other(format!(
                "checkpoint sidecar at {} is not valid JSON: {}",
                path.display(),
                e
            ))
        })?;

        if sidecar.arch.as_str() != "klein" {
            return Err(NodeError::Other(format!(
                "only klein supported in wave 2 (sidecar arch = {:?})",
                sidecar.arch
            )));
        }

        let device = flame_core::global_cuda_device();

        // ── DiT ─────────────────────────────────────────────────────
        // Mirrors `klein_lora_infer.rs:270`:
        //     let base = flame_core::serialization::load_file(&args.base, &device)?;
        let dit_weights = flame_core::serialization::load_file(&sidecar.dit, &device)?;
        let dit = KleinTransformer::from_weights(dit_weights)?;

        // ── Encoder + tokenizer ─────────────────────────────────────
        // Mirrors `klein_lora_infer.rs:209-234`. We accept either a
        // single `.safetensors` file (Klein 4B / Qwen3-4B) or a
        // directory of `model-*.safetensors` shards (Klein 9B / Qwen3-8B).
        let enc_weights = if sidecar.encoder.is_dir() {
            let mut shards: Vec<PathBuf> = std::fs::read_dir(&sidecar.encoder)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|s| s.to_str()) == Some("safetensors")
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with("model-"))
                            .unwrap_or(false)
                })
                .collect();
            shards.sort();
            let mut all = HashMap::new();
            for p in &shards {
                let s = flame_core::serialization::load_file(p, &device)?;
                all.extend(s);
            }
            all
        } else {
            flame_core::serialization::load_file(&sidecar.encoder, &device)?
        };
        let enc_config = Qwen3Encoder::config_from_weights(&enc_weights)?;
        let encoder = Qwen3Encoder::new(enc_weights, enc_config, device.clone());

        let tokenizer = tokenizers::Tokenizer::from_file(&sidecar.tokenizer).map_err(|e| {
            NodeError::Other(format!(
                "failed to load tokenizer at {}: {}",
                sidecar.tokenizer.display(),
                e
            ))
        })?;

        let clip_handle = Qwen3EncoderHandle { encoder, tokenizer };

        // ── VAE ─────────────────────────────────────────────────────
        // Mirrors `klein_lora_infer.rs:363-365`. `KleinVaeDecoder::load`
        // wants `&Device` (the wrapper); we adapt from the `Arc<CudaDevice>`
        // we already have, matching the binary's `Device::from_arc(...)` call.
        let vae_weights = flame_core::serialization::load_file(&sidecar.vae, &device)?;
        let vae_device = flame_core::Device::from_arc(device.clone());
        let decoder = KleinVaeDecoder::load(&vae_weights, &vae_device)?;
        let vae_handle = KleinVae {
            decoder,
            encoder: None,
        };

        // ── outputs ─────────────────────────────────────────────────
        let mut out = HashMap::new();
        out.insert(
            "model".into(),
            NodeValue::Model {
                arch: ArchTag::Klein,
                handle: Arc::new(dit) as Arc<dyn std::any::Any + Send + Sync>,
            },
        );
        out.insert(
            "clip".into(),
            NodeValue::Clip {
                arch: ArchTag::Klein,
                handle: Arc::new(clip_handle) as Arc<dyn std::any::Any + Send + Sync>,
            },
        );
        out.insert(
            "vae".into(),
            NodeValue::Vae {
                arch: ArchTag::Klein,
                handle: Arc::new(vae_handle) as Arc<dyn std::any::Any + Send + Sync>,
            },
        );
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Schema-only test (no GPU). Verifies port names, types, and
    /// field schema match the wave-2 spec.
    #[test]
    fn schema_matches_spec() {
        let n = LoadCheckpoint;
        assert_eq!(n.type_id(), "core/load_checkpoint");
        let s = n.schema();
        assert_eq!(s.display_name, "Load Checkpoint");
        assert_eq!(s.category, "Loaders");

        // No inputs.
        assert!(s.inputs.is_empty());

        // Outputs: model, clip, vae (in this order per spec).
        assert_eq!(s.outputs.len(), 3);
        assert_eq!(s.outputs[0].name, "model");
        assert_eq!(s.outputs[0].value_type, NodeValueType::Model);
        assert_eq!(s.outputs[1].name, "clip");
        assert_eq!(s.outputs[1].value_type, NodeValueType::Clip);
        assert_eq!(s.outputs[2].name, "vae");
        assert_eq!(s.outputs[2].value_type, NodeValueType::Vae);

        // One field: path : FilePath.
        assert_eq!(s.fields.len(), 1);
        let p = &s.fields[0];
        assert_eq!(p.name, "path");
        assert!(matches!(p.kind, FieldKind::FilePath { .. }));
        assert!(matches!(p.default, FieldValue::FilePath(_)));
    }
}

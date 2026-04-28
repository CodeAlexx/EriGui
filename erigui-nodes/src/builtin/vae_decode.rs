//! `core/vae_decode` — Klein VAE decode node.
//!
//! Wave 2, Klein only. Per `wave2_seed_spec.md`, the body is a single
//! forward call: validate arch, downcast the VAE handle to
//! [`crate::handles::KleinVae`], and run `decoder.decode(latent)` to get an
//! RGB tensor `[B, 3, H*16, W*16]` in `[-1, 1]`.
//!
//! Reconciliation note (B5):
//! - The wave-2 spec named the handle `KleinVae` and gestured at a
//!   `vae.decode(...)` method on it. B1's `handles::KleinVae` is actually
//!   `{ decoder: KleinVaeDecoder, encoder: Option<KleinVaeEncoder> }` — a
//!   bundle that holds both halves so a future `core/vae_encode` node can
//!   share the load. Decode therefore goes through `vae.decoder.decode(..)`.
//! - The concrete inference-flame type is `KleinVaeDecoder` (at
//!   `inference-flame/src/vae/klein_vae.rs`), not the spec's `KleinVae`.
//!   B1's `KleinVae` wrapper resolves the naming.
//! - Arches other than `Klein` return `NodeError::Other(..)` per the
//!   wave-2 "Klein only; others error" rule.

use std::collections::HashMap;

use crate::handles::KleinVae;
use crate::{
    ArchTag, FieldValue, NodeError, NodeSchema, NodeType, NodeValue, NodeValueType, PortSpec,
    ProgressSink,
};

pub struct VaeDecode;

impl NodeType for VaeDecode {
    fn type_id(&self) -> &'static str {
        "core/vae_decode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "VAE Decode",
            category: "VAE",
            inputs: vec![
                PortSpec {
                    name: "latent".into(),
                    value_type: NodeValueType::Latent,
                },
                PortSpec {
                    name: "vae".into(),
                    value_type: NodeValueType::Vae,
                },
            ],
            outputs: vec![PortSpec {
                name: "image".into(),
                value_type: NodeValueType::Image,
            }],
            fields: vec![],
        }
    }

    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        _fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        // -------- latent input --------
        let latent = inputs
            .get("latent")
            .ok_or_else(|| NodeError::MissingInput {
                port: "latent".into(),
            })?;
        let (latent_arch, latent_tensor) = match latent {
            NodeValue::Latent { arch, tensor } => (*arch, tensor.clone()),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "latent".into(),
                    expected: NodeValueType::Latent,
                    got: other.value_type(),
                })
            }
        };

        // -------- vae input --------
        let vae_value = inputs.get("vae").ok_or_else(|| NodeError::MissingInput {
            port: "vae".into(),
        })?;
        let (vae_arch, vae_handle) = match vae_value {
            NodeValue::Vae { arch, handle } => (*arch, handle.clone()),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "vae".into(),
                    expected: NodeValueType::Vae,
                    got: other.value_type(),
                })
            }
        };

        // -------- arch checks (Klein only in wave 2) --------
        if latent_arch != ArchTag::Klein {
            return Err(NodeError::ArchMismatch {
                port: "latent".into(),
                expected: ArchTag::Klein,
                got: latent_arch,
            });
        }
        if vae_arch != ArchTag::Klein {
            return Err(NodeError::ArchMismatch {
                port: "vae".into(),
                expected: ArchTag::Klein,
                got: vae_arch,
            });
        }

        // -------- downcast to the concrete Klein VAE bundle --------
        let vae = vae_handle.downcast_ref::<KleinVae>().ok_or_else(|| {
            NodeError::Other(
                "vae handle tagged Klein but did not downcast to handles::KleinVae".into(),
            )
        })?;

        // -------- forward pass --------
        // KleinVaeDecoder::decode takes [B, 128, H, W] packed Klein latents
        // and returns [B, 3, H*16, W*16] BF16 in roughly [-1, 1]. See
        // klein_lora_infer.rs:366 for the canonical call site.
        let rgb = vae.decoder.decode(&latent_tensor)?;

        let mut out = HashMap::new();
        out.insert("image".into(), NodeValue::Image(rgb));
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_shape() {
        let node = VaeDecode;
        let schema = node.schema();
        assert_eq!(node.type_id(), "core/vae_decode");
        assert_eq!(schema.display_name, "VAE Decode");
        assert_eq!(schema.category, "VAE");

        // 2 inputs: latent, vae — in that order so the UI lays them out
        // top-to-bottom matching the spec.
        assert_eq!(schema.inputs.len(), 2);
        assert_eq!(schema.inputs[0].name, "latent");
        assert_eq!(schema.inputs[0].value_type, NodeValueType::Latent);
        assert_eq!(schema.inputs[1].name, "vae");
        assert_eq!(schema.inputs[1].value_type, NodeValueType::Vae);

        // 1 output: image
        assert_eq!(schema.outputs.len(), 1);
        assert_eq!(schema.outputs[0].name, "image");
        assert_eq!(schema.outputs[0].value_type, NodeValueType::Image);

        // No fields — VAE decode is a pure forward pass.
        assert_eq!(schema.fields.len(), 0);
    }
}

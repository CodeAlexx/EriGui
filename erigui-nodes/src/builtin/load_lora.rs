//! `core/load_lora` — wave 2 implementation (Klein only).
//!
//! Carves the LoRA-attach stage out of
//! `inference-flame/src/bin/klein_lora_infer.rs` (around the
//! `KleinTransformer::set_lora` call) into a `NodeType::execute()`.
//!
//! # Assumes the wave-2 `lib.rs` refactor has landed
//!
//! Per `docs/nodes_spec.md` (decisions §2 and §6) and
//! `docs/wave2_seed_spec.md` ("core/load_lora"), this body is written
//! against the wave-2 `NodeValue` shape:
//!
//! ```ignore
//! NodeValue::Model { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> }
//! NodeValue::Clip  { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> }
//! ```
//!
//! and the wave-2 `NodeError` variants `ArchMismatch` /
//! `MissingField` / `Inference(_)` /  `FieldKindMismatch`. B1 owns the
//! lib.rs refactor (see seed coordination table). Until B1 lands,
//! `cargo check -p erigui-nodes` will fail at this file. The body
//! itself is internally consistent against the wave-2 shape.
//!
//! # Why we deep-clone the model
//!
//! `Arc<KleinTransformer>` is shared with any downstream branch that
//! also reads the un-LoRA'd model. We can't `Arc::get_mut` it without
//! breaking those branches, and we can't mutate through `&Arc<_>`. So
//! we `(*arc).clone()`, call `set_lora` on the clone, and return a
//! fresh `Arc`. `KleinTransformer: Clone` is cheap — `Tensor` storage
//! is Arc-shared, the `HashMap` spine is the only duplicated allocation.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use inference_flame::lora::LoraStack;
use inference_flame::models::klein::KleinTransformer;

use crate::{
    ArchTag, FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType, NodeValue,
    NodeValueType, PortSpec, ProgressSink,
};

pub struct LoadLora;

impl NodeType for LoadLora {
    fn type_id(&self) -> &'static str {
        "core/load_lora"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "Load LoRA",
            category: "Loaders",
            inputs: vec![
                PortSpec {
                    name: "model".into(),
                    value_type: NodeValueType::Model,
                },
                PortSpec {
                    name: "clip".into(),
                    value_type: NodeValueType::Clip,
                },
            ],
            outputs: vec![
                PortSpec {
                    name: "model".into(),
                    value_type: NodeValueType::Model,
                },
                PortSpec {
                    name: "clip".into(),
                    value_type: NodeValueType::Clip,
                },
            ],
            fields: vec![
                FieldSpec {
                    name: "path".into(),
                    label: "LoRA file".into(),
                    kind: FieldKind::FilePath {
                        extensions: vec!["safetensors".into()],
                    },
                    default: FieldValue::FilePath(PathBuf::new()),
                },
                FieldSpec {
                    name: "strength".into(),
                    label: "Strength".into(),
                    kind: FieldKind::Number {
                        min: 0.0,
                        max: 2.0,
                        step: 0.05,
                    },
                    default: FieldValue::Number(1.0),
                },
            ],
        }
    }

    fn execute(
        &self,
        inputs: &std::collections::HashMap<String, NodeValue>,
        fields: &std::collections::HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<std::collections::HashMap<String, NodeValue>, NodeError> {
        // ── inputs ──────────────────────────────────────────────────
        let model_in = inputs
            .get("model")
            .ok_or_else(|| NodeError::MissingInput { port: "model".into() })?;
        let clip_in = inputs
            .get("clip")
            .ok_or_else(|| NodeError::MissingInput { port: "clip".into() })?;

        let (model_arch, model_handle) = match model_in {
            NodeValue::Model { arch, handle } => (*arch, handle.clone()),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "model".into(),
                    expected: NodeValueType::Model,
                    got: other.value_type(),
                });
            }
        };
        if !matches!(model_arch, ArchTag::Klein) {
            return Err(NodeError::ArchMismatch {
                port: "model".into(),
                expected: ArchTag::Klein,
                got: model_arch,
            });
        }

        // CLIP arch check — same restriction (Klein-only wave 2).
        if let NodeValue::Clip { arch, .. } = clip_in {
            if !matches!(arch, ArchTag::Klein) {
                return Err(NodeError::ArchMismatch {
                    port: "clip".into(),
                    expected: ArchTag::Klein,
                    got: *arch,
                });
            }
        } else {
            return Err(NodeError::TypeMismatch {
                port: "clip".into(),
                expected: NodeValueType::Clip,
                got: clip_in.value_type(),
            });
        }

        // ── fields ──────────────────────────────────────────────────
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
        let strength: f32 = match fields
            .get("strength")
            .ok_or_else(|| NodeError::MissingField { name: "strength".into() })?
        {
            FieldValue::Number(v) => *v,
            _ => {
                return Err(NodeError::FieldKindMismatch {
                    name: "strength".into(),
                    expected: "Number",
                });
            }
        };

        let path_str = path.to_str().ok_or_else(|| {
            NodeError::Other(format!("LoRA path is not valid UTF-8: {}", path.display()))
        })?;

        // ── downcast model handle to KleinTransformer ───────────────
        let dit_arc: Arc<KleinTransformer> = model_handle
            .clone()
            .downcast::<KleinTransformer>()
            .map_err(|_| {
                NodeError::Other(
                    "Klein arch tag but handle is not KleinTransformer".into(),
                )
            })?;

        // ── load LoRA file ──────────────────────────────────────────
        let device = flame_core::global_cuda_device();
        let base_keys: HashSet<String> = dit_arc.weights().keys().cloned().collect();

        // `LoraStack::load` already folds the `multiplier` (== runtime
        // strength) into each entry's per-module scale, which is exactly
        // the seed's "multiply through per-module strengths before
        // construction" requirement. No second pass needed.
        let stack = LoraStack::load(path_str, &base_keys, strength, &device)
            .map_err(|e| NodeError::Inference(e))?;

        // CLIP-targeted LoRA branches (lora_te1_/lora_te2_ in kohya
        // SDXL format) are skipped by `LoraStack::load` itself with an
        // eprintln. Klein-trainer / ai-toolkit / ZImage formats don't
        // ship CLIP weights at all. Per the seed: pass clip through
        // unchanged for wave 2.

        // ── deep-clone the model and attach LoRA ────────────────────
        // Approach (b) from the seed: Arc::make_mut promotes an
        // exclusive-refcount Arc in place (zero-copy), and falls back
        // to clone-on-write when the Arc is shared with another
        // branch. Either way the upstream Arc is left untouched.
        let mut dit_owned: KleinTransformer = match Arc::try_unwrap(dit_arc) {
            Ok(t) => t,
            Err(arc) => (*arc).clone(),
        };
        dit_owned.set_lora(Arc::new(stack));

        // ── outputs ─────────────────────────────────────────────────
        let mut out = std::collections::HashMap::new();
        out.insert(
            "model".into(),
            NodeValue::Model {
                arch: ArchTag::Klein,
                handle: Arc::new(dit_owned) as Arc<dyn std::any::Any + Send + Sync>,
            },
        );
        out.insert("clip".into(), clip_in.clone());
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
        let n = LoadLora;
        assert_eq!(n.type_id(), "core/load_lora");
        let s = n.schema();
        assert_eq!(s.display_name, "Load LoRA");
        assert_eq!(s.category, "Loaders");

        // Inputs: model, clip.
        assert_eq!(s.inputs.len(), 2);
        assert_eq!(s.inputs[0].name, "model");
        assert!(matches!(s.inputs[0].value_type, NodeValueType::Model));
        assert_eq!(s.inputs[1].name, "clip");
        assert!(matches!(s.inputs[1].value_type, NodeValueType::Clip));

        // Outputs: model, clip.
        assert_eq!(s.outputs.len(), 2);
        assert_eq!(s.outputs[0].name, "model");
        assert!(matches!(s.outputs[0].value_type, NodeValueType::Model));
        assert_eq!(s.outputs[1].name, "clip");
        assert!(matches!(s.outputs[1].value_type, NodeValueType::Clip));

        // Fields: path (FilePath), strength (Number 0..2 step 0.05).
        assert_eq!(s.fields.len(), 2);

        let path = &s.fields[0];
        assert_eq!(path.name, "path");
        assert!(matches!(path.kind, FieldKind::FilePath { .. }));
        assert!(matches!(path.default, FieldValue::FilePath(_)));

        let strength = &s.fields[1];
        assert_eq!(strength.name, "strength");
        match &strength.kind {
            FieldKind::Number { min, max, step } => {
                assert_eq!(*min, 0.0);
                assert_eq!(*max, 2.0);
                assert!((*step - 0.05).abs() < 1e-6);
            }
            _ => panic!("strength field kind should be Number"),
        }
        assert!(matches!(strength.default, FieldValue::Number(v) if (v - 1.0).abs() < 1e-6));
    }
}

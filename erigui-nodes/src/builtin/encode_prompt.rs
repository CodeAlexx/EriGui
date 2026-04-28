//! `core/encode_prompt` — Klein text encoder node (wave-2, B3).
//!
//! Maps `clip: Clip` + (`text`, `negative`) fields → `conditioning: Conditioning`.
//! Klein-only for wave 2; non-Klein arches return `NodeError::ArchMismatch`.
//!
//! The pipeline is carved out of
//! `inference-flame/src/bin/klein_lora_infer.rs:184-241`:
//!
//! 1. Wrap `text` with the Klein chat template (`<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n`).
//! 2. Tokenize with the encoder's `tokenizers::Tokenizer`.
//! 3. Right-pad token ids to `KLEIN_TXT_PAD_LEN = 512` with `KLEIN_PAD_ID`.
//! 4. Build an attention mask: `1.0` for the real-token prefix, `0.0` for pad.
//! 5. `Qwen3Encoder::encode(&token_ids)` → BF16 tensor.
//! 6. Pack into `ConditioningPack { embeds, pooled: None, mask: Some(mask),
//!    arch: Klein }` and return `NodeValue::Conditioning(Arc::new(pack))`.
//!
//! `negative` field is a UI tag. Same encoder, same template, same call path.
//! The K-sampler picks which conditioning is "negative" via the `positive`
//! vs `negative` port name. Wave 2 stores the flag without behavior change;
//! future waves may use it for default-negative-prompt insertion when `text`
//! is empty.

use std::collections::HashMap;
use std::sync::Arc;

use flame_core::{Shape, Tensor};

use crate::handles::{
    Qwen3EncoderHandle, KLEIN_PAD_ID, KLEIN_TEMPLATE_POST, KLEIN_TEMPLATE_PRE, KLEIN_TXT_PAD_LEN,
};
use crate::values::ConditioningPack;
use crate::{
    ArchTag, FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType, NodeValue,
    NodeValueType, PortSpec, ProgressSink,
};

pub struct EncodePrompt;

impl NodeType for EncodePrompt {
    fn type_id(&self) -> &'static str {
        "core/encode_prompt"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "Encode Prompt",
            category: "Conditioning",
            inputs: vec![PortSpec {
                name: "clip".into(),
                value_type: NodeValueType::Clip,
            }],
            outputs: vec![PortSpec {
                name: "conditioning".into(),
                value_type: NodeValueType::Conditioning,
            }],
            fields: vec![
                FieldSpec {
                    name: "text".into(),
                    label: "Prompt".into(),
                    kind: FieldKind::Text,
                    default: FieldValue::Text(String::new()),
                },
                FieldSpec {
                    name: "negative".into(),
                    label: "Negative".into(),
                    kind: FieldKind::Bool,
                    default: FieldValue::Bool(false),
                },
            ],
        }
    }

    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        // -------- 1. Validate `clip` input. --------
        let clip = inputs
            .get("clip")
            .ok_or_else(|| NodeError::MissingInput {
                port: "clip".into(),
            })?;
        let (clip_arch, clip_handle) = match clip {
            NodeValue::Clip { arch, handle } => (*arch, handle.clone()),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "clip".into(),
                    expected: NodeValueType::Clip,
                    got: other.value_type(),
                });
            }
        };
        if clip_arch != ArchTag::Klein {
            return Err(NodeError::ArchMismatch {
                port: "clip".into(),
                expected: ArchTag::Klein,
                got: clip_arch,
            });
        }
        let handle: Arc<Qwen3EncoderHandle> =
            clip_handle
                .downcast::<Qwen3EncoderHandle>()
                .map_err(|_| NodeError::Other(
                    "clip handle was tagged Klein but did not downcast to Qwen3EncoderHandle".into(),
                ))?;

        // -------- 2. Read fields. --------
        let text = match fields.get("text") {
            Some(FieldValue::Text(s)) => s.clone(),
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "text".into(),
                    expected: "Text",
                });
            }
            None => {
                return Err(NodeError::MissingField {
                    name: "text".into(),
                });
            }
        };
        // Read but don't use yet (see module doc on `negative`).
        let _negative = match fields.get("negative") {
            Some(FieldValue::Bool(b)) => *b,
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "negative".into(),
                    expected: "Bool",
                });
            }
            None => {
                return Err(NodeError::MissingField {
                    name: "negative".into(),
                });
            }
        };

        // -------- 3. Klein chat-template wrap (verbatim from klein_lora_infer.rs:191). --------
        let formatted = format!("{KLEIN_TEMPLATE_PRE}{text}{KLEIN_TEMPLATE_POST}");

        // -------- 4. Tokenize. --------
        let enc = handle
            .tokenizer
            .encode(formatted.as_str(), false)
            .map_err(|e| NodeError::Other(format!("tokenize failed: {e}")))?;
        let mut ids: Vec<i32> = enc.get_ids().iter().map(|&i| i as i32).collect();
        let real_len = ids.len().min(KLEIN_TXT_PAD_LEN);
        // Right-pad to KLEIN_TXT_PAD_LEN with PAD_ID; if longer, Vec::resize
        // truncates. Matches klein_lora_infer.rs:204-205 semantics.
        ids.resize(KLEIN_TXT_PAD_LEN, KLEIN_PAD_ID);

        // -------- 5. Encode. --------
        let embeds = handle.encoder.encode(&ids)?;

        // -------- 6. Build attention mask: [1, KLEIN_TXT_PAD_LEN] BF16, 1.0 for
        //          real tokens, 0.0 for pad. The encoder does not return a mask;
        //          klein_lora_infer.rs builds one downstream. We hand the
        //          k-sampler a ready-made mask so it doesn't have to re-derive.
        let device = flame_core::global_cuda_device();
        let mut mask_data = vec![0.0f32; KLEIN_TXT_PAD_LEN];
        for slot in mask_data.iter_mut().take(real_len) {
            *slot = 1.0;
        }
        let mask = Tensor::from_f32_to_bf16(
            mask_data,
            Shape::from_dims(&[1, KLEIN_TXT_PAD_LEN]),
            device,
        )?;

        let pack = ConditioningPack {
            embeds,
            pooled: None,
            mask: Some(mask),
            arch: ArchTag::Klein,
        };

        let mut out = HashMap::new();
        out.insert(
            "conditioning".into(),
            NodeValue::Conditioning(Arc::new(pack)),
        );
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Schema-validation only: ports, fields, names, kinds. No GPU, no exec.
    #[test]
    fn schema_matches_wave2_seed_spec() {
        let n = EncodePrompt;
        assert_eq!(n.type_id(), "core/encode_prompt");

        let s = n.schema();
        assert_eq!(s.display_name, "Encode Prompt");
        assert_eq!(s.category, "Conditioning");

        // Single input: clip
        assert_eq!(s.inputs.len(), 1);
        assert_eq!(s.inputs[0].name, "clip");
        assert_eq!(s.inputs[0].value_type, NodeValueType::Clip);

        // Single output: conditioning
        assert_eq!(s.outputs.len(), 1);
        assert_eq!(s.outputs[0].name, "conditioning");
        assert_eq!(s.outputs[0].value_type, NodeValueType::Conditioning);

        // Two fields: text (Text), negative (Bool)
        assert_eq!(s.fields.len(), 2);

        let text = s
            .fields
            .iter()
            .find(|f| f.name == "text")
            .expect("text field");
        assert!(matches!(text.kind, FieldKind::Text));
        assert!(matches!(text.default, FieldValue::Text(_)));

        let neg = s
            .fields
            .iter()
            .find(|f| f.name == "negative")
            .expect("negative field");
        assert!(matches!(neg.kind, FieldKind::Bool));
        assert!(matches!(neg.default, FieldValue::Bool(false)));
    }

    /// Validation paths that don't require a real GPU/encoder.
    #[test]
    fn execute_reports_missing_clip_input() {
        let n = EncodePrompt;
        let inputs: HashMap<String, NodeValue> = HashMap::new();
        let mut fields: HashMap<String, FieldValue> = HashMap::new();
        fields.insert("text".into(), FieldValue::Text("hello".into()));
        fields.insert("negative".into(), FieldValue::Bool(false));

        let err = n.execute(&inputs, &fields, None).unwrap_err();
        match err {
            NodeError::MissingInput { port } => assert_eq!(port, "clip"),
            other => panic!("expected MissingInput, got {other:?}"),
        }
    }

    #[test]
    fn execute_reports_type_mismatch_on_clip() {
        let n = EncodePrompt;
        let mut inputs: HashMap<String, NodeValue> = HashMap::new();
        inputs.insert("clip".into(), NodeValue::Number(1.0));
        let mut fields: HashMap<String, FieldValue> = HashMap::new();
        fields.insert("text".into(), FieldValue::Text("hello".into()));
        fields.insert("negative".into(), FieldValue::Bool(false));

        let err = n.execute(&inputs, &fields, None).unwrap_err();
        match err {
            NodeError::TypeMismatch {
                port,
                expected,
                got,
            } => {
                assert_eq!(port, "clip");
                assert_eq!(expected, NodeValueType::Clip);
                assert_eq!(got, NodeValueType::Number);
            }
            other => panic!("expected TypeMismatch, got {other:?}"),
        }
    }

    /// Non-Klein arch on the clip port must surface `ArchMismatch` before any
    /// downcast or GPU call.
    #[test]
    fn execute_rejects_non_klein_arch() {
        let n = EncodePrompt;
        // Bare unit type as the handle — never inspected because the arch
        // check fires first.
        let dummy: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());
        let mut inputs: HashMap<String, NodeValue> = HashMap::new();
        inputs.insert(
            "clip".into(),
            NodeValue::Clip {
                arch: ArchTag::ZImage,
                handle: dummy,
            },
        );
        let mut fields: HashMap<String, FieldValue> = HashMap::new();
        fields.insert("text".into(), FieldValue::Text("hello".into()));
        fields.insert("negative".into(), FieldValue::Bool(false));

        let err = n.execute(&inputs, &fields, None).unwrap_err();
        match err {
            NodeError::ArchMismatch {
                port,
                expected,
                got,
            } => {
                assert_eq!(port, "clip");
                assert_eq!(expected, ArchTag::Klein);
                assert_eq!(got, ArchTag::ZImage);
            }
            other => panic!("expected ArchMismatch, got {other:?}"),
        }
    }
}

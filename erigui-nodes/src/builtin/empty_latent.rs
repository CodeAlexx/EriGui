//! `core/empty_latent` — Klein empty initial-noise latent generator.
//!
//! Wave 2 starter chain (LoadCheckpoint → EncodePrompt → KSampler → ...) was
//! missing the source for `KSampler`'s `latent` input port. This node fills
//! that gap: zero-tensor of Klein's packed latent shape `[B, 128, H/16, W/16]`
//! at the user's requested image size. KSampler's body overwrites the values
//! with `box_muller_noise(numel, seed)` — only the SHAPE matters here.
//!
//! Klein only in wave 2.

use std::collections::HashMap;

use flame_core::{global_cuda_device, DType, Shape, Tensor};

use crate::{
    ArchTag, FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType, NodeValue,
    NodeValueType, PortSpec, ProgressSink,
};

pub struct EmptyLatent;

impl NodeType for EmptyLatent {
    fn type_id(&self) -> &'static str {
        "core/empty_latent"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "Empty Latent",
            category: "Latent",
            inputs: vec![],
            outputs: vec![PortSpec {
                name: "latent".into(),
                value_type: NodeValueType::Latent,
            }],
            fields: vec![
                FieldSpec {
                    name: "width".into(),
                    label: "Width".into(),
                    kind: FieldKind::Number {
                        min: 64.0,
                        max: 2048.0,
                        step: 8.0,
                    },
                    default: FieldValue::Number(512.0),
                },
                FieldSpec {
                    name: "height".into(),
                    label: "Height".into(),
                    kind: FieldKind::Number {
                        min: 64.0,
                        max: 2048.0,
                        step: 8.0,
                    },
                    default: FieldValue::Number(512.0),
                },
                FieldSpec {
                    name: "batch_size".into(),
                    label: "Batch".into(),
                    kind: FieldKind::Number {
                        min: 1.0,
                        max: 4.0,
                        step: 1.0,
                    },
                    default: FieldValue::Number(1.0),
                },
            ],
        }
    }

    fn execute(
        &self,
        _inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        let width = read_dim(fields, "width", 512)?;
        let height = read_dim(fields, "height", 512)?;
        let batch = read_dim(fields, "batch_size", 1)?.max(1);

        // Klein packs to /16 spatial, 128 channels.
        if width % 16 != 0 || height % 16 != 0 {
            return Err(NodeError::Other(format!(
                "empty_latent: width and height must be multiples of 16 (got {width}x{height})"
            )));
        }
        let lat_h = height / 16;
        let lat_w = width / 16;

        let device = global_cuda_device();
        let tensor = Tensor::zeros_dtype(
            Shape::from_dims(&[batch as usize, 128, lat_h as usize, lat_w as usize]),
            DType::BF16,
            device.clone(),
        )?;

        let mut out = HashMap::new();
        out.insert(
            "latent".into(),
            NodeValue::Latent {
                arch: ArchTag::Klein,
                tensor,
            },
        );
        Ok(out)
    }
}

fn read_dim(fields: &HashMap<String, FieldValue>, name: &str, default: u32) -> Result<u32, NodeError> {
    match fields.get(name) {
        Some(FieldValue::Number(n)) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(NodeError::FieldKindMismatch {
                    name: name.into(),
                    expected: "non-negative finite number",
                });
            }
            Ok(*n as u32)
        }
        Some(_) => Err(NodeError::FieldKindMismatch {
            name: name.into(),
            expected: "Number",
        }),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_shape() {
        let node = EmptyLatent;
        let schema = node.schema();
        assert_eq!(node.type_id(), "core/empty_latent");
        assert_eq!(schema.display_name, "Empty Latent");
        assert_eq!(schema.category, "Latent");
        assert_eq!(schema.inputs.len(), 0);
        assert_eq!(schema.outputs.len(), 1);
        assert_eq!(schema.outputs[0].name, "latent");
        assert_eq!(schema.outputs[0].value_type, NodeValueType::Latent);
        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.fields[0].name, "width");
        assert_eq!(schema.fields[1].name, "height");
        assert_eq!(schema.fields[2].name, "batch_size");
    }
}

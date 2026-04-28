//! `core/save_image` — write a tensor image to a timestamped PNG.
//!
//! Wave-2 spec: `docs/wave2_seed_spec.md` §"core/save_image".
//!
//! Arch-agnostic: this node does not care which architecture produced the
//! image, only that it's a `[B, 3, H, W]` tensor whose values land in the
//! `[-1, 1]` range produced by every flame-diffusion VAE decoder. The
//! reference for the tensor → PNG conversion is the `klein_lora_infer`
//! binary: `inference-flame/src/bin/klein_lora_infer.rs:372-391`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use flame_core::DType;

use crate::{
    FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType, NodeValue, NodeValueType,
    PortSpec, ProgressSink,
};

/// Process-lifetime counter that disambiguates same-second filenames.
/// Two saves in the same second get distinct counter suffixes, so we
/// never silently clobber a prior PNG. Reset to 0 at process start.
static SAVE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Output directory for saved PNGs. Resolved relative to the process cwd
/// at save time — wave 3 will make this configurable.
const OUTPUT_DIR: &str = "./output";

pub struct SaveImage;

impl NodeType for SaveImage {
    fn type_id(&self) -> &'static str {
        "core/save_image"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "Save Image",
            category: "Image",
            inputs: vec![PortSpec {
                name: "image".into(),
                value_type: NodeValueType::Image,
            }],
            outputs: vec![],
            fields: vec![FieldSpec {
                name: "filename_prefix".into(),
                label: "Filename prefix".into(),
                kind: FieldKind::Text,
                default: FieldValue::Text("EriGui".into()),
            }],
        }
    }

    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        _progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        // -------------------------------------------------------------
        // 1. Pull the image tensor from inputs.
        // -------------------------------------------------------------
        let tensor = match inputs.get("image") {
            Some(NodeValue::Image(t)) => t,
            Some(other) => {
                return Err(NodeError::TypeMismatch {
                    port: "image".into(),
                    expected: NodeValueType::Image,
                    got: other.value_type(),
                });
            }
            None => {
                return Err(NodeError::MissingInput {
                    port: "image".into(),
                });
            }
        };

        // -------------------------------------------------------------
        // 2. Pull the filename prefix from fields.
        // -------------------------------------------------------------
        let prefix = match fields.get("filename_prefix") {
            Some(FieldValue::Text(s)) if !s.is_empty() => s.clone(),
            // Default mirrors the schema default. Both "field absent" and
            // "field present but empty" fall through to the default — UI
            // shouldn't allow empty, but be defensive.
            Some(FieldValue::Text(_)) | None => "EriGui".to_string(),
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "filename_prefix".into(),
                    expected: "Text",
                });
            }
        };

        // -------------------------------------------------------------
        // 3. Validate shape: [B, 3, H, W]. Only B==1 supported in wave 2.
        // -------------------------------------------------------------
        let dims = tensor.dims();
        if dims.len() != 4 {
            return Err(NodeError::Other(format!(
                "save_image: expected 4-D image tensor [B, 3, H, W], got {dims:?}"
            )));
        }
        let (b, c, h, w) = (dims[0], dims[1], dims[2], dims[3]);
        if c != 3 {
            return Err(NodeError::Other(format!(
                "save_image: expected 3 channels, got {c}"
            )));
        }
        if b != 1 {
            // Wave 2 has no convention for batch filenames — defer to wave 3.
            return Err(NodeError::Other(format!(
                "save_image: batch size {b} not supported in wave 2; expected 1"
            )));
        }

        // -------------------------------------------------------------
        // 4. Tensor → CPU f32. Mirrors klein_lora_infer.rs:372-385:
        //      let rgb_f32 = rgb.to_dtype(DType::F32)?;
        //      let data    = rgb_f32.to_vec()?;
        //      val = (127.5 * (data[c*H*W + y*W + x].clamp(-1, 1) + 1)) as u8
        //
        // VAE decoders in flame-diffusion produce [-1, 1]; the (x+1)*127.5
        // remap is the standard SD/SDXL/Klein convention.
        // -------------------------------------------------------------
        let rgb_f32 = tensor.to_dtype(DType::F32)?;
        let data = rgb_f32.to_vec()?;
        let mut pixels = vec![0u8; h * w * 3];
        for y in 0..h {
            for x in 0..w {
                for ch in 0..3 {
                    let src = ch * h * w + y * w + x;
                    let dst = (y * w + x) * 3 + ch;
                    pixels[dst] = (127.5 * (data[src].clamp(-1.0, 1.0) + 1.0)) as u8;
                }
            }
        }

        // -------------------------------------------------------------
        // 5. Build filename: {prefix}_{YYYYMMDD_HHMMSS}_{counter}.png
        // -------------------------------------------------------------
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let counter = SAVE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let filename = format!("{prefix}_{timestamp}_{counter:04}.png");

        // -------------------------------------------------------------
        // 6. Ensure output dir, write PNG.
        // -------------------------------------------------------------
        let dir = PathBuf::from(OUTPUT_DIR);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(&filename);

        let buf: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(w as u32, h as u32, pixels).ok_or_else(|| {
                NodeError::Other(
                    "save_image: pixel buffer length doesn't match width*height*3".into(),
                )
            })?;
        buf.save(&path)
            .map_err(|e| NodeError::Other(format!("save_image: PNG write failed: {e}")))?;

        // No outputs — this is a sink node.
        Ok(HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldValue, NodeType};

    #[test]
    fn schema_has_image_input_and_prefix_field() {
        let n = SaveImage;
        let schema = n.schema();
        assert_eq!(n.type_id(), "core/save_image");
        assert_eq!(schema.category, "Image");
        assert_eq!(schema.inputs.len(), 1);
        assert_eq!(schema.inputs[0].name, "image");
        assert_eq!(schema.inputs[0].value_type, NodeValueType::Image);
        assert!(schema.outputs.is_empty());
        assert_eq!(schema.fields.len(), 1);
        assert_eq!(schema.fields[0].name, "filename_prefix");
        match &schema.fields[0].default {
            FieldValue::Text(s) => assert_eq!(s, "EriGui"),
            other => panic!("expected Text default, got {other:?}"),
        }
    }

    // The PNG round-trip test that exercises real `flame_core::Tensor` and
    // a CUDA device lives in `tests/save_image_integration.rs` (a
    // GPU-required integration test). Keeping it out of `--lib` honors the
    // wave-2 contract: schema/validation tests must not need a GPU.
}

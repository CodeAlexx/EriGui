//! `core/k_sampler` — Klein Euler ODE sampler with CFG.
//!
//! Implements wave-2 sampling for the Klein architecture. Mirrors
//! `inference-flame/src/bin/klein_lora_infer.rs:247-345` stage-for-stage.
//! Other architectures error with `NodeError::Other`.
//!
//! # Latent shape contract
//!
//! Both input and output `latent` use the spatial form
//! `[B, 128, latent_h, latent_w]` — same layout the upstream
//! `EmptyLatent`/noise node produces in the verified Klein binary
//! (line 308) and the same form `vae_decode` consumes. The sampler
//! permutes+reshapes to token form `[B, latent_h*latent_w, 128]`
//! internally before calling `KleinTransformer::forward`, then converts
//! the denoised token tensor back to spatial form before returning
//! (mirrors klein_lora_infer.rs:340-342). This keeps the canonical
//! `NodeValue::Latent` shape architecture-agnostic and stops downstream
//! nodes from needing arch-aware shape detection — KSampler owns the
//! Klein-specific token bookkeeping because the H/W decomposition needed
//! for `img_ids` only survives if it does.
//!
//! # `shift` field
//!
//! The schema exposes `shift` for forward-compat. Wave 2 ignores it and
//! always uses `klein_sampling::get_schedule(steps, image_seq_len)` —
//! that's what the verified binary uses (empirical-mu schedule). A
//! future wave with a "use legacy ComfyUI shift" toggle can route to
//! `build_sigma_schedule(steps, shift)` instead.
//!
//! # Seed handling
//!
//! Schema declares `seed` as `FieldKind::Number` (the widget enum has no
//! `Seed` variant yet). We cast `n as u64` and warn if the value is out
//! of `[0, u32::MAX]` — UI clamps to that range, but be defensive.
//!
//! # Reference
//!
//! - `KleinTransformer::forward(img, txt, timesteps, img_ids, txt_ids)` —
//!   `inference-flame/src/models/klein.rs:728`.
//! - Schedule: `klein_sampling::get_schedule(steps, image_seq_len)`.
//! - Box-Muller: `klein_sampling::box_muller_noise(numel, seed)` (moved
//!   to `inference-flame` from the bin in fix #2).

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use flame_core::{global_cuda_device, DType, Shape, Tensor};
use inference_flame::models::klein::KleinTransformer;
use inference_flame::sampling::klein_sampling::{box_muller_noise, euler_denoise, get_schedule};

use crate::{
    ArchTag, ConditioningPack, FieldKind, FieldSpec, FieldValue, NodeError, NodeSchema, NodeType,
    NodeValue, NodeValueType, PortSpec, ProgressSink,
};

pub struct KSampler;

impl NodeType for KSampler {
    fn type_id(&self) -> &'static str {
        "core/k_sampler"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            display_name: "K Sampler",
            category: "Sampling",
            inputs: vec![
                PortSpec {
                    name: "model".into(),
                    value_type: NodeValueType::Model,
                },
                PortSpec {
                    name: "positive".into(),
                    value_type: NodeValueType::Conditioning,
                },
                PortSpec {
                    name: "negative".into(),
                    value_type: NodeValueType::Conditioning,
                },
                PortSpec {
                    name: "latent".into(),
                    value_type: NodeValueType::Latent,
                },
            ],
            outputs: vec![PortSpec {
                name: "latent".into(),
                value_type: NodeValueType::Latent,
            }],
            fields: vec![
                FieldSpec {
                    name: "seed".into(),
                    label: "Seed".into(),
                    // Seed is rendered as a number until a dedicated u64 field
                    // kind exists in erigui-widgets.
                    kind: FieldKind::Number {
                        min: 0.0,
                        max: u32::MAX as f32,
                        step: 1.0,
                    },
                    default: FieldValue::Number(42.0),
                },
                FieldSpec {
                    name: "steps".into(),
                    label: "Steps".into(),
                    kind: FieldKind::Number {
                        min: 1.0,
                        max: 200.0,
                        step: 1.0,
                    },
                    default: FieldValue::Number(30.0),
                },
                FieldSpec {
                    name: "cfg".into(),
                    label: "CFG".into(),
                    kind: FieldKind::Number {
                        min: 0.0,
                        max: 30.0,
                        step: 0.1,
                    },
                    default: FieldValue::Number(7.0),
                },
                FieldSpec {
                    name: "shift".into(),
                    label: "Shift".into(),
                    kind: FieldKind::Number {
                        min: 0.0,
                        max: 10.0,
                        step: 0.1,
                    },
                    default: FieldValue::Number(3.0),
                },
            ],
        }
    }

    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError> {
        // -------------------------------------------------------------
        // 1. Pull inputs.
        // -------------------------------------------------------------
        let model_v = inputs
            .get("model")
            .ok_or_else(|| NodeError::MissingInput { port: "model".into() })?;
        let pos_v = inputs
            .get("positive")
            .ok_or_else(|| NodeError::MissingInput { port: "positive".into() })?;
        let neg_v = inputs
            .get("negative")
            .ok_or_else(|| NodeError::MissingInput { port: "negative".into() })?;
        let lat_v = inputs
            .get("latent")
            .ok_or_else(|| NodeError::MissingInput { port: "latent".into() })?;

        // -------------------------------------------------------------
        // 2. Variant + arch checks. All four must be Klein.
        // -------------------------------------------------------------
        let (model_arch, model_handle) = match model_v {
            NodeValue::Model { arch, handle } => (*arch, handle),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "model".into(),
                    expected: NodeValueType::Model,
                    got: other.value_type(),
                });
            }
        };
        let pos: &ConditioningPack = match pos_v {
            NodeValue::Conditioning(p) => p.as_ref(),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "positive".into(),
                    expected: NodeValueType::Conditioning,
                    got: other.value_type(),
                });
            }
        };
        let neg: &ConditioningPack = match neg_v {
            NodeValue::Conditioning(p) => p.as_ref(),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "negative".into(),
                    expected: NodeValueType::Conditioning,
                    got: other.value_type(),
                });
            }
        };
        let (latent_arch, latent_in) = match lat_v {
            NodeValue::Latent { arch, tensor } => (*arch, tensor),
            other => {
                return Err(NodeError::TypeMismatch {
                    port: "latent".into(),
                    expected: NodeValueType::Latent,
                    got: other.value_type(),
                });
            }
        };
        for (port, got) in [
            ("model", model_arch),
            ("positive", pos.arch),
            ("negative", neg.arch),
            ("latent", latent_arch),
        ] {
            if got != ArchTag::Klein {
                return Err(NodeError::ArchMismatch {
                    port: port.into(),
                    expected: ArchTag::Klein,
                    got,
                });
            }
        }

        // -------------------------------------------------------------
        // 3. Downcast model handle to KleinTransformer.
        // -------------------------------------------------------------
        let model: &KleinTransformer = model_handle
            .downcast_ref::<KleinTransformer>()
            .ok_or_else(|| {
                NodeError::Other(
                    "k_sampler: model handle is Klein-tagged but not KleinTransformer".into(),
                )
            })?;

        // -------------------------------------------------------------
        // 4. Read fields. Defaults mirror the schema.
        // -------------------------------------------------------------
        let seed: u64 = match fields.get("seed") {
            Some(FieldValue::Number(n)) => {
                let n = *n;
                if !n.is_finite() || n < 0.0 || n > u32::MAX as f32 {
                    eprintln!(
                        "k_sampler: seed {n} outside [0, u32::MAX] — clamping. UI should prevent this."
                    );
                }
                n.max(0.0).min(u32::MAX as f32) as u64
            }
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "seed".into(),
                    expected: "Number",
                });
            }
            None => 42,
        };
        let steps: usize = match fields.get("steps") {
            Some(FieldValue::Number(n)) => (*n as usize).max(1),
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "steps".into(),
                    expected: "Number",
                });
            }
            None => 30,
        };
        let cfg: f32 = match fields.get("cfg") {
            Some(FieldValue::Number(n)) => *n,
            Some(_) => {
                return Err(NodeError::FieldKindMismatch {
                    name: "cfg".into(),
                    expected: "Number",
                });
            }
            None => 7.0,
        };
        // `shift` field is read for forward-compat but ignored: Klein uses
        // empirical-mu in `get_schedule`. See module doc-comment.
        let _ = fields.get("shift");

        // -------------------------------------------------------------
        // 5. Latent shape: spatial `[B, C, latent_h, latent_w]` (per
        //    klein_lora_infer.rs:308). C must be 128 for Klein.
        // -------------------------------------------------------------
        let dims = latent_in.dims();
        if dims.len() != 4 {
            return Err(NodeError::Other(format!(
                "k_sampler: expected 4-D latent [B, 128, h, w], got shape {dims:?}"
            )));
        }
        let (b, c, latent_h, latent_w) = (dims[0], dims[1], dims[2], dims[3]);
        if b != 1 {
            return Err(NodeError::Other(format!(
                "k_sampler: batch {b} not supported in wave 2; expected 1"
            )));
        }
        if c != 128 {
            return Err(NodeError::Other(format!(
                "k_sampler: Klein expects C=128 channels, got {c}"
            )));
        }
        let n_img = latent_h * latent_w;

        let device = global_cuda_device();

        // -------------------------------------------------------------
        // 6. Initial noise — Box-Muller seeded by `seed`. Mirrors
        //    klein_lora_infer.rs:303-313 verbatim. Build spatial then
        //    permute+reshape to token form `[B, n_img, 128]`.
        // -------------------------------------------------------------
        let numel = c * latent_h * latent_w;
        let noise_data = box_muller_noise(numel, seed);
        let noise_spatial = Tensor::from_f32_to_bf16(
            noise_data,
            Shape::from_dims(&[1, c, latent_h, latent_w]),
            device.clone(),
        )?;
        let noise = noise_spatial
            .permute(&[0, 2, 3, 1])?
            .reshape(&[1, n_img, c])?;

        // -------------------------------------------------------------
        // 7. img_ids / txt_ids. Mirrors klein_lora_infer.rs:247-261
        //    verbatim. txt_seq_len comes from the positive conditioning
        //    embed (the encoder pads to a fixed length).
        // -------------------------------------------------------------
        let pos_dims = pos.embeds.dims();
        if pos_dims.len() != 3 {
            return Err(NodeError::Other(format!(
                "k_sampler: positive embeds expected 3-D [B, T, D], got {pos_dims:?}"
            )));
        }
        let txt_seq_len = pos_dims[1];

        let mut img_data = vec![0.0f32; n_img * 4];
        for r in 0..latent_h {
            for col in 0..latent_w {
                let idx = r * latent_w + col;
                img_data[idx * 4 + 1] = r as f32;
                img_data[idx * 4 + 2] = col as f32;
            }
        }
        let img_ids = Tensor::from_f32_to_bf16(
            img_data,
            Shape::from_dims(&[n_img, 4]),
            device.clone(),
        )?;
        let txt_ids = Tensor::zeros_dtype(
            Shape::from_dims(&[txt_seq_len, 4]),
            DType::BF16,
            device.clone(),
        )?;

        // -------------------------------------------------------------
        // 8. Schedule — empirical-mu (Klein default). `shift` ignored.
        // -------------------------------------------------------------
        let timesteps = get_schedule(steps, n_img);

        // -------------------------------------------------------------
        // 9. Denoise loop. CFG: pred = pred_uncond + cfg*(pred_cond - pred_uncond).
        //    Mirrors klein_lora_infer.rs:330-344.
        // -------------------------------------------------------------
        let total = (timesteps.len().saturating_sub(1)) as u32;
        let step_idx = AtomicUsize::new(0);
        let denoised = euler_denoise(
            |x, t_curr| {
                let i = step_idx.fetch_add(1, Ordering::Relaxed) as u32;
                if let Some(s) = progress {
                    s.step(i, total, "klein euler");
                }
                let t_vec = Tensor::from_f32_to_bf16(
                    vec![t_curr],
                    Shape::from_dims(&[1]),
                    device.clone(),
                )?;
                let pred_cond = model.forward(x, &pos.embeds, &t_vec, &img_ids, &txt_ids)?;
                let pred_uncond = model.forward(x, &neg.embeds, &t_vec, &img_ids, &txt_ids)?;
                let diff = pred_cond.sub(&pred_uncond)?;
                pred_uncond.add(&diff.mul_scalar(cfg)?)
            },
            noise,
            &timesteps,
        )?;

        // -------------------------------------------------------------
        // 10. Token-form → spatial conversion. `euler_denoise` returns
        //     `[B, n_img, 128]` (Klein transformer's internal shape).
        //     Downstream `vae_decode` requires the canonical "Latent value
        //     as [B, C, H, W]" spatial form — same as the input latent.
        //     Mirrors klein_lora_infer.rs:340-342 verbatim:
        //       reshape [B, latent_h, latent_w, 128] → permute [0,3,1,2].
        //     Owning the conversion here keeps vae_decode architecture-
        //     agnostic; KSampler is already Klein-only at this point.
        // -------------------------------------------------------------
        let denoised_spatial = denoised
            .reshape(&[b, latent_h, latent_w, c])?
            .permute(&[0, 3, 1, 2])?;

        let mut out = HashMap::new();
        out.insert(
            "latent".into(),
            NodeValue::Latent {
                arch: ArchTag::Klein,
                tensor: denoised_spatial,
            },
        );
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
        let s = KSampler.schema();
        assert_eq!(s.display_name, "K Sampler");
        assert_eq!(s.category, "Sampling");

        let in_names: Vec<&str> = s.inputs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(in_names, vec!["model", "positive", "negative", "latent"]);

        let out_names: Vec<&str> = s.outputs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(out_names, vec!["latent"]);

        let field_names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(field_names, vec!["seed", "steps", "cfg", "shift"]);
    }

    #[test]
    fn schema_field_defaults() {
        let s = KSampler.schema();
        let by_name: HashMap<&str, &FieldValue> =
            s.fields.iter().map(|f| (f.name.as_str(), &f.default)).collect();
        assert!(matches!(by_name["seed"],  FieldValue::Number(n) if *n == 42.0));
        assert!(matches!(by_name["steps"], FieldValue::Number(n) if *n == 30.0));
        assert!(matches!(by_name["cfg"],   FieldValue::Number(n) if *n == 7.0));
        assert!(matches!(by_name["shift"], FieldValue::Number(n) if *n == 3.0));
    }

    #[test]
    fn type_id_stable() {
        assert_eq!(KSampler.type_id(), "core/k_sampler");
    }
}

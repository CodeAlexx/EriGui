//! Standalone GPU smoke test for B5 (`core/vae_decode`).
//!
//! Loads only the Klein VAE (no DiT, no encoder), wraps it in a `KleinVae`
//! handle, builds a synthetic [1, 128, 32, 32] noise latent, runs
//! `VaeDecode::execute()`, verifies output shape [1, 3, 512, 512] and that
//! values are finite, then saves a sanity-check PNG via `SaveImage`.
//!
//! Run:
//!   LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:/usr/local/cuda/lib64 \
//!     cargo run --release --bin smoke_vae_decode -p erigui-nodes

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use flame_core::{global_cuda_device, DType, Shape, Tensor};

use erigui_nodes::builtin::save_image::SaveImage;
use erigui_nodes::builtin::vae_decode::VaeDecode;
use erigui_nodes::handles::KleinVae;
use erigui_nodes::{ArchTag, FieldValue, NodeType, NodeValue};

fn main() -> Result<(), Box<dyn Error>> {
    println!("== Smoke 2: B5 vae_decode ==");

    // Klein VAE only (skip DiT + encoder per spec).
    let vae_path = PathBuf::from("/home/alex/EriDiffusion/Models/vaes/flux2-vae.safetensors");
    println!("loading VAE from {}", vae_path.display());

    let device = global_cuda_device();
    let vae_weights = flame_core::serialization::load_file(&vae_path, &device)?;
    let vae_device = flame_core::Device::from_arc(device.clone());
    let decoder =
        inference_flame::vae::klein_vae::KleinVaeDecoder::load(&vae_weights, &vae_device)?;
    drop(vae_weights);
    println!("VAE decoder loaded");

    let vae_handle = KleinVae {
        decoder,
        encoder: None,
    };

    // Synthetic noise latent: [1, 128, 32, 32] BF16. Klein decode takes
    // [B, 128, H, W] (from klein_lora_infer.rs:340-346 — denoised reshaped
    // to [1, latent_h, latent_w, 128] then permuted to NCHW).
    // 32 * 16 = 512 RGB pixels per side.
    let latent_f32 =
        Tensor::randn(Shape::from_dims(&[1, 128, 32, 32]), 0.0, 1.0, device.clone())?;
    let latent = latent_f32.to_dtype(DType::BF16)?;
    println!("synthetic latent shape: {:?}", latent.dims());

    // Build NodeValue inputs.
    let mut inputs: HashMap<String, NodeValue> = HashMap::new();
    inputs.insert(
        "latent".into(),
        NodeValue::Latent {
            arch: ArchTag::Klein,
            tensor: latent,
        },
    );
    inputs.insert(
        "vae".into(),
        NodeValue::Vae {
            arch: ArchTag::Klein,
            handle: Arc::new(vae_handle) as Arc<dyn std::any::Any + Send + Sync>,
        },
    );

    let fields: HashMap<String, FieldValue> = HashMap::new();

    println!("calling VaeDecode::execute()...");
    let t0 = std::time::Instant::now();
    let out = VaeDecode.execute(&inputs, &fields, None)?;
    println!("decode took {:.2}s", t0.elapsed().as_secs_f32());

    let img = out
        .get("image")
        .ok_or("missing 'image' output")?;

    let tensor = match img {
        NodeValue::Image(t) => t.clone(),
        other => {
            return Err(format!("expected Image, got {:?}", other.value_type()).into());
        }
    };

    println!("output tensor shape: {:?}", tensor.dims());
    let dims = tensor.dims();
    if dims != &[1, 3, 512, 512] {
        return Err(format!("wrong output shape: {dims:?}").into());
    }

    // Finiteness check + value-range stat.
    let f32t = tensor.to_dtype(DType::F32)?;
    let data = f32t.to_vec()?;
    let mut nan_count = 0usize;
    let mut inf_count = 0usize;
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    for &v in &data {
        if v.is_nan() {
            nan_count += 1;
        } else if !v.is_finite() {
            inf_count += 1;
        } else {
            if v < min_v {
                min_v = v;
            }
            if v > max_v {
                max_v = v;
            }
        }
    }
    println!(
        "value stats: min={:.4} max={:.4} nan={} inf={}",
        min_v, max_v, nan_count, inf_count
    );
    if nan_count != 0 {
        return Err("NaN in decoder output".into());
    }
    if inf_count != 0 {
        return Err("Inf in decoder output".into());
    }

    // Save sanity-check PNG via SaveImage.
    let mut save_inputs: HashMap<String, NodeValue> = HashMap::new();
    save_inputs.insert("image".into(), NodeValue::Image(tensor));
    let mut save_fields: HashMap<String, FieldValue> = HashMap::new();
    save_fields.insert(
        "filename_prefix".into(),
        FieldValue::Text("smoke_b5_vae".into()),
    );
    SaveImage.execute(&save_inputs, &save_fields, None)?;
    println!("PNG saved (./output/smoke_b5_vae_*.png)");

    println!("== Smoke 2: PASS ==");
    Ok(())
}

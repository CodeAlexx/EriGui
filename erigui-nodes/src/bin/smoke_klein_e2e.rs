//! Wave-2 end-to-end smoke: chain all six core nodes through their real
//! `execute()` methods on real Klein weights and produce a PNG.
//!
//! Pipeline: LoadCheckpoint → EncodePrompt(pos) + EncodePrompt(neg) →
//! KSampler → VAEDecode → SaveImage.
//!
//! KSampler returns spatial-form latent `[B, 128, H, W]` (post-fix);
//! the driver no longer bridges between sampler output and VAE input.
//!
//! Uses 512×512 (latent 32×32) and 20 steps to stay under thermal/time budget.
//!
//! Run:
//!   LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:/usr/local/cuda/lib64 \
//!     cargo run --release --bin smoke_klein_e2e -p erigui-nodes

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use flame_core::{global_cuda_device, DType, Shape, Tensor};

use erigui_nodes::{
    ArchTag, FieldValue, NodeRegistry, NodeValue, ProgressLevel, ProgressSink,
};

struct StdoutSink;
impl ProgressSink for StdoutSink {
    fn step(&self, current: u32, total: u32, label: &str) {
        eprintln!("[progress] {} {}/{}", label, current, total);
    }
    fn preview(&self, _: &Tensor) {}
    fn message(&self, level: ProgressLevel, msg: &str) {
        eprintln!("[msg/{:?}] {}", level, msg);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let t_total = Instant::now();
    println!("== Wave-2 Klein E2E Smoke ==");

    let registry = NodeRegistry::with_builtins();
    let device = global_cuda_device();

    // -------------------------------------------------------------
    // 1. LoadCheckpoint  (sidecar already on disk from B1 smoke)
    // -------------------------------------------------------------
    let sidecar_path = PathBuf::from("/tmp/klein_smoke_sidecar.json");
    if !sidecar_path.exists() {
        return Err(format!("sidecar not found at {}", sidecar_path.display()).into());
    }
    let load_node = registry
        .get("core/load_checkpoint")
        .ok_or("missing core/load_checkpoint")?;
    let mut load_fields: HashMap<String, FieldValue> = HashMap::new();
    load_fields.insert("path".into(), FieldValue::FilePath(sidecar_path));

    println!("[1] LoadCheckpoint: loading Klein 4B weights …");
    let t = Instant::now();
    let mut load_out = load_node.execute(&HashMap::new(), &load_fields, None)?;
    println!("[1] LoadCheckpoint: ok in {:.1}s", t.elapsed().as_secs_f32());

    let model = load_out.remove("model").ok_or("model missing")?;
    let clip = load_out.remove("clip").ok_or("clip missing")?;
    let vae = load_out.remove("vae").ok_or("vae missing")?;

    // -------------------------------------------------------------
    // 2. EncodePrompt (positive)
    // -------------------------------------------------------------
    let encode_node = registry
        .get("core/encode_prompt")
        .ok_or("missing core/encode_prompt")?;

    let mut pos_inputs: HashMap<String, NodeValue> = HashMap::new();
    pos_inputs.insert("clip".into(), clip.clone());
    let mut pos_fields: HashMap<String, FieldValue> = HashMap::new();
    pos_fields.insert(
        "text".into(),
        FieldValue::Text(
            "a portrait of a young woman with long dark hair, soft natural lighting, photograph"
                .into(),
        ),
    );
    pos_fields.insert("negative".into(), FieldValue::Bool(false));

    println!("[2] EncodePrompt(positive) …");
    let t = Instant::now();
    let pos = encode_node
        .execute(&pos_inputs, &pos_fields, None)?
        .remove("conditioning")
        .ok_or("positive conditioning missing")?;
    println!(
        "[2] EncodePrompt(positive): ok in {:.1}s",
        t.elapsed().as_secs_f32()
    );

    // -------------------------------------------------------------
    // 3. EncodePrompt (negative)
    // -------------------------------------------------------------
    let mut neg_inputs: HashMap<String, NodeValue> = HashMap::new();
    neg_inputs.insert("clip".into(), clip);
    let mut neg_fields: HashMap<String, FieldValue> = HashMap::new();
    neg_fields.insert(
        "text".into(),
        FieldValue::Text("blurry, deformed, low quality".into()),
    );
    neg_fields.insert("negative".into(), FieldValue::Bool(true));

    println!("[3] EncodePrompt(negative) …");
    let t = Instant::now();
    let neg = encode_node
        .execute(&neg_inputs, &neg_fields, None)?
        .remove("conditioning")
        .ok_or("negative conditioning missing")?;
    println!(
        "[3] EncodePrompt(negative): ok in {:.1}s",
        t.elapsed().as_secs_f32()
    );

    // -------------------------------------------------------------
    // 4. Synthesize initial latent (zeros). Klein @ 512×512 → [1,128,32,32].
    //    K-sampler internally seeds noise via Box-Muller and overwrites this
    //    placeholder; only its shape matters.
    // -------------------------------------------------------------
    let initial_latent = Tensor::zeros_dtype(
        Shape::from_dims(&[1, 128, 32, 32]),
        DType::BF16,
        device.clone(),
    )?;
    println!("[4] initial latent: shape {:?}", initial_latent.dims());

    // -------------------------------------------------------------
    // 5. KSampler — 20 steps, CFG 4.0, seed 42
    // -------------------------------------------------------------
    let sampler_node = registry
        .get("core/k_sampler")
        .ok_or("missing core/k_sampler")?;
    let mut sampler_inputs: HashMap<String, NodeValue> = HashMap::new();
    sampler_inputs.insert("model".into(), model);
    sampler_inputs.insert("positive".into(), pos);
    sampler_inputs.insert("negative".into(), neg);
    sampler_inputs.insert(
        "latent".into(),
        NodeValue::Latent {
            arch: ArchTag::Klein,
            tensor: initial_latent,
        },
    );
    let mut sampler_fields: HashMap<String, FieldValue> = HashMap::new();
    sampler_fields.insert("seed".into(), FieldValue::Number(42.0));
    sampler_fields.insert("steps".into(), FieldValue::Number(20.0));
    sampler_fields.insert("cfg".into(), FieldValue::Number(4.0));
    sampler_fields.insert("shift".into(), FieldValue::Number(3.0));

    let sink = StdoutSink;
    println!("[5] KSampler: 20 steps @ CFG 4.0, seed 42 …");
    let t = Instant::now();
    let denoised_v = sampler_node
        .execute(&sampler_inputs, &sampler_fields, Some(&sink))?
        .remove("latent")
        .ok_or("k_sampler latent missing")?;
    println!(
        "[5] KSampler: ok in {:.1}s",
        t.elapsed().as_secs_f32()
    );

    // -------------------------------------------------------------
    // 6. VAEDecode — k_sampler now returns spatial `[1, 128, H, W]`
    //    directly (Option B fix); no driver-side reshape needed.
    // -------------------------------------------------------------
    if let NodeValue::Latent { arch, tensor } = &denoised_v {
        if *arch != ArchTag::Klein {
            return Err(format!("denoised latent arch {arch:?} != Klein").into());
        }
        let d = tensor.dims();
        if d != [1, 128, 32, 32] {
            return Err(format!(
                "denoised spatial shape unexpected: {d:?} (want [1, 128, 32, 32])"
            )
            .into());
        }
        println!("[6] denoised (spatial from sampler): {:?}", d);
    } else {
        return Err(format!("expected Latent, got {:?}", denoised_v.value_type()).into());
    }

    let vae_node = registry
        .get("core/vae_decode")
        .ok_or("missing core/vae_decode")?;
    let mut vae_inputs: HashMap<String, NodeValue> = HashMap::new();
    vae_inputs.insert("latent".into(), denoised_v);
    vae_inputs.insert("vae".into(), vae);

    println!("[7] VaeDecode …");
    let t = Instant::now();
    let image_v = vae_node
        .execute(&vae_inputs, &HashMap::new(), None)?
        .remove("image")
        .ok_or("vae image missing")?;
    println!("[7] VaeDecode: ok in {:.2}s", t.elapsed().as_secs_f32());

    // Quick value-range sanity print so we know the VAE didn't return NaN garbage.
    if let NodeValue::Image(t) = &image_v {
        let dims = t.dims().to_vec();
        let f32t = t.to_dtype(DType::F32)?;
        let data = f32t.to_vec()?;
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;
        let mut nans = 0usize;
        for &v in &data {
            if v.is_nan() {
                nans += 1;
            } else {
                mn = mn.min(v);
                mx = mx.max(v);
            }
        }
        println!(
            "    image shape {:?}  min={:.3}  max={:.3}  nans={}",
            dims, mn, mx, nans
        );
        if nans > 0 {
            return Err(format!("decoder produced {nans} NaNs").into());
        }
    }

    // -------------------------------------------------------------
    // 8. SaveImage
    // -------------------------------------------------------------
    let save_node = registry
        .get("core/save_image")
        .ok_or("missing core/save_image")?;
    let mut save_inputs: HashMap<String, NodeValue> = HashMap::new();
    save_inputs.insert("image".into(), image_v);
    let mut save_fields: HashMap<String, FieldValue> = HashMap::new();
    save_fields.insert(
        "filename_prefix".into(),
        FieldValue::Text("klein_e2e".into()),
    );
    println!("[8] SaveImage …");
    let t = Instant::now();
    save_node.execute(&save_inputs, &save_fields, None)?;
    println!("[8] SaveImage: ok in {:.2}s", t.elapsed().as_secs_f32());

    println!(
        "== ALL DONE in {:.1}s — check ./output/klein_e2e_*.png ==",
        t_total.elapsed().as_secs_f32()
    );
    Ok(())
}

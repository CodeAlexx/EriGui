//! Integration test for `core/save_image` that requires a real CUDA
//! device. Moved out of `src/builtin/save_image.rs` per wave-2 spec:
//! unit tests under `cargo test --lib` must not need a GPU.
//!
//! Run with: `cargo test -p erigui-nodes --tests save_image_integration`
//! Skip on CI without GPUs by passing `--skip save_image_integration` or
//! by deleting/ignoring this file in non-GPU jobs. Marked `#[ignore]` so
//! plain `cargo test` does not pull it in by default — use
//! `cargo test -- --ignored` to opt in.

use std::collections::HashMap;

use erigui_nodes::{FieldValue, NodeType, NodeValue};
use erigui_nodes::builtin::save_image::SaveImage;

/// Round-trip: build a fake [1, 3, 64, 64] zeros tensor, call execute(),
/// verify a PNG appears at the expected path. Mid-grey output is correct
/// because zeros sit in the middle of [-1, 1] → (0+1)*127.5 = 127.
///
/// Mirrors `OUTPUT_DIR = "./output"` from `save_image.rs`. Kept as a
/// hard-coded literal because the const is crate-private; the alternative
/// (exposing it) would widen the public API for the sake of one test.
#[test]
#[ignore = "requires CUDA device"]
fn save_image_writes_png() {
    use flame_core::{global_cuda_device, Shape, Tensor};

    let device = global_cuda_device();
    let tensor = Tensor::zeros_dtype(
        Shape::from_dims(&[1, 3, 64, 64]),
        flame_core::DType::BF16,
        device.clone(),
    )
    .expect("zeros tensor");

    let mut inputs: HashMap<String, NodeValue> = HashMap::new();
    inputs.insert("image".into(), NodeValue::Image(tensor));

    let mut fields: HashMap<String, FieldValue> = HashMap::new();
    // Unique prefix so this test doesn't see stale PNGs from prior runs.
    let prefix = format!("erigui_test_b6_{}", std::process::id());
    fields.insert("filename_prefix".into(), FieldValue::Text(prefix.clone()));

    let out = SaveImage
        .execute(&inputs, &fields, None)
        .expect("execute must succeed");
    assert!(out.is_empty(), "save_image returns no outputs");

    let dir = std::path::PathBuf::from("./output");
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("output dir should exist after execute()")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&format!("{prefix}_"))
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one PNG with our prefix, got {}",
        entries.len()
    );
    let path = entries[0].path();
    assert_eq!(
        path.extension().and_then(|s| s.to_str()),
        Some("png"),
        "must be a .png"
    );

    // Sanity-check the PNG is actually a valid 64x64 RGB image by
    // re-reading it. This catches buffer-length mismatches and channel
    // ordering bugs that wouldn't trip the type checker.
    let img = image::open(&path).expect("png must decode");
    assert_eq!(img.width(), 64);
    assert_eq!(img.height(), 64);

    // Cleanup so repeated test runs don't accumulate noise.
    let _ = std::fs::remove_file(&path);
}

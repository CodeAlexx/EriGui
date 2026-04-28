//! Standalone GPU smoke test for B1 (`core/load_checkpoint`).
//!
//! Writes a Klein sidecar JSON, instantiates `LoadCheckpoint`, calls
//! `execute()`, asserts three Klein-tagged handles come back, and prints
//! the inner type-ids.
//!
//! Run:
//!   LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:/usr/local/cuda/lib64 \
//!     cargo run --release --bin smoke_load_checkpoint -p erigui-nodes

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use erigui_nodes::builtin::load_checkpoint::LoadCheckpoint;
use erigui_nodes::{ArchTag, FieldValue, NodeType, NodeValue};

fn main() -> Result<(), Box<dyn Error>> {
    println!("== Smoke 3: B1 load_checkpoint ==");

    // Real Klein 4B paths from klein_lora_infer.rs defaults.
    let sidecar_path = PathBuf::from("/tmp/klein_smoke_sidecar.json");
    let sidecar_json = serde_json::json!({
        "arch": "klein",
        "dit": "/home/alex/EriDiffusion/Models/checkpoints/flux-2-klein-base-4b.safetensors",
        "encoder": "/home/alex/.serenity/models/text_encoders/qwen_3_4b.safetensors",
        "vae": "/home/alex/EriDiffusion/Models/vaes/flux2-vae.safetensors",
        "tokenizer": "/home/alex/.cache/huggingface/hub/models--Qwen--Qwen3-8B/snapshots/b968826d9c46dd6066d109eabc6255188de91218/tokenizer.json"
    });
    std::fs::write(&sidecar_path, serde_json::to_string_pretty(&sidecar_json)?)?;
    println!("wrote sidecar to {}", sidecar_path.display());

    let inputs: HashMap<String, NodeValue> = HashMap::new();
    let mut fields: HashMap<String, FieldValue> = HashMap::new();
    fields.insert("path".into(), FieldValue::FilePath(sidecar_path.clone()));

    println!("calling LoadCheckpoint::execute() (this loads ~16GB of weights)...");
    let t0 = std::time::Instant::now();
    let out = LoadCheckpoint.execute(&inputs, &fields, None)?;
    println!(
        "load_checkpoint completed in {:.1}s",
        t0.elapsed().as_secs_f32()
    );

    // Verify three outputs: model, clip, vae — all Klein-tagged.
    for port in &["model", "clip", "vae"] {
        let v = out.get(*port).ok_or_else(|| -> Box<dyn Error> {
            format!("missing port `{port}`").into()
        })?;
        match v {
            NodeValue::Model { arch, handle } => {
                if *arch != ArchTag::Klein {
                    return Err("model arch != Klein".into());
                }
                println!(
                    "  model: arch=Klein, type_id={:?}",
                    Any::type_id(handle.as_ref())
                );
            }
            NodeValue::Clip { arch, handle } => {
                if *arch != ArchTag::Klein {
                    return Err("clip arch != Klein".into());
                }
                println!(
                    "  clip:  arch=Klein, type_id={:?}",
                    Any::type_id(handle.as_ref())
                );
            }
            NodeValue::Vae { arch, handle } => {
                if *arch != ArchTag::Klein {
                    return Err("vae arch != Klein".into());
                }
                println!(
                    "  vae:   arch=Klein, type_id={:?}",
                    Any::type_id(handle.as_ref())
                );
            }
            other => {
                return Err(format!("port {port}: unexpected variant {:?}", other.value_type()).into());
            }
        }
    }

    println!("== Smoke 3: PASS ==");
    Ok(())
}

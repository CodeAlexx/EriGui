//! Integration tests for `erigui-workflow`.
//!
//! These tests intentionally use `serde_json::json!` to construct any JSON
//! that needs to be hand-shaped (malformed-version case, unknown-type case)
//! rather than concatenating raw strings. The exception is the
//! version-mismatch test which still uses `json!` — only truly malformed
//! input would justify a raw string, and we don't test parser-level errors
//! here.

use std::collections::HashMap;

use erigui_nodes::NodeRegistry;
use erigui_workflow::{
    PortRef, Position, Workflow, WorkflowEdge, WorkflowError, WorkflowNode,
};
use serde_json::json;

/// Construct a representative 3-node workflow:
///   LoadCheckpoint(id=1) -> KSampler(id=7) -> SaveImage(id=12)
/// with a couple of edges between them.
fn sample_workflow() -> Workflow {
    let load = WorkflowNode {
        id: 1,
        type_id: "core/load_checkpoint".to_string(),
        position: Position { x: 40.0, y: 80.0 },
        fields: {
            let mut m = HashMap::new();
            m.insert("path".to_string(), json!("/models/foo.safetensors"));
            m
        },
    };
    let sampler = WorkflowNode {
        id: 7,
        type_id: "core/k_sampler".to_string(),
        position: Position { x: 320.0, y: 140.0 },
        fields: {
            let mut m = HashMap::new();
            m.insert("seed".to_string(), json!(42));
            m.insert("steps".to_string(), json!(30));
            m.insert("cfg".to_string(), json!(7.0));
            m.insert("sampler".to_string(), json!("euler"));
            m.insert("scheduler".to_string(), json!("normal"));
            m
        },
    };
    let save = WorkflowNode {
        id: 12,
        type_id: "core/save_image".to_string(),
        position: Position { x: 600.0, y: 140.0 },
        fields: {
            let mut m = HashMap::new();
            m.insert("filename_prefix".to_string(), json!("eri"));
            m
        },
    };

    Workflow {
        version: Workflow::CURRENT_VERSION,
        nodes: vec![load, sampler, save],
        edges: vec![
            WorkflowEdge {
                from: PortRef {
                    node: 1,
                    port: "model".to_string(),
                },
                to: PortRef {
                    node: 7,
                    port: "model".to_string(),
                },
            },
            WorkflowEdge {
                from: PortRef {
                    node: 7,
                    port: "latent".to_string(),
                },
                to: PortRef {
                    node: 12,
                    port: "image".to_string(),
                },
            },
        ],
    }
}

#[test]
fn roundtrip_simple_graph() {
    let wf = sample_workflow();
    let s = wf.save_to_string().expect("save");
    let back = Workflow::load_from_string(&s).expect("load");
    assert_eq!(
        wf, back,
        "round-tripped workflow must equal the original; structural equality is required"
    );
}

#[test]
fn field_value_roundtrip() {
    // Mix string, number, bool to exercise FieldValue's natural JSON forms.
    let mut fields = HashMap::new();
    fields.insert("text".to_string(), json!("hello"));
    fields.insert("count".to_string(), json!(3));
    fields.insert("ratio".to_string(), json!(0.25));
    fields.insert("enabled".to_string(), json!(true));

    let wf = Workflow {
        version: Workflow::CURRENT_VERSION,
        nodes: vec![WorkflowNode {
            id: 1,
            type_id: "core/encode_prompt".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            fields,
        }],
        edges: vec![],
    };

    let first = wf.save_to_string().expect("save 1");
    let parsed = Workflow::load_from_string(&first).expect("load 1");
    let second = parsed.save_to_string().expect("save 2");

    // Idempotent format: saving a parsed workflow must produce byte-identical
    // output. This guards against accidental nondeterminism (e.g. switching
    // away from a stable JSON ordering in the field map).
    assert_eq!(
        first, second,
        "second save must equal first save — workflow JSON format must be deterministic"
    );
    assert_eq!(wf, parsed);
}

#[test]
fn unknown_type_id_is_unresolved() {
    // Build raw JSON by serializing a structured Workflow rather than a
    // hand-rolled string. The "nonexistent/foo" type id is the load-bearing
    // bit — it must not match any builtin (builtins all live under `core/`).
    let payload = json!({
        "version": Workflow::CURRENT_VERSION,
        "nodes": [
            {
                "id": 1,
                "type_id": "nonexistent/foo",
                "position": { "x": 0.0, "y": 0.0 },
                "fields": {}
            },
            {
                "id": 2,
                "type_id": "core/save_image",
                "position": { "x": 100.0, "y": 0.0 },
                "fields": {}
            }
        ],
        "edges": []
    });
    let s = serde_json::to_string(&payload).unwrap();

    let wf = Workflow::load_from_string(&s).expect("load must not crash on unknown type_id");

    // Resolve against the default registry. `nonexistent/foo` cannot be in
    // any registry by construction.
    let registry = NodeRegistry::with_builtins();
    let report = wf.resolve(&registry);

    assert!(
        report
            .unresolved
            .iter()
            .any(|u| u.node_id == 1 && u.type_id == "nonexistent/foo"),
        "node 1 with type_id 'nonexistent/foo' must appear in unresolved; got {:?}",
        report
    );
    // Node 2 references a builtin and should resolve.
    assert!(
        report.resolved.contains(&2u32),
        "node 2 (core/save_image) should resolve against builtins; got {:?}",
        report
    );
}

#[test]
fn roundtrip_field_bool() {
    // Per A1's spec amendment: FieldKind::Bool serializes as JSON true/false
    // and must survive a save/load round trip exactly. We test both polarities
    // because a buggy serializer that, say, coerces bool->number would only
    // round-trip one of them.
    let mut fields = HashMap::new();
    fields.insert("negative".to_string(), json!(true));
    fields.insert("clip_skip_enabled".to_string(), json!(false));

    let wf = Workflow {
        version: Workflow::CURRENT_VERSION,
        nodes: vec![WorkflowNode {
            id: 1,
            type_id: "core/encode_prompt".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            fields,
        }],
        edges: vec![],
    };

    let s = wf.save_to_string().expect("save");
    let back = Workflow::load_from_string(&s).expect("load");

    assert_eq!(
        wf, back,
        "Bool fields must round-trip exactly (true and false both)"
    );

    // Belt-and-suspenders: assert the JSON value is a real bool, not a
    // number or string. Catches accidental coercion in the serializer.
    let n = &back.nodes[0];
    assert_eq!(n.fields.get("negative"), Some(&json!(true)));
    assert_eq!(n.fields.get("clip_skip_enabled"), Some(&json!(false)));
    assert!(n.fields["negative"].is_boolean());
    assert!(n.fields["clip_skip_enabled"].is_boolean());
}

#[test]
fn roundtrip_field_filepath() {
    // FieldKind::FilePath { extensions } serializes as a plain JSON string
    // (the path); the `extensions` filter is schema-side metadata, not part
    // of the wire value. Use characters that historically trip up encoders:
    // spaces, unicode, backslashes (for Windows-style paths).
    let mut fields = HashMap::new();
    fields.insert(
        "checkpoint".to_string(),
        json!("/home/alex/Models with spaces/flux1-dev.safetensors"),
    );
    fields.insert(
        "lora".to_string(),
        json!("C:\\Users\\alex\\loras\\café.safetensors"),
    );

    let wf = Workflow {
        version: Workflow::CURRENT_VERSION,
        nodes: vec![WorkflowNode {
            id: 1,
            type_id: "core/load_checkpoint".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            fields,
        }],
        edges: vec![],
    };

    let s = wf.save_to_string().expect("save");
    let back = Workflow::load_from_string(&s).expect("load");

    assert_eq!(wf, back, "FilePath fields must round-trip exactly");

    let n = &back.nodes[0];
    assert_eq!(
        n.fields.get("checkpoint").and_then(|v| v.as_str()),
        Some("/home/alex/Models with spaces/flux1-dev.safetensors"),
    );
    assert_eq!(
        n.fields.get("lora").and_then(|v| v.as_str()),
        Some("C:\\Users\\alex\\loras\\café.safetensors"),
    );
}

#[test]
fn version_mismatch_errors_cleanly() {
    // Version 999 is reserved as a "definitely not us" sentinel. Loading it
    // must return UnsupportedVersion, not panic, not generic JSON error.
    let payload = json!({
        "version": 999,
        "nodes": [],
        "edges": []
    });
    let s = serde_json::to_string(&payload).unwrap();

    match Workflow::load_from_string(&s) {
        Err(WorkflowError::UnsupportedVersion { found, supported }) => {
            assert_eq!(found, 999);
            assert_eq!(supported, Workflow::CURRENT_VERSION);
        }
        other => panic!(
            "expected UnsupportedVersion error, got {:?}",
            other.as_ref().err()
        ),
    }
}

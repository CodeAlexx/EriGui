//! Translation between the widget-side [`Graph`] (in-memory editor model,
//! used by `NodeGraph` and the runtime executor) and the [`Workflow`] JSON
//! envelope (on-disk save format from `erigui-workflow`).
//!
//! Two type systems disagree on a few axes:
//!   - `Node::position` is `Point { i32, i32 }`; `WorkflowNode::position`
//!     is `Position { f32, f32 }`. We convert verbatim — fractional pixels
//!     get rounded on the way back in.
//!   - `Node::id` is `usize`; `WorkflowNode::id` is `u32`. We narrow with
//!     `as u32` (host-platform safe; widget ids are application-generated
//!     and stay well under `u32::MAX`).
//!   - Edges in the widget reference port *integer ids* on each node;
//!     workflow edges reference port *names*. We resolve via the node's
//!     `inputs` / `outputs` lists.
//!   - Field values: the widget uses the rich `FieldValue` enum; the
//!     workflow stores `serde_json::Value`. The mapping is the natural one
//!     (`Number` ↔ JSON number, `Bool` ↔ JSON bool, `Text`/`Select`/
//!     `FilePath` ↔ JSON string).
//!
//! On load, nodes whose `type_id` is unknown to the registry are dropped
//! with a warning logged via `log::warn!`. The caller already invokes
//! `Workflow::resolve(&registry)` to surface unresolved nodes in the UI;
//! this conversion is best-effort beyond that point.

use std::collections::HashMap;

use erigui_core::{Point, Size};
use erigui_nodes::{NodeRegistry, NodeSchema};
use erigui_widgets::node_graph::{Edge, Field, FieldValue, Graph, Node, Port};
use erigui_workflow::{PortRef, Position, Workflow, WorkflowEdge, WorkflowNode};

const DEFAULT_NODE_WIDTH: i32 = 220;
const DEFAULT_NODE_HEIGHT: i32 = 160;

/// Convert the in-memory editor graph to the on-disk workflow envelope.
///
/// Nodes whose `component_type` is `None` are skipped — they're widget-only
/// scratch nodes that have no persistable identity.
pub fn graph_to_workflow(graph: &Graph) -> Workflow {
    let mut nodes = Vec::with_capacity(graph.nodes.len());
    for n in &graph.nodes {
        let type_id = match &n.component_type {
            Some(s) => s.clone(),
            None => continue,
        };
        let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
        for f in &n.fields {
            fields_map.insert(f.label.clone(), field_value_to_json(&f.value));
        }
        nodes.push(WorkflowNode {
            id: n.id as u32,
            type_id,
            position: Position {
                x: n.position.x as f32,
                y: n.position.y as f32,
            },
            fields: fields_map,
        });
    }

    // Build per-node port-id → port-name lookup so we can translate edges.
    let mut input_name: HashMap<(usize, usize), String> = HashMap::new();
    let mut output_name: HashMap<(usize, usize), String> = HashMap::new();
    for n in &graph.nodes {
        for p in &n.inputs {
            input_name.insert((n.id, p.id), p.label.clone());
        }
        for p in &n.outputs {
            output_name.insert((n.id, p.id), p.label.clone());
        }
    }

    let mut edges = Vec::with_capacity(graph.edges.len());
    for e in &graph.edges {
        let from_name = match output_name.get(&(e.from_node, e.from_port)) {
            Some(s) => s.clone(),
            None => continue,
        };
        let to_name = match input_name.get(&(e.to_node, e.to_port)) {
            Some(s) => s.clone(),
            None => continue,
        };
        edges.push(WorkflowEdge {
            from: PortRef {
                node: e.from_node as u32,
                port: from_name,
            },
            to: PortRef {
                node: e.to_node as u32,
                port: to_name,
            },
        });
    }

    Workflow {
        version: Workflow::CURRENT_VERSION,
        nodes,
        edges,
    }
}

/// Convert a loaded workflow back into a widget graph, using the supplied
/// registry to recover each node's port/field schema. Unknown nodes are
/// dropped with a `log::warn!`.
pub fn workflow_to_graph(wf: &Workflow, registry: &NodeRegistry) -> Graph {
    let mut nodes: Vec<Node> = Vec::with_capacity(wf.nodes.len());
    let mut schemas: HashMap<u32, NodeSchema> = HashMap::with_capacity(wf.nodes.len());

    for wn in &wf.nodes {
        let nt = match registry.get(&wn.type_id) {
            Some(t) => t,
            None => {
                log::warn!(
                    "workflow_to_graph: dropping node id={} type_id={} (not in registry)",
                    wn.id,
                    wn.type_id
                );
                continue;
            }
        };
        let schema = nt.schema();

        let inputs: Vec<Port> = schema
            .inputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: true,
            })
            .collect();
        let outputs: Vec<Port> = schema
            .outputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: false,
            })
            .collect();

        // Reconstruct fields in schema order so their `id`s match the
        // widget's expectations. Field values come from the saved JSON map;
        // missing keys fall back to the schema default.
        let fields: Vec<Field> = schema
            .fields
            .iter()
            .enumerate()
            .map(|(i, fs)| {
                let value = wn
                    .fields
                    .get(&fs.label)
                    .or_else(|| wn.fields.get(&fs.name))
                    .and_then(|v| json_to_field_value(v, &fs.default))
                    .unwrap_or_else(|| fs.default.clone());
                Field {
                    id: i,
                    label: fs.label.clone(),
                    kind: fs.kind.clone(),
                    value,
                }
            })
            .collect();

        let node = Node {
            id: wn.id as usize,
            title: schema.display_name.to_string(),
            position: Point::new(wn.position.x.round() as i32, wn.position.y.round() as i32),
            size: Size::new(DEFAULT_NODE_WIDTH, DEFAULT_NODE_HEIGHT),
            inputs,
            outputs,
            fields,
            component_type: Some(wn.type_id.clone()),
        };
        schemas.insert(wn.id, schema);
        nodes.push(node);
    }

    // Edges: look up port name → integer id via each side's schema.
    let mut edges: Vec<Edge> = Vec::with_capacity(wf.edges.len());
    for we in &wf.edges {
        let from_schema = match schemas.get(&we.from.node) {
            Some(s) => s,
            None => continue,
        };
        let to_schema = match schemas.get(&we.to.node) {
            Some(s) => s,
            None => continue,
        };
        let from_port = match from_schema
            .outputs
            .iter()
            .position(|p| p.name == we.from.port)
        {
            Some(i) => i,
            None => continue,
        };
        let to_port = match to_schema.inputs.iter().position(|p| p.name == we.to.port) {
            Some(i) => i,
            None => continue,
        };
        edges.push(Edge {
            from_node: we.from.node as usize,
            from_port,
            to_node: we.to.node as usize,
            to_port,
        });
    }

    Graph { nodes, edges }
}

fn field_value_to_json(v: &FieldValue) -> serde_json::Value {
    match v {
        FieldValue::Text(s) => serde_json::Value::String(s.clone()),
        FieldValue::Number(n) => serde_json::Number::from_f64(*n as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        FieldValue::Select(s) => serde_json::Value::String(s.clone()),
        FieldValue::FilePath(p) => serde_json::Value::String(p.to_string_lossy().into_owned()),
        FieldValue::Bool(b) => serde_json::Value::Bool(*b),
    }
}

fn json_to_field_value(v: &serde_json::Value, default: &FieldValue) -> Option<FieldValue> {
    // Match on the default's variant so we coerce the JSON into the
    // shape the widget expects rather than trusting the JSON alone.
    Some(match default {
        FieldValue::Text(_) => FieldValue::Text(v.as_str()?.to_string()),
        FieldValue::Number(_) => FieldValue::Number(v.as_f64()? as f32),
        FieldValue::Select(_) => FieldValue::Select(v.as_str()?.to_string()),
        FieldValue::FilePath(_) => FieldValue::FilePath(std::path::PathBuf::from(v.as_str()?)),
        FieldValue::Bool(_) => FieldValue::Bool(v.as_bool()?),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use erigui_nodes::NodeRegistry;
    use erigui_widgets::node_graph::{FieldKind, FieldValue};

    /// Build a representative two-node graph using real builtin schemas
    /// so the round-trip exercises actual port/field name resolution.
    fn sample_graph(registry: &NodeRegistry) -> Graph {
        // LoadCheckpoint -> KSampler via the "model" port.
        let load_schema = registry
            .get("core/load_checkpoint")
            .expect("load_checkpoint registered")
            .schema();
        let sampler_schema = registry
            .get("core/k_sampler")
            .expect("k_sampler registered")
            .schema();

        let load_inputs: Vec<Port> = load_schema
            .inputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: true,
            })
            .collect();
        let load_outputs: Vec<Port> = load_schema
            .outputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: false,
            })
            .collect();
        let load_fields: Vec<Field> = load_schema
            .fields
            .iter()
            .enumerate()
            .map(|(i, fs)| Field {
                id: i,
                label: fs.label.clone(),
                kind: fs.kind.clone(),
                value: match &fs.kind {
                    FieldKind::FilePath { .. } => {
                        FieldValue::FilePath(std::path::PathBuf::from("/tmp/foo.json"))
                    }
                    _ => fs.default.clone(),
                },
            })
            .collect();

        let load = Node {
            id: 1,
            title: load_schema.display_name.to_string(),
            position: Point::new(40, 60),
            size: Size::new(220, 160),
            inputs: load_inputs,
            outputs: load_outputs,
            fields: load_fields,
            component_type: Some("core/load_checkpoint".to_string()),
        };

        let sampler_inputs: Vec<Port> = sampler_schema
            .inputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: true,
            })
            .collect();
        let sampler_outputs: Vec<Port> = sampler_schema
            .outputs
            .iter()
            .enumerate()
            .map(|(i, p)| Port {
                id: i,
                label: p.name.clone(),
                is_input: false,
            })
            .collect();
        let sampler_fields: Vec<Field> = sampler_schema
            .fields
            .iter()
            .enumerate()
            .map(|(i, fs)| Field {
                id: i,
                label: fs.label.clone(),
                kind: fs.kind.clone(),
                value: fs.default.clone(),
            })
            .collect();

        let sampler = Node {
            id: 7,
            title: sampler_schema.display_name.to_string(),
            position: Point::new(320, 80),
            size: Size::new(220, 160),
            inputs: sampler_inputs,
            outputs: sampler_outputs,
            fields: sampler_fields,
            component_type: Some("core/k_sampler".to_string()),
        };

        // Find the "model" output on load and the "model" input on sampler.
        let from_port = load
            .outputs
            .iter()
            .position(|p| p.label == "model")
            .expect("load_checkpoint has a `model` output");
        let to_port = sampler
            .inputs
            .iter()
            .position(|p| p.label == "model")
            .expect("k_sampler has a `model` input");

        let edge = Edge {
            from_node: 1,
            from_port,
            to_node: 7,
            to_port,
        };

        Graph {
            nodes: vec![load, sampler],
            edges: vec![edge],
        }
    }

    #[test]
    fn graph_workflow_roundtrip() {
        let registry = NodeRegistry::with_builtins();
        let g1 = sample_graph(&registry);
        let wf = graph_to_workflow(&g1);

        // Workflow envelope sanity.
        assert_eq!(wf.version, Workflow::CURRENT_VERSION);
        assert_eq!(wf.nodes.len(), 2);
        assert_eq!(wf.edges.len(), 1);

        // Save → load → graph2.
        let s = wf.save_to_string().expect("save");
        let wf2 = Workflow::load_from_string(&s).expect("load");
        let g2 = workflow_to_graph(&wf2, &registry);

        // Structural equality on the bits the workflow actually persists:
        // node ids, type_ids, positions (within rounding), edge endpoints
        // by port name.
        assert_eq!(g1.nodes.len(), g2.nodes.len(), "node count preserved");
        assert_eq!(g1.edges.len(), g2.edges.len(), "edge count preserved");

        for (a, b) in g1.nodes.iter().zip(g2.nodes.iter()) {
            assert_eq!(a.id, b.id, "node id");
            assert_eq!(a.component_type, b.component_type, "component_type");
            assert_eq!(a.position, b.position, "position");
        }

        for (a, b) in g1.edges.iter().zip(g2.edges.iter()) {
            assert_eq!(a.from_node, b.from_node, "edge from_node");
            assert_eq!(a.to_node, b.to_node, "edge to_node");
            assert_eq!(a.from_port, b.from_port, "edge from_port");
            assert_eq!(a.to_port, b.to_port, "edge to_port");
        }
    }
}

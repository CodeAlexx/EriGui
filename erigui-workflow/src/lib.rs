//! Workflow JSON save/load with versioning for the EriGui node graph.
//!
//! A workflow is a serializable snapshot of a node graph: its nodes (each
//! identified by a registered `type_id`), their UI positions, their field
//! values, and the edges between them. Tensor-bearing values are NEVER
//! serialized — only graph topology and field values.
//!
//! Versioning: `Workflow::CURRENT_VERSION` is the single supported wire
//! version. Loading a workflow with a different `version` returns
//! `WorkflowError::UnsupportedVersion`. Migration logic is intentionally not
//! implemented in wave 1 — only the error path.
//!
//! Forward compatibility: a workflow may reference a `type_id` that is not
//! registered in the current `NodeRegistry`. Loading still succeeds; callers
//! invoke `Workflow::resolve` to discover such nodes and present a UI choice
//! (replace, remove, or leave as a placeholder).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize, Serializer};

use erigui_nodes::NodeRegistry;

/// Top-level workflow envelope. This is the on-disk representation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Workflow {
    pub version: u32,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

/// A single node instance within a workflow.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorkflowNode {
    /// Unique within this workflow. Edges reference this id.
    ///
    /// `u32` is chosen over `usize` so the wire format is identical on
    /// 32-bit and 64-bit hosts (a saved workflow round-trips bit-for-bit
    /// between platforms). 4B distinct nodes per workflow is overkill and
    /// the saved JSON is the contract — `usize` would leak host width.
    pub id: u32,
    /// Stable type id; looked up in `NodeRegistry`.
    pub type_id: String,
    pub position: Position,
    /// Field values keyed by field name. Each value is whatever the
    /// corresponding `FieldKind` serializes to in JSON (string / number /
    /// bool). Stored as `serde_json::Value` so the workflow crate is
    /// decoupled from the concrete `FieldValue` enum in `erigui-widgets`.
    ///
    /// Serialization is forced through a sorted (BTreeMap) intermediate so
    /// that re-saving a loaded workflow produces byte-identical output —
    /// `HashMap`'s iteration order is not stable, which would otherwise
    /// generate spurious diffs when workflows are version-controlled.
    #[serde(default, serialize_with = "serialize_fields_sorted")]
    pub fields: HashMap<String, serde_json::Value>,
}

fn serialize_fields_sorted<S>(
    fields: &HashMap<String, serde_json::Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let sorted: BTreeMap<&String, &serde_json::Value> = fields.iter().collect();
    sorted.serialize(serializer)
}

/// A single edge between two ports on two nodes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorkflowEdge {
    pub from: PortRef,
    pub to: PortRef,
}

/// Reference to a specific port on a specific node. Port identity is by name
/// (not index) so that adding/reordering ports in a node implementation does
/// not silently break saved workflows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PortRef {
    pub node: u32,
    pub port: String,
}

/// Canvas position in the node graph (UI coordinates).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Result of matching a loaded workflow against a `NodeRegistry`.
///
/// `resolved` are node ids whose `type_id` is currently known; `unresolved`
/// are nodes that the registry can't construct (likely a custom node type
/// that isn't loaded, or a workflow saved by a newer build).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolveReport {
    pub resolved: Vec<u32>,
    pub unresolved: Vec<UnresolvedNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedNode {
    pub node_id: u32,
    pub type_id: String,
}

/// Errors from save/load and version checks. I/O and JSON errors are
/// preserved verbatim; version mismatch is structured so callers can render a
/// dedicated dialog without parsing the message.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("workflow JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("workflow I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported workflow version: found {found}, this build supports {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
}

impl Workflow {
    /// The wire format version this build emits and accepts on load.
    pub const CURRENT_VERSION: u32 = 1;

    /// Construct an empty workflow at the current version.
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Serialize to a pretty-printed JSON string. Pretty-printing is chosen
    /// over compact form because workflows are user-visible artifacts that
    /// often live in version control; the diff cost matters more than bytes.
    pub fn save_to_string(&self) -> Result<String, WorkflowError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse a workflow from a JSON string.
    ///
    /// Version handling: the version field is checked first. A version that
    /// does not equal `CURRENT_VERSION` returns `UnsupportedVersion` rather
    /// than attempting partial parsing. This is the conservative choice for
    /// wave 1; future waves may add migration paths.
    pub fn load_from_string(s: &str) -> Result<Self, WorkflowError> {
        // Peek at version first so we can give a structured error before
        // falling into a generic deserialization failure if a future schema
        // adds incompatible fields.
        let peek: VersionPeek = serde_json::from_str(s)?;
        if peek.version != Self::CURRENT_VERSION {
            return Err(WorkflowError::UnsupportedVersion {
                found: peek.version,
                supported: Self::CURRENT_VERSION,
            });
        }
        let wf: Workflow = serde_json::from_str(s)?;
        Ok(wf)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), WorkflowError> {
        let s = self.save_to_string()?;
        fs::write(path, s)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self, WorkflowError> {
        let s = fs::read_to_string(path)?;
        Self::load_from_string(&s)
    }

    /// Match this workflow's nodes against `registry`. Nodes whose `type_id`
    /// is not in the registry are reported in `unresolved`; the caller
    /// decides whether to drop, replace, or keep them as placeholders.
    pub fn resolve(&self, registry: &NodeRegistry) -> ResolveReport {
        let mut report = ResolveReport::default();
        for node in &self.nodes {
            if registry.get(&node.type_id).is_some() {
                report.resolved.push(node.id);
            } else {
                report.unresolved.push(UnresolvedNode {
                    node_id: node.id,
                    type_id: node.type_id.clone(),
                });
            }
        }
        report
    }
}

impl Default for Workflow {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal envelope used to extract just the version field for early
/// rejection. Keeps the version check independent of any additional fields a
/// future schema might add.
#[derive(Deserialize)]
struct VersionPeek {
    version: u32,
}

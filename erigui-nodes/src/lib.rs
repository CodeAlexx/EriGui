//! erigui-nodes — node trait, registry, and starter node implementations
//! for the EriGui diffusion node graph.
//!
//! Wave 1 design: `docs/nodes_spec.md`. Wave 2 (Klein-arch implementations):
//! `docs/wave2_seed_spec.md`. This file is the wave-2 lib.rs migration
//! described in those specs (B1 scope): the opaque per-handle stub
//! newtypes are gone; `NodeValue`'s model/clip/vae/lora variants now
//! carry an `ArchTag` discriminant and an `Arc<dyn Any + Send + Sync>`
//! holding the concrete inference-flame type.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub use erigui_widgets::node_graph::{FieldKind, FieldValue};
use flame_core::Tensor;

pub mod builtin;
pub mod handles;
pub mod values;

pub use values::ConditioningPack;

// ---------------------------------------------------------------------------
// Architecture tag
// ---------------------------------------------------------------------------

/// Architecture discriminant on `NodeValue::{Model,Vae,Clip,Lora,Latent,Conditioning}`.
///
/// Edge-connect-time validation in the editor only checks `NodeValueType`
/// (Model-to-Model, Latent-to-Latent, …). Architecture compatibility is a
/// runtime check inside `execute()` because the architecture is data, not
/// shape — a single port could in principle accept any arch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArchTag {
    Klein,
    ZImage,
    Flux1,
    Sdxl,
    Sd35,
}

impl ArchTag {
    pub fn as_str(self) -> &'static str {
        match self {
            ArchTag::Klein => "klein",
            ArchTag::ZImage => "zimage",
            ArchTag::Flux1 => "flux1",
            ArchTag::Sdxl => "sdxl",
            ArchTag::Sd35 => "sd35",
        }
    }
}

impl std::fmt::Display for ArchTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Value types ("what flows on edges")
// ---------------------------------------------------------------------------

/// Type-tag for a port. Used by the editor to validate edge connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeValueType {
    Latent,
    Image,
    Conditioning,
    Model,
    Vae,
    Clip,
    Lora,
    Number,
    Text,
    Seed,
    Bool,
}

/// Runtime value carried by an edge between nodes.
///
/// Tensor-bearing variants clone in O(1) (flame_core::Tensor is Arc-internal).
/// Handle variants carry the concrete inference-flame type erased to
/// `Arc<dyn Any + Send + Sync>` plus an `arch` discriminator so a node
/// can fail with `NodeError::ArchMismatch` before downcasting.
#[derive(Clone)]
pub enum NodeValue {
    Latent {
        arch: ArchTag,
        tensor: Tensor,
    },
    Image(Tensor),
    Conditioning(Arc<ConditioningPack>),
    Model {
        arch: ArchTag,
        handle: Arc<dyn std::any::Any + Send + Sync>,
    },
    Vae {
        arch: ArchTag,
        handle: Arc<dyn std::any::Any + Send + Sync>,
    },
    Clip {
        arch: ArchTag,
        handle: Arc<dyn std::any::Any + Send + Sync>,
    },
    Lora {
        arch: ArchTag,
        handle: Arc<dyn std::any::Any + Send + Sync>,
    },
    Number(f32),
    Text(String),
    Seed(u64),
    Bool(bool),
}

impl std::fmt::Debug for NodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeValue::Latent { arch, .. } => write!(f, "Latent {{ arch: {arch:?}, .. }}"),
            NodeValue::Image(_) => write!(f, "Image(..)"),
            NodeValue::Conditioning(_) => write!(f, "Conditioning(..)"),
            NodeValue::Model { arch, .. } => write!(f, "Model {{ arch: {arch:?}, .. }}"),
            NodeValue::Vae { arch, .. } => write!(f, "Vae {{ arch: {arch:?}, .. }}"),
            NodeValue::Clip { arch, .. } => write!(f, "Clip {{ arch: {arch:?}, .. }}"),
            NodeValue::Lora { arch, .. } => write!(f, "Lora {{ arch: {arch:?}, .. }}"),
            NodeValue::Number(n) => write!(f, "Number({n})"),
            NodeValue::Text(s) => write!(f, "Text({s:?})"),
            NodeValue::Seed(s) => write!(f, "Seed({s})"),
            NodeValue::Bool(b) => write!(f, "Bool({b})"),
        }
    }
}

impl NodeValue {
    /// Reflect the runtime variant back to its [`NodeValueType`].
    pub fn value_type(&self) -> NodeValueType {
        match self {
            NodeValue::Latent { .. } => NodeValueType::Latent,
            NodeValue::Image(_) => NodeValueType::Image,
            NodeValue::Conditioning(_) => NodeValueType::Conditioning,
            NodeValue::Model { .. } => NodeValueType::Model,
            NodeValue::Vae { .. } => NodeValueType::Vae,
            NodeValue::Clip { .. } => NodeValueType::Clip,
            NodeValue::Lora { .. } => NodeValueType::Lora,
            NodeValue::Number(_) => NodeValueType::Number,
            NodeValue::Text(_) => NodeValueType::Text,
            NodeValue::Seed(_) => NodeValueType::Seed,
            NodeValue::Bool(_) => NodeValueType::Bool,
        }
    }
}

// ---------------------------------------------------------------------------
// Schema types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PortSpec {
    pub name: String,
    pub value_type: NodeValueType,
}

#[derive(Clone, Debug)]
pub struct FieldSpec {
    pub name: String,
    pub label: String,
    pub kind: FieldKind,
    pub default: FieldValue,
}

#[derive(Clone, Debug)]
pub struct NodeSchema {
    pub display_name: &'static str,
    /// "Loaders" | "Sampling" | "VAE" | "Image" | "Conditioning"
    pub category: &'static str,
    pub inputs: Vec<PortSpec>,
    pub outputs: Vec<PortSpec>,
    pub fields: Vec<FieldSpec>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    /// Required input port was not connected or upstream produced nothing.
    #[error("missing required input on port `{port}`")]
    MissingInput { port: String },

    /// Input was the wrong NodeValueType for the port.
    #[error("type mismatch on port `{port}`: expected {expected:?}, got {got:?}")]
    TypeMismatch {
        port: String,
        expected: NodeValueType,
        got: NodeValueType,
    },

    /// Architecture mismatch (e.g. SDXL conditioning into Klein sampler).
    /// Distinct from `TypeMismatch` because both ports are `Conditioning`/
    /// `Model`/etc — the architecture discriminant differs.
    #[error("architecture mismatch on port `{port}`: expected {expected}, got {got}")]
    ArchMismatch {
        port: String,
        expected: ArchTag,
        got: ArchTag,
    },

    /// Field value missing from the fields map.
    #[error("missing required field `{name}`")]
    MissingField { name: String },

    /// Field present but wrong `FieldValue` variant. UI shouldn't allow
    /// this; defensive.
    #[error("field `{name}` has wrong kind: expected {expected}")]
    FieldKindMismatch {
        name: String,
        expected: &'static str,
    },

    /// User clicked Cancel mid-execution.
    #[error("execution aborted")]
    Aborted,

    /// Underlying inference-flame / flame-core error.
    #[error("flame error: {0}")]
    Inference(flame_core::FlameError),

    /// I/O error (file not found, save failed, malformed sidecar JSON, …).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for invariants without a dedicated variant.
    #[error("{0}")]
    Other(String),
}

impl From<flame_core::FlameError> for NodeError {
    fn from(e: flame_core::FlameError) -> Self {
        NodeError::Inference(e)
    }
}

// ---------------------------------------------------------------------------
// Progress sink
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressLevel {
    Info,
    Warn,
    Error,
}

pub trait ProgressSink: Send + Sync {
    fn step(&self, current: u32, total: u32, label: &str);
    /// Wave-2 spec collapsed `ImageTensor` into plain `Tensor`.
    fn preview(&self, image: &Tensor);
    fn message(&self, level: ProgressLevel, msg: &str);
}

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

pub trait NodeType: Send + Sync {
    /// Stable type id used by workflow JSON. NEVER change for an existing node.
    fn type_id(&self) -> &'static str;

    /// Schema: ports + fields. Called by UI to draw the node.
    fn schema(&self) -> NodeSchema;

    /// Execute the node. `inputs` come from upstream edges; `fields` from UI.
    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct NodeRegistry {
    types: HashMap<&'static str, Box<dyn NodeType>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    /// Build a registry preloaded with the wave-1 starter nodes.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        r.register(Box::new(builtin::load_checkpoint::LoadCheckpoint));
        r.register(Box::new(builtin::load_lora::LoadLora));
        r.register(Box::new(builtin::encode_prompt::EncodePrompt));
        r.register(Box::new(builtin::k_sampler::KSampler));
        r.register(Box::new(builtin::vae_decode::VaeDecode));
        r.register(Box::new(builtin::save_image::SaveImage));
        r
    }

    pub fn register(&mut self, n: Box<dyn NodeType>) {
        let id = n.type_id();
        self.types.insert(id, n);
    }

    /// Look up a node type by its stable id. Returned reference is borrowed
    /// from the registry; A4 (workflow loader) calls this for type resolution.
    pub fn get(&self, type_id: &str) -> Option<&dyn NodeType> {
        self.types.get(type_id).map(|b| b.as_ref())
    }

    /// Group registered nodes by `schema().category` for UI menus.
    pub fn by_category(&self) -> BTreeMap<&'static str, Vec<&dyn NodeType>> {
        let mut map: BTreeMap<&'static str, Vec<&dyn NodeType>> = BTreeMap::new();
        for n in self.types.values() {
            let cat = n.schema().category;
            map.entry(cat).or_default().push(n.as_ref());
        }
        map
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

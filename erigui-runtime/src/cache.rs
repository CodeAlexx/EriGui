//! Content-hash cache for executed nodes.
//!
//! Per the wave-1 spec ("Cache key" section in `docs/nodes_spec.md`):
//!
//! ```text
//! sha256(type_id || sorted(field_name=hash(field_value)) || sorted(port_name=hash(input_value)))
//! ```
//!
//! The wave-3 executor uses this to skip re-running a node when its
//! `(type_id, fields, inputs)` are bit-identical to the prior run.

use erigui_nodes::{ArchTag, NodeValue};
use erigui_widgets::node_graph::FieldValue;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

use crate::NodeId;

/// 32-byte SHA-256 digest used as the composed cache key.
pub type CacheKey = [u8; 32];

/// One cache slot per node id. We keep the composed key alongside the
/// outputs because the executor compares keys (not pointers / not values)
/// to decide hit vs miss.
#[derive(Clone)]
pub struct CachedOutputs {
    pub key: CacheKey,
    pub outputs: HashMap<String, NodeValue>,
}

/// Per-node cache. Most graphs have <100 nodes; `BTreeMap` is plenty and
/// gives us deterministic iteration if we ever want to dump cache state.
#[derive(Default)]
pub struct Cache {
    entries: BTreeMap<NodeId, CachedOutputs>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Look up a cache entry. Caller compares `entry.key` to a freshly
    /// computed [`composed_key`] — if equal, the entry is a hit and
    /// `entry.outputs` should be reused without calling `execute()`.
    pub fn get(&self, node_id: NodeId) -> Option<&CachedOutputs> {
        self.entries.get(&node_id)
    }

    /// Store outputs for a node, replacing any prior entry. Replacement on
    /// re-run is intentional: the spec stores at most one entry per node.
    pub fn insert(&mut self, node_id: NodeId, key: CacheKey, outputs: HashMap<String, NodeValue>) {
        self.entries.insert(node_id, CachedOutputs { key, outputs });
    }

    /// Remove a cached entry (used when the executor decides to force-rerun).
    pub fn invalidate(&mut self, node_id: NodeId) {
        self.entries.remove(&node_id);
    }

    /// Drop all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// -- composed key -----------------------------------------------------------

/// Compute the wave-1 composed cache key for one node run.
///
/// Inputs are passed by `&HashMap` so the caller doesn't have to clone; we
/// sort by port-name / field-name internally so iteration order isn't
/// observable to the cache key.
pub fn composed_key(
    type_id: &str,
    inputs: &HashMap<String, NodeValue>,
    fields: &HashMap<String, FieldValue>,
) -> CacheKey {
    let mut hasher = Sha256::new();
    hasher.update(type_id.as_bytes());
    hasher.update([0u8]); // domain separator

    // Sort field names for stable hashing.
    let mut field_names: Vec<&String> = fields.keys().collect();
    field_names.sort();
    for name in field_names {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        let h = hash_field_value(&fields[name]);
        hasher.update(h.to_le_bytes());
    }

    hasher.update([1u8]); // separator between fields and inputs

    let mut input_names: Vec<&String> = inputs.keys().collect();
    input_names.sort();
    for name in input_names {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        let h = hash_node_value(&inputs[name]);
        hasher.update(h.to_le_bytes());
    }

    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Hash a single `FieldValue` to a u64 fold-in. We use `DefaultHasher` for
/// the variant payload because the spec only requires "value-stable, run-to-
/// run identical" semantics — collision resistance isn't needed at this
/// inner stage; the SHA-256 outer pass dominates.
fn hash_field_value(v: &FieldValue) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    match v {
        FieldValue::Text(s) => {
            0u8.hash(&mut h);
            s.hash(&mut h);
        }
        FieldValue::Number(n) => {
            1u8.hash(&mut h);
            // f32 has no Hash impl; bit-cast for stable hashing.
            n.to_bits().hash(&mut h);
        }
        FieldValue::Select(s) => {
            2u8.hash(&mut h);
            s.hash(&mut h);
        }
        FieldValue::FilePath(p) => {
            3u8.hash(&mut h);
            p.as_os_str().hash(&mut h);
        }
        FieldValue::Bool(b) => {
            4u8.hash(&mut h);
            b.hash(&mut h);
        }
    }
    h.finish()
}

/// Hash a single `NodeValue` to a u64 fold-in. Mirrors the spec's
/// "Cache key" table:
///
/// | Variant         | Hash input                                        |
/// |-----------------|---------------------------------------------------|
/// | Latent          | (arch, tensor.id().0, tensor.shape().dims())      |
/// | Image(t)        | (t.id().0, t.shape().dims())                      |
/// | Conditioning    | (arch, embeds.id, pooled.map(id), mask.map(id))   |
/// | Model/Vae/Clip/Lora | (arch, Arc-ptr identity)                      |
/// | Number / Text / Seed / Bool | the value itself                      |
fn hash_node_value(v: &NodeValue) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    match v {
        NodeValue::Latent { arch, tensor } => {
            10u8.hash(&mut h);
            arch_tag_byte(*arch).hash(&mut h);
            tensor.id().0.hash(&mut h);
            tensor.shape().dims().hash(&mut h);
        }
        NodeValue::Image(t) => {
            11u8.hash(&mut h);
            t.id().0.hash(&mut h);
            t.shape().dims().hash(&mut h);
        }
        NodeValue::Conditioning(pack) => {
            12u8.hash(&mut h);
            arch_tag_byte(pack.arch).hash(&mut h);
            pack.embeds.id().0.hash(&mut h);
            pack.pooled.as_ref().map(|t| t.id().0).hash(&mut h);
            pack.mask.as_ref().map(|t| t.id().0).hash(&mut h);
        }
        NodeValue::Model { arch, handle } => {
            13u8.hash(&mut h);
            arch_tag_byte(*arch).hash(&mut h);
            (Arc_as_ptr_usize(handle)).hash(&mut h);
        }
        NodeValue::Vae { arch, handle } => {
            14u8.hash(&mut h);
            arch_tag_byte(*arch).hash(&mut h);
            (Arc_as_ptr_usize(handle)).hash(&mut h);
        }
        NodeValue::Clip { arch, handle } => {
            15u8.hash(&mut h);
            arch_tag_byte(*arch).hash(&mut h);
            (Arc_as_ptr_usize(handle)).hash(&mut h);
        }
        NodeValue::Lora { arch, handle } => {
            16u8.hash(&mut h);
            arch_tag_byte(*arch).hash(&mut h);
            (Arc_as_ptr_usize(handle)).hash(&mut h);
        }
        NodeValue::Number(n) => {
            20u8.hash(&mut h);
            n.to_bits().hash(&mut h);
        }
        NodeValue::Text(s) => {
            21u8.hash(&mut h);
            s.hash(&mut h);
        }
        NodeValue::Seed(s) => {
            22u8.hash(&mut h);
            s.hash(&mut h);
        }
        NodeValue::Bool(b) => {
            23u8.hash(&mut h);
            b.hash(&mut h);
        }
    }
    h.finish()
}

fn arch_tag_byte(t: ArchTag) -> u8 {
    match t {
        ArchTag::Klein => 1,
        ArchTag::ZImage => 2,
        ArchTag::Flux1 => 3,
        ArchTag::Sdxl => 4,
        ArchTag::Sd35 => 5,
    }
}

#[allow(non_snake_case)]
fn Arc_as_ptr_usize<T: ?Sized>(arc: &std::sync::Arc<T>) -> usize {
    std::sync::Arc::as_ptr(arc) as *const () as usize
}

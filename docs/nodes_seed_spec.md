# EriGui Node Graph — Seed Spec (Wave 1 working draft)

> Status: SEED. A1 architect agent is refining this; A2-A4 builders code to this draft and reconcile if A1 changes anything load-bearing.

## Goal

Comfy-equivalent diffusion frontend on top of `erigui-widgets/src/node_graph.rs` (which already has Node/Edge/Port/Graph + pan/zoom/drag/drop).

## Crate layout

- `erigui-nodes/` — NEW crate. Trait definitions, registry, 6 starter node implementations. Depends on `inference-flame` for actual model calls.
- `erigui-workflow/` — NEW crate. Workflow JSON serialization with versioning. Depends on `erigui-nodes` for type-id resolution.
- `erigui-widgets/src/node_graph.rs` — modified to render `Field` values as inline widgets and to query `erigui-nodes::NodeRegistry` for the right-click "add node" menu.
- `erigui-runtime/` (deferred to wave 3) — topo-sort executor + progress channel.

## Core trait

```rust
// erigui-nodes/src/lib.rs

pub trait NodeType: Send + Sync {
    /// Stable type ID used by workflow JSON. NEVER change for an existing node.
    fn type_id(&self) -> &'static str;

    /// Schema: ports + fields. Called by UI to draw the node.
    fn schema(&self) -> NodeSchema;

    /// Execute the node. inputs from upstream, fields from UI.
    /// Returns outputs keyed by output port name.
    /// progress sink optional; sampler nodes use it for live denoise progress.
    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError>;
}
```

## Schema types

```rust
pub struct NodeSchema {
    pub display_name: &'static str,
    pub category: &'static str,           // "Loaders" | "Sampling" | "VAE" | "Image" | "Conditioning"
    pub inputs: Vec<PortSpec>,
    pub outputs: Vec<PortSpec>,
    pub fields: Vec<FieldSpec>,
}

pub struct PortSpec {
    pub name: String,
    pub value_type: NodeValueType,
}

pub struct FieldSpec {
    pub name: String,
    pub label: String,
    pub kind: FieldKind,                  // reuse erigui-widgets::FieldKind
    pub default: FieldValue,
}
```

## Value types (the "what flows on edges")

```rust
pub enum NodeValueType {
    Latent, Image, Conditioning, Model, Vae, Clip, Lora, Number, Text, Seed,
}

pub enum NodeValue {
    Latent(Arc<LatentTensor>),            // [B, C, H/8, W/8] f32 or bf16
    Image(Arc<ImageTensor>),              // [B, 3, H, W] u8 or f32
    Conditioning(Arc<ConditioningPack>),  // text embeds + pooled + mask
    Model(Arc<ModelHandle>),              // diffusion backbone (loaded weights)
    Vae(Arc<VaeHandle>),
    Clip(Arc<ClipHandle>),
    Lora(Arc<LoraHandle>),
    Number(f32),
    Text(String),
    Seed(u64),
}
```

The `*Tensor` and `*Handle` types are opaque newtypes wrapping inference-flame's internal types. erigui-nodes never imports flame-core directly — it goes through inference-flame's public API.

## Registry

```rust
pub struct NodeRegistry {
    types: HashMap<&'static str, Box<dyn NodeType>>,
}

impl NodeRegistry {
    pub fn with_builtins() -> Self;       // registers the 6 starter nodes
    pub fn register(&mut self, n: Box<dyn NodeType>);
    pub fn get(&self, type_id: &str) -> Option<&dyn NodeType>;
    pub fn by_category(&self) -> BTreeMap<&'static str, Vec<&dyn NodeType>>;
}
```

## Progress sink

```rust
pub trait ProgressSink: Send + Sync {
    fn step(&self, current: u32, total: u32, label: &str);
    fn preview(&self, image: &ImageTensor);  // optional intermediate denoise preview
    fn message(&self, level: ProgressLevel, msg: &str);
}

pub enum ProgressLevel { Info, Warn, Error }
```

The UI implements `ProgressSink` to push events to a channel; the node-graph widget consumes events to update inline progress bars / image previews.

## 6 starter nodes (signatures only in wave 1)

| type_id              | category      | inputs                                | outputs        | fields                                       |
|----------------------|---------------|---------------------------------------|----------------|----------------------------------------------|
| `core/load_checkpoint` | Loaders     | —                                     | model, clip, vae | path: FilePath                              |
| `core/load_lora`     | Loaders       | model: Model, clip: Clip              | model, clip    | path: FilePath, strength: Number(0..2)       |
| `core/encode_prompt` | Conditioning  | clip: Clip                            | conditioning   | text: Text, negative: Bool                   |
| `core/k_sampler`     | Sampling      | model: Model, positive: Conditioning, negative: Conditioning, latent: Latent | latent | seed: Seed, steps: Number(1..200), cfg: Number(0..30), sampler: Select, scheduler: Select |
| `core/vae_decode`    | VAE           | latent: Latent, vae: Vae              | image          | —                                            |
| `core/save_image`    | Image         | image: Image                          | —              | filename_prefix: Text                        |

Wave 2 implements bodies; wave 1 only nails the schema + stub execute.

## Workflow JSON envelope

```jsonc
{
  "version": 1,
  "nodes": [
    {
      "id": 7,                         // node instance id, unique within workflow
      "type_id": "core/k_sampler",     // looked up in registry
      "position": {"x": 320.0, "y": 140.0},
      "fields": {
        "seed": 42,
        "steps": 30,
        "cfg": 7.0
      }
    }
  ],
  "edges": [
    {"from": {"node": 1, "port": "model"}, "to": {"node": 7, "port": "model"}}
  ]
}
```

- Forward-compat: unknown `type_id` becomes a placeholder node; the workflow loads but warns. Don't crash.
- Field values: any FieldKind serializes as its natural JSON type.
- Tensor outputs are NEVER serialized; only the graph topology + field values.

## Inline widgets in nodes

Node body lays out:
1. Title bar (existing)
2. Input ports column (left edge, existing)
3. **Field stack** (center, NEW) — one row per `FieldSpec`:
   - `Text` → text_input widget
   - `Number {min,max,step}` → slider widget
   - `Select { options }` → combo_box widget
   - `FilePath` → button that opens file_dialog
4. Output ports column (right edge, existing)

All widgets are existing erigui-widgets primitives — no new widget code.

## Open questions for A1 to nail down

1. **Tensor sharing**: `Arc<LatentTensor>` lets multiple downstream nodes consume the same output without copy. But inference-flame tensors are CUDA-resident — is `Arc<>` the right wrapper, or do we need explicit lifetime management? A1 should pick.
2. **NodeError variants**: enumerate. At minimum: missing-input, type-mismatch, executor-aborted, inference-flame-error.
3. **Re-execution policy**: if user changes a field in node X, does executor re-run X and everything downstream, or wait for explicit "Queue" button? (Comfy = explicit Queue. I default to that.)
4. **Threading**: is `execute()` called on UI thread or worker pool? I assume worker pool — that's why the `Send + Sync` bound. A1 confirm.
5. **Cache key**: deterministic hash of (type_id + field values + input value hashes). Each `NodeValue` needs a `content_hash()` method. A1 propose what that looks like for Tensor types (which are opaque).

## What this seed does NOT specify (deferred to later waves)

- Executor topo-sort algorithm (wave 3)
- Live progress channel routing UI side (wave 3)
- Image preview texture upload to GL (wave 3)
- Right-click "add node" search menu integration (wave 3)
- Custom-node plugin system (wave 4 or never)

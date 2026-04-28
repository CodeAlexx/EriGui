# EriGui Node Graph — Wave 1 Final Spec

> Status: FINAL for wave 1. A1 architect refined the seed at `nodes_seed_spec.md`. A2-A5 code to THIS file.
> Crate layout, `NodeType` trait shape, and the 6 starter type_ids are unchanged from the seed (A2 is mid-flight on those).

## Decisions (vs seed)

| # | Topic | Decision | Why |
|---|-------|----------|-----|
| 1 | Tensor sharing | `Tensor` flows by **value-clone** (it's already Arc-internal); models flow as `Arc<T>` because they aren't `Clone`. No `Arc<LatentTensor>` newtype layer. | `flame_core::Tensor` is `#[derive(Clone)]` with Arc-shared `TensorStorage` — wrapping it in another `Arc` is double indirection. |
| 2 | Tensor newtypes | Replaced opaque `LatentTensor`/`ImageTensor` with **a tagged enum on architecture** (`Latent { arch, tensor }`) and `Image` is plain `Tensor`. `Model`/`Vae`/`Clip`/`Lora` are `Arc<dyn Any + Send + Sync>` + an `arch: ArchTag` discriminator. | inference-flame has no unified `Model` type — every architecture has its own struct. ArchTag enables type-mismatch errors at edge-connect time before execution. |
| 3 | Re-execution | **Explicit Queue button.** | Comfy precedent + sampler nodes are 30+s. Auto-rerun would thrash. |
| 4 | Threading | **Worker pool.** Confirmed. `execute()` is `Send`-only via the trait bound; no `Sync` requirement on the *call*. UI thread enqueues; one job runs at a time on a single worker thread. | Single worker (not pool) because GPU is the bottleneck — parallel `execute()` calls would just contend on the same CudaDevice. |
| 5 | Cache key | **TensorId-based version counter** for tensor-bearing values (not GPU-memory hash). For handles, hash the load-source path + file mtime. For scalars/text, hash the value. | `flame_core::Tensor::id` is a monotonic `TensorId` already assigned at every constructor — exactly the "version counter assigned at creation" scheme. |
| 6 | NodeError variants | See §NodeError. | — |
| 7 | FieldValue extension | Add `Bool(bool)` and `FilePath(PathBuf)` variants to `erigui-widgets::FieldValue`. | Seed's `negative: Bool` and the `FilePath` field on loaders had no representation in the existing enum — would have blocked A2. |
| 8 | NodeId in workflow JSON | Switch from `usize` (existing widget) to `u32`. | `usize` is target-dependent; serialized graphs must be portable. |

## Critical: Seed Spec Issue

**1. The seed's `core/load_lora` node treats LoRA as a separate stage that wraps a model.** Looking at `inference-flame/src/bin/zimage_lora_infer.rs`, LoRAs are loaded *before* the model is constructed (`NextDiT::new_resident(weights)` then `model.set_lora(stack)`). LoRA is a **modifier on model construction**, not a post-construction transform. There is no public "apply LoRA to already-built model" path; `set_lora` exists but the current usage builds the LoRA stack against the raw `weights: HashMap<String, Tensor>` *before* construction.

**Resolution for wave 1**: keep the `core/load_lora` schema as the seed has it (it's the right user-facing UX — Comfy does the same). Wave 2 implements it via `Arc<Mutex<Model>>` and an internal `apply_lora_to_built_model` helper in inference-flame that A5 (or whoever owns wave 2) adds. Note this in the wave-2 work entry. Schema doesn't change; the implementation contract does.

**2. `FieldValue` enum in `erigui-widgets/src/node_graph.rs:64-68` has no `Bool` and no `FilePath` variant.** The seed schema uses both. Wave 1 must extend the existing enum. This is a small but breaking change to the widget crate — A4 (workflow serializer) and A2 (node implementations) both depend on it. **Action**: edit `erigui-widgets::FieldValue` to add `Bool(bool)` and `FilePath(PathBuf)`. No other variants needed.

Otherwise the seed is sound.

## Crate layout (unchanged)

- `erigui-nodes/` — trait definitions, registry, 6 starter node implementations. Depends on `inference-flame`.
- `erigui-workflow/` — workflow JSON serialization with versioning. Depends on `erigui-nodes`.
- `erigui-widgets/src/node_graph.rs` — extend `FieldValue` (Bool, FilePath); add inline-widget rendering and registry-driven add-menu hook.
- `erigui-runtime/` — deferred to wave 3.

## Core trait (unchanged)

```rust
pub trait NodeType: Send + Sync {
    fn type_id(&self) -> &'static str;
    fn schema(&self) -> NodeSchema;
    fn execute(
        &self,
        inputs: &HashMap<String, NodeValue>,
        fields: &HashMap<String, FieldValue>,
        progress: Option<&dyn ProgressSink>,
    ) -> Result<HashMap<String, NodeValue>, NodeError>;
}
```

## Schema types (unchanged from seed)

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
    pub kind: FieldKind,                  // erigui-widgets::FieldKind
    pub default: FieldValue,              // erigui-widgets::FieldValue (extended; see §FieldValue)
}
```

## NodeValueType (refined)

```rust
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
```

Type-compat at edge-connect time: ports of the same `NodeValueType` may connect; cross-type connect rejected by the widget. Architecture compat (e.g. SDXL Model → Klein KSampler) is checked at `execute()` time, not connect time, because architecture is runtime data.

## NodeValue (refined)

```rust
#[derive(Clone)]
pub enum NodeValue {
    Latent  { arch: ArchTag, tensor: Tensor },          // [B, C, H/8, W/8] BF16 in flame_core::Tensor
    Image   ( Tensor ),                                  // [B, 3, H, W] BF16 or F32; arch-agnostic
    Conditioning(Arc<ConditioningPack>),
    Model   { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> },
    Vae     { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> },
    Clip    { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> },
    Lora    { arch: ArchTag, handle: Arc<dyn Any + Send + Sync> },
    Number(f32),
    Text(String),
    Seed(u64),
    Bool(bool),
}

pub enum ArchTag {
    Klein,
    ZImage,
    Flux1,
    Sdxl,
    Sd35,
    // grow as wave 2/3 adds nodes
}

pub struct ConditioningPack {
    pub embeds: Tensor,             // [B, T, D] BF16
    pub pooled: Option<Tensor>,
    pub mask: Option<Tensor>,
    pub arch: ArchTag,
}
```

`Tensor` clones are O(1) (Arc-shared storage). `Model`/`Vae`/`Clip`/`Lora` are `Arc<dyn Any + Send + Sync>` because each architecture has its own concrete type (`NextDiT`, `LdmVAEDecoder`, `LoraStack`, etc.) and unifying them under a trait would force a 30-method API surface this wave can't justify. Each starter node downcasts on the way in:

```rust
let model: &NextDiT = handle.downcast_ref::<NextDiT>()
    .ok_or(NodeError::TypeMismatch { expected: "NextDiT", got: arch.into() })?;
```

The `arch` discriminator lets a node fail fast with a readable error before downcasting.

## NodeError

```rust
pub enum NodeError {
    /// Required input port was not connected or upstream produced nothing.
    MissingInput { port: String },

    /// Input was the wrong NodeValueType for the port (caught earlier by edge validation,
    /// but possible if downstream type changed via field edit).
    TypeMismatch { port: String, expected: NodeValueType, got: NodeValueType },

    /// Architecture mismatch (e.g. SDXL conditioning into Klein sampler). Distinct from
    /// TypeMismatch because both ports are `Conditioning`/`Model`/etc — the discriminant differs.
    ArchMismatch { port: String, expected: ArchTag, got: ArchTag },

    /// Field value missing from the fields map.
    MissingField { name: String },

    /// Field present but wrong FieldValue variant (UI shouldn't allow this; defensive).
    FieldKindMismatch { name: String, expected: &'static str },

    /// User clicked Cancel mid-execution.
    Aborted,

    /// Underlying inference-flame / flame-core error.
    Inference(flame_core::Error),

    /// I/O error (file not found, save failed).
    Io(std::io::Error),

    /// Catch-all for invariants the node author wants to express without inventing variants.
    Other(String),
}

// flame_core::Error implements Display + Debug; auto-conversion via `From`.
impl From<flame_core::Error> for NodeError { /* ... */ }
impl From<std::io::Error>     for NodeError { /* ... */ }
```

## Re-execution policy

**Explicit Queue button** (Comfy semantics).

- Editing a field marks the node "dirty" and propagates dirtiness downstream (visual hint only).
- Clicking Queue runs all dirty nodes in topo order (executor lives in wave 3; for wave 1 the runner is a stub that walks the graph in input-order and calls `execute()` directly).
- Saved cache keys (see §Cache key) skip re-execution of nodes whose inputs and fields are bit-identical to the prior run.

Auto-rerun is rejected: a 30-step k_sampler triggered on every keystroke in the prompt field would render the UI unusable.

## Threading

- `execute()` is called on **a single worker thread**, not the UI thread.
- Trait bound is `Send + Sync` on `dyn NodeType` (registry needs Sync to hand out `&dyn NodeType` references across threads); the **values** flowing through `inputs`/`fields` are owned, no lifetimes leak.
- `ProgressSink` is `Send + Sync` (the UI side wraps a channel sender).
- A future wave 3 may parallelize independent branches across multiple workers; for now: one worker, FIFO queue. The single CUDA device makes parallel `execute()` calls a contention loss anyway.

## Cache key / `content_hash()`

Each `NodeValue` exposes:

```rust
impl NodeValue {
    pub fn content_hash(&self) -> u64 { ... }
}
```

**Strategy by variant**:

| Variant | Hash input |
|---------|------------|
| `Latent { arch, tensor }` | `(arch, tensor.id().0, tensor.shape().dims())` |
| `Image(t)` | `(t.id().0, t.shape().dims())` |
| `Conditioning(pack)` | `(arch, embeds.id().0, pooled.as_ref().map(|t| t.id().0), mask.as_ref().map(|t| t.id().0))` |
| `Model`/`Vae`/`Clip`/`Lora` | The hash provided by the **producing node**, recorded against the `Arc` ptr identity. (Nodes that load a handle hash `(path, mtime, fields)` and stash that as the node's output hash.) |
| `Number(n)` | `n.to_bits() as u64` |
| `Text(s)` | xxhash of bytes |
| `Seed(s)` | `s` |
| `Bool(b)` | `b as u64` |

**Why TensorId works**: every flame_core constructor (`Tensor::randn`, `Tensor::zeros`, every kernel output) calls `TensorId::new()` which is a monotonic atomic counter. Two `Tensor`s produced by the same op on the same inputs will have *different* IDs but identical *contents* — that's fine because we never need "same content, different tensor" equality; the executor only cares "is this the same value the cache saw?". Same-`Arc`-storage tensors share the same `TensorId` (cloned, not recomputed). So the cache hit case (identical inputs feeding a deterministic op) doesn't apply at the tensor level — it applies at the **node-output level**, where the executor compares `(node_id, input_hashes, field_hashes)` to a stored entry and reuses the saved `NodeValue` directly. We never re-hash GPU memory.

**Composed cache key** for a node run:

```
sha256(type_id || sorted(field_name=hash(field_value)) || sorted(port_name=hash(input_value)))
```

Stored as `[u8; 32]`. Wave 1 ships `content_hash()` only; the persistent cache itself is wave 3.

## FieldValue (extended)

The widget enum becomes:

```rust
// erigui-widgets/src/node_graph.rs
pub enum FieldValue {
    Text(String),
    Number(f32),
    Select(String),
    Bool(bool),                  // NEW
    FilePath(std::path::PathBuf),// NEW
}
```

`FieldKind` already has `FilePath`; it just had no value to round-trip. Adding `Bool(bool)` is needed for `core/encode_prompt::negative` and any future toggle. Serde `tag` auto-handled by serde derive.

## Registry (unchanged)

```rust
pub struct NodeRegistry {
    types: HashMap<&'static str, Box<dyn NodeType>>,
}
impl NodeRegistry {
    pub fn with_builtins() -> Self;
    pub fn register(&mut self, n: Box<dyn NodeType>);
    pub fn get(&self, type_id: &str) -> Option<&dyn NodeType>;
    pub fn by_category(&self) -> BTreeMap<&'static str, Vec<&dyn NodeType>>;
}
```

## ProgressSink (unchanged)

```rust
pub trait ProgressSink: Send + Sync {
    fn step(&self, current: u32, total: u32, label: &str);
    fn preview(&self, image: &Tensor);
    fn message(&self, level: ProgressLevel, msg: &str);
}
pub enum ProgressLevel { Info, Warn, Error }
```

`preview` takes `&Tensor` directly (was `&ImageTensor` in seed; collapsed since `Image` is now plain Tensor).

## 6 starter nodes (signatures unchanged; field types pinned)

| type_id | category | inputs | outputs | fields |
|---------|----------|--------|---------|--------|
| `core/load_checkpoint` | Loaders | — | model, clip, vae | `path: FilePath` |
| `core/load_lora` | Loaders | model: Model, clip: Clip | model, clip | `path: FilePath`, `strength: Number{0..2, step:0.05}` |
| `core/encode_prompt` | Conditioning | clip: Clip | conditioning | `text: Text`, `negative: Bool` |
| `core/k_sampler` | Sampling | model: Model, positive: Conditioning, negative: Conditioning, latent: Latent | latent | `seed: Seed (default 42)`, `steps: Number{1..200, step:1}`, `cfg: Number{0..30, step:0.5}`, `sampler: Select [euler, euler_a, dpmpp_2m]`, `scheduler: Select [normal, karras, exponential]` |
| `core/vae_decode` | VAE | latent: Latent, vae: Vae | image | — |
| `core/save_image` | Image | image: Image | — | `filename_prefix: Text (default "EriGui")` |

Wave 1 implements stub bodies that:
- Validate inputs (return appropriate `NodeError`).
- For loaders: open the file, return `Arc<()>` placeholder + correct `ArchTag`. (Real load wave 2.)
- For ops: return correctly-shaped zero tensors with the right `ArchTag`.

This lets A4's workflow serializer round-trip a real graph and A2's UI renderer draw real nodes without wave 2 being complete.

## Workflow JSON envelope (refined)

```jsonc
{
  "version": 1,
  "nodes": [
    {
      "id": 7,                         // u32
      "type_id": "core/k_sampler",
      "position": {"x": 320.0, "y": 140.0},
      "fields": {
        "seed":    {"Seed":   42},
        "steps":   {"Number": 30},
        "cfg":     {"Number": 7.0},
        "sampler": {"Select": "euler"},
        "scheduler": {"Select": "normal"}
      }
    }
  ],
  "edges": [
    {"from": {"node": 1, "port": "model"}, "to": {"node": 7, "port": "model"}}
  ]
}
```

- Forward-compat: unknown `type_id` becomes a placeholder node with the original JSON kept verbatim in `_unknown` field; loads with a warning, doesn't crash.
- Field values use serde-default tagged form for `FieldValue`.
- Tensor outputs are NEVER serialized.
- Field maps are `BTreeMap`-ordered on save for deterministic diffs.

## Inline widgets in nodes (unchanged)

Node body lays out:
1. Title bar (existing).
2. Input ports column (left, existing).
3. **Field stack** (center, NEW) — one row per `FieldSpec`:
   - `Text` → text_input.
   - `Number {min,max,step}` → slider.
   - `Select { options }` → combo_box.
   - `Bool` → checkbox (new field-kind handler — checkbox primitive already exists in erigui-widgets).
   - `FilePath` → button → file_dialog.
4. Output ports column (right, existing).

## Deferred to later waves

- Executor topo-sort + persistent cache (wave 3, owns `erigui-runtime/`).
- Live progress channel routing UI side (wave 3).
- Image preview texture upload to GL (wave 3).
- Right-click "add node" search menu integration (wave 3).
- Custom-node plugin system (wave 4 or never).
- Architecture-compat matrix at edge-connect time (currently runtime-only).
- LoRA application to already-built models (needs an inference-flame helper; wave 2).

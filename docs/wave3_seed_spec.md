# EriGui Node Graph — Wave 3 Seed Spec

> Status: SEED. Wave 3 makes the graph actually usable as a UI: executor, inline image preview, live progress, right-click add-node menu.
> Wave 2 finished: 6 starter nodes have real Klein bodies. Workspace builds clean.

## Crate layout (additive)

- **NEW**: `erigui-runtime/` — executor + content-hash cache + worker thread + Queue button bridge.
- **EXTENDED**: `erigui-widgets/src/node_graph.rs` — image preview rendering inside Image-output nodes, progress bar inside in-progress nodes, right-click "add node" menu integration.
- **EXTENDED**: `erigui-nodes/` — minor: re-export ProgressSink + content-hash trait so erigui-runtime can use them.

## C1: Executor (`erigui-runtime/`)

```rust
pub struct Executor {
    registry: Arc<NodeRegistry>,
    cache: HashMap<NodeId, CacheEntry>,        // keyed by node-id, value = (input_hashes_at_run, output_values, output_hashes)
    worker_tx: Sender<ExecMessage>,
    progress_rx: Receiver<ProgressEvent>,      // UI side polls this
}

pub enum ExecMessage {
    Run { graph: Graph, dirty: HashSet<NodeId> },
    Cancel,
}

pub enum ProgressEvent {
    NodeStart   { node_id: NodeId },
    NodeStep    { node_id: NodeId, current: u32, total: u32, label: String },
    NodePreview { node_id: NodeId, image: Tensor },   // optional intermediate
    NodeDone    { node_id: NodeId, outputs: Vec<(String, NodeValue)> },
    NodeError   { node_id: NodeId, error: NodeError },
    QueueIdle,
}

impl Executor {
    pub fn new(registry: Arc<NodeRegistry>) -> Self;
    pub fn enqueue(&self, graph: Graph, dirty: HashSet<NodeId>) -> Result<(), ExecError>;
    pub fn poll_progress(&self) -> Vec<ProgressEvent>;     // drains the channel, non-blocking
    pub fn cancel(&self);
}
```

**Topo-sort**: standard Kahn's algorithm over `Graph.edges`. If cycle detected → `ExecError::Cycle`. Run nodes in topo order on the worker thread.

**Cache hit decision** per node:
1. Compute `input_hashes`: for each input port, `inputs[port].content_hash()`.
2. Compute `field_hashes`: `(name, value.hash())` sorted by name.
3. Compute `composed_key`: `sha256(type_id || sorted(input_hashes) || sorted(field_hashes))`.
4. If `cache[node_id].composed_key == composed_key` → reuse stored outputs; emit `NodeDone` with the cached values. No `execute()` call.
5. Else → call `execute()`, store outputs + composed_key in cache.

**Dirty propagation**: a node is dirty if (a) user edited a field, (b) any upstream node was re-executed (re-running upstream invalidates the cache key for downstream because input_hashes changed). The UI tells the executor which nodes the user explicitly dirtied via the `dirty` set on `enqueue`; the executor expands dirty to all transitive descendants.

**ProgressSink for the running node**: a small struct that wraps `Sender<ProgressEvent>` + the `node_id` of the currently-running node; passed as `Some(&sink)` into `execute()`.

**Worker thread**: single thread per A1's wave-1 spec (single CudaDevice, parallel = contention loss). FIFO queue.

**Tests**:
- `topo_sort_linear` — 3 nodes A→B→C, expect order [A, B, C]
- `topo_sort_diamond` — A→B, A→C, B→D, C→D, expect any topo order with A first, D last
- `cycle_detection` — A→B→A, expect `ExecError::Cycle`
- `cache_hit_skips_execute` — manual fake registry with a counter on execute(); run twice, assert counter == 1
- `field_change_invalidates_cache` — run, change a field, run again, assert counter == 2
- All tests are pure CPU (no GPU); fake `NodeType` impls.

## C2: Image preview in nodes

When a node outputs a `NodeValue::Image(tensor)`, the node's body in the canvas should display it inline.

**Architecture**:
- `node_graph::NodeGraph` gains a `node_image_textures: HashMap<NodeId, Texture>` — a GL texture handle per node-with-image-output.
- After executor emits `NodeDone { outputs }`, the graph widget extracts any `NodeValue::Image` and uploads to a GL texture (downsample to 256x256 max for in-canvas display). Original full-resolution tensor stays in `cache` for SaveImage.
- `draw_nodes()` queries `node_image_textures` for each node; if present, renders inside the node body below the field stack.

**Texture upload**: BF16/F32 tensor → CPU u8 RGB (same as B6 conversion) → `gl::TexImage2D`. The texture upload is on the UI thread; pushing a `NodeValue::Image` event from worker triggers a UI-thread upload via the existing widget-event loop.

**Node height auto-sizes** to fit preview (existing field-row layout extended).

**Tests** (no GPU required if you fake the texture upload via a trait):
- `image_output_creates_texture` — fake node that emits `NodeValue::Image`, executor runs it, verify `node_image_textures[node_id]` exists
- `repeated_run_replaces_texture` — run twice with different output, verify texture is replaced not duplicated
- Real GL upload only verified manually (GUI required).

## C3: Live progress

When `KSampler` (or any node) calls `progress.step(current, total, label)` mid-execute, the node body in the canvas shows a progress bar.

**Architecture**:
- Add a `node_progress: HashMap<NodeId, NodeProgress>` to `NodeGraph` where `NodeProgress = { current, total, label }`.
- `draw_nodes()` renders a thin progress bar above the field stack when present.
- UI thread drains `executor.poll_progress()` per frame; on `NodeStep`, update `node_progress[node_id]`. On `NodeDone`/`NodeError`, clear it.
- Stretch goal: optional `NodePreview { image }` events update an in-progress preview texture (same path as C2).

**Tests**:
- `progress_event_updates_node_state` — manually inject NodeStep events, verify NodeGraph state updates
- `progress_clears_on_done` — inject NodeStep then NodeDone, verify NodeGraph clears
- `progress_clears_on_error` — inject NodeStep then NodeError, verify NodeGraph clears + paints error indicator

## C4: Right-click "add node" search menu

Right-click on canvas (not on a node) → menu pops up with searchable list of registered node types, grouped by category.

**Architecture**:
- `NodeGraph` gets `add_node_menu: Option<AddNodeMenuState>` field.
- Right-click handler: if click landed on canvas background (not a node, not a port), open `add_node_menu` at the click position.
- `AddNodeMenuState`: a `search_box` widget (existing primitive) + scrollable filtered list grouped by `category`.
- Selecting a type → call `NodeRegistry::get(type_id).schema()` to seed a new `Node` with default fields, insert at click position, close menu.
- Esc / click-outside → close.

**Tests**:
- `right_click_canvas_opens_menu` — fire MousePress::Right at empty canvas, verify menu state present
- `right_click_node_does_not_open_menu` — fire MousePress::Right on a node, verify menu state absent (the node's context menu opens instead, but that's separate)
- `search_filters_list` — populate registry with 6 mock types across 3 categories, type "k", verify only kinds containing "k" remain visible
- `select_inserts_node` — select an item, verify `Graph.nodes` has one more entry with correct schema

## Cross-cutting

- **erigui-runtime depends on erigui-nodes** (NodeRegistry, NodeType, NodeValue, ProgressSink, NodeError).
- **erigui-runtime is pure CPU** — no GPU, no inference-flame. Just orchestrates.
- **Workspace Cargo.toml**: add `erigui-runtime` to members.
- **No env-gates / feature flags**.
- **Threading**: executor on its own worker thread. Channels (`std::sync::mpsc` or `crossbeam`) bridge to UI.

## Files each builder owns

| Builder | Files written |
|---------|---------------|
| C1 (executor) | NEW crate `erigui-runtime/` (Cargo.toml + src/lib.rs + tests) |
| C2 (image preview) | `erigui-widgets/src/node_graph.rs` (additive) + maybe a small `image_upload.rs` helper |
| C3 (live progress) | `erigui-widgets/src/node_graph.rs` (additive) — coordinate with C2 since both touch this file |
| C4 (add-node menu) | `erigui-widgets/src/node_graph.rs` (additive) — coordinate with C2/C3 |
| Skeptic | reads only |

## Coordination warning

**C2, C3, C4 all touch `erigui-widgets/src/node_graph.rs`.** This is the same conflict pattern that bit wave 2.

Resolution: each agent writes a separate `impl NodeGraph` block in a separate sub-section of the file, or — better — each agent puts their additions in a separate file and adds it as a module:
- `node_graph/preview.rs` (C2)
- `node_graph/progress.rs` (C3)
- `node_graph/add_menu.rs` (C4)

Then `node_graph.rs` becomes `mod preview; mod progress; mod add_menu;` plus the existing struct/impl. C2/C3/C4 each touch their own file + add one `mod` line at top of node_graph.rs.

The first one to land claims the `pub mod node_graph;` reorg; later ones just add their `mod X;` line.

## What wave 3 does NOT do

- Persistent cache to disk (in-memory only; lost on app restart).
- Custom node plugin system (wave 4 if ever).
- Multi-architecture nodes (still Klein-only — wave 4 adds ZImage/Flux/SD3/etc.).
- Shader-side image preview (CPU upload to GL texture is fine for now).

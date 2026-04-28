use crate::checkbox::Checkbox;
use crate::combo_box::ComboBox;
use crate::file_dialog::{FileDialog, FileDialogMode};
use crate::slider::Slider;
use crate::text_input::TextInput;
use erigui_core::{
    Color, DragData, DragDropEvent, DrawContext, Event, EventResult, Key, KeyPressEvent,
    LayoutConstraints, Modifiers, MouseButton, MouseButtonEvent, Point, Rect, Size, Theme, Widget,
    WidgetId, WidgetState,
};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Wave 3 (C3): live per-node progress bars driven by the executor.
// The submodule defines `NodeProgress`, the setter API, and the bar
// renderer; this `mod` line and the corresponding `node_progress` field
// on `NodeGraph` (initialized below) are the only edits made here.
mod progress;
pub use progress::NodeProgress;

// Wave 3 (C4): right-click "add node" search menu. Self-contained submodule;
// see `add_menu.rs` for the menu state, registry-handle trait, and tests.
pub mod add_menu;
pub use add_menu::{
    AddMenuFieldSpec, AddMenuPortSpec, AddMenuSchema, AddNodeMenuState, NodeRegistryHandle,
    RegistryEntry,
};

// Wave 3 (C2): inline image-preview rendering for nodes that emit
// `NodeValue::Image`. The submodule defines `NodeImagePreview`,
// `upload_image_to_texture`, and the `set_node_image` / `clear_node_image`
// methods on `NodeGraph`. The `node_image_textures` field on `NodeGraph`
// (initialized below) and the `draw_node_image_preview` invocation inside
// `draw_nodes` are the only edits made here.
mod preview;
pub use preview::{upload_image_to_texture, NodeImagePreview, PreviewError};

/// Per-field stateful widget instance cached across frames.
/// Holds cursor / drag / dropdown / checked state that must persist between draws.
enum FieldWidgetEntry {
    Text(TextInput),
    Number(Slider),
    Select(ComboBox),
    Bool(Checkbox),
    /// FilePath uses the existing inline file_dialog flow; the entry stores no state.
    FilePath,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Port {
    pub id: usize,
    pub label: String,
    pub is_input: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub title: String,
    pub position: Point,
    pub size: Size,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub component_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_node: usize,
    pub from_port: usize,
    pub to_node: usize,
    pub to_port: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub id: usize,
    /// Programmatic key (matches FieldSpec.name in erigui-nodes). Used
    /// by the executor when building the per-node fields HashMap. May
    /// differ from `label` (e.g. name="path", label="Path"). Defaults
    /// to empty string for back-compat with workflows saved before
    /// this field was added; in that case the executor falls back to
    /// using `label` as the key.
    #[serde(default)]
    pub name: String,
    pub label: String,
    pub kind: FieldKind,
    pub value: FieldValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FieldKind {
    Text,
    Number { min: f32, max: f32, step: f32 },
    Select { options: Vec<String> },
    /// File-picker field. `extensions` is a hint passed to the file dialog
    /// (e.g. `["safetensors", "ckpt"]`); empty = no filter.
    FilePath {
        #[serde(default)]
        extensions: Vec<String>,
    },
    /// Boolean toggle, rendered inline as a checkbox.
    Bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FieldValue {
    Text(String),
    Number(f32),
    Select(String),
    /// Typed file-path value. Existing flows that wrote `FieldValue::Text(path)`
    /// for FilePath fields are still accepted for back-compat.
    FilePath(PathBuf),
    Bool(bool),
}

#[derive(Clone, Debug)]
struct PreviewBitmap {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphState {
    pub graph: Graph,
    pub pan: Vec2f,
    pub zoom: f32,
    pub groups: Vec<Group>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PortRef {
    node_id: usize,
    port_id: usize,
    is_input: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FieldRef {
    node_id: usize,
    field_id: usize,
}

#[derive(Clone, Debug)]
enum Interaction {
    NodeDrag {
        node_id: usize,
        last_world: Point,
    },
    NodeResize {
        node_id: usize,
        last_world: Point,
    },
    LinkDrag {
        from: PortRef,
        cursor: Point,
    },
    Pan {
        last: Point,
        moved: bool,
    },
    Marquee {
        start_world: Point,
        current_world: Point,
    },
    /// Wave 1 (A3): user pressed inside a Number field; mouse-move events are
    /// forwarded to the cached Slider until release.
    FieldSliderDrag {
        fref: FieldRef,
    },
}

/// Process-wide monotonic origin used to derive the caret blink phase.
/// Cheap to compute once via `LazyLock`; `Instant::elapsed` is a single
/// syscall on Linux. Renderer reads `BLINK_EPOCH.elapsed()` per frame —
/// no allocation, no global mutex.
static BLINK_EPOCH: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

/// Direction for cursor movement in [`FieldEditState`]. Modeled on egui's
/// `CursorRange`/`Galley` movement primitives, simplified to single-line text:
/// no Up/Down rows, no PageUp/PageDown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorMove {
    /// One char left.
    Left,
    /// One char right.
    Right,
    /// Start of text (always 0).
    Home,
    /// End of text (always `text.len()`).
    End,
    /// Previous word boundary.
    WordLeft,
    /// Next word boundary.
    WordRight,
}

/// Per-field editor state for the inline Text/FilePath/Number cells inside a
/// node. Replaces the old flat `String` "type appends, click commits" model
/// with a real cursor + selection + undo/redo stack. Single-line only —
/// node fields are not multi-line, so we do not track rows or wrap.
///
/// Cursor is a **byte** offset into `text`, but movement steps over **chars**
/// (UTF-8 safe via `text.char_indices()`). Selection is whatever lies between
/// `selection_anchor` (if `Some`) and `cursor`, in either order.
///
/// Ported (lightly) from `egui-0.29.1`'s `TextEditState` + the `events()`
/// dispatch in `widgets/text_edit/builder.rs`. Egui's full state machine
/// (galley-aware cursor, IME, multi-row navigation, paragraph deletes) is
/// intentionally **not** copied — see the bottom of this file's
/// `field_edit_basic` test for the contract we do honor.
#[derive(Clone, Debug)]
pub struct FieldEditState {
    pub text: String,
    /// Byte offset in `text` where the cursor sits. Always on a char
    /// boundary; methods that move it use `char_indices()`.
    pub cursor: usize,
    /// If `Some`, selection runs between `selection_anchor` and `cursor`
    /// (either order is valid; helpers normalize).
    pub selection_anchor: Option<usize>,
    /// Undo stack: snapshots of `(text, cursor)` taken before each
    /// text-changing event. `redo` is the inverse stack, populated when
    /// `undo()` succeeds.
    pub undo: Vec<(String, usize)>,
    pub redo: Vec<(String, usize)>,
    /// Last edit/cursor-move time, in seconds since process start. Used to
    /// pause cursor blink while the user is actively editing (egui does the
    /// same in `TextEditState::last_edit_time`).
    pub last_edit: std::time::Instant,
}

impl FieldEditState {
    /// New state with the cursor placed at the **end** of `text` and no
    /// selection. Matches the typical click-into-edit behavior for a
    /// previously-committed value.
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            selection_anchor: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: std::time::Instant::now(),
        }
    }

    /// Push the current `(text, cursor)` onto the undo stack and clear
    /// redo. Called before any mutation that should be reversible.
    pub fn push_undo(&mut self) {
        self.undo.push((self.text.clone(), self.cursor));
        self.redo.clear();
        // Cap stack growth — single-line fields rarely accumulate huge
        // histories, but a runaway loop shouldn't OOM.
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some((prev_text, prev_cursor)) = self.undo.pop() {
            self.redo.push((self.text.clone(), self.cursor));
            self.text = prev_text;
            self.cursor = prev_cursor;
            self.selection_anchor = None;
            self.last_edit = std::time::Instant::now();
        }
    }

    pub fn redo(&mut self) {
        if let Some((next_text, next_cursor)) = self.redo.pop() {
            self.undo.push((self.text.clone(), self.cursor));
            self.text = next_text;
            self.cursor = next_cursor;
            self.selection_anchor = None;
            self.last_edit = std::time::Instant::now();
        }
    }

    /// Normalized selection range (start <= end) if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            None
        } else {
            Some((anchor.min(self.cursor), anchor.max(self.cursor)))
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection_range().is_some()
    }

    pub fn selected_text(&self) -> &str {
        match self.selection_range() {
            Some((a, b)) => &self.text[a..b],
            None => "",
        }
    }

    /// Replace the current selection (or insert at cursor if no selection)
    /// with `s`. Snapshots undo first.
    pub fn replace_selection(&mut self, s: &str) {
        self.push_undo();
        if let Some((a, b)) = self.selection_range() {
            self.text.replace_range(a..b, s);
            self.cursor = a + s.len();
        } else {
            self.text.insert_str(self.cursor, s);
            self.cursor += s.len();
        }
        self.selection_anchor = None;
        self.last_edit = std::time::Instant::now();
    }

    /// Identical to `replace_selection` but named per the task spec.
    pub fn insert_text(&mut self, s: &str) {
        self.replace_selection(s);
    }

    /// Expand selection to the word containing `byte_pos`. For
    /// double-click word-select. Different from Ctrl+arrow word-jump:
    /// this selects ONLY the word (no trailing whitespace/punctuation).
    /// No-op when the click lands on whitespace or non-word chars.
    pub fn select_word_at(&mut self, byte_pos: usize) {
        if self.text.is_empty() {
            return;
        }
        let pos = byte_pos.min(self.text.len());

        // Determine if pos lands inside a word. If pos == text.len(),
        // peek at the previous char so a click past the end of a word
        // still selects that word.
        let char_at = self.text[pos..]
            .chars()
            .next()
            .or_else(|| self.text[..pos].chars().next_back());
        let in_word = char_at.map(is_word_char).unwrap_or(false);
        if !in_word {
            return;
        }

        // Walk left from pos while previous char is a word char.
        let mut start = pos;
        for (i, c) in self.text[..pos].char_indices().rev() {
            if is_word_char(c) {
                start = i;
            } else {
                break;
            }
        }

        // Walk right from pos while current char is a word char.
        let mut end = self.text.len();
        for (i, c) in self.text[pos..].char_indices() {
            if !is_word_char(c) {
                end = pos + i;
                break;
            }
        }

        if start < end {
            self.selection_anchor = Some(start);
            self.cursor = end;
        }
    }

    /// Select-all at the current cursor — same as Ctrl+A. Provided as a
    /// distinct entry point for triple-click line-select (single-line so
    /// "line" and "all" coincide).
    pub fn select_line(&mut self) {
        self.select_all();
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Backspace: delete selection if any, else delete one char before cursor.
    pub fn delete_selection_or_one_back(&mut self) {
        if self.has_selection() {
            self.replace_selection("");
            return;
        }
        if self.cursor == 0 {
            return;
        }
        // Step back one char (UTF-8 safe).
        let prev = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.push_undo();
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
        self.selection_anchor = None;
        self.last_edit = std::time::Instant::now();
    }

    /// Delete: delete selection if any, else delete one char at/after cursor.
    pub fn delete_selection_or_one_forward(&mut self) {
        if self.has_selection() {
            self.replace_selection("");
            return;
        }
        if self.cursor >= self.text.len() {
            return;
        }
        // Find the byte index just after the next char.
        let next = self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.text.len());
        self.push_undo();
        self.text.replace_range(self.cursor..next, "");
        self.selection_anchor = None;
        self.last_edit = std::time::Instant::now();
    }

    /// Move the cursor. If `extend_selection`, the existing
    /// `selection_anchor` is preserved (or set to the pre-move cursor
    /// position); otherwise the selection is dropped.
    pub fn move_cursor(&mut self, dir: CursorMove, extend_selection: bool) {
        let pre = self.cursor;
        let new = match dir {
            CursorMove::Left => self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0),
            CursorMove::Right => self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.text.len()),
            CursorMove::Home => 0,
            CursorMove::End => self.text.len(),
            CursorMove::WordLeft => prev_word_boundary(&self.text, self.cursor),
            CursorMove::WordRight => next_word_boundary(&self.text, self.cursor),
        };
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(pre);
            }
        } else {
            self.selection_anchor = None;
        }
        self.cursor = new;
        self.last_edit = std::time::Instant::now();
    }

    /// Best-effort cursor placement from a click x-offset relative to the
    /// text origin. `measure` should return the rendered width in px of a
    /// `&str`. This is `O(chars)` — fine for short single-line fields.
    pub fn cursor_from_click(&mut self, click_x: i32, measure: &mut dyn FnMut(&str) -> i32) {
        let mut best = 0usize;
        let mut best_dx = i32::MAX;
        for (idx, _ch) in self.text.char_indices().chain(std::iter::once((self.text.len(), '\0'))) {
            let w = measure(&self.text[..idx]);
            let dx = (w - click_x).abs();
            if dx < best_dx {
                best_dx = dx;
                best = idx;
            }
        }
        self.cursor = best;
        self.selection_anchor = None;
    }
}

/// Word-boundary helpers — match egui's "alphanumerics are word chars,
/// everything else is a separator" heuristic, simplified.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn prev_word_boundary(text: &str, mut pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    // Walk left across non-word chars first, then across one run of word chars.
    let chars: Vec<(usize, char)> = text[..pos].char_indices().collect();
    let mut i = chars.len();
    // Skip non-word chars adjacent to the cursor.
    while i > 0 && !is_word_char(chars[i - 1].1) {
        i -= 1;
        pos = chars[i].0;
    }
    // Skip the word run.
    while i > 0 && is_word_char(chars[i - 1].1) {
        i -= 1;
        pos = chars[i].0;
    }
    pos
}

fn next_word_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    let chars: Vec<(usize, char)> = text[pos..].char_indices().collect();
    let mut i = 0usize;
    // Skip a word run.
    while i < chars.len() && is_word_char(chars[i].1) {
        i += 1;
    }
    // Skip following non-word chars.
    while i < chars.len() && !is_word_char(chars[i].1) {
        i += 1;
    }
    if i >= chars.len() {
        text.len()
    } else {
        pos + chars[i].0
    }
}

pub struct NodeGraph {
    state: WidgetState,
    pub graph: Graph,
    port_positions: HashMap<PortRef, Point>,
    interaction: Option<Interaction>,
    hovered_port: Option<PortRef>,
    pan: Vec2f,
    zoom: f32,
    hovered_edge: Option<usize>,
    selected_nodes: HashSet<usize>,
    last_cursor_world: Point,
    field_rects: HashMap<FieldRef, Rect>,
    active_field: Option<FieldRef>,
    field_edit: FieldEditState,
    groups: Vec<Group>,
    select_overlay: Option<SelectOverlay>,
    file_dialog: Option<FileDialog>,
    pending_file_field: Option<FieldRef>,
    pending_file_node: Option<usize>,
    image_previews: HashMap<usize, String>,
    image_cache: HashMap<usize, PreviewBitmap>,
    hovered_field: Option<FieldRef>,
    show_grid: bool,
    drop_highlight: Option<usize>,
    /// Wave 1 (A3): cached widget instances for inline field rendering.
    /// Keyed by FieldRef; preserves cursor / drag / dropdown state across frames.
    field_widgets: HashMap<FieldRef, FieldWidgetEntry>,
    /// Wave 3 (C3): live execution-progress per node, driven by the
    /// application draining `Executor::poll_progress()` each frame and
    /// calling `set_node_progress(...)`. The widget itself does not pull
    /// from any channel; it only renders what the host pushes.
    pub node_progress: HashMap<usize, NodeProgress>,
    /// Wave 3 (C4): currently-open right-click "add node" search menu.
    /// `None` when closed. The host must wire a `NodeRegistryHandle` via
    /// [`NodeGraph::set_node_registry`] before right-clicks can open it.
    pub add_node_menu: Option<AddNodeMenuState>,
    /// Wave 3 (C4): registry handle the menu queries when the user picks an
    /// entry. Stored as `Arc<dyn ...>` so the same handle can be held by the
    /// executor and the widget without ownership games.
    add_node_registry: Option<Arc<dyn NodeRegistryHandle>>,
    /// Wave 3 (C2): per-node image preview state. Keyed by `Node::id`. The
    /// host calls `set_node_image(node_id, &tensor)` after the executor
    /// emits a `NodeValue::Image` output; that uploads to a GL texture (or
    /// falls back to CPU-side bytes when no GL context is current) and
    /// `draw_nodes` blits the result inline below the field stack. Drop
    /// frees the GL texture.
    pub node_image_textures: HashMap<usize, NodeImagePreview>,
    /// Per-node dirty set: nodes the user touched (field edited, edge wired
    /// or removed) since the last `clear_dirty()`. The host's Queue button
    /// reads this to enqueue only the work that actually needs to re-run,
    /// instead of marking every node dirty.
    pub dirty_nodes: HashSet<usize>,
}

impl NodeGraph {
    pub fn new(id: WidgetId, graph: Graph) -> Self {
        Self {
            state: WidgetState::new(id),
            graph,
            port_positions: HashMap::new(),
            interaction: None,
            hovered_port: None,
            pan: Vec2f::ZERO,
            zoom: 1.0,
            hovered_edge: None,
            selected_nodes: HashSet::new(),
            last_cursor_world: Point::ZERO,
            field_rects: HashMap::new(),
            active_field: None,
            field_edit: FieldEditState::new(String::new()),
            groups: Vec::new(),
            select_overlay: None,
            file_dialog: None,
            pending_file_field: None,
            pending_file_node: None,
            image_previews: HashMap::new(),
            image_cache: HashMap::new(),
            hovered_field: None,
            show_grid: true,
            drop_highlight: None,
            field_widgets: HashMap::new(),
            node_progress: HashMap::new(),
            add_node_menu: None,
            add_node_registry: None,
            node_image_textures: HashMap::new(),
            dirty_nodes: HashSet::new(),
        }
    }

    // ---- Dirty-tracking API (host's Queue reads dirty_set, then clears) ----

    /// Read-only view of the current dirty set.
    pub fn dirty_set(&self) -> &HashSet<usize> {
        &self.dirty_nodes
    }

    /// Drop every entry from the dirty set. Called by the host after it has
    /// enqueued the dirty work so subsequent edits start a fresh frontier.
    pub fn clear_dirty(&mut self) {
        self.dirty_nodes.clear();
    }

    /// Mark `node_id` dirty. Idempotent. Called from every user-driven field
    /// edit and from edge add/remove paths.
    pub fn mark_dirty(&mut self, node_id: usize) {
        self.dirty_nodes.insert(node_id);
    }

    // ---- Wave 3 (C4): add-node search menu API -----------------------------

    /// Install the registry handle the right-click "add node" menu queries
    /// when populating its list and seeding new nodes. Until this is set,
    /// right-click on empty canvas is a no-op (existing edge/port-context
    /// behavior is preserved unconditionally).
    pub fn set_node_registry(&mut self, registry: Arc<dyn NodeRegistryHandle>) {
        self.add_node_registry = Some(registry);
    }

    /// Open the add-node menu at the given **world** position. If no registry
    /// is wired, this is a no-op (the menu cannot list anything).
    pub fn open_add_menu_at(&mut self, world_pos: Point, registry: Arc<dyn NodeRegistryHandle>) {
        self.add_node_menu = Some(AddNodeMenuState::new(world_pos, registry));
    }

    /// Close the add-node menu, dropping any in-progress search text.
    pub fn close_add_menu(&mut self) {
        self.add_node_menu = None;
    }

    /// Whether the add-node menu is currently open. Used by tests and by the
    /// host to suppress other UI hotkeys while the menu owns the keyboard.
    pub fn add_menu_open(&self) -> bool {
        self.add_node_menu.is_some()
    }

    /// Test-only helper: simulate the user picking the first filtered entry
    /// in the open menu. Returns `true` if a node was inserted. The real UI
    /// invokes the same `select() + add_node()` flow when the user clicks an
    /// item; exposing it here keeps the integration test independent of the
    /// (still-stubbed) menu rendering pipeline.
    pub fn add_menu_pick_first(&mut self) -> bool {
        let node = match &self.add_node_menu {
            Some(menu) => menu.select(),
            None => None,
        };
        if let Some(n) = node {
            self.add_node(n);
            self.close_add_menu();
            true
        } else {
            false
        }
    }

    pub fn add_group(&mut self, group: Group) {
        self.groups.push(group);
    }

    pub fn set_grid_visible(&mut self, show: bool) {
        self.show_grid = show;
    }

    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    fn node_accepts_drop(&self, node_id: usize) -> bool {
        self.graph
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|node| {
                node.fields
                    .iter()
                    .any(|f| matches!(f.kind, FieldKind::FilePath { .. }))
            })
            .unwrap_or(false)
    }

    fn update_drop_highlight(&mut self, screen_pos: Point) {
        let world = self.screen_to_world(screen_pos);
        self.drop_highlight = self
            .node_at(world)
            .filter(|node_id| self.node_accepts_drop(*node_id));
    }

    fn clear_drop_highlight(&mut self) {
        self.drop_highlight = None;
    }

    fn handle_file_drop(&mut self, files: &[String], screen_pos: Point) -> EventResult {
        if files.is_empty() {
            return EventResult::Ignored;
        }
        let world = self.screen_to_world(screen_pos);
        let node_id = match self.node_at(world) {
            Some(id) if self.node_accepts_drop(id) => id,
            _ => return EventResult::Ignored,
        };
        let path = files.first().unwrap();
        if !Self::is_supported_image(path) {
            return EventResult::Ignored;
        }
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            if let Some(field) = node
                .fields
                .iter_mut()
                .find(|f| matches!(f.kind, FieldKind::FilePath { .. }))
            {
                field.value = FieldValue::FilePath(PathBuf::from(path));
            }
            node.title = format!("Image: {}", file_name);
            self.refresh_preview_for(node_id, path);
            self.mark_dirty(node_id);
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }

    fn is_supported_image(path: &str) -> bool {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
                )
            })
            .unwrap_or(false)
    }

    pub fn selected_node_ids(&self) -> Vec<usize> {
        self.selected_nodes.iter().copied().collect()
    }

    pub fn first_selected_node(&self) -> Option<Node> {
        self.selected_nodes
            .iter()
            .next()
            .and_then(|id| self.graph.nodes.iter().find(|n| n.id == *id))
            .cloned()
    }

    pub fn set_node_title(&mut self, node_id: usize, title: impl Into<String>) -> bool {
        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            node.title = title.into();
            true
        } else {
            false
        }
    }

    pub fn add_input_port(&mut self, node_id: usize, label: impl Into<String>) -> bool {
        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            let next_id = node.inputs.iter().map(|p| p.id).max().unwrap_or(0) + 1;
            node.inputs.push(Port {
                id: next_id,
                label: label.into(),
                is_input: true,
            });
            self.update_port_positions();
            true
        } else {
            false
        }
    }

    pub fn add_output_port(&mut self, node_id: usize, label: impl Into<String>) -> bool {
        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            let next_id = node.outputs.iter().map(|p| p.id).max().unwrap_or(0) + 1;
            node.outputs.push(Port {
                id: next_id,
                label: label.into(),
                is_input: false,
            });
            self.update_port_positions();
            true
        } else {
            false
        }
    }

    pub fn delete_selected(&mut self) -> bool {
        self.remove_selected_nodes()
    }

    /// Test helper: whether a file dialog overlay is currently active.
    /// Useful for headless tests that drive a click on a `FilePath` field
    /// and want to assert the dialog opened.
    pub fn is_file_dialog_open(&self) -> bool {
        self.file_dialog.is_some()
    }

    fn to_screen_point(&self, world: Point) -> Point {
        Point::new(
            ((world.x as f32) * self.zoom + self.pan.x).round() as i32,
            ((world.y as f32) * self.zoom + self.pan.y).round() as i32,
        )
    }

    fn to_screen_size(&self, world: Size) -> Size {
        Size::new(
            (world.width as f32 * self.zoom).round() as i32,
            (world.height as f32 * self.zoom).round() as i32,
        )
    }

    fn to_screen_rect(&self, world: Rect) -> Rect {
        Rect::from_origin_size(
            self.to_screen_point(world.origin),
            self.to_screen_size(world.size),
        )
    }

    fn screen_to_world(&self, screen: Point) -> Point {
        Point::new(
            ((screen.x as f32 - self.pan.x) / self.zoom).round() as i32,
            ((screen.y as f32 - self.pan.y) / self.zoom).round() as i32,
        )
    }

    fn scale(&self, value: i32) -> i32 {
        (value as f32 * self.zoom).round() as i32
    }

    fn update_port_positions(&mut self) {
        self.port_positions.clear();
        for node in &self.graph.nodes {
            let mut y = node.position.y + 28;
            for p in &node.inputs {
                let pos = Point::new(node.position.x + 8, y);
                self.port_positions.insert(
                    PortRef {
                        node_id: node.id,
                        port_id: p.id,
                        is_input: true,
                    },
                    pos,
                );
                y += 20;
            }
            y = node.position.y + 28;
            for p in &node.outputs {
                let pos = Point::new(node.position.x + node.size.width - 8, y);
                self.port_positions.insert(
                    PortRef {
                        node_id: node.id,
                        port_id: p.id,
                        is_input: false,
                    },
                    pos,
                );
                y += 20;
            }
        }
        self.update_field_rects();
    }

    fn node_at(&self, point: Point) -> Option<usize> {
        self.graph
            .nodes
            .iter()
            .find(|n| Rect::from_origin_size(n.position, n.size).contains(point))
            .map(|n| n.id)
    }

    fn port_at_screen(&self, point: Point) -> Option<PortRef> {
        let radius = (8.0 * self.zoom).max(6.0) as i32;
        self.port_positions.iter().find_map(|(pref, pos)| {
            let screen = self.to_screen_point(*pos);
            let dx = point.x - screen.x;
            let dy = point.y - screen.y;
            if dx * dx + dy * dy <= radius * radius {
                Some(*pref)
            } else {
                None
            }
        })
    }

    fn resize_handle_at(&self, point: Point) -> Option<usize> {
        for node in &self.graph.nodes {
            let rect = self.to_screen_rect(Rect::from_origin_size(node.position, node.size));
            let handle_size = self.scale(12).clamp(10, 16);
            let handle = Rect::new(
                rect.right() - handle_size,
                rect.bottom() - handle_size,
                handle_size,
                handle_size,
            );
            if handle.contains(point) {
                return Some(node.id);
            }
        }
        None
    }

    fn draw_background(&self, ctx: &mut dyn DrawContext) {
        let bounds = self.state.bounds;
        ctx.set_color(Color::from_hex(0x111827));
        ctx.fill_rect(bounds);
        if !self.show_grid {
            return;
        }
        ctx.set_color(Color::rgba(255, 255, 255, 20));
        let step = (24.0 * self.zoom).clamp(12.0, 64.0) as i32;
        let Size { width, height } = bounds.size;
        let offset_x = ((self.pan.x.round() as i32) % step + step) % step;
        let offset_y = ((self.pan.y.round() as i32) % step + step) % step;
        for x in (offset_x..=width + step).step_by(step as usize) {
            ctx.draw_line(
                Point::new(bounds.x() + x, bounds.y()),
                Point::new(bounds.x() + x, bounds.y() + height),
                1,
            );
        }
        for y in (offset_y..=height + step).step_by(step as usize) {
            ctx.draw_line(
                Point::new(bounds.x(), bounds.y() + y),
                Point::new(bounds.x() + width, bounds.y() + y),
                1,
            );
        }
    }

    fn layout_file_dialog(&mut self, theme: &Theme) {
        if let Some(dialog) = self.file_dialog.as_mut() {
            let bounds = self.state.bounds;
            let desired_w = 700;
            let desired_h = 520;
            let w = desired_w.min(bounds.width()).max(320);
            let h = desired_h.min(bounds.height()).max(260);
            let x = bounds.x() + (bounds.width() - w) / 2;
            let y = bounds.y() + (bounds.height() - h) / 2;
            dialog.layout(Rect::new(x, y, w, h), theme);
        }
    }

    fn draw_bezier(
        &self,
        ctx: &mut dyn DrawContext,
        start: Point,
        end: Point,
        color: Color,
        thickness: i32,
    ) {
        let start_x = start.x as f32;
        let start_y = start.y as f32;
        let end_x = end.x as f32;
        let end_y = end.y as f32;
        let dx = (end_x - start_x).abs().max(1.0);
        let control = dx * 0.35;
        let c1x = start_x + control;
        let c1y = start_y;
        let c2x = end_x - control;
        let c2y = end_y;
        ctx.set_color(color);
        let segments = 24;
        let mut prev_x = start_x;
        let mut prev_y = start_y;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let inv = 1.0 - t;
            let px = inv * inv * inv * start_x
                + 3.0 * inv * inv * t * c1x
                + 3.0 * inv * t * t * c2x
                + t * t * t * end_x;
            let py = inv * inv * inv * start_y
                + 3.0 * inv * inv * t * c1y
                + 3.0 * inv * t * t * c2y
                + t * t * t * end_y;
            let p = Point::new(px.round() as i32, py.round() as i32);
            ctx.draw_line(
                Point::new(prev_x.round() as i32, prev_y.round() as i32),
                p,
                thickness,
            );
            prev_x = px;
            prev_y = py;
        }
    }

    fn draw_edges(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        for (idx, edge) in self.graph.edges.iter().enumerate() {
            let from = self.port_positions.get(&PortRef {
                node_id: edge.from_node,
                port_id: edge.from_port,
                is_input: false,
            });
            let to = self.port_positions.get(&PortRef {
                node_id: edge.to_node,
                port_id: edge.to_port,
                is_input: true,
            });
            if let (Some(start), Some(end)) = (from, to) {
                let start = self.to_screen_point(*start);
                let end = self.to_screen_point(*end);
                let mut thickness = self.scale(3).clamp(2, 8);
                // Hardcoded amber/gold for edges so they stand out against
                // the dark canvas instead of getting lost in theme blue.
                const EDGE_COLOR: Color = Color::rgb(245, 158, 11);
                const EDGE_HOVER: Color = Color::rgb(252, 211, 77);
                let color = if Some(idx) == self.hovered_edge {
                    thickness = (thickness + 1).clamp(thickness, 10);
                    EDGE_HOVER
                } else {
                    EDGE_COLOR
                };
                self.draw_bezier(ctx, start, end, color, thickness);
                self.draw_arrowhead(ctx, start, end, color, thickness);
            }
        }
    }

    fn draw_nodes(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        for node in &self.graph.nodes {
            let rect_world = Rect::from_origin_size(node.position, node.size);
            let rect = self.to_screen_rect(rect_world);
            let corner = self.scale(theme.borders.radius).clamp(4, 16);
            let is_selected = self.selected_nodes.contains(&node.id);
            let is_drop_target = self.drop_highlight == Some(node.id);

            // subtle shadow and card
            ctx.draw_shadow(
                rect,
                theme.colors.shadow,
                self.scale(8),
                Point::new(0, self.scale(3)),
            );
            ctx.set_color(if is_drop_target {
                theme.colors.success.with_alpha(120)
            } else {
                theme.colors.surface
            });
            ctx.fill_rounded_rect(rect, corner);
            // Active node (currently executing per ProgressEvent::NodeStep)
            // gets a thicker accent border so the user can see which node
            // the executor is on. We use the presence of a non-error
            // progress entry as the "active" signal.
            let is_active = self
                .node_progress
                .get(&node.id)
                .map(|p| p.error.is_none())
                .unwrap_or(false);
            // Hardcoded vivid magenta for active border (NOT theme.primary
            // because that's blue and the user wanted clearly different).
            const ACTIVE_BORDER: Color = Color::rgb(236, 72, 153);
            ctx.set_color(if is_drop_target {
                theme.colors.success
            } else if is_active {
                ACTIVE_BORDER
            } else if is_selected {
                theme.colors.border_focus
            } else {
                theme.colors.border
            });
            ctx.draw_rounded_rect(rect, corner);

            // header
            let header_h = self.scale(26).clamp(18, 38);
            let header = Rect::from_origin_size(rect.origin, Size::new(rect.size.width, header_h));
            let header_font = self
                .scale(theme.typography.font_size_base)
                .clamp(10, 24);
            ctx.set_color(if is_drop_target {
                theme.colors.success.with_alpha(180)
            } else {
                theme.colors.surface_variant
            });
            ctx.fill_rounded_rect(header, corner);
            ctx.set_color(theme.colors.text);
            let header_label = self.ellipsize_text(
                ctx,
                &node.title,
                header_font,
                header.size.width - self.scale(20).clamp(12, 40),
            );
            ctx.draw_text(
                &header_label,
                Point::new(header.x() + self.scale(10), header.y() + self.scale(16)),
                header_font,
            );

            // Wave 3 (C3): draw the live-progress bar between the header and
            // the field stack when the host has pushed a step event for this
            // node. The setter API lives in `progress.rs`; here we only call
            // the renderer.
            if let Some(np) = self.node_progress.get(&node.id) {
                progress::draw_node_progress_bar(
                    ctx,
                    theme,
                    np,
                    rect.origin,
                    header_h,
                    rect.size.width,
                    self.zoom,
                );
            }

            // ports
            for port in &node.inputs {
                if let Some(pos) = self.port_positions.get(&PortRef {
                    node_id: node.id,
                    port_id: port.id,
                    is_input: true,
                }) {
                    let screen = self.to_screen_point(*pos);
                    let color = if Some(PortRef {
                        node_id: node.id,
                        port_id: port.id,
                        is_input: true,
                    }) == self.hovered_port
                    {
                        theme.colors.primary
                    } else {
                        theme.colors.text_secondary
                    };
                    ctx.set_color(color);
                    let r = self.scale(6).clamp(4, 10);
                    ctx.fill_circle(screen, r, 18);
                    ctx.set_color(theme.colors.text);
                    ctx.draw_text(
                        &port.label,
                        Point::new(screen.x + r + self.scale(6), screen.y + self.scale(4)),
                        self.scale(theme.typography.font_size_small).clamp(10, 18),
                    );
                }
            }
            for port in &node.outputs {
                if let Some(pos) = self.port_positions.get(&PortRef {
                    node_id: node.id,
                    port_id: port.id,
                    is_input: false,
                }) {
                    let screen = self.to_screen_point(*pos);
                    let color = if Some(PortRef {
                        node_id: node.id,
                        port_id: port.id,
                        is_input: false,
                    }) == self.hovered_port
                    {
                        theme.colors.primary
                    } else {
                        theme.colors.text_secondary
                    };
                    ctx.set_color(color);
                    let r = self.scale(6).clamp(4, 10);
                    ctx.fill_circle(screen, r, 18);
                    let text_size = ctx.measure_text(
                        &port.label,
                        self.scale(theme.typography.font_size_small).clamp(10, 18),
                    );
                    ctx.set_color(theme.colors.text);
                    ctx.draw_text(
                        &port.label,
                        Point::new(
                            screen.x - r - self.scale(6) - text_size.width,
                            screen.y + self.scale(4),
                        ),
                        self.scale(theme.typography.font_size_small).clamp(10, 18),
                    );
                }
            }

            // fields
            let mut last_field_bottom = node.position.y + 26;
            for field in &node.fields {
                let world_rect = self
                    .field_rects
                    .get(&FieldRef {
                        node_id: node.id,
                        field_id: field.id,
                    })
                    .cloned()
                    .unwrap_or_else(|| {
                        Rect::new(
                            node.position.x + 8,
                            node.position.y + header_h + 8,
                            node.size.width - 16,
                            28,
                        )
                    });
                last_field_bottom = last_field_bottom.max(world_rect.bottom());
                let field_rect = self.to_screen_rect(world_rect);
                let active = self.active_field
                    == Some(FieldRef {
                        node_id: node.id,
                        field_id: field.id,
                    });
                let hovered = self.hovered_field
                    == Some(FieldRef {
                        node_id: node.id,
                        field_id: field.id,
                    });
                ctx.set_color(if active {
                    theme.colors.surface_variant
                } else if hovered {
                    theme.colors.surface_variant.with_alpha(200)
                } else {
                    theme.colors.surface
                });
                ctx.fill_rounded_rect(field_rect, self.scale(6).clamp(4, 10));
                ctx.set_color(if hovered {
                    theme.colors.primary
                } else {
                    theme.colors.border
                });
                ctx.draw_rounded_rect(field_rect, self.scale(6).clamp(4, 10));

                let label = &field.label;
                let value_text = if active {
                    self.field_edit.text.clone()
                } else {
                    self.field_value_str(field)
                };
                let pad = self.scale(8).clamp(6, 14);
                let label_font = self.scale(theme.typography.font_size_small).clamp(10, 18);
                ctx.set_color(theme.colors.text_secondary);
                let label_text = if active {
                    label.to_string()
                } else {
                    self.ellipsize_text(
                        ctx,
                        label,
                        label_font,
                        field_rect.size.width - pad * 2,
                    )
                };
                ctx.draw_text(
                    &label_text,
                    Point::new(field_rect.x() + pad, field_rect.y() + pad),
                    label_font,
                );
                ctx.set_color(theme.colors.text);
                let value_font = (label_font + 2).clamp(12, 22);
                let value_y = field_rect.y() + pad + label_font + self.scale(4).clamp(2, 8);
                let is_active = self
                    .active_field
                    .map(|af| af.field_id == field.id && af.node_id == node.id)
                    .unwrap_or(false);
                let display_value = if active {
                    value_text.clone()
                } else {
                    self.ellipsize_text(
                        ctx,
                        &value_text,
                        value_font,
                        field_rect.size.width - pad * 2,
                    )
                };
                ctx.draw_text(
                    &display_value,
                    Point::new(field_rect.x() + pad, value_y),
                    value_font,
                );
                if is_active {
                    // Use the real cursor byte-offset into `field_edit.text`
                    // (not the on-screen ellipsized substring). Single-line
                    // node fields don't ellipsize while editing — the
                    // `display_value` above is the full edit buffer when
                    // active — so this maps cleanly.
                    let cursor_byte = self.field_edit.cursor.min(self.field_edit.text.len());
                    let pre_cursor = &self.field_edit.text[..cursor_byte];
                    let pre_w = ctx
                        .measure_text(pre_cursor, value_font)
                        .width
                        .min(field_rect.width() - pad * 2);
                    // Selection highlight (drawn under the text glyphs would
                    // require z-order; we draw it before the caret bar and
                    // accept it sitting on top — the alpha keeps text
                    // legible).
                    if let Some((a, b)) = self.field_edit.selection_range() {
                        let a_clamped = a.min(self.field_edit.text.len());
                        let b_clamped = b.min(self.field_edit.text.len());
                        let sel_x_start = ctx
                            .measure_text(
                                &self.field_edit.text[..a_clamped],
                                value_font,
                            )
                            .width;
                        let sel_x_end = ctx
                            .measure_text(
                                &self.field_edit.text[..b_clamped],
                                value_font,
                            )
                            .width;
                        let sel_left = field_rect.x() + pad + sel_x_start;
                        let sel_right = field_rect.x() + pad + sel_x_end;
                        let sel_top = value_y - value_font + 2;
                        let sel_bottom = value_y + 4;
                        ctx.set_color(theme.colors.primary.with_alpha(80));
                        ctx.fill_rect(Rect::new(
                            sel_left,
                            sel_top,
                            (sel_right - sel_left).max(1),
                            (sel_bottom - sel_top).max(1),
                        ));
                        ctx.set_color(theme.colors.text);
                    }
                    // Blink: on/off every 500ms. Pause-while-editing — if
                    // the user just typed (last_edit < 500ms ago), force
                    // the caret on so it doesn't visually disappear
                    // mid-keystroke (egui does the same in
                    // `TextEditState::last_edit_time`).
                    let just_edited =
                        self.field_edit.last_edit.elapsed().as_millis() < 500;
                    let blink_phase =
                        (BLINK_EPOCH.elapsed().as_millis() / 500) % 2 == 0;
                    if just_edited || blink_phase {
                        let caret_x = field_rect.x() + pad + pre_w;
                        let caret_top = value_y - value_font + 2;
                        let caret_bottom = value_y + 4;
                        // 2px-wide bar via two adjacent vertical lines.
                        ctx.draw_line(
                            Point::new(caret_x, caret_top),
                            Point::new(caret_x, caret_bottom),
                            1,
                        );
                        ctx.draw_line(
                            Point::new(caret_x + 1, caret_top),
                            Point::new(caret_x + 1, caret_bottom),
                            1,
                        );
                    }
                }
            }

            if let Some(preview_path) = self.image_previews.get(&node.id) {
                // derive desired height from field selection if present
                let mut desired_h: Option<i32> = None;
                if let Some(sel_field) = node
                    .fields
                    .iter()
                    .find(|f| f.label.eq_ignore_ascii_case("preview"))
                {
                    if let FieldValue::Select(v) = &sel_field.value {
                        desired_h = match v.as_str() {
                            "none" => Some(0),
                            "small" => Some(120),
                            "full" => None, // will use available height
                            _ => None,
                        };
                    }
                }
                let available_h =
                    (node.size.height - (last_field_bottom - node.position.y) - 16).max(100);
                let mut preview_h = available_h;
                if let Some(h) = desired_h {
                    if h == 0 {
                        continue;
                    }
                    preview_h = h.min(available_h);
                }
                let preview_world = Rect::new(
                    node.position.x + 8,
                    last_field_bottom + 8,
                    node.size.width - 16,
                    preview_h,
                );
                let preview_rect = self.to_screen_rect(preview_world);
                let radius = self.scale(8).clamp(4, 12);
                ctx.set_color(theme.colors.surface);
                ctx.fill_rounded_rect(preview_rect, radius);
                ctx.set_color(theme.colors.border);
                ctx.draw_rounded_rect(preview_rect, radius);
                if let Some(preview) = self.image_cache.get(&node.id) {
                    let pad = self.scale(8).clamp(4, 16);
                    let avail_w = (preview_rect.width() - pad * 2).max(1);
                    let avail_h = (preview_rect.height() - pad * 2).max(1);
                    let scale = f32::min(
                        avail_w as f32 / preview.width.max(1) as f32,
                        avail_h as f32 / preview.height.max(1) as f32,
                    );
                    let draw_w = (preview.width as f32 * scale).round().max(1.0) as i32;
                    let draw_h = (preview.height as f32 * scale).round().max(1.0) as i32;
                    let dx = preview_rect.x() + (avail_w - draw_w) / 2 + pad;
                    let dy = preview_rect.y() + (avail_h - draw_h) / 2 + pad;
                    let dest = Rect::new(dx, dy, draw_w, draw_h);
                    ctx.set_color(Color::WHITE);
                    ctx.draw_image_rgba(dest, preview.width, preview.height, &preview.data);
                }
                let file_name = preview_path
                    .rsplit(&['/', '\\'][..])
                    .next()
                    .unwrap_or(preview_path);
                ctx.set_color(theme.colors.text);
                ctx.draw_text(
                    file_name,
                    Point::new(
                        preview_rect.x() + self.scale(8),
                        preview_rect.bottom() - self.scale(10),
                    ),
                    self.scale(theme.typography.font_size_small).clamp(10, 18),
                );
            }

            // Wave 3 (C2): inline tensor-image preview for nodes that emit
            // `NodeValue::Image`. Independent of the file-based `image_previews`
            // path above — this one is sourced from the executor at runtime.
            if let Some(np) = self.node_image_textures.get(&node.id) {
                preview::draw_node_image_preview(
                    ctx,
                    theme,
                    np,
                    Rect::from_origin_size(node.position, node.size),
                    last_field_bottom,
                    self.pan.x,
                    self.pan.y,
                    self.zoom,
                );
            }

            // resize handle (bottom-right) — bumped from 12/16 to 24/32
            // because 16px on a 4K screen is impossible to grab.
            let handle_size = self.scale(24).clamp(20, 32);
            let handle_rect = Rect::new(
                rect.right() - handle_size,
                rect.bottom() - handle_size,
                handle_size,
                handle_size,
            );
            ctx.set_color(theme.colors.surface_variant);
            ctx.fill_rect(handle_rect);
            ctx.set_color(theme.colors.border);
            ctx.draw_rect(handle_rect);
        }

        // C4: right-click add-node menu. Rendered last so it sits above the
        // node bodies. Anchor in screen space so the panel doesn't pan/zoom.
        if let Some(state) = &self.add_node_menu {
            let screen_pos = self.to_screen_point(state.position);
            add_menu::draw_add_menu(ctx, theme, state, screen_pos);
        }
    }

    fn draw_temp_link(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        if let Some(Interaction::LinkDrag { from, cursor }) = &self.interaction {
            if let Some(start) = self.port_positions.get(from) {
                let start = self.to_screen_point(*start);
                let end = self.to_screen_point(*cursor);
                let thickness = self.scale(2).clamp(1, 6);
                self.draw_bezier(ctx, start, end, theme.colors.primary_hover, thickness);
                self.draw_arrowhead(ctx, start, end, theme.colors.primary_hover, thickness);
            }
        }
    }

    fn draw_marquee(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        if let Some(Interaction::Marquee {
            start_world,
            current_world,
        }) = &self.interaction
        {
            let rect_world = rect_from_points(*start_world, *current_world);
            let rect = self.to_screen_rect(rect_world);
            ctx.set_color(theme.colors.selection);
            ctx.fill_rect(rect);
            ctx.set_color(theme.colors.primary);
            ctx.draw_rect(rect);
        }
    }

    fn field_value_str(&self, field: &Field) -> String {
        match &field.value {
            FieldValue::Text(s) => s.clone(),
            FieldValue::Number(n) => {
                if (n.fract() - 0.0).abs() < f32::EPSILON {
                    format!("{:.0}", n)
                } else {
                    format!("{:.2}", n)
                }
            }
            FieldValue::Select(s) => s.clone(),
            FieldValue::FilePath(p) => p.display().to_string(),
            FieldValue::Bool(b) => b.to_string(),
        }
    }

    fn draw_arrowhead(
        &self,
        ctx: &mut dyn DrawContext,
        start: Point,
        end: Point,
        color: Color,
        _thickness: i32,
    ) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let len = ((dx * dx + dy * dy) as f32).sqrt();
        if len < 1.0 {
            return;
        }
        let size = (8.0 * self.zoom).clamp(6.0, 14.0);
        let inv = 1.0 / len;
        let dir_x = dx as f32 * inv;
        let dir_y = dy as f32 * inv;
        let tip_x = end.x as f32;
        let tip_y = end.y as f32;
        let back_x = tip_x - dir_x * size * 1.2;
        let back_y = tip_y - dir_y * size * 1.2;
        let perp_x = -dir_y;
        let perp_y = dir_x;
        let spread = size * 0.6;
        let left = Point::new(
            (back_x + perp_x * spread).round() as i32,
            (back_y + perp_y * spread).round() as i32,
        );
        let right = Point::new(
            (back_x - perp_x * spread).round() as i32,
            (back_y - perp_y * spread).round() as i32,
        );
        let tip = Point::new(tip_x.round() as i32, tip_y.round() as i32);
        ctx.set_color(color);
        ctx.fill_polygon(&[tip, left, right]);
    }

    fn update_field_rects(&mut self) {
        self.field_rects.clear();
        // Track which field refs still exist so we can prune stale cached widgets.
        let mut live_refs: HashSet<FieldRef> = HashSet::new();
        for node in &mut self.graph.nodes {
            let header_h = 26;
            let mut y = node.position.y + header_h + 8;
            let max_ports = node.inputs.len().max(node.outputs.len()) as i32;
            if max_ports > 0 {
                y = (node.position.y + 28 + 20 * max_ports).max(y);
            }
            for field in &node.fields {
                let field_h = match field.kind {
                    FieldKind::Text | FieldKind::FilePath { .. } => 44,
                    FieldKind::Number { .. } | FieldKind::Select { .. } => 32,
                    FieldKind::Bool => 28,
                };
                let rect = Rect::new(node.position.x + 8, y, node.size.width - 16, field_h);
                let fref = FieldRef {
                    node_id: node.id,
                    field_id: field.id,
                };
                self.field_rects.insert(fref, rect);
                live_refs.insert(fref);
                y += field_h + 8;
            }
            let required_height = (y - node.position.y + 16).max(MIN_NODE_SIZE.height);
            if node.size.height < required_height {
                node.size.height = required_height;
            }
        }
        // Prune cached widget instances for fields that have been removed.
        self.field_widgets.retain(|fref, _| live_refs.contains(fref));
        self.ensure_field_widgets();
    }

    /// Wave 1 (A3): create cached widget instances for any field that doesn't
    /// have one yet, seeded from its current FieldValue. Called from
    /// `update_field_rects()` so the cache stays in sync with the model.
    fn ensure_field_widgets(&mut self) {
        // Snapshot the (FieldRef, FieldKind, FieldValue) tuples so we don't
        // hold a borrow on self.graph while mutating self.field_widgets.
        let snapshot: Vec<(FieldRef, FieldKind, FieldValue)> = self
            .graph
            .nodes
            .iter()
            .flat_map(|n| {
                n.fields.iter().map(move |f| {
                    (
                        FieldRef {
                            node_id: n.id,
                            field_id: f.id,
                        },
                        f.kind.clone(),
                        f.value.clone(),
                    )
                })
            })
            .collect();
        for (fref, kind, value) in snapshot {
            if self.field_widgets.contains_key(&fref) {
                // Sync existing entry from the current value (cheap idempotent).
                self.sync_field_widget_from_value_inner(fref, &kind, &value);
                continue;
            }
            let entry = match &kind {
                FieldKind::Text => {
                    let mut ti = TextInput::new(WidgetId::default());
                    if let FieldValue::Text(s) = &value {
                        ti.set_text(s.clone());
                    }
                    FieldWidgetEntry::Text(ti)
                }
                FieldKind::Number { min, max, .. } => {
                    let initial = match &value {
                        FieldValue::Number(n) => *n,
                        _ => *min,
                    };
                    FieldWidgetEntry::Number(Slider::new(
                        WidgetId::default(),
                        *min,
                        *max,
                        initial,
                    ))
                }
                FieldKind::Select { options } => {
                    let mut cb = ComboBox::new(WidgetId::default()).with_items(options.clone());
                    if let FieldValue::Select(s) = &value {
                        if let Some(idx) = options.iter().position(|o| o == s) {
                            cb.set_selected(Some(idx));
                        }
                    }
                    FieldWidgetEntry::Select(cb)
                }
                FieldKind::Bool => {
                    let checked = matches!(value, FieldValue::Bool(true));
                    FieldWidgetEntry::Bool(
                        Checkbox::new(WidgetId::default(), "").with_checked(checked),
                    )
                }
                FieldKind::FilePath { .. } => FieldWidgetEntry::FilePath,
            };
            self.field_widgets.insert(fref, entry);
        }
    }

    /// Compute the screen-space rect for a field's widget (after pan/zoom).
    fn field_widget_screen_rect(&self, fref: FieldRef) -> Option<Rect> {
        self.field_rects
            .get(&fref)
            .map(|world_rect| self.to_screen_rect(*world_rect))
    }

    /// Route a press event into the cached Slider. Lays the slider out at its
    /// current screen-space rect, forwards the event, then syncs the new value
    /// back into the FieldValue and starts a `FieldSliderDrag` interaction so
    /// subsequent MouseMove events reach the slider.
    fn route_press_to_slider(
        &mut self,
        fref: FieldRef,
        ev: &MouseButtonEvent,
        theme: &Theme,
    ) {
        let Some(rect) = self.field_widget_screen_rect(fref) else {
            return;
        };
        let event = Event::MouseButton(ev.clone());
        if let Some(FieldWidgetEntry::Number(slider)) = self.field_widgets.get_mut(&fref) {
            slider.layout(rect, theme);
            let _ = slider.handle_event(&event, theme);
            // Slider may have set is_dragging via track-or-thumb hit; keep
            // ourselves in drag mode either way so move events reach it.
            let new_val = slider.value();
            // Sync FieldValue.
            self.set_field_number(fref, new_val);
            self.interaction = Some(Interaction::FieldSliderDrag { fref });
        }
    }

    /// Forward a MouseMove during a slider drag to the cached slider.
    fn route_move_to_slider(&mut self, fref: FieldRef, screen_pos: Point, theme: &Theme) {
        let Some(rect) = self.field_widget_screen_rect(fref) else {
            return;
        };
        let event = Event::MouseMove(erigui_core::MouseMoveEvent {
            position: screen_pos,
            delta: Point::ZERO,
            modifiers: Modifiers::empty(),
        });
        let mut new_val: Option<f32> = None;
        if let Some(FieldWidgetEntry::Number(slider)) = self.field_widgets.get_mut(&fref) {
            slider.layout(rect, theme);
            let _ = slider.handle_event(&event, theme);
            new_val = Some(slider.value());
        }
        if let Some(v) = new_val {
            self.set_field_number(fref, v);
        }
    }

    /// Forward a MouseButton release during a slider drag (just resets drag flag).
    fn route_release_to_slider(&mut self, fref: FieldRef, ev: &MouseButtonEvent, theme: &Theme) {
        let Some(rect) = self.field_widget_screen_rect(fref) else {
            return;
        };
        let event = Event::MouseButton(ev.clone());
        if let Some(FieldWidgetEntry::Number(slider)) = self.field_widgets.get_mut(&fref) {
            slider.layout(rect, theme);
            let _ = slider.handle_event(&event, theme);
        }
    }

    fn set_field_number(&mut self, fref: FieldRef, v: f32) {
        if let Some(field) = self.get_field_mut(fref) {
            if let FieldKind::Number { min, max, .. } = &field.kind {
                field.value = FieldValue::Number(v.clamp(*min, *max));
            }
        }
        self.mark_dirty(fref.node_id);
    }

    /// Push the current FieldValue into the cached widget. Cheap idempotent.
    fn sync_field_widget_from_value(&mut self, fref: FieldRef) {
        let snap = self
            .get_field(fref)
            .map(|f| (f.kind.clone(), f.value.clone()));
        if let Some((kind, value)) = snap {
            self.sync_field_widget_from_value_inner(fref, &kind, &value);
        }
    }

    fn sync_field_widget_from_value_inner(
        &mut self,
        fref: FieldRef,
        kind: &FieldKind,
        value: &FieldValue,
    ) {
        let Some(entry) = self.field_widgets.get_mut(&fref) else {
            return;
        };
        match (entry, kind, value) {
            (FieldWidgetEntry::Text(ti), FieldKind::Text, FieldValue::Text(s)) => {
                if ti.text() != s.as_str() {
                    ti.set_text(s.clone());
                }
            }
            (FieldWidgetEntry::Number(slider), FieldKind::Number { .. }, FieldValue::Number(n)) => {
                if (slider.value() - *n).abs() > f32::EPSILON {
                    slider.set_value(*n);
                }
            }
            (
                FieldWidgetEntry::Select(cb),
                FieldKind::Select { options },
                FieldValue::Select(s),
            ) => {
                let want = options.iter().position(|o| o == s);
                if cb.selected_index() != want {
                    cb.set_selected(want);
                }
            }
            (FieldWidgetEntry::Bool(c), FieldKind::Bool, FieldValue::Bool(b)) => {
                if c.is_checked() != *b {
                    c.set_checked(*b);
                }
            }
            _ => {}
        }
    }

    pub fn refresh_preview_for(&mut self, node_id: usize, path: &str) {
        self.image_previews.insert(node_id, path.to_string());
        self.image_cache.remove(&node_id);
        if let Some(preview) = load_image_preview(path) {
            self.image_cache.insert(node_id, preview);
        }
    }

    fn draw_groups(&self, ctx: &mut dyn DrawContext) {
        for group in &self.groups {
            let rect = self.to_screen_rect(group.rect);
            ctx.set_color(group.color.with_alpha(25));
            ctx.fill_rect(rect);
            ctx.set_color(group.color.with_alpha(70));
            ctx.draw_rect(rect);
            ctx.set_color(group.color);
            ctx.draw_text(
                &group.title,
                Point::new(rect.x() + self.scale(8), rect.y() + self.scale(16)),
                self.scale(14),
            );
        }
    }

    fn draw_select_overlay(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        if let Some(overlay) = &self.select_overlay {
            ctx.set_color(theme.colors.overlay.with_alpha(220));
            ctx.fill_rect(overlay.rect);
            ctx.set_color(theme.colors.border);
            ctx.draw_rect(overlay.rect);

            let row_h = self.scale(22).clamp(18, 32);
            let mut y = overlay.rect.y() + self.scale(6);
            for opt in &overlay.options {
                if y + row_h > overlay.rect.bottom() {
                    break;
                }
                let item_rect = Rect::new(
                    overlay.rect.x() + self.scale(4),
                    y,
                    overlay.rect.width() - self.scale(8),
                    row_h,
                );
                ctx.set_color(theme.colors.surface);
                ctx.fill_rect(item_rect);
                ctx.set_color(theme.colors.text);
                ctx.draw_text(
                    opt,
                    Point::new(
                        item_rect.x() + self.scale(6),
                        item_rect.y() + self.scale(14),
                    ),
                    self.scale(theme.typography.font_size_small).clamp(10, 18),
                );
                y += row_h + self.scale(2);
            }
        }
    }

    fn field_at_screen(&self, point: Point) -> Option<FieldRef> {
        self.field_rects.iter().find_map(|(fref, rect)| {
            let screen = self.to_screen_rect(*rect);
            if screen.contains(point) {
                Some(*fref)
            } else {
                None
            }
        })
    }

    fn get_field_mut(&mut self, r: FieldRef) -> Option<&mut Field> {
        self.graph
            .nodes
            .iter_mut()
            .find(|n| n.id == r.node_id)
            .and_then(|n| n.fields.iter_mut().find(|f| f.id == r.field_id))
    }

    fn get_field(&self, r: FieldRef) -> Option<&Field> {
        self.graph
            .nodes
            .iter()
            .find(|n| n.id == r.node_id)
            .and_then(|n| n.fields.iter().find(|f| f.id == r.field_id))
    }

    #[allow(dead_code)]
    fn field_value_string(&self, field: &Field) -> String {
        self.field_value_str(field)
    }

    #[allow(dead_code)]
    fn cycle_select(field: &mut Field) {
        if let FieldKind::Select { options } = &field.kind {
            if options.is_empty() {
                return;
            }
            let current = match &field.value {
                FieldValue::Select(s) => s,
                _ => "",
            };
            let idx = options.iter().position(|o| o == current).unwrap_or(0);
            let next = (idx + 1) % options.len();
            field.value = FieldValue::Select(options[next].clone());
        }
    }

    fn nearest_edge_to_screen(&self, point: Point, threshold: i32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (idx, edge) in self.graph.edges.iter().enumerate() {
            let start = self.port_positions.get(&PortRef {
                node_id: edge.from_node,
                port_id: edge.from_port,
                is_input: false,
            })?;
            let end = self.port_positions.get(&PortRef {
                node_id: edge.to_node,
                port_id: edge.to_port,
                is_input: true,
            })?;
            let a = self.to_screen_point(*start);
            let b = self.to_screen_point(*end);
            let dist = distance_to_segment(point, a, b);
            if dist <= threshold as f32 {
                if let Some((_, best_dist)) = best {
                    if dist < best_dist {
                        best = Some((idx, dist));
                    }
                } else {
                    best = Some((idx, dist));
                }
            }
        }
        best.map(|(idx, _)| idx)
    }

    fn remove_edges_for_port(&mut self, port: PortRef) -> bool {
        let before = self.graph.edges.len();
        // Snapshot endpoints of the edges we're about to drop so we can
        // mark both sides dirty.
        let touched: Vec<(usize, usize)> = self
            .graph
            .edges
            .iter()
            .filter(|e| {
                (e.from_node == port.node_id && e.from_port == port.port_id && !port.is_input)
                    || (e.to_node == port.node_id && e.to_port == port.port_id && port.is_input)
            })
            .map(|e| (e.from_node, e.to_node))
            .collect();
        self.graph.edges.retain(|e| {
            !((e.from_node == port.node_id && e.from_port == port.port_id && !port.is_input)
                || (e.to_node == port.node_id && e.to_port == port.port_id && port.is_input))
        });
        for (a, b) in touched {
            self.mark_dirty(a);
            self.mark_dirty(b);
        }
        before != self.graph.edges.len()
    }

    fn disconnect_selected(&mut self) {
        if self.selected_nodes.is_empty() {
            return;
        }
        let before = self.graph.edges.len();
        let touched: Vec<(usize, usize)> = self
            .graph
            .edges
            .iter()
            .filter(|e| {
                self.selected_nodes.contains(&e.from_node)
                    || self.selected_nodes.contains(&e.to_node)
            })
            .map(|e| (e.from_node, e.to_node))
            .collect();
        self.graph.edges.retain(|e| {
            !self.selected_nodes.contains(&e.from_node) && !self.selected_nodes.contains(&e.to_node)
        });
        if before != self.graph.edges.len() {
            self.hovered_edge = None;
            for (a, b) in touched {
                self.mark_dirty(a);
                self.mark_dirty(b);
            }
        }
    }

    fn duplicate_selected(&mut self) {
        if self.selected_nodes.is_empty() {
            return;
        }
        let offset = Point::new(30, 30);
        let mut id_map = HashMap::new();
        let mut new_nodes = Vec::new();
        for n in self
            .graph
            .nodes
            .iter()
            .filter(|n| self.selected_nodes.contains(&n.id))
        {
            let new_id = self.next_node_id();
            id_map.insert(n.id, new_id);
            let mut clone = n.clone();
            clone.id = new_id;
            clone.position = Point::new(clone.position.x + offset.x, clone.position.y + offset.y);
            new_nodes.push(clone);
        }
        self.graph.nodes.extend(new_nodes);
        let mut new_edges = Vec::new();
        for e in &self.graph.edges {
            if self.selected_nodes.contains(&e.from_node)
                && self.selected_nodes.contains(&e.to_node)
            {
                if let (Some(f), Some(t)) = (id_map.get(&e.from_node), id_map.get(&e.to_node)) {
                    new_edges.push(Edge {
                        from_node: *f,
                        from_port: e.from_port,
                        to_node: *t,
                        to_port: e.to_port,
                    });
                }
            }
        }
        self.graph.edges.extend(new_edges);
        self.selected_nodes.clear();
        self.selected_nodes.extend(id_map.values());
        self.update_port_positions();
        self.update_field_rects();
    }

    fn ellipsize_text(
        &self,
        ctx: &dyn DrawContext,
        text: &str,
        font_size: i32,
        max_width: i32,
    ) -> String {
        if max_width <= 0 || text.is_empty() {
            return String::new();
        }
        if ctx.measure_text(text, font_size).width <= max_width {
            return text.to_string();
        }
        let ellipsis = "…";
        let ellipsis_width = ctx.measure_text(ellipsis, font_size).width;
        if ellipsis_width > max_width {
            return String::new();
        }
        let mut trimmed = text.to_string();
        while trimmed.len() > 1 {
            trimmed.pop();
            let candidate = format!("{trimmed}{ellipsis}");
            if ctx.measure_text(&candidate, font_size).width <= max_width {
                return candidate;
            }
        }
        ellipsis.to_string()
    }

    fn graph_bounds(&self) -> Option<Rect> {
        let mut it = self.graph.nodes.iter();
        let first = it.next()?;
        let mut min_x = first.position.x;
        let mut min_y = first.position.y;
        let mut max_x = first.position.x + first.size.width;
        let mut max_y = first.position.y + first.size.height;
        for n in it {
            min_x = min_x.min(n.position.x);
            min_y = min_y.min(n.position.y);
            max_x = max_x.max(n.position.x + n.size.width);
            max_y = max_y.max(n.position.y + n.size.height);
        }
        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    pub fn fit_view(&mut self) {
        if let (Some(bounds), view) = (self.graph_bounds(), self.state.bounds.size) {
            if bounds.size.width <= 0
                || bounds.size.height <= 0
                || view.width <= 0
                || view.height <= 0
            {
                return;
            }
            let pad = 80.0;
            let scale_x = (view.width as f32 - pad).max(10.0) / (bounds.size.width as f32 + pad);
            let scale_y = (view.height as f32 - pad).max(10.0) / (bounds.size.height as f32 + pad);
            self.zoom = scale_x.min(scale_y).clamp(0.3, 3.0);
            let world_center = Point::new(
                bounds.x() + bounds.size.width / 2,
                bounds.y() + bounds.size.height / 2,
            );
            let screen_center = Point::new(
                self.state.bounds.x() + view.width / 2,
                self.state.bounds.y() + view.height / 2,
            );
            self.pan.x = screen_center.x as f32 - world_center.x as f32 * self.zoom;
            self.pan.y = screen_center.y as f32 - world_center.y as f32 * self.zoom;
        }
    }

    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2f::ZERO;
    }

    pub fn state(&self) -> GraphState {
        GraphState {
            graph: self.graph.clone(),
            pan: self.pan,
            zoom: self.zoom,
            groups: self.groups.clone(),
        }
    }

    pub fn set_state(&mut self, state: GraphState) {
        self.graph = state.graph;
        self.pan = state.pan;
        self.zoom = state.zoom;
        self.groups = state.groups;
        self.image_previews.clear();
        self.image_cache.clear();
        let file_fields: Vec<(usize, String)> = self
            .graph
            .nodes
            .iter()
            .flat_map(|node| {
                node.fields.iter().filter_map(move |field| {
                    if !matches!(field.kind, FieldKind::FilePath { .. }) {
                        return None;
                    }
                    let path: Option<String> = match &field.value {
                        FieldValue::FilePath(p) if !p.as_os_str().is_empty() => {
                            Some(p.display().to_string())
                        }
                        // Back-compat: legacy graphs stored FilePath as Text.
                        FieldValue::Text(s) if !s.is_empty() => Some(s.clone()),
                        _ => None,
                    };
                    path.map(|p| (node.id, p))
                })
            })
            .collect();
        for (id, path) in file_fields {
            self.refresh_preview_for(id, &path);
        }
        self.update_port_positions();
        self.update_field_rects();
    }

    pub fn save_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.state())
    }

    pub fn load_json(&mut self, data: &str) -> Result<(), serde_json::Error> {
        let state: GraphState = serde_json::from_str(data)?;
        self.set_state(state);
        Ok(())
    }

    fn snap_after_drag(&mut self, node_id: usize) {
        let ids: Vec<usize> =
            if self.selected_nodes.contains(&node_id) && !self.selected_nodes.is_empty() {
                self.selected_nodes.iter().copied().collect()
            } else {
                vec![node_id]
            };
        for id in ids {
            if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == id) {
                snap_point(&mut node.position);
            }
        }
        self.update_port_positions();
    }

    fn select_nodes_in_rect(&mut self, rect: Rect, toggle: bool) {
        let hits: Vec<usize> = self
            .graph
            .nodes
            .iter()
            .filter(|n| rect.intersects(&Rect::from_origin_size(n.position, n.size)))
            .map(|n| n.id)
            .collect();
        if toggle {
            for id in hits {
                if !self.selected_nodes.insert(id) {
                    self.selected_nodes.remove(&id);
                }
            }
        } else {
            self.selected_nodes.clear();
            self.selected_nodes.extend(hits);
        }
    }

    fn handle_selection(&mut self, node_id: usize, toggle: bool) {
        if toggle {
            if !self.selected_nodes.insert(node_id) {
                self.selected_nodes.remove(&node_id);
            }
        } else {
            self.selected_nodes.clear();
            self.selected_nodes.insert(node_id);
        }
    }

    pub fn next_node_id(&self) -> usize {
        self.graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    pub fn add_node(&mut self, mut node: Node) {
        if node.id == 0 {
            node.id = self.next_node_id();
        }
        self.graph.nodes.push(node);
        self.update_port_positions();
        self.update_field_rects();
    }

    fn remove_selected_nodes(&mut self) -> bool {
        if self.selected_nodes.is_empty() {
            return false;
        }
        let to_remove: HashSet<usize> = self.selected_nodes.clone();
        let before_nodes = self.graph.nodes.len();
        self.graph.nodes.retain(|n| !to_remove.contains(&n.id));
        let before_edges = self.graph.edges.len();
        self.graph
            .edges
            .retain(|e| !to_remove.contains(&e.from_node) && !to_remove.contains(&e.to_node));
        self.update_port_positions();
        self.selected_nodes.clear();
        before_nodes != self.graph.nodes.len() || before_edges != self.graph.edges.len()
    }

    fn add_node_at_cursor(&mut self) {
        let id = self.next_node_id();
        let mut pos = self.last_cursor_world;
        snap_point(&mut pos);
        let node = Node {
            id,
            title: format!("Node {}", id),
            position: Point::new(pos.x, pos.y),
            size: Size::new(200, 140),
            inputs: vec![Port {
                id: 0,
                label: "in".to_string(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "out".to_string(),
                is_input: false,
            }],
            fields: vec![],
            component_type: None,
        };
        self.graph.nodes.push(node);
        self.selected_nodes.clear();
        self.selected_nodes.insert(id);
        self.update_port_positions();
    }

    fn commit_field_edit(&mut self) {
        if let Some(fref) = self.active_field {
            let edit = self.field_edit.text.clone();
            if let Some(field) = self.get_field_mut(fref) {
                match &field.kind {
                    FieldKind::Text => field.value = FieldValue::Text(edit),
                    FieldKind::Number { min, max, .. } => {
                        if let Ok(v) = edit.trim().parse::<f32>() {
                            field.value = FieldValue::Number(v.clamp(*min, *max));
                        }
                    }
                    FieldKind::Select { .. } | FieldKind::Bool => {}
                    FieldKind::FilePath { .. } => {
                        field.value = FieldValue::FilePath(PathBuf::from(edit));
                    }
                }
            }
            self.mark_dirty(fref.node_id);
        }
        self.active_field = None;
    }

    fn keypress_to_char(key: &KeyPressEvent) -> Option<char> {
        let shift = key.modifiers.contains(Modifiers::SHIFT);
        match key.key {
            Key::Character(c) => Some(c),
            Key::Space => Some(' '),
            Key::Comma => Some(','),
            Key::Period => Some('.'),
            Key::Slash => Some('/'),
            Key::Backslash => Some('\\'),
            Key::Semicolon => Some(';'),
            Key::Quote => Some('\''),
            Key::LeftBracket => Some('['),
            Key::RightBracket => Some(']'),
            Key::Minus => Some(if shift { '_' } else { '-' }),
            Key::Equals => Some(if shift { '+' } else { '=' }),
            Key::Grave => Some('`'),
            Key::Num0 => Some(if shift { ')' } else { '0' }),
            Key::Num1 => Some(if shift { '!' } else { '1' }),
            Key::Num2 => Some(if shift { '@' } else { '2' }),
            Key::Num3 => Some(if shift { '#' } else { '3' }),
            Key::Num4 => Some(if shift { '$' } else { '4' }),
            Key::Num5 => Some(if shift { '%' } else { '5' }),
            Key::Num6 => Some(if shift { '^' } else { '6' }),
            Key::Num7 => Some(if shift { '&' } else { '7' }),
            Key::Num8 => Some(if shift { '*' } else { '8' }),
            Key::Num9 => Some(if shift { '(' } else { '9' }),
            Key::A => Some(if shift { 'A' } else { 'a' }),
            Key::B => Some(if shift { 'B' } else { 'b' }),
            Key::C => Some(if shift { 'C' } else { 'c' }),
            Key::D => Some(if shift { 'D' } else { 'd' }),
            Key::E => Some(if shift { 'E' } else { 'e' }),
            Key::F => Some(if shift { 'F' } else { 'f' }),
            Key::G => Some(if shift { 'G' } else { 'g' }),
            Key::H => Some(if shift { 'H' } else { 'h' }),
            Key::I => Some(if shift { 'I' } else { 'i' }),
            Key::J => Some(if shift { 'J' } else { 'j' }),
            Key::K => Some(if shift { 'K' } else { 'k' }),
            Key::L => Some(if shift { 'L' } else { 'l' }),
            Key::M => Some(if shift { 'M' } else { 'm' }),
            Key::N => Some(if shift { 'N' } else { 'n' }),
            Key::O => Some(if shift { 'O' } else { 'o' }),
            Key::P => Some(if shift { 'P' } else { 'p' }),
            Key::Q => Some(if shift { 'Q' } else { 'q' }),
            Key::R => Some(if shift { 'R' } else { 'r' }),
            Key::S => Some(if shift { 'S' } else { 's' }),
            Key::T => Some(if shift { 'T' } else { 't' }),
            Key::U => Some(if shift { 'U' } else { 'u' }),
            Key::V => Some(if shift { 'V' } else { 'v' }),
            Key::W => Some(if shift { 'W' } else { 'w' }),
            Key::X => Some(if shift { 'X' } else { 'x' }),
            Key::Y => Some(if shift { 'Y' } else { 'y' }),
            Key::Z => Some(if shift { 'Z' } else { 'z' }),
            Key::NumpadNum0 => Some('0'),
            Key::NumpadNum1 => Some('1'),
            Key::NumpadNum2 => Some('2'),
            Key::NumpadNum3 => Some('3'),
            Key::NumpadNum4 => Some('4'),
            Key::NumpadNum5 => Some('5'),
            Key::NumpadNum6 => Some('6'),
            Key::NumpadNum7 => Some('7'),
            Key::NumpadNum8 => Some('8'),
            Key::NumpadNum9 => Some('9'),
            Key::NumpadAdd => Some('+'),
            Key::NumpadSubtract => Some('-'),
            Key::NumpadMultiply => Some('*'),
            Key::NumpadDivide => Some('/'),
            Key::NumpadDecimal => Some('.'),
            _ => None,
        }
    }

    fn apply_text_input(&mut self, text: &str) -> bool {
        if let Some(fref) = self.active_field {
            if let Some(field_snapshot) = self.get_field(fref).cloned() {
                match field_snapshot.kind {
                    FieldKind::Text => {
                        self.field_edit.insert_text(text);
                        let new_val = self.field_edit.text.clone();
                        if let Some(f) = self.get_field_mut(fref) {
                            f.value = FieldValue::Text(new_val);
                        }
                        self.mark_dirty(fref.node_id);
                        return true;
                    }
                    FieldKind::FilePath { .. } => {
                        self.field_edit.insert_text(text);
                        let new_val = self.field_edit.text.clone();
                        if let Some(f) = self.get_field_mut(fref) {
                            f.value = FieldValue::FilePath(PathBuf::from(new_val));
                        }
                        self.mark_dirty(fref.node_id);
                        return true;
                    }
                    FieldKind::Number { min, max, .. } => {
                        let filtered: String = text
                            .chars()
                            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E')
                            .collect();
                        if filtered.is_empty() {
                            return false;
                        }
                        self.field_edit.insert_text(&filtered);
                        if let Ok(v) = self.field_edit.text.trim().parse::<f32>() {
                            if let Some(f) = self.get_field_mut(fref) {
                                f.value = FieldValue::Number(v.clamp(min, max));
                            }
                        }
                        self.mark_dirty(fref.node_id);
                        return true;
                    }
                    FieldKind::Select { .. } | FieldKind::Bool => {}
                }
            }
        }
        false
    }

    /// Push the post-mutation text from `field_edit` back into the active
    /// field's `FieldValue`, after a key-driven edit (backspace, delete,
    /// undo, paste, ...). Mirrors what `apply_text_input` does for inserts.
    fn sync_active_field_from_edit_buffer(&mut self) {
        let Some(fref) = self.active_field else {
            return;
        };
        let new_val = self.field_edit.text.clone();
        let kind = match self.get_field(fref) {
            Some(f) => f.kind.clone(),
            None => return,
        };
        match kind {
            FieldKind::Text => {
                if let Some(f) = self.get_field_mut(fref) {
                    f.value = FieldValue::Text(new_val);
                }
            }
            FieldKind::FilePath { .. } => {
                if let Some(f) = self.get_field_mut(fref) {
                    f.value = FieldValue::FilePath(PathBuf::from(new_val));
                }
            }
            FieldKind::Number { min, max, .. } => {
                if let Ok(v) = new_val.trim().parse::<f32>() {
                    if let Some(f) = self.get_field_mut(fref) {
                        f.value = FieldValue::Number(v.clamp(min, max));
                    }
                }
            }
            FieldKind::Select { .. } | FieldKind::Bool => {}
        }
        self.mark_dirty(fref.node_id);
    }

    /// Dispatch a key event to the active field's editor. Returns
    /// [`EventResult::Consumed`] if the key was handled (cursor moved,
    /// text mutated, undo/redo, copy/cut/paste, select-all). Enter and
    /// Escape are *not* handled here — those are still handled by the
    /// caller (commit / cancel respectively) so the higher-level KeyPress
    /// routing keeps its semantics.
    /// If a Text field is active and has a cached TextInput, forward the
    /// event to the library widget and sync the FieldValue from its post-
    /// event text. Returns Some(result) if routed; None to indicate this
    /// path doesn't apply (no active field, not a Text field, no cached
    /// widget).
    ///
    /// Centralizing event routing through the library `TextInput` widget
    /// kills the duplicate FieldEditState code path for Text fields and
    /// gives us proper click-positioned cursor, selection rendering, and
    /// all the keyboard editing the wave 1+2 fix wave already shipped.
    fn route_text_event_to_input(
        &mut self,
        event: &Event,
        theme: &Theme,
    ) -> Option<EventResult> {
        let fref = self.active_field?;
        let kind = self
            .graph
            .nodes
            .iter()
            .find(|n| n.id == fref.node_id)?
            .fields
            .iter()
            .find(|f| f.id == fref.field_id)?
            .kind
            .clone();
        if !matches!(kind, FieldKind::Text) {
            return None;
        }
        // Layout TextInput to the field's screen rect so its hit-testing
        // (used for click-positioning) sees the right coordinates, then
        // forward.
        let world_rect = self.field_rects.get(&fref).copied()?;
        let screen_rect = self.to_screen_rect(world_rect);
        let result = if let Some(FieldWidgetEntry::Text(ti)) =
            self.field_widgets.get_mut(&fref)
        {
            ti.layout(screen_rect, theme);
            // Force focused — TextInput clears focus on outside-click and
            // we may have set active_field via a different code path.
            // TextInput's KeyPress branch checks focused before consuming.
            <TextInput as erigui_core::Widget>::set_focused(ti, true);
            let r = ti.handle_event(event, theme);
            if matches!(r, EventResult::Consumed) {
                let new_text = ti.text().to_string();
                Some((r, new_text))
            } else {
                Some((r, String::new()))
            }
        } else {
            None
        };
        match result {
            Some((r, new_text)) if matches!(r, EventResult::Consumed) => {
                if let Some(field) = self.get_field_mut(fref) {
                    if let FieldValue::Text(ref s) = field.value {
                        if *s != new_text {
                            field.value = FieldValue::Text(new_text);
                            self.mark_dirty(fref.node_id);
                        }
                    }
                }
                Some(r)
            }
            Some((r, _)) => Some(r),
            None => None,
        }
    }

    fn handle_edit_key(&mut self, key: &KeyPressEvent) -> EventResult {
        if self.active_field.is_none() {
            return EventResult::Ignored;
        }
        let shift = key.modifiers.contains(Modifiers::SHIFT);
        // Linux AltGr emits Ctrl+Alt; require CTRL without ALT so AltGr-
        // typed characters (@, €, etc.) aren't eaten as shortcuts.
        let ctrl = key.modifiers.contains(Modifiers::CTRL)
            && !key.modifiers.contains(Modifiers::ALT);
        match key.key {
            erigui_core::Key::Left => {
                let dir = if ctrl { CursorMove::WordLeft } else { CursorMove::Left };
                self.field_edit.move_cursor(dir, shift);
                EventResult::Consumed
            }
            erigui_core::Key::Right => {
                let dir = if ctrl { CursorMove::WordRight } else { CursorMove::Right };
                self.field_edit.move_cursor(dir, shift);
                EventResult::Consumed
            }
            erigui_core::Key::Home => {
                self.field_edit.move_cursor(CursorMove::Home, shift);
                EventResult::Consumed
            }
            erigui_core::Key::End => {
                self.field_edit.move_cursor(CursorMove::End, shift);
                EventResult::Consumed
            }
            erigui_core::Key::Backspace => {
                self.field_edit.delete_selection_or_one_back();
                self.sync_active_field_from_edit_buffer();
                EventResult::Consumed
            }
            erigui_core::Key::Delete => {
                self.field_edit.delete_selection_or_one_forward();
                self.sync_active_field_from_edit_buffer();
                EventResult::Consumed
            }
            erigui_core::Key::A if ctrl => {
                self.field_edit.select_all();
                EventResult::Consumed
            }
            erigui_core::Key::C if ctrl => {
                let sel = self.field_edit.selected_text().to_owned();
                if !sel.is_empty() {
                    crate::clipboard::set_clipboard(&sel);
                }
                EventResult::Consumed
            }
            erigui_core::Key::X if ctrl => {
                let sel = self.field_edit.selected_text().to_owned();
                if !sel.is_empty() {
                    crate::clipboard::set_clipboard(&sel);
                    self.field_edit.replace_selection("");
                    self.sync_active_field_from_edit_buffer();
                }
                EventResult::Consumed
            }
            erigui_core::Key::V if ctrl => {
                let pasted = crate::clipboard::get_clipboard();
                if !pasted.is_empty() {
                    // Single-line: strip newlines so a multi-line paste
                    // doesn't break field rendering.
                    let cleaned: String =
                        pasted.chars().filter(|c| *c != '\n' && *c != '\r').collect();
                    if !cleaned.is_empty() {
                        self.field_edit.insert_text(&cleaned);
                        self.sync_active_field_from_edit_buffer();
                    }
                }
                EventResult::Consumed
            }
            erigui_core::Key::Z if ctrl => {
                if shift {
                    self.field_edit.redo();
                } else {
                    self.field_edit.undo();
                }
                self.sync_active_field_from_edit_buffer();
                EventResult::Consumed
            }
            erigui_core::Key::Y if ctrl => {
                self.field_edit.redo();
                self.sync_active_field_from_edit_buffer();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

impl Widget for NodeGraph {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(
            constraints.max_width.unwrap_or(800),
            constraints.max_height.unwrap_or(600),
        )
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        self.update_port_positions();
        self.update_field_rects();
        self.layout_file_dialog(theme);
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        self.draw_background(context);
        self.draw_groups(context);
        self.draw_edges(context, theme);
        self.draw_nodes(context, theme);
        self.draw_temp_link(context, theme);
        self.draw_marquee(context, theme);
        self.draw_select_overlay(context, theme);
        if self.file_dialog.is_some() {
            context.set_color(theme.colors.overlay.with_alpha(235));
            context.fill_rect(self.state.bounds);
        }
        if let Some(dialog) = &self.file_dialog {
            dialog.draw(context, theme);
        }
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if let Some(overlay) = &self.select_overlay {
            if let Event::MouseButton(_ev) = event {
                if overlay.rect.contains(match event {
                    Event::MouseButton(ev) => ev.position,
                    Event::MouseMove(ev) => ev.position,
                    Event::MouseWheel(ev) => ev.position,
                    _ => Point::ZERO,
                }) {
                    // let MouseButton logic handle it
                }
            }
        }
        let mut file_selected: Option<String> = None;
        if let Some(dialog) = &mut self.file_dialog {
            let r = dialog.handle_event(event, theme);
            if r.is_consumed() {
                return r;
            }
            if let Some(path) = dialog.get_selected_path() {
                file_selected = Some(path.display().to_string());
            }
            if !dialog.is_visible() {
                self.file_dialog = None;
            }
        }
        if let Some(path_str) = file_selected {
            if let Some(fref) = self.pending_file_field {
                if let Some(field) = self.get_field_mut(fref) {
                    field.value = FieldValue::FilePath(PathBuf::from(&path_str));
                }
                if let Some(node_id) = self.pending_file_node {
                    self.refresh_preview_for(node_id, &path_str);
                }
                self.mark_dirty(fref.node_id);
                self.pending_file_field = None;
                self.pending_file_node = None;
            }
            self.file_dialog = None;
        }
        match event {
            Event::DragDrop(dd) => match dd {
                DragDropEvent::Enter { position, data }
                | DragDropEvent::Over { position, data } => {
                    if matches!(data, DragData::Files(_)) {
                        self.update_drop_highlight(*position);
                        return EventResult::Consumed;
                    }
                }
                DragDropEvent::Leave => {
                    self.clear_drop_highlight();
                    return EventResult::Consumed;
                }
                DragDropEvent::Drop { position, data } => {
                    let result = match data {
                        DragData::Files(files) => self.handle_file_drop(files, *position),
                        _ => EventResult::Ignored,
                    };
                    self.clear_drop_highlight();
                    return result;
                }
            },
            Event::TextInput(te) => {
                self.state.focused = true;
                // First try routing through the cached library TextInput
                // (the library widget IS the editor for Text fields). If
                // it consumed, return; else fall through to the legacy
                // FieldEditState path which handles non-Text fields and
                // provides backstop for Text when the cache is missing.
                if let Some(r) = self.route_text_event_to_input(event, theme) {
                    if matches!(r, EventResult::Consumed) {
                        return r;
                    }
                }
                if self.apply_text_input(&te.text) {
                    if let Some(active) = self.active_field {
                        self.sync_field_widget_from_value(active);
                    }
                    return EventResult::Consumed;
                }
            }
            Event::MouseMove(ev) => {
                let screen_pos = ev.position;
                let world_pos = self.screen_to_world(screen_pos);
                let mut needs_hover_update = false;
                let mut result = EventResult::Ignored;
                // Slider drag: forward to the cached slider, then continue.
                if let Some(Interaction::FieldSliderDrag { fref }) = self.interaction.clone() {
                    self.route_move_to_slider(fref, screen_pos, theme);
                    self.last_cursor_world = world_pos;
                    return EventResult::Consumed;
                }
                if let Some(interaction) = self.interaction.as_mut() {
                    match interaction {
                        Interaction::NodeDrag {
                            node_id,
                            last_world,
                        } => {
                            let delta =
                                Point::new(world_pos.x - last_world.x, world_pos.y - last_world.y);
                            if delta.x != 0 || delta.y != 0 {
                                let moving_ids: Vec<usize> =
                                    if self.selected_nodes.contains(node_id) {
                                        self.selected_nodes.iter().copied().collect()
                                    } else {
                                        vec![*node_id]
                                    };
                                for id in moving_ids {
                                    if let Some(node) =
                                        self.graph.nodes.iter_mut().find(|n| n.id == id)
                                    {
                                        node.position.x += delta.x;
                                        node.position.y += delta.y;
                                    }
                                }
                                *last_world = world_pos;
                                self.update_port_positions();
                                result = EventResult::Consumed;
                            }
                        }
                        Interaction::LinkDrag { cursor, .. } => {
                            *cursor = world_pos;
                            needs_hover_update = true;
                            result = EventResult::Consumed;
                        }
                        Interaction::NodeResize {
                            node_id,
                            last_world,
                        } => {
                            let delta =
                                Point::new(world_pos.x - last_world.x, world_pos.y - last_world.y);
                            if delta.x != 0 || delta.y != 0 {
                                if let Some(node) =
                                    self.graph.nodes.iter_mut().find(|n| n.id == *node_id)
                                {
                                    let new_w =
                                        (node.size.width + delta.x).max(MIN_NODE_SIZE.width);
                                    let new_h =
                                        (node.size.height + delta.y).max(MIN_NODE_SIZE.height);
                                    node.size = Size::new(new_w, new_h);
                                }
                                *last_world = world_pos;
                                self.update_field_rects();
                                self.update_port_positions();
                                result = EventResult::Consumed;
                            }
                        }
                        Interaction::Pan { last, moved } => {
                            let delta = Point::new(screen_pos.x - last.x, screen_pos.y - last.y);
                            if delta.x != 0 || delta.y != 0 {
                                *moved = true;
                                self.pan.x += delta.x as f32;
                                self.pan.y += delta.y as f32;
                            }
                            *last = screen_pos;
                            result = EventResult::Consumed;
                        }
                        Interaction::Marquee { current_world, .. } => {
                            *current_world = world_pos;
                            needs_hover_update = false;
                            result = EventResult::Consumed;
                        }
                        Interaction::FieldSliderDrag { .. } => {
                            // Already handled by early-return above.
                        }
                    }
                } else {
                    needs_hover_update = true;
                }
                self.last_cursor_world = world_pos;
                if needs_hover_update {
                    self.hovered_port = self.port_at_screen(screen_pos);
                    self.hovered_field = self.field_at_screen(screen_pos);
                    self.hovered_edge =
                        self.nearest_edge_to_screen(screen_pos, self.scale(12).clamp(8, 18));
                }
                if result.is_consumed() {
                    return result;
                }
            }
            Event::MouseButton(ev) => {
                if ev.pressed && self.field_at_screen(ev.position).is_none() {
                    self.state.focused = false;
                    self.active_field = None;
                }
                if let Some(overlay) = self.select_overlay.clone() {
                    if ev.pressed {
                        if overlay.rect.contains(ev.position) {
                            let row_h = self.scale(22).clamp(18, 32);
                            let mut y = overlay.rect.y() + self.scale(6);
                            for opt in overlay.options.iter() {
                                let item_rect = Rect::new(
                                    overlay.rect.x() + self.scale(4),
                                    y,
                                    overlay.rect.width() - self.scale(8),
                                    row_h,
                                );
                                if item_rect.contains(ev.position) {
                                    let node_id = overlay.field.node_id;
                                    if let Some(field) = self.get_field_mut(overlay.field) {
                                        field.value = FieldValue::Select(opt.clone());
                                    }
                                    // Keep cached ComboBox in sync for downstream queries.
                                    self.sync_field_widget_from_value(overlay.field);
                                    self.select_overlay = None;
                                    self.mark_dirty(node_id);
                                    return EventResult::Consumed;
                                }
                                y += row_h + self.scale(2);
                            }
                        }
                        // click outside closes
                        self.select_overlay = None;
                    }
                }

                let ev: &MouseButtonEvent = ev;
                let world_pos = self.screen_to_world(ev.position);
                match ev.button {
                    MouseButton::Left => {
                        if ev.pressed {
                            // Wave 3 (C4): any left-click while the add-node
                            // menu is open closes it (the rendered menu would
                            // otherwise own the click; until that landing,
                            // close-on-outside-click here keeps the state
                            // consistent with the test surface).
                            if self.add_node_menu.is_some() {
                                self.close_add_menu();
                                return EventResult::Consumed;
                            }
                            // field interaction first
                            if let Some(field_ref) = self.field_at_screen(ev.position) {
                                if let Some(field_snapshot) = self
                                    .graph
                                    .nodes
                                    .iter()
                                    .find(|n| n.id == field_ref.node_id)
                                    .and_then(|n| {
                                        n.fields.iter().find(|f| f.id == field_ref.field_id)
                                    })
                                {
                                    match &field_snapshot.kind {
                                        FieldKind::Select { options } => {
                                            if let Some(fr) =
                                                self.field_rects.get(&field_ref).cloned()
                                            {
                                                let screen_rect = self.to_screen_rect(fr);
                                                let row_h = self.scale(22).clamp(18, 32);
                                                let max_rows = 8;
                                                let h = ((options.len().min(max_rows) as i32)
                                                    * (row_h + self.scale(2))
                                                    + self.scale(12))
                                                .min(self.state.bounds.height() / 2);
                                                let rect = Rect::new(
                                                    screen_rect.x(),
                                                    (screen_rect.bottom())
                                                        .min(self.state.bounds.bottom() - h),
                                                    screen_rect.width(),
                                                    h,
                                                );
                                                self.select_overlay = Some(SelectOverlay {
                                                    field: field_ref,
                                                    options: options.clone(),
                                                    rect,
                                                });
                                            }
                                        }
                                        FieldKind::FilePath { .. } => {
                                            self.pending_file_field = Some(field_ref);
                                            self.pending_file_node = Some(field_ref.node_id);
                                            let dlg = FileDialog::new(
                                                WidgetId::default(),
                                                FileDialogMode::Open,
                                            );
                                            self.file_dialog = Some(dlg);
                                            self.layout_file_dialog(theme);
                                        }
                                        FieldKind::Bool => {
                                            // Toggle in-place; route to cached checkbox state.
                                            let new_val = match &field_snapshot.value {
                                                FieldValue::Bool(b) => !b,
                                                _ => true,
                                            };
                                            if let Some(f) = self.get_field_mut(field_ref) {
                                                f.value = FieldValue::Bool(new_val);
                                            }
                                            if let Some(FieldWidgetEntry::Bool(cb)) =
                                                self.field_widgets.get_mut(&field_ref)
                                            {
                                                cb.set_checked(new_val);
                                            }
                                            self.mark_dirty(field_ref.node_id);
                                        }
                                        FieldKind::Number { .. } => {
                                            // Route the press to the cached slider so a
                                            // press-then-drag updates the value live.
                                            self.route_press_to_slider(field_ref, ev, theme);
                                        }
                                        FieldKind::Text => {
                                            // Route through the cached library
                                            // TextInput widget — it's the source
                                            // of truth for cursor/selection/edit
                                            // logic. Sync first so its text
                                            // matches the field, then layout it
                                            // to the field's screen rect so
                                            // hit-testing inside TextInput uses
                                            // correct coordinates, then forward
                                            // the click to position the cursor.
                                            self.active_field = Some(field_ref);
                                            self.state.focused = true;
                                            self.ensure_field_widgets();
                                            self.sync_field_widget_from_value(field_ref);
                                            if let Some(world_rect) =
                                                self.field_rects.get(&field_ref).copied()
                                            {
                                                let screen_rect =
                                                    self.to_screen_rect(world_rect);
                                                if let Some(FieldWidgetEntry::Text(ti)) =
                                                    self.field_widgets.get_mut(&field_ref)
                                                {
                                                    ti.layout(screen_rect, theme);
                                                    let _ = ti.handle_event(
                                                        &Event::MouseButton(*ev),
                                                        theme,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                return EventResult::Consumed;
                            }

                            self.active_field = None;
                            // resize handle check
                            if let Some(node_id) = self.resize_handle_at(ev.position) {
                                self.interaction = Some(Interaction::NodeResize {
                                    node_id,
                                    last_world: world_pos,
                                });
                                return EventResult::Consumed;
                            }
                            // start a link if we clicked a port
                            if let Some(port) = self.port_at_screen(ev.position) {
                                self.interaction = Some(Interaction::LinkDrag {
                                    from: port,
                                    cursor: world_pos,
                                });
                                return EventResult::Consumed;
                            }

                            // select and drag a node
                            if let Some(node_id) = self.node_at(world_pos) {
                                let toggle = ev.modifiers.contains(Modifiers::SHIFT);
                                self.handle_selection(node_id, toggle);
                                self.interaction = Some(Interaction::NodeDrag {
                                    node_id,
                                    last_world: world_pos,
                                });
                                self.last_cursor_world = world_pos;
                                return EventResult::Consumed;
                            }

                            if ev.modifiers.contains(Modifiers::SHIFT) {
                                self.interaction = Some(Interaction::Marquee {
                                    start_world: world_pos,
                                    current_world: world_pos,
                                });
                            } else {
                                self.interaction = Some(Interaction::Pan {
                                    last: ev.position,
                                    moved: false,
                                });
                            }
                            return EventResult::Consumed;
                        } else {
                            let mut consumed = false;
                            if let Some(interaction) = self.interaction.take() {
                                match interaction {
                                    Interaction::NodeDrag { node_id, .. } => {
                                        self.snap_after_drag(node_id);
                                        self.update_field_rects();
                                        consumed = true;
                                    }
                                    Interaction::NodeResize { node_id, .. } => {
                                        self.update_port_positions();
                                        self.update_field_rects();
                                        self.snap_after_drag(node_id);
                                        consumed = true;
                                    }
                                    Interaction::LinkDrag { from, .. } => {
                                        if let Some(target) = self.port_at_screen(ev.position) {
                                            if target != from && target.is_input != from.is_input {
                                                let (out, inp) = if from.is_input {
                                                    (target, from)
                                                } else {
                                                    (from, target)
                                                };
                                                if out.node_id != inp.node_id {
                                                    // remove existing duplicate edge
                                                    if !self.graph.edges.iter().any(|e| {
                                                        e.from_node == out.node_id
                                                            && e.from_port == out.port_id
                                                            && e.to_node == inp.node_id
                                                            && e.to_port == inp.port_id
                                                    }) {
                                                        self.graph.edges.push(Edge {
                                                            from_node: out.node_id,
                                                            from_port: out.port_id,
                                                            to_node: inp.node_id,
                                                            to_port: inp.port_id,
                                                        });
                                                        self.mark_dirty(out.node_id);
                                                        self.mark_dirty(inp.node_id);
                                                    }
                                                }
                                            }
                                        }
                                        consumed = true;
                                    }
                                    Interaction::Marquee {
                                        start_world,
                                        current_world,
                                    } => {
                                        let toggle = ev.modifiers.contains(Modifiers::SHIFT);
                                        let rect = rect_from_points(start_world, current_world);
                                        self.select_nodes_in_rect(rect, toggle);
                                        consumed = true;
                                    }
                                    Interaction::FieldSliderDrag { fref } => {
                                        self.route_release_to_slider(fref, ev, theme);
                                        consumed = true;
                                    }
                                    Interaction::Pan { .. } => {}
                                }
                            }
                            return if consumed {
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            };
                        }
                    }
                    MouseButton::Right => {
                        if ev.pressed {
                            self.interaction = Some(Interaction::Pan {
                                last: ev.position,
                                moved: false,
                            });
                        } else {
                            let mut consumed = false;
                            // Snapshot whether the press was a click (no pan
                            // drag) before we discard the Pan interaction —
                            // we'll consult it for the Wave-3-C4 add-menu
                            // open path even after the port/edge checks.
                            let was_click = matches!(
                                &self.interaction,
                                Some(Interaction::Pan { moved: false, .. })
                            );
                            if was_click {
                                if let Some(port) = self.port_at_screen(ev.position) {
                                    consumed = self.remove_edges_for_port(port);
                                } else if let Some(edge_idx) = self.nearest_edge_to_screen(
                                    ev.position,
                                    self.scale(12).clamp(8, 18),
                                ) {
                                    self.graph.edges.remove(edge_idx);
                                    consumed = true;
                                }
                            }
                            self.interaction = None;
                            if consumed {
                                self.hovered_edge = None;
                                return EventResult::Consumed;
                            }
                            // Wave 3 (C4): if the click was on empty canvas
                            // (no port, no edge, no node) and a registry is
                            // wired, open the add-node menu at the world
                            // position the user right-clicked.
                            if was_click {
                                let on_node = self.node_at(world_pos).is_some();
                                let on_port = self.port_at_screen(ev.position).is_some();
                                let on_edge = self
                                    .nearest_edge_to_screen(
                                        ev.position,
                                        self.scale(12).clamp(8, 18),
                                    )
                                    .is_some();
                                let on_handle = self.resize_handle_at(ev.position).is_some();
                                if !on_node && !on_port && !on_edge && !on_handle {
                                    if let Some(reg) = self.add_node_registry.clone() {
                                        self.open_add_menu_at(world_pos, reg);
                                        return EventResult::Consumed;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::KeyPress(key) => {
                // Wave 3 (C4): Esc closes the add-node menu before any other
                // KeyPress routing so it doesn't fall through into field-edit
                // commit-or-cancel logic below.
                if self.add_node_menu.is_some() && key.key == erigui_core::Key::Escape {
                    self.close_add_menu();
                    return EventResult::Consumed;
                }
                // If no active field but we're hovering a text field, start editing on first keypress.
                if self.active_field.is_none() {
                    if let Some(hov) = self.hovered_field {
                        if let Some(f_snapshot) = self.get_field(hov).cloned() {
                            if matches!(
                                f_snapshot.kind,
                                FieldKind::Text
                                    | FieldKind::FilePath { .. }
                                    | FieldKind::Number { .. }
                            ) {
                                self.active_field = Some(hov);
                                self.field_edit =
                                    FieldEditState::new(self.field_value_str(&f_snapshot));
                            }
                        }
                    }
                }

                if let Some(active) = self.active_field {
                    match key.key {
                        erigui_core::Key::Enter => {
                            self.commit_field_edit();
                            return EventResult::Consumed;
                        }
                        erigui_core::Key::Escape => {
                            self.active_field = None;
                            return EventResult::Consumed;
                        }
                        _ => {
                            // Wave 4: route Text-field key events through
                            // the cached library TextInput. Falls through
                            // to legacy handle_edit_key for non-Text fields.
                            if let Some(r) = self.route_text_event_to_input(event, theme) {
                                if matches!(r, EventResult::Consumed) {
                                    return r;
                                }
                            }
                            // Route arrows, Home/End, Backspace, Delete,
                            // Ctrl+A/C/X/V/Z/Y through the editor first.
                            // `handle_edit_key` returns Ignored for keys
                            // it doesn't claim, in which case we fall
                            // through to the printable-char path.
                            let r = self.handle_edit_key(key);
                            if matches!(r, EventResult::Consumed) {
                                self.sync_field_widget_from_value(active);
                                return EventResult::Consumed;
                            }
                            // Wave 1 (A3): translate printable KeyPress events into
                            // character input on the active field. The host normally
                            // sends Event::TextInput for typed characters, but tests
                            // and some headless drivers fire only KeyPress events.
                            // Skip when Ctrl is held — those are reserved for
                            // editor shortcuts handled above (and any unhandled
                            // ones, e.g. Ctrl+S, should NOT type a literal "s").
                            if !key.modifiers.contains(Modifiers::CTRL) {
                                if let Some(ch) = Self::keypress_to_char(key) {
                                    let s = ch.to_string();
                                    if self.apply_text_input(&s) {
                                        self.sync_field_widget_from_value(active);
                                        return EventResult::Consumed;
                                    }
                                }
                            }
                            return EventResult::Ignored;
                        }
                    }
                }
                match key.key {
                    erigui_core::Key::Delete | erigui_core::Key::Backspace => {
                        let mut removed = false;
                        if let Some(port) = self.hovered_port {
                            removed |= self.remove_edges_for_port(port);
                        }
                        if let Some(edge_idx) = self.hovered_edge {
                            if edge_idx < self.graph.edges.len() {
                                let e = self.graph.edges.remove(edge_idx);
                                self.mark_dirty(e.from_node);
                                self.mark_dirty(e.to_node);
                                removed = true;
                            }
                        }
                        if !self.selected_nodes.is_empty() {
                            removed |= self.remove_selected_nodes();
                        }
                        if removed {
                            self.hovered_edge = None;
                            return EventResult::Consumed;
                        }
                    }
                    erigui_core::Key::A => {
                        self.add_node_at_cursor();
                        return EventResult::Consumed;
                    }
                    erigui_core::Key::Space | erigui_core::Key::F => {
                        self.fit_view();
                        return EventResult::Consumed;
                    }
                    erigui_core::Key::R | erigui_core::Key::Home => {
                        self.reset_view();
                        return EventResult::Consumed;
                    }
                    erigui_core::Key::D => {
                        self.duplicate_selected();
                        return EventResult::Consumed;
                    }
                    erigui_core::Key::X => {
                        self.disconnect_selected();
                        return EventResult::Consumed;
                    }
                    _ => {}
                }
            }
            Event::MouseWheel(wheel) => {
                let cursor = wheel.position;
                let world_before = self.screen_to_world(cursor);
                let scroll = wheel.delta.y as f32;
                let factor = 1.0 + scroll * 0.1;
                let new_zoom = (self.zoom * factor).clamp(0.5, 2.5);
                self.zoom = new_zoom;
                self.pan.x = cursor.x as f32 - world_before.x as f32 * self.zoom;
                self.pan.y = cursor.y as f32 - world_before.y as f32 * self.zoom;
                return EventResult::Consumed;
            }
            _ => {}
        }
        EventResult::Ignored
    }

    fn children(&self) -> &[WidgetId] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [WidgetId] {
        &mut []
    }

    fn hit_test(&self, point: Point) -> Option<WidgetId> {
        if self.state.bounds.contains(point) {
            Some(self.state.id)
        } else {
            None
        }
    }

    fn bounds(&self) -> Rect {
        self.state.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
        self.update_port_positions();
        self.update_field_rects();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn is_visible(&self) -> bool {
        self.state.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.state.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn is_focused(&self) -> bool {
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
    }
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f32 {
    let px = p.x as f32;
    let py = p.y as f32;
    let ax = a.x as f32;
    let ay = a.y as f32;
    let bx = b.x as f32;
    let by = b.y as f32;
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab_len2 = abx * abx + aby * aby;
    let t = if ab_len2.abs() < f32::EPSILON {
        0.0
    } else {
        (apx * abx + apy * aby) / ab_len2
    }
    .clamp(0.0, 1.0);
    let closest_x = ax + abx * t;
    let closest_y = ay + aby * t;
    let dx = px - closest_x;
    let dy = py - closest_y;
    (dx * dx + dy * dy).sqrt()
}

fn rect_from_points(a: Point, b: Point) -> Rect {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn load_image_preview(path: &str) -> Option<PreviewBitmap> {
    let img = image::open(path).ok()?;
    let (orig_w, orig_h) = img.dimensions();
    let max_dim: u32 = 1600;
    let scaled = if orig_w > max_dim || orig_h > max_dim {
        let scale = f32::min(
            max_dim as f32 / orig_w as f32,
            max_dim as f32 / orig_h as f32,
        );
        let new_w = (orig_w as f32 * scale).round().max(1.0) as u32;
        let new_h = (orig_h as f32 * scale).round().max(1.0) as u32;
        img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let thumb = scaled.to_rgba8();
    let (w, h) = thumb.dimensions();
    Some(PreviewBitmap {
        width: w as i32,
        height: h as i32,
        data: thumb.into_raw(),
    })
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: usize,
    pub title: String,
    pub rect: Rect,
    pub color: Color,
}

#[derive(Clone, Debug)]
struct SelectOverlay {
    field: FieldRef,
    options: Vec<String>,
    rect: Rect, // screen-space
}

impl Vec2f {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

const SNAP: i32 = 12;
const MIN_NODE_SIZE: Size = Size {
    width: 160,
    height: 120,
};

fn snap_point(p: &mut Point) {
    p.x = ((p.x + SNAP / 2) / SNAP) * SNAP;
    p.y = ((p.y + SNAP / 2) / SNAP) * SNAP;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_edit_basic() {
        // 1. New state with "hello", cursor at end.
        let mut s = FieldEditState::new("hello".to_string());
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5);

        // 2. Move cursor to position 2 ("he|llo").
        s.cursor = 2;
        s.selection_anchor = None;

        // 3. Insert "X" -> "heXllo", cursor at 3.
        s.insert_text("X");
        assert_eq!(s.text, "heXllo");
        assert_eq!(s.cursor, 3);

        // 4. Backspace -> "hello", cursor at 2.
        s.delete_selection_or_one_back();
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 2);

        // 5. Select all + delete (forward) -> empty.
        s.select_all();
        assert_eq!(s.selected_text(), "hello");
        s.delete_selection_or_one_forward();
        assert_eq!(s.text, "");
        assert_eq!(s.cursor, 0);
        assert!(s.selection_anchor.is_none());
    }

    #[test]
    fn field_edit_word_motion_and_undo() {
        // Word motion + undo/redo coverage in the same test so a
        // single-test failure surfaces both regressions independently
        // (each assertion below carries its own check).
        let mut s = FieldEditState::new("foo bar  baz".to_string());
        s.cursor = 0;
        s.selection_anchor = None;

        // WordRight: "foo" -> "bar"
        s.move_cursor(CursorMove::WordRight, false);
        assert_eq!(s.cursor, 4, "WordRight should land on 'b' of 'bar'");

        // End -> last index
        s.move_cursor(CursorMove::End, false);
        assert_eq!(s.cursor, s.text.len());

        // WordLeft from end -> "baz"
        s.move_cursor(CursorMove::WordLeft, false);
        assert_eq!(s.cursor, 9, "WordLeft from end should land on 'b' of 'baz'");

        // Shift+WordLeft from 9 -> selects "  " (anchor=9, cursor=7)
        s.move_cursor(CursorMove::WordLeft, true);
        assert_eq!(s.cursor, 4);
        assert_eq!(s.selection_anchor, Some(9));

        // Replace selection
        s.replace_selection("Q");
        assert_eq!(s.text, "foo Qbaz");

        // Undo restores prior state.
        s.undo();
        assert_eq!(s.text, "foo bar  baz");

        // Redo re-applies it.
        s.redo();
        assert_eq!(s.text, "foo Qbaz");
    }

    #[test]
    fn field_edit_utf8_multibyte_cursor_moves_by_char() {
        // "héllo" — 'é' is 2 bytes (0xC3 0xA9). Total len = 6 bytes, 5 chars.
        let mut s = FieldEditState::new("héllo".to_string());
        assert_eq!(s.text.len(), 6);
        s.cursor = 0;

        // Right once: should land at byte 1 (after 'h').
        s.move_cursor(CursorMove::Right, false);
        assert_eq!(s.cursor, 1);

        // Right again: should land at byte 3 (skipping the 2-byte 'é').
        s.move_cursor(CursorMove::Right, false);
        assert_eq!(s.cursor, 3, "must skip both bytes of 'é' as one char");

        // Left from 3: back to 1 (single char step over 'é').
        s.move_cursor(CursorMove::Left, false);
        assert_eq!(s.cursor, 1);
    }

    #[test]
    fn field_edit_backspace_at_start_is_noop() {
        let mut s = FieldEditState::new("hello".to_string());
        s.cursor = 0;
        s.delete_selection_or_one_back();
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn field_edit_delete_at_end_is_noop() {
        let mut s = FieldEditState::new("hello".to_string());
        s.cursor = 5;
        s.delete_selection_or_one_forward();
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5);
    }

    #[test]
    fn field_edit_select_all_then_type_replaces() {
        let mut s = FieldEditState::new("hello world".to_string());
        s.select_all();
        assert_eq!(s.selected_text(), "hello world");
        s.insert_text("X");
        assert_eq!(s.text, "X");
        assert_eq!(s.cursor, 1);
        assert!(s.selection_anchor.is_none());
    }

    #[test]
    fn field_edit_shift_right_extends_selection_anchor_stays() {
        let mut s = FieldEditState::new("hello".to_string());
        s.cursor = 1;
        s.selection_anchor = None;

        s.move_cursor(CursorMove::Right, true);
        assert_eq!(s.cursor, 2);
        assert_eq!(s.selection_anchor, Some(1), "anchor should pin at start");

        s.move_cursor(CursorMove::Right, true);
        assert_eq!(s.cursor, 3);
        assert_eq!(s.selection_anchor, Some(1), "anchor must NOT move");
        assert_eq!(s.selected_text(), "el");
    }

    #[test]
    fn field_edit_home_clears_selection_when_no_shift() {
        let mut s = FieldEditState::new("hello".to_string());
        s.cursor = 4;
        s.selection_anchor = Some(1);

        s.move_cursor(CursorMove::Home, false);
        assert_eq!(s.cursor, 0);
        assert!(s.selection_anchor.is_none(), "Home without shift must clear anchor");
    }

    #[test]
    fn field_edit_redo_cleared_after_typing_post_undo() {
        let mut s = FieldEditState::new("a".to_string());
        s.cursor = 1;

        s.insert_text("b");
        assert_eq!(s.text, "ab");
        s.undo();
        assert_eq!(s.text, "a");

        // Now type something different — redo of the original "b" should be lost.
        s.insert_text("c");
        assert_eq!(s.text, "ac");

        s.redo();
        // Redo should be a no-op; not bring "ab" back.
        assert_eq!(s.text, "ac", "redo must be invalidated after a new edit");
    }

    #[test]
    fn field_edit_word_boundaries_skip_punctuation() {
        // Punctuation is non-word, treated like whitespace for jumps.
        let mut s = FieldEditState::new("hello, world!".to_string());
        s.cursor = 0;

        // Egui-style WordRight: skip the word AND following non-word run
        // in one jump. From 0 in "hello, world!": skip "hello" then ", "
        // → lands at start of "world" (byte 7).
        s.move_cursor(CursorMove::WordRight, false);
        assert_eq!(s.cursor, 7);

        // From 7: skip "world" then "!" (no trailing chars) → end-of-text.
        s.move_cursor(CursorMove::WordRight, false);
        assert_eq!(s.cursor, 13);
    }

    #[test]
    fn field_edit_select_word_at_inside_word() {
        let mut s = FieldEditState::new("foo bar baz".to_string());
        s.select_word_at(5); // middle of "bar"
        assert_eq!(s.selected_text(), "bar");
    }

    #[test]
    fn field_edit_select_word_at_in_whitespace_is_noop() {
        let mut s = FieldEditState::new("foo bar".to_string());
        // Position 3 is the space — both boundaries collapse, no selection.
        s.select_word_at(3);
        // Either no selection OR an empty one.
        assert!(s.selected_text().is_empty());
    }

    #[test]
    fn field_edit_select_word_at_empty_text_is_noop() {
        let mut s = FieldEditState::new(String::new());
        s.select_word_at(0);
        assert_eq!(s.text, "");
        assert!(s.selection_anchor.is_none());
    }

    #[test]
    fn field_edit_replace_selection_with_empty_deletes() {
        let mut s = FieldEditState::new("hello world".to_string());
        s.cursor = 5;
        s.selection_anchor = Some(0);
        // Selected "hello".
        s.replace_selection("");
        assert_eq!(s.text, " world");
        assert_eq!(s.cursor, 0);
        assert!(s.selection_anchor.is_none());
    }

    #[test]
    fn field_edit_insert_at_cursor_no_selection() {
        let mut s = FieldEditState::new("ac".to_string());
        s.cursor = 1;
        s.insert_text("b");
        assert_eq!(s.text, "abc");
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn field_edit_select_line_is_select_all() {
        let mut s = FieldEditState::new("hello".to_string());
        s.select_line();
        assert_eq!(s.selected_text(), "hello");
    }

    // Integration: end-to-end click-then-type flow, the actual user path
    // that "edit does not work" complaints come from. This is what real
    // event dispatch through NodeGraph::handle_event does.
    #[test]
    fn text_field_click_then_type_inserts_char() {
        use erigui_core::{
            Event, KeyPressEvent, Modifiers, MouseButton, MouseButtonEvent, TextInputEvent,
        };

        let theme = Theme::dark();
        let mut g = NodeGraph::new(WidgetId::default(), Graph::default());

        let fref = FieldRef {
            node_id: 1,
            field_id: 0,
        };
        let node = Node {
            id: 1,
            title: "TestNode".to_string(),
            position: Point::new(0, 0),
            size: Size::new(300, 200),
            inputs: vec![],
            outputs: vec![],
            fields: vec![Field {
                id: 0,
                name: "text".into(),
                label: "Text".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("hello".into()),
            }],
            component_type: None,
        };
        g.graph.nodes.push(node);
        g.layout(Rect::new(0, 0, 800, 600), &theme);

        let world_rect = g
            .field_rects
            .get(&fref)
            .copied()
            .expect("field rect must exist after layout");
        let screen_rect = g.to_screen_rect(world_rect);
        let click_pos = Point::new(
            screen_rect.x() + screen_rect.width() / 2,
            screen_rect.y() + screen_rect.height() / 2,
        );

        // Click on the field to activate it.
        let click = MouseButtonEvent {
            button: MouseButton::Left,
            pressed: true,
            position: click_pos,
            modifiers: Modifiers::empty(),
        };
        g.handle_event(&Event::MouseButton(click), &theme);
        assert_eq!(
            g.active_field,
            Some(fref),
            "click on Text field should set active_field"
        );

        // Synth TextInput("X") — what the renderer emits when user types X.
        let _ = g.handle_event(
            &Event::TextInput(TextInputEvent {
                text: "X".to_string(),
            }),
            &theme,
        );

        // The Field's value must now reflect the typed character.
        let v = &g.graph.nodes[0].fields[0].value;
        match v {
            FieldValue::Text(s) => {
                assert!(
                    s.contains('X'),
                    "expected 'X' in field value after typing, got: {s:?}"
                );
                assert_ne!(s, "hello", "field value must have CHANGED from initial");
            }
            _ => panic!("field should still be FieldValue::Text"),
        }

        // Backspace should remove the X.
        let bs = KeyPressEvent {
            key: erigui_core::Key::Backspace,
            modifiers: Modifiers::empty(),
            repeat: false,
        };
        g.handle_event(&Event::KeyPress(bs), &theme);
        let v = &g.graph.nodes[0].fields[0].value;
        match v {
            FieldValue::Text(s) => {
                assert!(
                    !s.contains('X'),
                    "backspace should have removed 'X', got: {s:?}"
                );
            }
            _ => panic!("field should still be FieldValue::Text"),
        }
    }

    /// Click in the LEFT portion of a text field. Cursor should land near
    /// the start, not at the end. This is what double-click-to-edit-then-
    /// type-in-the-middle workflows depend on. With the FieldEditState
    /// path this fails (cursor always lands at end).
    #[test]
    fn text_field_click_in_middle_positions_cursor() {
        use erigui_core::{Event, MouseButton, MouseButtonEvent};

        let theme = Theme::dark();
        let mut g = NodeGraph::new(WidgetId::default(), Graph::default());

        let fref = FieldRef { node_id: 1, field_id: 0 };
        let node = Node {
            id: 1,
            title: "TestNode".to_string(),
            position: Point::new(0, 0),
            size: Size::new(400, 200),
            inputs: vec![],
            outputs: vec![],
            fields: vec![Field {
                id: 0,
                name: "text".into(),
                label: "Text".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("aaaaaaaaaaaaaaaaaaaa".into()), // 20 a's
            }],
            component_type: None,
        };
        g.graph.nodes.push(node);
        g.layout(Rect::new(0, 0, 800, 600), &theme);

        let world_rect = g.field_rects.get(&fref).copied().unwrap();
        let screen_rect = g.to_screen_rect(world_rect);

        // Click near the LEFT edge — should put cursor near 0, not at 20.
        let click = MouseButtonEvent {
            button: MouseButton::Left,
            pressed: true,
            position: Point::new(
                screen_rect.x() + 8, // tiny offset from left edge
                screen_rect.y() + screen_rect.height() / 2,
            ),
            modifiers: Modifiers::empty(),
        };
        g.handle_event(&Event::MouseButton(click), &theme);
        assert_eq!(g.active_field, Some(fref));

        // Pull cursor position from whatever editor is active. With the
        // refactored path this comes from TextInput::cursor_pos via the
        // cached widget. With the broken FieldEditState path, cursor is
        // at the END (20), not near 0.
        let cursor = if let Some(FieldWidgetEntry::Text(ti)) =
            g.field_widgets.get(&fref)
        {
            ti.cursor_pos()
        } else {
            // Fallback: read from FieldEditState (current path).
            g.field_edit.cursor
        };

        assert!(
            cursor < 10,
            "cursor should be in the left half after a left-edge click, got {cursor}"
        );
    }
}

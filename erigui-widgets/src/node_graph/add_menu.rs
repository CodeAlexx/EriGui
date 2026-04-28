//! Wave 3 (C4): right-click "add node" search menu.
//!
//! Right-clicking on empty canvas pops up a menu listing every registered
//! `NodeType`, grouped by category, with a search box that filters by
//! `display_name` (case-insensitive). Selecting an entry inserts a new
//! [`Node`] at the click position with field defaults seeded from the node's
//! schema, then closes the menu.
//!
//! ## Decoupling note
//!
//! `erigui-widgets` cannot depend on `erigui-nodes` (the dependency goes the
//! other way), so the menu can't reference `erigui_nodes::NodeRegistry`
//! directly. Instead it reads through a [`NodeRegistryHandle`] trait that
//! erigui-nodes (or any host) implements; the widget stays generic over
//! "something that can list registered node types and seed a Node from a
//! type_id".

use std::sync::Arc;

use erigui_core::{DrawContext, Point, Rect, Size, Theme};

use super::{Field, FieldKind, FieldValue, Node, Port};

// ---------------------------------------------------------------------------
// Schema description (widget-side, decoupled from erigui-nodes)
// ---------------------------------------------------------------------------

/// Field metadata as exposed to the add-menu seeding code.
///
/// `name` is the schema field name (used for the value-form field label since
/// the existing `Field::label` is the one that shows in the node body).
#[derive(Clone, Debug)]
pub struct AddMenuFieldSpec {
    pub name: String,
    pub label: String,
    pub kind: FieldKind,
    pub default: FieldValue,
}

#[derive(Clone, Debug)]
pub struct AddMenuPortSpec {
    pub name: String,
}

/// Schema view used by the add-menu. Mirrors `erigui_nodes::NodeSchema` but
/// owns its strings so the widget can render without borrowing back into
/// the registry.
#[derive(Clone, Debug)]
pub struct AddMenuSchema {
    pub display_name: String,
    pub category: String,
    pub inputs: Vec<AddMenuPortSpec>,
    pub outputs: Vec<AddMenuPortSpec>,
    pub fields: Vec<AddMenuFieldSpec>,
}

// ---------------------------------------------------------------------------
// Registry handle trait (widget side)
// ---------------------------------------------------------------------------

/// Abstract registry shape the add-menu needs.
///
/// Widget code calls `entries()` to populate the menu and `schema(type_id)`
/// when the user clicks an item to seed a new [`Node`]. erigui-nodes (or any
/// other host) provides the impl that bridges this to its own `NodeRegistry`.
pub trait NodeRegistryHandle: Send + Sync {
    /// All registered node types. Each entry is `(type_id, display_name,
    /// category)`. Stable ordering not required.
    fn entries(&self) -> Vec<RegistryEntry>;

    /// Full schema for one type_id, or `None` if unknown.
    fn schema(&self, type_id: &str) -> Option<AddMenuSchema>;
}

/// Lightweight description of one registered node type (no schema body).
#[derive(Clone, Debug)]
pub struct RegistryEntry {
    pub type_id: String,
    pub display_name: String,
    pub category: String,
}

// ---------------------------------------------------------------------------
// Menu state
// ---------------------------------------------------------------------------

/// State for the add-node menu attached to a [`super::NodeGraph`].
///
/// `position` is in **world** coordinates (so the inserted node lands at the
/// world point the user right-clicked, regardless of pan/zoom). `filtered`
/// holds the type_ids currently visible after applying `search_text`.
pub struct AddNodeMenuState {
    pub position: Point,
    pub search_text: String,
    pub registry: Arc<dyn NodeRegistryHandle>,
    pub selected_idx: Option<usize>,
    /// Owned type_id strings rather than `&'static str` — registry entries
    /// are not `'static` by contract.
    pub filtered: Vec<String>,
    /// Snapshot of all registry entries taken at construction. Held so
    /// filtering doesn't have to round-trip the registry on every keystroke
    /// and so we can render `display_name`/`category` without re-querying.
    pub all_entries: Vec<RegistryEntry>,
}

impl AddNodeMenuState {
    /// Build a fresh menu anchored at `position` (world coords). `filtered`
    /// is populated with every registered type_id.
    pub fn new(position: Point, registry: Arc<dyn NodeRegistryHandle>) -> Self {
        let all_entries = registry.entries();
        let filtered = all_entries.iter().map(|e| e.type_id.clone()).collect();
        Self {
            position,
            search_text: String::new(),
            registry,
            selected_idx: None,
            filtered,
            all_entries,
        }
    }

    /// Re-filter `filtered` to only the type_ids whose display_name contains
    /// `query` (case-insensitive). Empty `query` resets to all entries.
    pub fn update_search(&mut self, query: &str) {
        self.search_text = query.to_string();
        let needle = query.trim().to_lowercase();
        self.selected_idx = None;
        if needle.is_empty() {
            self.filtered = self.all_entries.iter().map(|e| e.type_id.clone()).collect();
            return;
        }
        self.filtered = self
            .all_entries
            .iter()
            .filter(|e| e.display_name.to_lowercase().contains(&needle))
            .map(|e| e.type_id.clone())
            .collect();
    }

    /// Build a [`Node`] from the currently `selected_idx` (or the only
    /// filtered entry if there is one), with default fields seeded from the
    /// schema and position = `self.position`. Returns `None` if there is no
    /// resolvable selection or if the registry can't produce a schema.
    ///
    /// The caller (NodeGraph) is responsible for assigning `node.id` (it
    /// uses `next_node_id` already in `add_node`).
    pub fn select(&self) -> Option<Node> {
        let type_id = self.resolve_selection()?;
        let schema = self.registry.schema(&type_id)?;
        Some(node_from_schema(&type_id, &schema, self.position))
    }

    /// Pick the type_id the user is about to insert: explicit selection if
    /// present, else the first filtered entry (so a search-then-Enter flow
    /// works without a separate "select first row" step).
    fn resolve_selection(&self) -> Option<String> {
        let idx = self.selected_idx.unwrap_or(0);
        self.filtered.get(idx).cloned()
    }
}

/// Produce a default [`Node`] from a schema. `id` is left at `0` — caller
/// (NodeGraph) reassigns via `next_node_id()` in `add_node`.
fn node_from_schema(type_id: &str, schema: &AddMenuSchema, position: Point) -> Node {
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
    let fields: Vec<Field> = schema
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| Field {
            id: i,
            name: f.name.clone(),
            label: f.label.clone(),
            kind: f.kind.clone(),
            value: f.default.clone(),
        })
        .collect();
    Node {
        id: 0,
        title: schema.display_name.clone(),
        position,
        size: Size::new(220, 140),
        inputs,
        outputs,
        fields,
        component_type: Some(type_id.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Drawing (screen-space; called by NodeGraph::draw after node bodies are painted)
// ---------------------------------------------------------------------------

/// Pixel layout for the add-menu panel. Tuned for 1× zoom; `NodeGraph::draw`
/// passes a screen-space anchor so the menu doesn't pan/zoom with the canvas.
const MENU_W: i32 = 250;
const MENU_H: i32 = 400;
const SEARCH_BOX_H: i32 = 28;
const ROW_H: i32 = 22;
const PAD: i32 = 6;
const TEXT_PAD_X: i32 = 8;

/// Draw the add-node menu. `screen_pos` is the top-left corner of the panel
/// in screen coordinates (the caller is expected to convert
/// `state.position` from world → screen via `NodeGraph::to_screen_point`).
///
/// This is intentionally primitive — DrawContext exposes only rects and
/// text, and the menu doesn't need a real text input or scrollbar to be
/// useful for the C4 happy path (the data plane and event handlers already
/// drive `state.search_text` / `state.selected_idx`; this just makes the
/// menu visible).
pub(crate) fn draw_add_menu(
    ctx: &mut dyn DrawContext,
    theme: &Theme,
    state: &AddNodeMenuState,
    screen_pos: Point,
) {
    let panel = Rect::new(screen_pos.x, screen_pos.y, MENU_W, MENU_H);

    // Drop shadow + panel body + border. Mirrors the node-card style so the
    // menu reads as part of the same surface family.
    ctx.draw_shadow(panel, theme.colors.shadow, 8, Point::new(0, 3));
    ctx.set_color(theme.colors.surface);
    ctx.fill_rounded_rect(panel, 6);
    ctx.set_color(theme.colors.border);
    ctx.draw_rounded_rect(panel, 6);

    // Search box across the top.
    let search_rect = Rect::new(
        panel.x() + PAD,
        panel.y() + PAD,
        panel.width() - PAD * 2,
        SEARCH_BOX_H,
    );
    ctx.set_color(theme.colors.surface_variant);
    ctx.fill_rounded_rect(search_rect, 4);
    ctx.set_color(theme.colors.border);
    ctx.draw_rounded_rect(search_rect, 4);

    let font = theme.typography.font_size_base;
    ctx.set_color(theme.colors.text);
    let search_text = if state.search_text.is_empty() {
        "Search…"
    } else {
        state.search_text.as_str()
    };
    if state.search_text.is_empty() {
        ctx.set_color(theme.colors.text_secondary);
    }
    ctx.draw_text(
        search_text,
        Point::new(
            search_rect.x() + TEXT_PAD_X,
            search_rect.y() + (SEARCH_BOX_H - font) / 2,
        ),
        font,
    );

    // Scrollable-ish list area. We don't actually scroll — entries past the
    // visible area are clipped by the rect intersection done by the renderer.
    // `selected_idx` decides which row is highlighted.
    let list_top = search_rect.bottom() + PAD;
    let list_rect = Rect::new(
        panel.x() + PAD,
        list_top,
        panel.width() - PAD * 2,
        panel.bottom() - list_top - PAD,
    );

    let max_visible = (list_rect.height() / ROW_H).max(0) as usize;
    let selected = state.selected_idx.unwrap_or(0);

    for (i, type_id) in state.filtered.iter().take(max_visible).enumerate() {
        let row = Rect::new(
            list_rect.x(),
            list_rect.y() + (i as i32) * ROW_H,
            list_rect.width(),
            ROW_H,
        );
        if i == selected {
            ctx.set_color(theme.colors.primary);
            ctx.fill_rect(row);
            ctx.set_color(theme.colors.text);
        } else {
            ctx.set_color(theme.colors.text);
        }

        // Prefer the human-readable display_name when we can find it; fall
        // back to the type_id.
        let label = state
            .all_entries
            .iter()
            .find(|e| &e.type_id == type_id)
            .map(|e| e.display_name.as_str())
            .unwrap_or(type_id.as_str());

        ctx.draw_text(
            label,
            Point::new(row.x() + TEXT_PAD_X, row.y() + (ROW_H - font) / 2),
            font,
        );
    }
}

// ---------------------------------------------------------------------------
// Tests (pure CPU; no GL)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::{Graph, NodeGraph};
    use erigui_core::{
        Event, Modifiers, MouseButton, MouseButtonEvent, Point, Rect, Theme, Widget, WidgetId,
    };

    /// Minimal fake registry for tests. `categorize` is just a per-entry
    /// category string so we can verify the by-category grouping in the UI
    /// integration without depending on erigui-nodes.
    struct FakeRegistry {
        entries: Vec<RegistryEntry>,
    }

    impl FakeRegistry {
        fn new(items: &[(&str, &str, &str)]) -> Self {
            Self {
                entries: items
                    .iter()
                    .map(|(tid, name, cat)| RegistryEntry {
                        type_id: (*tid).to_string(),
                        display_name: (*name).to_string(),
                        category: (*cat).to_string(),
                    })
                    .collect(),
            }
        }

        fn arc(items: &[(&str, &str, &str)]) -> Arc<dyn NodeRegistryHandle> {
            Arc::new(Self::new(items))
        }
    }

    impl NodeRegistryHandle for FakeRegistry {
        fn entries(&self) -> Vec<RegistryEntry> {
            self.entries.clone()
        }

        fn schema(&self, type_id: &str) -> Option<AddMenuSchema> {
            let entry = self.entries.iter().find(|e| e.type_id == type_id)?;
            Some(AddMenuSchema {
                display_name: entry.display_name.clone(),
                category: entry.category.clone(),
                inputs: vec![AddMenuPortSpec {
                    name: "in".to_string(),
                }],
                outputs: vec![AddMenuPortSpec {
                    name: "out".to_string(),
                }],
                fields: vec![AddMenuFieldSpec {
                    name: "n".to_string(),
                    label: "n".to_string(),
                    kind: FieldKind::Number {
                        min: 0.0,
                        max: 10.0,
                        step: 1.0,
                    },
                    default: FieldValue::Number(3.0),
                }],
            })
        }
    }

    fn theme() -> Theme {
        Theme::dark()
    }

    fn id() -> WidgetId {
        WidgetId::default()
    }

    fn six_types() -> Arc<dyn NodeRegistryHandle> {
        FakeRegistry::arc(&[
            ("core/load_checkpoint", "Load Checkpoint", "Loaders"),
            ("core/load_lora", "Load LoRA", "Loaders"),
            ("core/encode_prompt", "Encode Prompt", "Conditioning"),
            ("core/k_sampler", "K Sampler", "Sampling"),
            ("core/vae_decode", "VAE Decode", "VAE"),
            ("core/save_image", "Save Image", "Image"),
        ])
    }

    fn ng_with_registry() -> NodeGraph {
        let mut g = NodeGraph::new(id(), Graph::default());
        g.set_node_registry(six_types());
        g.layout(Rect::new(0, 0, 1000, 800), &theme());
        g
    }

    fn right_press(x: i32, y: i32) -> Event {
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Right,
            position: Point::new(x, y),
            pressed: true,
            modifiers: Modifiers::empty(),
        })
    }

    fn right_release(x: i32, y: i32) -> Event {
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Right,
            position: Point::new(x, y),
            pressed: false,
            modifiers: Modifiers::empty(),
        })
    }

    // ---- C4-1: right-click on empty canvas opens the menu ----
    #[test]
    fn right_click_canvas_opens_menu() {
        let mut g = ng_with_registry();
        // Fire on an empty patch of canvas.
        let _ = g.handle_event(&right_press(500, 400), &theme());
        let _ = g.handle_event(&right_release(500, 400), &theme());
        assert!(
            g.add_menu_open(),
            "right-click on empty canvas should open the add-node menu"
        );
    }

    // ---- C4-2: right-click on a node does NOT open the menu ----
    #[test]
    fn right_click_node_does_not_open_menu() {
        let mut g = ng_with_registry();
        // Add a node big enough to land a right-click on.
        g.add_node(Node {
            id: 1,
            title: "T".into(),
            position: Point::new(40, 40),
            size: Size::new(220, 120),
            inputs: vec![],
            outputs: vec![],
            fields: vec![],
            component_type: None,
        });
        // Re-layout so port_positions / hit-testing are valid.
        g.layout(Rect::new(0, 0, 1000, 800), &theme());

        // Right-click squarely inside the node's body (not on a port edge).
        let cx = 40 + 110;
        let cy = 40 + 60;
        let _ = g.handle_event(&right_press(cx, cy), &theme());
        let _ = g.handle_event(&right_release(cx, cy), &theme());

        assert!(
            !g.add_menu_open(),
            "right-click on a node body should NOT open the add-node menu"
        );
    }

    // ---- C4-3: search filters list (case-insensitive substring on display_name) ----
    #[test]
    fn search_filters_list() {
        // 6 mock types across 3 categories. Three contain "k" (case-insensitive)
        // in display_name: "Load Checkpoint", "Load LoRA" → no, "K Sampler",
        // "Encode Prompt" → no. Only "Load Checkpoint" and "K Sampler" match.
        let registry = six_types();
        let mut state = AddNodeMenuState::new(Point::new(10, 10), registry);
        assert_eq!(
            state.filtered.len(),
            6,
            "fresh menu lists every registered type_id"
        );

        state.update_search("k");
        let names: Vec<String> = state
            .filtered
            .iter()
            .map(|tid| {
                state
                    .all_entries
                    .iter()
                    .find(|e| &e.type_id == tid)
                    .unwrap()
                    .display_name
                    .clone()
            })
            .collect();
        assert!(
            names.iter().all(|n| n.to_lowercase().contains('k')),
            "every filtered display_name must contain 'k', got {names:?}"
        );
        // Spot-check the expected matches.
        assert!(names.contains(&"Load Checkpoint".to_string()));
        assert!(names.contains(&"K Sampler".to_string()));
        assert!(!names.contains(&"Save Image".to_string()));
    }

    // ---- C4-4: selecting an item inserts a node with schema-default fields ----
    #[test]
    fn select_inserts_node() {
        let mut g = ng_with_registry();
        let before = g.graph.nodes.len();

        // Open via right-click on empty canvas.
        let _ = g.handle_event(&right_press(300, 300), &theme());
        let _ = g.handle_event(&right_release(300, 300), &theme());
        assert!(g.add_menu_open());

        // Simulate "user picked the first filtered entry".
        g.add_menu_pick_first();

        assert_eq!(
            g.graph.nodes.len(),
            before + 1,
            "selecting from the menu should append exactly one node"
        );
        let added = g.graph.nodes.last().unwrap();
        assert_eq!(added.fields.len(), 1, "FakeRegistry seeds a single field");
        match &added.fields[0].value {
            FieldValue::Number(n) => assert!(
                (*n - 3.0).abs() < f32::EPSILON,
                "schema-default value (3.0) should propagate",
            ),
            other => panic!("expected Number(3.0) default, got {other:?}"),
        }
        assert!(!g.add_menu_open(), "menu should close after a selection");
    }
}

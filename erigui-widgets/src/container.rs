//! Container is a hierarchical wrapper for organizing child `WidgetId`s.
//!
//! It does **not** compute child bounds. Hosts must lay out children
//! directly. Earlier revisions of this widget exposed a
//! `with_layout(LayoutConfig)` builder and a `layout_children` walker, but
//! the walker required mutable access to the widget manager (a borrow
//! Container itself does not own) and was never wired up. The misleading
//! API has been removed; Container is now a pure marker / parent-id
//! holder. See `erigui-app` and `node_graph` for examples of hosts that
//! lay out children manually.

use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, Rect, Size, Theme, Widget, WidgetId,
    WidgetState,
};
use smallvec::SmallVec;
use std::any::Any;

pub struct Container {
    state: WidgetState,
    children: SmallVec<[WidgetId; 8]>,
}

impl Container {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            children: SmallVec::new(),
        }
    }

    pub fn add_child(&mut self, child_id: WidgetId) {
        self.children.push(child_id);
    }

    pub fn remove_child(&mut self, child_id: WidgetId) -> bool {
        if let Some(index) = self.children.iter().position(|&id| id == child_id) {
            self.children.remove(index);
            true
        } else {
            false
        }
    }

    pub fn clear_children(&mut self) {
        self.children.clear();
    }
}

impl Widget for Container {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        // Container is a marker: it has no opinion about size. It just
        // reflects the constraints back so a host that uses the value
        // gets something sensible.
        Size::new(
            constraints.max_width.unwrap_or(0),
            constraints.max_height.unwrap_or(0),
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        // Set our own bounds. Children are NOT laid out here -- the host
        // must walk them and call `child.layout(...)` itself.
        self.state.bounds = rect;
    }

    fn draw(&self, _context: &mut dyn DrawContext, _theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Containers don't draw themselves; their children are drawn by
        // the host's draw walk.
    }

    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
        // Container doesn't handle events directly.
        EventResult::Ignored
    }

    fn children(&self) -> &[WidgetId] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [WidgetId] {
        &mut self.children
    }

    fn bounds(&self) -> Rect {
        self.state.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
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

    fn is_focused(&self) -> bool {
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn can_focus(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

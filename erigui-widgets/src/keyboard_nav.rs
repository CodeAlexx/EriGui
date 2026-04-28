//! Keyboard navigation registry — process-singleton by design.
//!
//! `KeyboardNavigationManager` is a `OnceLock<Mutex<...>>` global.
//! Intentional; not a Mojo-port artifact. Tab traversal across the
//! whole widget tree (including widgets in different host crates,
//! plug-ins, and embedded subtrees) converges on a single
//! registration point so that **Tab** / **Shift+Tab** can walk the
//! union of all currently-mounted focusables in declaration order
//! regardless of which crate owns each one. A host-owned navigator
//! would force every embedder (the app, every plug-in, every
//! sub-widget tree) to re-register their focusables at every nesting
//! boundary — fragile and easy to forget.
//!
//! Per-widget keyboard handlers (Ctrl+Tab inside TabControl,
//! Up/Down inside ListView, etc.) belong on the widget itself and
//! do **not** go through this manager — they only handle keys
//! within the focused widget's domain. This manager is purely the
//! Tab-cycle layer above all widgets.
//!
//! Cross-platform note: Windows / macOS / GTK / Cocoa all assume a
//! single per-process focus chain; this matches.

use erigui_core::{Key, Point, Rect, WidgetId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationDirection {
    Forward,
    Backward,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct FocusableWidget {
    pub id: WidgetId,
    pub bounds: Rect,
    pub tab_index: Option<i32>,
    pub focusable: bool,
    pub group_id: Option<String>,
}

pub struct KeyboardNavigationManager {
    focusable_widgets: HashMap<WidgetId, FocusableWidget>,
    focus_order: Vec<WidgetId>,
    current_focus: Option<WidgetId>,
    focus_groups: HashMap<String, Vec<WidgetId>>,
    focus_trap: Option<String>, // Group ID that traps focus
}

impl Default for KeyboardNavigationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardNavigationManager {
    pub fn new() -> Self {
        Self {
            focusable_widgets: HashMap::new(),
            focus_order: Vec::new(),
            current_focus: None,
            focus_groups: HashMap::new(),
            focus_trap: None,
        }
    }

    pub fn register_widget(&mut self, widget: FocusableWidget) {
        let id = widget.id;
        let group_id = widget.group_id.clone();

        self.focusable_widgets.insert(id, widget);

        // Add to group if specified
        if let Some(group) = group_id {
            self.focus_groups
                .entry(group)
                .or_default()
                .push(id);
        }

        self.rebuild_focus_order();
    }

    pub fn unregister_widget(&mut self, id: WidgetId) {
        if let Some(widget) = self.focusable_widgets.remove(&id) {
            // Remove from group
            if let Some(group_id) = widget.group_id {
                if let Some(group) = self.focus_groups.get_mut(&group_id) {
                    group.retain(|&w| w != id);
                }
            }

            // Clear focus if this widget had it
            if self.current_focus == Some(id) {
                self.current_focus = None;
            }

            self.rebuild_focus_order();
        }
    }

    pub fn update_widget_bounds(&mut self, id: WidgetId, bounds: Rect) {
        if let Some(widget) = self.focusable_widgets.get_mut(&id) {
            widget.bounds = bounds;
        }
    }

    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        self.current_focus = id;
    }

    pub fn get_focused_widget(&self) -> Option<WidgetId> {
        self.current_focus
    }

    pub fn set_focus_trap(&mut self, group_id: Option<String>) {
        self.focus_trap = group_id;
    }

    pub fn handle_navigation(&mut self, direction: NavigationDirection) -> Option<WidgetId> {
        match direction {
            NavigationDirection::Forward | NavigationDirection::Backward => {
                self.navigate_tab(direction == NavigationDirection::Forward)
            }
            NavigationDirection::Up
            | NavigationDirection::Down
            | NavigationDirection::Left
            | NavigationDirection::Right => self.navigate_directional(direction),
        }
    }

    pub fn handle_key(&mut self, key: Key) -> Option<WidgetId> {
        match key {
            Key::Tab => self.navigate_tab(true),
            Key::Up => self.navigate_directional(NavigationDirection::Up),
            Key::Down => self.navigate_directional(NavigationDirection::Down),
            Key::Left => self.navigate_directional(NavigationDirection::Left),
            Key::Right => self.navigate_directional(NavigationDirection::Right),
            _ => None,
        }
    }

    fn rebuild_focus_order(&mut self) {
        let mut widgets: Vec<_> = self
            .focusable_widgets
            .values()
            .filter(|w| w.focusable)
            .collect();

        // Sort by tab index, then by position (top-to-bottom, left-to-right)
        widgets.sort_by(|a, b| {
            match (a.tab_index, b.tab_index) {
                (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => {
                    // Sort by position
                    let a_pos = (a.bounds.y(), a.bounds.x());
                    let b_pos = (b.bounds.y(), b.bounds.x());
                    a_pos.cmp(&b_pos)
                }
            }
        });

        self.focus_order = widgets.into_iter().map(|w| w.id).collect();
    }

    fn navigate_tab(&mut self, forward: bool) -> Option<WidgetId> {
        if self.focus_order.is_empty() {
            return None;
        }

        let current_index = self
            .current_focus
            .and_then(|id| self.focus_order.iter().position(|&w| w == id));

        let mut candidates = self.focus_order.clone();

        // Apply focus trap if active
        if let Some(trap_group) = &self.focus_trap {
            if let Some(group_widgets) = self.focus_groups.get(trap_group) {
                candidates.retain(|id| group_widgets.contains(id));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        let next_index = match current_index {
            Some(idx) => {
                let current_pos = candidates
                    .iter()
                    .position(|&id| id == self.focus_order[idx])?;
                if forward {
                    (current_pos + 1) % candidates.len()
                } else if current_pos == 0 {
                    candidates.len() - 1
                } else {
                    current_pos - 1
                }
            }
            None => 0,
        };

        let next_widget = candidates[next_index];
        self.current_focus = Some(next_widget);
        Some(next_widget)
    }

    fn navigate_directional(&mut self, direction: NavigationDirection) -> Option<WidgetId> {
        let current = self.current_focus?;
        let current_widget = self.focusable_widgets.get(&current)?;
        let current_center = current_widget.bounds.center();

        let mut candidates: Vec<_> = self
            .focusable_widgets
            .values()
            .filter(|w| {
                w.id != current
                    && w.focusable
                    && self.is_in_direction(&current_center, &w.bounds, direction)
            })
            .collect();

        // Apply focus trap if active
        if let Some(trap_group) = &self.focus_trap {
            if let Some(group_widgets) = self.focus_groups.get(trap_group) {
                candidates.retain(|w| group_widgets.contains(&w.id));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Find the closest widget in the given direction
        candidates.sort_by_key(|w| {
            let center = w.bounds.center();
            let dx = center.x - current_center.x;
            let dy = center.y - current_center.y;
            dx * dx + dy * dy // Distance squared
        });

        let next_widget = candidates[0].id;
        self.current_focus = Some(next_widget);
        Some(next_widget)
    }

    fn is_in_direction(&self, from: &Point, to: &Rect, direction: NavigationDirection) -> bool {
        let to_center = to.center();

        match direction {
            NavigationDirection::Up => to_center.y < from.y,
            NavigationDirection::Down => to_center.y > from.y,
            NavigationDirection::Left => to_center.x < from.x,
            NavigationDirection::Right => to_center.x > from.x,
            _ => false,
        }
    }
}

// Global keyboard navigation manager
use std::sync::{Mutex, OnceLock};

static KEYBOARD_NAV_MANAGER: OnceLock<Mutex<KeyboardNavigationManager>> = OnceLock::new();

pub fn keyboard_nav_manager() -> std::sync::MutexGuard<'static, KeyboardNavigationManager> {
    KEYBOARD_NAV_MANAGER
        .get_or_init(|| Mutex::new(KeyboardNavigationManager::new()))
        .lock()
        .unwrap()
}

// Trait for widgets that support keyboard navigation
pub trait KeyboardNavigable {
    fn get_focusable_info(&self) -> Option<FocusableWidget>;
    fn on_focus_gained(&mut self);
    fn on_focus_lost(&mut self);
    fn handle_keyboard_input(&mut self, key: Key) -> bool; // Returns true if handled
}

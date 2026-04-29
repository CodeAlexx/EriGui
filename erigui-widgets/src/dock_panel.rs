use crate::{TabControl, TabItem};
use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent,
    MouseMoveEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::collections::HashMap;

/// Callback fired when a docked panel is closed. Argument: panel id.
type PanelCloseCallback = Box<dyn FnMut(&str)>;
/// Callback fired when the dock layout changes.
type LayoutChangeCallback = Box<dyn FnMut()>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockSplitDirection {
    Horizontal,
    Vertical,
}

pub struct DockablePanel {
    pub id: String,
    pub title: String,
    pub content: Box<dyn Widget>,
    pub can_close: bool,
    pub can_float: bool,
    pub icon: Option<String>,
}

impl DockablePanel {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: Box<dyn Widget>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content,
            can_close: true,
            can_float: true,
            icon: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_can_close(mut self, can_close: bool) -> Self {
        self.can_close = can_close;
        self
    }

    pub fn with_can_float(mut self, can_float: bool) -> Self {
        self.can_float = can_float;
        self
    }
}

enum DockNode {
    Split {
        direction: DockSplitDirection,
        ratio: f32,
        first: Box<DockNode>,
        second: Box<DockNode>,
    },
    Tabs {
        panels: Vec<String>, // Panel IDs
        active_index: usize,
        tab_control: TabControl,
    },
    Empty,
}

pub struct FloatingWindow {
    panel_id: String,
    bounds: Rect,
    dragging: bool,
    drag_offset: Point,
    resizing: bool,
}

/// Path to a split node in the dock tree, used for safe splitter ratio updates
/// Each element is 0 for 'first' child or 1 for 'second' child
#[derive(Debug, Clone)]
struct SplitterPath {
    /// Path indices from root to the split node (0 = first, 1 = second)
    indices: Vec<usize>,
}

impl SplitterPath {
    fn new() -> Self {
        Self { indices: Vec::new() }
    }

    fn push(&mut self, index: usize) {
        self.indices.push(index);
    }
}

pub struct DockPanel {
    state: WidgetState,
    panels: HashMap<String, DockablePanel>,
    root_node: DockNode,
    floating_windows: Vec<FloatingWindow>,

    // Drag state
    drag_preview_rect: Option<Rect>,

    // Splitter state - now uses safe path-based navigation instead of raw pointers
    active_splitter: Option<usize>,
    splitter_rects: Vec<(Rect, DockSplitDirection, SplitterPath)>,

    // Visual settings
    splitter_size: i32,
    tab_height: i32,
    title_height: i32,

    // Callbacks
    on_panel_close: Option<PanelCloseCallback>,
    on_layout_change: Option<LayoutChangeCallback>,
}

impl DockPanel {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            panels: HashMap::new(),
            root_node: DockNode::Empty,
            floating_windows: Vec::new(),
            drag_preview_rect: None,
            active_splitter: None,
            splitter_rects: Vec::new(),
            splitter_size: 4,
            tab_height: 30,
            title_height: 25,
            on_panel_close: None,
            on_layout_change: None,
        }
    }

    pub fn with_on_panel_close<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_panel_close = Some(Box::new(f));
        self
    }

    pub fn with_on_layout_change<F: FnMut() + 'static>(mut self, f: F) -> Self {
        self.on_layout_change = Some(Box::new(f));
        self
    }

    pub fn add_panel(&mut self, panel: DockablePanel, position: DockPosition) {
        let panel_id = panel.id.clone();
        self.panels.insert(panel_id.clone(), panel);

        match position {
            DockPosition::Floating => {
                // Add as floating window
                let window = FloatingWindow {
                    panel_id,
                    bounds: Rect::new(100, 100, 400, 300),
                    dragging: false,
                    drag_offset: Point::ZERO,
                    resizing: false,
                };
                self.floating_windows.push(window);
            }
            _ => {
                // Add to dock tree
                self.add_to_dock_tree(panel_id, position);
            }
        }

        if let Some(callback) = &mut self.on_layout_change {
            callback();
        }
    }

    pub fn remove_panel(&mut self, panel_id: &str) -> Option<DockablePanel> {
        let panel = self.panels.remove(panel_id)?;

        // Remove from floating windows
        self.floating_windows.retain(|w| w.panel_id != panel_id);

        // Remove from dock tree
        self.remove_from_dock_tree(panel_id);

        if let Some(callback) = &mut self.on_layout_change {
            callback();
        }

        Some(panel)
    }

    fn add_to_dock_tree(&mut self, panel_id: String, position: DockPosition) {
        match &mut self.root_node {
            DockNode::Empty => {
                // First panel becomes root
                self.root_node = DockNode::Tabs {
                    panels: vec![panel_id],
                    active_index: 0,
                    tab_control: TabControl::new(WidgetId::default()),
                };
            }
            _ => {
                // Add to existing tree based on position
                match position {
                    DockPosition::Center => {
                        // Add as tab. If root is a Tabs node, append
                        // directly. If root is a Split, descend into the
                        // first (left/top) child recursively until we
                        // hit a Tabs leaf, then append there. This
                        // matches how every production dock-panel system
                        // (VS Code, JetBrains, Eclipse) handles a Center
                        // add against a non-trivial layout: the "main"
                        // panel area is conventionally the first leaf.
                        // Hosts that want a specific destination should
                        // pass Right/Left/Top/Bottom/Floating explicitly.
                        Self::push_tab_to_first_leaf(&mut self.root_node, panel_id);
                    }
                    DockPosition::Left => {
                        let old_root = std::mem::replace(&mut self.root_node, DockNode::Empty);
                        self.root_node = DockNode::Split {
                            direction: DockSplitDirection::Horizontal,
                            ratio: 0.3,
                            first: Box::new(DockNode::Tabs {
                                panels: vec![panel_id],
                                active_index: 0,
                                tab_control: TabControl::new(WidgetId::default()),
                            }),
                            second: Box::new(old_root),
                        };
                    }
                    DockPosition::Right => {
                        let old_root = std::mem::replace(&mut self.root_node, DockNode::Empty);
                        self.root_node = DockNode::Split {
                            direction: DockSplitDirection::Horizontal,
                            ratio: 0.7,
                            first: Box::new(old_root),
                            second: Box::new(DockNode::Tabs {
                                panels: vec![panel_id],
                                active_index: 0,
                                tab_control: TabControl::new(WidgetId::default()),
                            }),
                        };
                    }
                    DockPosition::Top => {
                        let old_root = std::mem::replace(&mut self.root_node, DockNode::Empty);
                        self.root_node = DockNode::Split {
                            direction: DockSplitDirection::Vertical,
                            ratio: 0.3,
                            first: Box::new(DockNode::Tabs {
                                panels: vec![panel_id],
                                active_index: 0,
                                tab_control: TabControl::new(WidgetId::default()),
                            }),
                            second: Box::new(old_root),
                        };
                    }
                    DockPosition::Bottom => {
                        let old_root = std::mem::replace(&mut self.root_node, DockNode::Empty);
                        self.root_node = DockNode::Split {
                            direction: DockSplitDirection::Vertical,
                            ratio: 0.7,
                            first: Box::new(old_root),
                            second: Box::new(DockNode::Tabs {
                                panels: vec![panel_id],
                                active_index: 0,
                                tab_control: TabControl::new(WidgetId::default()),
                            }),
                        };
                    }
                    _ => {}
                }
            }
        }
    }

    /// Test-only: walk down `Split.first` until a Tabs leaf, return the
    /// panel-ids in order. Returns an empty Vec if the tree has no
    /// reachable Tabs leaf via the first-child chain (only Splits and
    /// Empty). Used to verify Center-add-to-Split-root descent.
    #[doc(hidden)]
    pub fn first_leaf_panels_for_test(&self) -> Vec<String> {
        fn walk(node: &DockNode) -> Vec<String> {
            match node {
                DockNode::Tabs { panels, .. } => panels.clone(),
                DockNode::Split { first, .. } => walk(first),
                DockNode::Empty => Vec::new(),
            }
        }
        walk(&self.root_node)
    }

    /// Test-only: active panel index of the first Tabs leaf reachable
    /// via `Split.first`. Returns `None` if no Tabs leaf is reachable.
    /// Used to verify a tab-strip click switches the active panel.
    #[doc(hidden)]
    pub fn first_leaf_active_index_for_test(&self) -> Option<usize> {
        fn walk(node: &DockNode) -> Option<usize> {
            match node {
                DockNode::Tabs { active_index, .. } => Some(*active_index),
                DockNode::Split { first, .. } => walk(first),
                DockNode::Empty => None,
            }
        }
        walk(&self.root_node)
    }

    /// Push `panel_id` onto the first Tabs leaf reachable from `node`,
    /// descending into `Split.first` recursively. If the tree contains
    /// no Tabs leaf at all (only nested Splits ending in Empty — which
    /// shouldn't happen with the current `add_to_dock_tree` paths),
    /// the panel is dropped on the floor and the call is a no-op. The
    /// outer caller has already recorded the panel in `self.panels`,
    /// so a no-op here would be a silent leak; we instead initialize
    /// the first Empty we find into a Tabs node, which matches the
    /// behavior `add_to_dock_tree` uses for an empty root.
    fn push_tab_to_first_leaf(node: &mut DockNode, panel_id: String) {
        match node {
            DockNode::Tabs { panels, .. } => {
                panels.push(panel_id);
            }
            DockNode::Split { first, .. } => {
                Self::push_tab_to_first_leaf(first.as_mut(), panel_id);
            }
            DockNode::Empty => {
                *node = DockNode::Tabs {
                    panels: vec![panel_id],
                    active_index: 0,
                    tab_control: TabControl::new(WidgetId::default()),
                };
            }
        }
    }

    fn remove_from_dock_tree(&mut self, panel_id: &str) {
        let mut new_root = DockNode::Empty;
        std::mem::swap(&mut new_root, &mut self.root_node);
        let empty = self.remove_from_node(&mut new_root, panel_id);
        if !empty {
            self.root_node = new_root;
        }
    }

    fn remove_from_node(&mut self, node: &mut DockNode, panel_id: &str) -> bool {
        match node {
            DockNode::Tabs {
                panels,
                active_index,
                ..
            } => {
                if let Some(pos) = panels.iter().position(|id| id == panel_id) {
                    panels.remove(pos);
                    if *active_index >= panels.len() && *active_index > 0 {
                        *active_index = panels.len() - 1;
                    }
                    return panels.is_empty();
                }
            }
            DockNode::Split { first, second, .. } => {
                if self.remove_from_node(first, panel_id) {
                    // First node is empty, replace with second
                    *node = std::mem::replace(&mut **second, DockNode::Empty);
                    return false;
                }
                if self.remove_from_node(second, panel_id) {
                    // Second node is empty, replace with first
                    *node = std::mem::replace(&mut **first, DockNode::Empty);
                    return false;
                }
            }
            DockNode::Empty => {}
        }
        false
    }

    fn layout_node(&mut self, node: &mut DockNode, rect: Rect, theme: &Theme) {
        // Use path-based layout with empty initial path
        let path = SplitterPath::new();
        self.layout_node_with_path(node, rect, theme, path);
    }

    fn layout_node_with_path(&mut self, node: &mut DockNode, rect: Rect, theme: &Theme, current_path: SplitterPath) {
        match node {
            DockNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                // Store splitter info with current path (before recursing)
                let splitter_path = current_path.clone();

                match direction {
                    DockSplitDirection::Horizontal => {
                        let split_x = rect.x() + (rect.width() as f32 * *ratio) as i32;

                        // Layout first node
                        let first_rect = Rect::new(
                            rect.x(),
                            rect.y(),
                            split_x - rect.x() - self.splitter_size / 2,
                            rect.height(),
                        );
                        let mut first_path = current_path.clone();
                        first_path.push(0);
                        self.layout_node_with_path(first, first_rect, theme, first_path);

                        // Layout second node
                        let second_rect = Rect::new(
                            split_x + self.splitter_size / 2,
                            rect.y(),
                            rect.right() - split_x - self.splitter_size / 2,
                            rect.height(),
                        );
                        let mut second_path = current_path;
                        second_path.push(1);
                        self.layout_node_with_path(second, second_rect, theme, second_path);

                        // Store splitter rect with safe path reference
                        let splitter_rect = Rect::new(
                            split_x - self.splitter_size / 2,
                            rect.y(),
                            self.splitter_size,
                            rect.height(),
                        );
                        self.splitter_rects
                            .push((splitter_rect, *direction, splitter_path));
                    }
                    DockSplitDirection::Vertical => {
                        let split_y = rect.y() + (rect.height() as f32 * *ratio) as i32;

                        // Layout first node
                        let first_rect = Rect::new(
                            rect.x(),
                            rect.y(),
                            rect.width(),
                            split_y - rect.y() - self.splitter_size / 2,
                        );
                        let mut first_path = current_path.clone();
                        first_path.push(0);
                        self.layout_node_with_path(first, first_rect, theme, first_path);

                        // Layout second node
                        let second_rect = Rect::new(
                            rect.x(),
                            split_y + self.splitter_size / 2,
                            rect.width(),
                            rect.bottom() - split_y - self.splitter_size / 2,
                        );
                        let mut second_path = current_path;
                        second_path.push(1);
                        self.layout_node_with_path(second, second_rect, theme, second_path);

                        // Store splitter rect with safe path reference
                        let splitter_rect = Rect::new(
                            rect.x(),
                            split_y - self.splitter_size / 2,
                            rect.width(),
                            self.splitter_size,
                        );
                        self.splitter_rects
                            .push((splitter_rect, *direction, splitter_path));
                    }
                }
            }
            DockNode::Tabs {
                panels,
                tab_control,
                ..
            } => {
                if !panels.is_empty() {
                    // Create tab items
                    let mut tab_items = Vec::new();
                    for panel_id in panels.iter() {
                        if let Some(panel) = self.panels.get(panel_id) {
                            tab_items.push(TabItem::new(panel.title.clone(), panel_id.clone()));
                        }
                    }

                    // Layout tab control
                    tab_control.set_tabs(tab_items);
                    tab_control.layout(
                        Rect::new(rect.x(), rect.y(), rect.width(), self.tab_height),
                        theme,
                    );

                    // Layout active panel content
                    if let Some(active_tab) = tab_control.get_active_tab() {
                        if let Some(panel_id) = panels.get(active_tab) {
                            if let Some(panel) = self.panels.get_mut(panel_id) {
                                panel.content.layout(
                                    Rect::new(
                                        rect.x(),
                                        rect.y() + self.tab_height,
                                        rect.width(),
                                        rect.height() - self.tab_height,
                                    ),
                                    theme,
                                );
                            }
                        }
                    }
                }
            }
            DockNode::Empty => {}
        }
    }

    fn draw_node(&self, node: &DockNode, context: &mut dyn DrawContext, theme: &Theme) {
        match node {
            DockNode::Split { first, second, .. } => {
                self.draw_node(first, context, theme);
                self.draw_node(second, context, theme);
            }
            DockNode::Tabs {
                panels,
                tab_control,
                ..
            } => {
                if !panels.is_empty() {
                    // Draw tab control
                    tab_control.draw(context, theme);

                    // Draw active panel content
                    if let Some(active_tab) = tab_control.get_active_tab() {
                        if let Some(panel_id) = panels.get(active_tab) {
                            if let Some(panel) = self.panels.get(panel_id) {
                                panel.content.draw(context, theme);
                            }
                        }
                    }
                }
            }
            DockNode::Empty => {}
        }
    }

    fn draw_floating_window(
        &self,
        window: &FloatingWindow,
        context: &mut dyn DrawContext,
        theme: &Theme,
    ) {
        if let Some(panel) = self.panels.get(&window.panel_id) {
            // Draw window background
            context.set_color(theme.colors.surface);
            context.fill_rect(window.bounds);

            // Draw title bar
            let title_rect = Rect::new(
                window.bounds.x(),
                window.bounds.y(),
                window.bounds.width(),
                self.title_height,
            );

            context.set_color(theme.colors.primary);
            context.fill_rect(title_rect);

            // Draw title text
            context.set_color(theme.colors.background);
            context.draw_text(
                &panel.title,
                Point::new(title_rect.x() + 8, title_rect.center().y + 4),
                theme.typography.font_size_base,
            );

            // Draw close button if allowed
            if panel.can_close {
                let close_rect = Rect::new(
                    title_rect.right() - self.title_height,
                    title_rect.y(),
                    self.title_height,
                    self.title_height,
                );

                context.set_color(theme.colors.background);
                context.draw_text(
                    "×",
                    Point::new(close_rect.center().x - 4, close_rect.center().y + 6),
                    theme.typography.font_size_large,
                );
            }

            // Draw content
            panel.content.draw(context, theme);

            // Draw window border
            context.set_color(theme.colors.border);
            context.draw_rect(window.bounds);
        }
    }

    fn draw_drop_preview(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if let Some(rect) = &self.drag_preview_rect {
            context.set_color(theme.colors.primary.with_alpha(64));
            context.fill_rect(*rect);

            context.set_color(theme.colors.primary);
            context.draw_rect(*rect);
        }
    }

    /// Safely update the ratio of a split node at the given path
    /// Returns true if the ratio was successfully updated
    fn update_ratio_at_path(&mut self, path: &SplitterPath, new_ratio: f32) -> bool {
        Self::update_ratio_in_node(&mut self.root_node, &path.indices, new_ratio)
    }

    /// Recursively navigate to the split node and update its ratio
    fn update_ratio_in_node(node: &mut DockNode, path: &[usize], new_ratio: f32) -> bool {
        match node {
            DockNode::Split { ratio, first, second, .. } => {
                if path.is_empty() {
                    // We're at the target split node, update the ratio
                    *ratio = new_ratio.clamp(0.1, 0.9);
                    true
                } else {
                    // Navigate deeper into the tree
                    let (next_index, remaining_path) = (path[0], &path[1..]);
                    match next_index {
                        0 => Self::update_ratio_in_node(first, remaining_path, new_ratio),
                        1 => Self::update_ratio_in_node(second, remaining_path, new_ratio),
                        _ => false, // Invalid path index
                    }
                }
            }
            _ => false, // Can't update ratio on non-split nodes
        }
    }
}

impl Widget for DockPanel {
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
        self.splitter_rects.clear();
        // Theme-driven tab/title heights so HiDPI / Theme::with_scale
        // reaches DockPanel. Constructor defaults of 30 / 25 were
        // unscaled raw px — at scale 2x, tab strip stayed 30 while text
        // wanted ~60 and tab labels overlapped panel content.
        let font = theme.typography.font_size_base;
        let pad = theme.spacing.padding.top.max(8);
        self.tab_height = font + pad * 2;
        self.title_height = font + pad;

        // Layout dock tree - swap out root to avoid borrow issues
        let mut root = DockNode::Empty;
        std::mem::swap(&mut root, &mut self.root_node);
        self.layout_node(&mut root, rect, theme);
        std::mem::swap(&mut root, &mut self.root_node);

        // Layout floating windows
        for window in &mut self.floating_windows {
            if let Some(panel) = self.panels.get_mut(&window.panel_id) {
                let content_rect = Rect::new(
                    window.bounds.x(),
                    window.bounds.y() + self.title_height,
                    window.bounds.width(),
                    window.bounds.height() - self.title_height,
                );
                panel.content.layout(content_rect, theme);
            }
        }
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw docked panels
        self.draw_node(&self.root_node, context, theme);

        // Draw splitters
        for (rect, direction, _) in &self.splitter_rects {
            context.set_color(theme.colors.border);
            context.fill_rect(*rect);

            // Draw handle
            context.set_color(theme.colors.text_secondary);
            let center = rect.center();
            match direction {
                DockSplitDirection::Horizontal => {
                    for i in -1..=1 {
                        context.fill_rect(Rect::new(center.x - 1, center.y + i * 6 - 1, 2, 2));
                    }
                }
                DockSplitDirection::Vertical => {
                    for i in -1..=1 {
                        context.fill_rect(Rect::new(center.x + i * 6 - 1, center.y - 1, 2, 2));
                    }
                }
            }
        }

        // Draw floating windows
        for window in &self.floating_windows {
            self.draw_floating_window(window, context, theme);
        }

        // Draw drag preview
        self.draw_drop_preview(context, theme);
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Handle floating window events
        for window in &mut self.floating_windows {
            match event {
                Event::MouseButton(MouseButtonEvent {
                    button: MouseButton::Left,
                    position,
                    pressed,
                    ..
                }) => {
                    let title_rect = Rect::new(
                        window.bounds.x(),
                        window.bounds.y(),
                        window.bounds.width(),
                        self.title_height,
                    );

                    if *pressed && title_rect.contains(*position) {
                        window.dragging = true;
                        window.drag_offset = Point::new(
                            position.x - window.bounds.x(),
                            position.y - window.bounds.y(),
                        );
                        return EventResult::Consumed;
                    } else if !pressed {
                        window.dragging = false;
                        window.resizing = false;
                    }
                }
                Event::MouseMove(MouseMoveEvent { position, .. }) => {
                    if window.dragging {
                        window.bounds = Rect::new(
                            position.x - window.drag_offset.x,
                            position.y - window.drag_offset.y,
                            window.bounds.width(),
                            window.bounds.height(),
                        );
                        return EventResult::Consumed;
                    }
                }
                _ => {}
            }
        }

        // Handle splitter dragging
        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if *pressed {
                    // Check if clicking on a splitter
                    for (i, (rect, _, _)) in self.splitter_rects.iter().enumerate() {
                        if rect.contains(*position) {
                            self.active_splitter = Some(i);
                            return EventResult::Consumed;
                        }
                    }
                } else {
                    self.active_splitter = None;
                }
            }
            Event::MouseMove(MouseMoveEvent { position, .. }) => {
                if let Some(index) = self.active_splitter {
                    if let Some((_rect, direction, splitter_path)) = self.splitter_rects.get(index) {
                        // Calculate new ratio based on mouse position and direction
                        let new_ratio = match direction {
                            DockSplitDirection::Horizontal => {
                                let relative_x = position.x - self.state.bounds.x();
                                relative_x as f32 / self.state.bounds.width() as f32
                            }
                            DockSplitDirection::Vertical => {
                                let relative_y = position.y - self.state.bounds.y();
                                relative_y as f32 / self.state.bounds.height() as f32
                            }
                        };
                        // Clone the path since we need to borrow self mutably
                        let path = splitter_path.clone();
                        // Safely update the ratio using path-based navigation
                        self.update_ratio_at_path(&path, new_ratio);
                        return EventResult::Consumed;
                    }
                }
            }
            _ => {}
        }

        // Handle events in dock nodes - swap out root to avoid borrow issues
        let mut root = DockNode::Empty;
        std::mem::swap(&mut root, &mut self.root_node);
        let result = self.handle_node_event(&mut root, event, theme);
        std::mem::swap(&mut root, &mut self.root_node);
        result
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
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
    }

    fn can_focus(&self) -> bool {
        self.state.enabled && self.state.visible
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DockPanel {
    fn handle_node_event(
        &mut self,
        node: &mut DockNode,
        event: &Event,
        theme: &Theme,
    ) -> EventResult {
        match node {
            DockNode::Split { first, second, .. } => {
                let first_result = self.handle_node_event(first, event, theme);
                if first_result == EventResult::Consumed {
                    return first_result;
                }
                self.handle_node_event(second, event, theme)
            }
            DockNode::Tabs {
                panels,
                tab_control,
                active_index,
            } => {
                // Handle tab control events
                let before = tab_control.get_active_tab();
                let tab_result = tab_control.handle_event(event, theme);
                let after = tab_control.get_active_tab();

                // ERIGUI_DEBUG_DOCK=1 dumps tab-control click outcomes so a
                // user-reported "click does nothing" can be diagnosed
                // without rebuilding. Set the env var, run kitchen_sink,
                // click the secondary.rs tab, watch stderr.
                if std::env::var("ERIGUI_DEBUG_DOCK").is_ok() {
                    if let Event::MouseButton(mb) = event {
                        if mb.pressed {
                            eprintln!(
                                "[dock] click@({},{}) tabs={:?} bounds={:?} active before={:?} after={:?} consumed={:?}",
                                mb.position.x,
                                mb.position.y,
                                panels,
                                tab_control.bounds(),
                                before,
                                after,
                                tab_result,
                            );
                        }
                    }
                }

                // Update active index if tab changed
                if let Some(new_active) = after {
                    if new_active != *active_index {
                        *active_index = new_active;
                        return EventResult::Consumed;
                    }
                }

                // Handle panel content events
                if let Some(panel_id) = panels.get(*active_index) {
                    if let Some(panel) = self.panels.get_mut(panel_id) {
                        let content_result = panel.content.handle_event(event, theme);
                        if content_result == EventResult::Consumed {
                            return content_result;
                        }
                    }
                }

                tab_result
            }
            DockNode::Empty => EventResult::Ignored,
        }
    }
}

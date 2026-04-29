use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

/// Callback fired when the selected tree node changes. Argument: selected node id.
type SelectionChangeCallback = Box<dyn FnMut(&str)>;
/// Callback fired when a tree node is expanded or collapsed.
/// Arguments: node id, expanded flag.
type ExpandCallback = Box<dyn FnMut(&str, bool)>;

pub struct TreeNode {
    pub id: String,
    pub text: String,
    pub icon: Option<String>,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub selected: bool,
}

impl TreeNode {
    pub fn new(id: String, text: String) -> Self {
        Self {
            id,
            text,
            icon: None,
            children: Vec::new(),
            expanded: false,
            selected: false,
        }
    }

    pub fn add_child(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut TreeNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(node) = child.find_node_mut(id) {
                return Some(node);
            }
        }

        None
    }
}

pub struct TreeView {
    state: WidgetState,
    pub root_nodes: Vec<TreeNode>,
    item_height: i32,
    indent_width: i32,
    scroll_offset: i32,
    on_selection_change: Option<SelectionChangeCallback>,
    on_expand: Option<ExpandCallback>,
}

impl TreeView {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            root_nodes: Vec::new(),
            item_height: 24,
            indent_width: 20,
            scroll_offset: 0,
            on_selection_change: None,
            on_expand: None,
        }
    }

    pub fn with_root_nodes(mut self, nodes: Vec<TreeNode>) -> Self {
        self.root_nodes = nodes;
        self
    }

    pub fn with_on_selection_change<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_selection_change = Some(Box::new(f));
        self
    }

    pub fn with_on_expand<F: FnMut(&str, bool) + 'static>(mut self, f: F) -> Self {
        self.on_expand = Some(Box::new(f));
        self
    }

    pub fn add_root_node(&mut self, node: TreeNode) {
        self.root_nodes.push(node);
    }

    pub fn clear_nodes(&mut self) {
        self.root_nodes.clear();
    }

    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut TreeNode> {
        for node in &mut self.root_nodes {
            if let Some(found) = node.find_node_mut(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn selected_id(&self) -> Option<String> {
        fn find_selected(node: &TreeNode) -> Option<String> {
            if node.selected {
                return Some(node.id.clone());
            }
            if node.expanded {
                for child in &node.children {
                    if let Some(id) = find_selected(child) {
                        return Some(id);
                    }
                }
            }
            None
        }

        for node in &self.root_nodes {
            if let Some(id) = find_selected(node) {
                return Some(id);
            }
        }
        None
    }

    /// Depth-first list of visible node ids, descending only into
    /// expanded nodes. Mirrors the draw order in `draw_node`.
    fn visible_ids(&self) -> Vec<String> {
        fn walk(node: &TreeNode, out: &mut Vec<String>) {
            out.push(node.id.clone());
            if node.expanded {
                for child in &node.children {
                    walk(child, out);
                }
            }
        }
        let mut out = Vec::new();
        for node in &self.root_nodes {
            walk(node, &mut out);
        }
        out
    }

    /// Find the parent id of `target_id`, or `None` if `target_id` is a
    /// root node or not present.
    fn parent_id(&self, target_id: &str) -> Option<String> {
        fn walk(node: &TreeNode, target_id: &str) -> Option<String> {
            for child in &node.children {
                if child.id == target_id {
                    return Some(node.id.clone());
                }
                if let Some(found) = walk(child, target_id) {
                    return Some(found);
                }
            }
            None
        }
        for node in &self.root_nodes {
            if node.id == target_id {
                return None;
            }
            if let Some(found) = walk(node, target_id) {
                return Some(found);
            }
        }
        None
    }

    /// Apply a fresh selection to `target_id`, clearing prior selection
    /// across the entire tree. Fires `on_selection_change` if a callback
    /// is installed.
    fn set_selected_id(&mut self, target_id: &str) {
        fn clear(node: &mut TreeNode) {
            node.selected = false;
            for child in &mut node.children {
                clear(child);
            }
        }
        for node in &mut self.root_nodes {
            clear(node);
        }
        if let Some(node) = self.find_node_mut(target_id) {
            node.selected = true;
        }
        if let Some(callback) = &mut self.on_selection_change {
            callback(target_id);
        }
    }

    /// Adjust `scroll_offset` so the currently-selected node is fully
    /// visible. Mirror of `ListView::scroll_to_selected`.
    fn scroll_to_selected(&mut self) {
        let Some(selected) = self.selected_id() else {
            return;
        };
        let visible = self.visible_ids();
        let Some(index) = visible.iter().position(|id| id == &selected) else {
            return;
        };
        let item_top = index as i32 * self.item_height;
        let item_bottom = item_top + self.item_height;
        let viewport_top = self.scroll_offset;
        let viewport_bottom = viewport_top + self.state.bounds.height();
        if item_top < viewport_top {
            self.scroll_offset = item_top;
        } else if item_bottom > viewport_bottom {
            self.scroll_offset = item_bottom - self.state.bounds.height();
        }
        // Clamp to [0, max_scroll].
        let total_height = self.count_visible_nodes() as i32 * self.item_height;
        let max_scroll = (total_height - self.state.bounds.height()).max(0);
        if self.scroll_offset < 0 {
            self.scroll_offset = 0;
        } else if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    fn count_visible_nodes(&self) -> usize {
        fn count_recursive(node: &TreeNode) -> usize {
            let mut count = 1;
            if node.expanded {
                for child in &node.children {
                    count += count_recursive(child);
                }
            }
            count
        }

        self.root_nodes.iter().map(count_recursive).sum()
    }

    fn draw_node(
        &self,
        node: &TreeNode,
        context: &mut dyn DrawContext,
        theme: &Theme,
        y: &mut i32,
        indent: i32,
    ) {
        // STOP if we're past the bottom
        if *y >= self.state.bounds.bottom() {
            return;
        }

        // Skip if completely above visible area
        if *y + self.item_height < self.state.bounds.y() {
            *y += self.item_height;
            if node.expanded {
                for child in &node.children {
                    self.draw_node(child, context, theme, y, indent + 1);
                }
            }
            return;
        }

        let item_rect = Rect::new(
            self.state.bounds.x(),
            *y,
            self.state.bounds.width(),
            self.item_height,
        );

        // Draw selection background
        if node.selected {
            context.set_color(theme.colors.selection);
            context.fill_rect(item_rect);
        }

        // Draw expand/collapse button
        let arrow_x = self.state.bounds.x() + indent * self.indent_width + 4;
        let arrow_y = item_rect.center().y;

        if !node.children.is_empty() {
            let button_size = 16;
            let button_rect =
                Rect::new(arrow_x, arrow_y - button_size / 2, button_size, button_size);

            // Draw button background
            context.set_color(theme.colors.surface_variant);
            context.fill_rect(button_rect);

            // Draw button border
            context.set_color(theme.colors.border);
            context.draw_rect(button_rect);

            // Draw expand/collapse icon
            context.set_color(theme.colors.text);
            let center_x = arrow_x + button_size / 2;
            let center_y = arrow_y;

            if node.expanded {
                // Minus sign
                context.fill_rect(Rect::new(center_x - 4, center_y - 1, 8, 2));
            } else {
                // Plus sign
                context.fill_rect(Rect::new(center_x - 4, center_y - 1, 8, 2));
                context.fill_rect(Rect::new(center_x - 1, center_y - 4, 2, 8));
            }
        }

        // Draw text
        let text_color = if node.selected {
            theme.colors.background
        } else {
            theme.colors.text
        };

        context.set_color(text_color);
        let text_x = if !node.children.is_empty() {
            arrow_x + 20
        } else {
            arrow_x + 4
        };
        let text_y = item_rect.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            &node.text,
            Point::new(text_x, text_y),
            theme.typography.font_size_base,
        );

        *y += self.item_height;

        // Draw children if expanded
        if node.expanded {
            for child in &node.children {
                self.draw_node(child, context, theme, y, indent + 1);
            }
        }
    }

    fn node_at_position(&self, position: Point) -> Option<String> {
        if !self.state.bounds.contains(position) {
            return None;
        }

        // Adjust for scroll offset
        let adjusted_y = position.y + self.scroll_offset;
        let mut y = self.state.bounds.y();

        fn find_at_y(
            node: &TreeNode,
            target_y: i32,
            current_y: &mut i32,
            item_height: i32,
        ) -> Option<String> {
            if *current_y <= target_y && target_y < *current_y + item_height {
                return Some(node.id.clone());
            }

            *current_y += item_height;

            if node.expanded {
                for child in &node.children {
                    if let Some(id) = find_at_y(child, target_y, current_y, item_height) {
                        return Some(id);
                    }
                }
            }

            None
        }

        for node in &self.root_nodes {
            if let Some(id) = find_at_y(node, adjusted_y, &mut y, self.item_height) {
                return Some(id);
            }
        }

        None
    }
}

impl Widget for TreeView {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        let visible_nodes = self.count_visible_nodes();
        let height = (visible_nodes as i32 * self.item_height)
            .min(constraints.max_height.unwrap_or(i32::MAX));

        Size::new(
            constraints.max_width.unwrap_or(300),
            height.max(100), // Ensure minimum height
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Enable clipping
        context.push_clip_rect(self.state.bounds);

        // Draw nodes with scroll offset
        let mut y = self.state.bounds.y() - self.scroll_offset;
        for node in &self.root_nodes {
            self.draw_node(node, context, theme, &mut y, 0);
        }

        // Disable clipping
        context.pop_clip_rect();

        // Draw scrollbar if needed
        let total_height = self.count_visible_nodes() as i32 * self.item_height;
        if total_height > self.state.bounds.height() {
            let scrollbar_width = 12;
            let scrollbar_x = self.state.bounds.right() - scrollbar_width;

            // Scrollbar track
            context.set_color(theme.colors.surface_variant);
            context.fill_rect(Rect::new(
                scrollbar_x,
                self.state.bounds.y(),
                scrollbar_width,
                self.state.bounds.height(),
            ));

            // Scrollbar thumb
            let thumb_height = (self.state.bounds.height() as f32
                * self.state.bounds.height() as f32
                / total_height as f32) as i32;
            let thumb_y = self.state.bounds.y()
                + (self.scroll_offset as f32 * self.state.bounds.height() as f32
                    / total_height as f32) as i32;

            context.set_color(theme.colors.primary);
            context.fill_rect(Rect::new(
                scrollbar_x,
                thumb_y,
                scrollbar_width,
                thumb_height.max(20),
            ));
        }

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                if let Some(node_id) = self.node_at_position(*position) {
                    // Find node depth to calculate expand button position
                    let mut node_depth = 0;
                    let mut node_y = self.state.bounds.y();

                    fn find_node_info(
                        nodes: &[TreeNode],
                        target_id: &str,
                        current_y: &mut i32,
                        current_depth: i32,
                        item_height: i32,
                    ) -> Option<i32> {
                        for node in nodes {
                            if node.id == target_id {
                                return Some(current_depth);
                            }
                            *current_y += item_height;

                            if node.expanded {
                                if let Some(depth) = find_node_info(
                                    &node.children,
                                    target_id,
                                    current_y,
                                    current_depth + 1,
                                    item_height,
                                ) {
                                    return Some(depth);
                                }
                            }
                        }
                        None
                    }

                    if let Some(depth) =
                        find_node_info(&self.root_nodes, &node_id, &mut node_y, 0, self.item_height)
                    {
                        node_depth = depth;
                    }

                    // Calculate expand button bounds
                    let arrow_x = self.state.bounds.x() + node_depth * self.indent_width + 4;
                    let click_x = position.x;

                    // Check if we have a node with children and if click is on expand button
                    let mut clicked_expand = false;
                    if let Some(node) = self.find_node_mut(&node_id) {
                        if !node.children.is_empty()
                            && click_x >= arrow_x
                            && click_x <= arrow_x + 16
                        {
                            clicked_expand = true;
                        }
                    }

                    // Clear previous selection
                    fn clear_selection(node: &mut TreeNode) {
                        node.selected = false;
                        for child in &mut node.children {
                            clear_selection(child);
                        }
                    }

                    for node in &mut self.root_nodes {
                        clear_selection(node);
                    }

                    // Handle the click
                    let (expanded_changed, new_expanded) =
                        if let Some(node) = self.find_node_mut(&node_id) {
                            node.selected = true;

                            if clicked_expand && !node.children.is_empty() {
                                node.expanded = !node.expanded;
                                (true, node.expanded)
                            } else {
                                (false, false)
                            }
                        } else {
                            (false, false)
                        };

                    if expanded_changed {
                        if let Some(callback) = &mut self.on_expand {
                            callback(&node_id, new_expanded);
                        }
                    }

                    if let Some(callback) = &mut self.on_selection_change {
                        callback(&node_id);
                    }

                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            Event::MouseWheel(wheel_event) => {
                if !self.state.bounds.contains(wheel_event.position) {
                    return EventResult::Ignored;
                }

                let total_height = self.count_visible_nodes() as i32 * self.item_height;
                if total_height > self.state.bounds.height() {
                    self.scroll_offset = (self.scroll_offset - wheel_event.delta.y * 3)
                        .max(0)
                        .min(total_height - self.state.bounds.height());
                    return EventResult::Consumed;
                }

                EventResult::Ignored
            }
            Event::KeyPress(KeyPressEvent { key, .. }) => {
                // Keyboard navigation when focused. Without focus we
                // return Ignored so siblings (notably any text input
                // nested in a tree row) keep their keys.
                if !self.state.focused {
                    return EventResult::Ignored;
                }

                let visible = self.visible_ids();
                if visible.is_empty() {
                    return EventResult::Ignored;
                }

                let current = self.selected_id();
                let current_index = current
                    .as_ref()
                    .and_then(|id| visible.iter().position(|v| v == id));

                match key {
                    Key::Down => {
                        let next = match current_index {
                            Some(i) => (i + 1).min(visible.len() - 1),
                            None => 0,
                        };
                        let target = visible[next].clone();
                        self.set_selected_id(&target);
                        self.scroll_to_selected();
                        EventResult::Consumed
                    }
                    Key::Up => {
                        let next = match current_index {
                            Some(i) => i.saturating_sub(1),
                            None => 0,
                        };
                        let target = visible[next].clone();
                        self.set_selected_id(&target);
                        self.scroll_to_selected();
                        EventResult::Consumed
                    }
                    Key::Home => {
                        let target = visible[0].clone();
                        self.set_selected_id(&target);
                        self.scroll_to_selected();
                        EventResult::Consumed
                    }
                    Key::End => {
                        let target = visible[visible.len() - 1].clone();
                        self.set_selected_id(&target);
                        self.scroll_to_selected();
                        EventResult::Consumed
                    }
                    Key::Right => {
                        // If selection has children: collapsed ->
                        // expand; expanded -> move to first child. If
                        // leaf or no selection: noop-Consumed.
                        let Some(id) = current else {
                            return EventResult::Consumed;
                        };
                        let (has_children, expanded) = match self.find_node_mut(&id) {
                            Some(node) => (!node.children.is_empty(), node.expanded),
                            None => (false, false),
                        };
                        if has_children && !expanded {
                            if let Some(node) = self.find_node_mut(&id) {
                                node.expanded = true;
                            }
                            if let Some(callback) = &mut self.on_expand {
                                callback(&id, true);
                            }
                        } else if has_children && expanded {
                            // Move selection to first child.
                            let first_child_id =
                                self.find_node_mut(&id).and_then(|node| {
                                    node.children.first().map(|c| c.id.clone())
                                });
                            if let Some(child_id) = first_child_id {
                                self.set_selected_id(&child_id);
                                self.scroll_to_selected();
                            }
                        }
                        EventResult::Consumed
                    }
                    Key::Left => {
                        // If expanded: collapse. If collapsed (or leaf):
                        // move selection to parent.
                        let Some(id) = current else {
                            return EventResult::Consumed;
                        };
                        let (has_children, expanded) = match self.find_node_mut(&id) {
                            Some(node) => (!node.children.is_empty(), node.expanded),
                            None => (false, false),
                        };
                        if has_children && expanded {
                            if let Some(node) = self.find_node_mut(&id) {
                                node.expanded = false;
                            }
                            if let Some(callback) = &mut self.on_expand {
                                callback(&id, false);
                            }
                        } else if let Some(parent) = self.parent_id(&id) {
                            self.set_selected_id(&parent);
                            self.scroll_to_selected();
                        }
                        EventResult::Consumed
                    }
                    Key::Enter | Key::Space => {
                        // Treat as activate. Mirrors a click on the
                        // selected node: re-fire the selection callback.
                        if let Some(id) = current {
                            if let Some(callback) = &mut self.on_selection_change {
                                callback(&id);
                            }
                            return EventResult::Consumed;
                        }
                        EventResult::Ignored
                    }
                    _ => EventResult::Ignored,
                }
            }
            _ => EventResult::Ignored,
        }
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

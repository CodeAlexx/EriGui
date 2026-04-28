use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton, Point,
    Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

/// Pixels for a separator item's height. Bug cm24: extracted from five
/// scattered `9` literals for clarity.
const SEPARATOR_HEIGHT: i32 = 9;
/// Approximate per-character pixel width when sizing menu rows. Used in
/// place of `text.len()` (which counts bytes, not characters and was
/// over-counting multi-byte text). Bug cm20.
fn char_count(s: &str) -> i32 {
    s.chars().count() as i32
}

#[derive(Clone)]
pub struct ContextMenuItem {
    pub text: String,
    pub shortcut: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub submenu: Option<Vec<ContextMenuItem>>,
    pub on_click: Option<fn()>,
}

impl ContextMenuItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shortcut: String::new(),
            enabled: true,
            is_separator: false,
            submenu: None,
            on_click: None,
        }
    }

    pub fn separator() -> Self {
        Self {
            text: String::new(),
            shortcut: String::new(),
            enabled: true,
            is_separator: true,
            submenu: None,
            on_click: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self
    }

    pub fn with_on_click(mut self, handler: fn()) -> Self {
        self.on_click = Some(handler);
        self
    }

    pub fn with_submenu(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

pub struct ContextMenu {
    state: WidgetState,
    items: Vec<ContextMenuItem>,
    is_open: bool,
    hover_index: Option<usize>,
    position: Point,
    item_height: i32,
    min_width: i32,
    submenu_open: Option<usize>,
    submenu: Option<Box<ContextMenu>>,
    /// Viewport that bounded the parent menu, propagated to submenus so
    /// edge-anchored submenus stay on-screen. Bug cm16.
    viewport: Option<Rect>,
}

impl ContextMenu {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            is_open: false,
            hover_index: None,
            position: Point::ZERO,
            item_height: 25,
            min_width: 150,
            submenu_open: None,
            submenu: None,
            viewport: None,
        }
    }

    pub fn with_items(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.items = items;
        self
    }

    pub fn add_item(&mut self, item: ContextMenuItem) {
        self.items.push(item);
    }

    pub fn show_at(&mut self, position: Point) {
        self.show_at_bounded(position, None);
    }

    pub fn show_at_bounded(&mut self, position: Point, viewport: Option<Rect>) {
        // Bug cm19: only reset submenu state when re-opening at a *new*
        // position. Calling show_at_bounded again at the same anchor
        // shouldn't blow away an existing submenu.
        let position_changed = self.position != position;
        let was_closed = !self.is_open;
        self.is_open = true;
        if was_closed || position_changed {
            self.hover_index = None;
            self.submenu_open = None;
            self.submenu = None;
        }
        self.viewport = viewport;

        // Calculate menu size
        let width = self.calculate_width();
        let height = self.calculate_height();

        // Adjust position to keep menu within viewport bounds
        let (x, y) = if let Some(vp) = viewport {
            let mut x = position.x;
            let mut y = position.y;

            // Keep within right edge
            if x + width > vp.right() {
                x = vp.right() - width;
            }
            // Keep within bottom edge
            if y + height > vp.bottom() {
                y = vp.bottom() - height;
            }
            // Keep within left edge
            if x < vp.x() {
                x = vp.x();
            }
            // Keep within top edge
            if y < vp.y() {
                y = vp.y();
            }
            (x, y)
        } else {
            (position.x, position.y)
        };

        self.position = Point::new(x, y);
        self.state.bounds = Rect::new(x, y, width, height);
    }

    pub fn hide(&mut self) {
        self.is_open = false;
        self.hover_index = None;
        self.submenu_open = None;
        self.submenu = None;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Test/debug accessor: whether a submenu is currently active.
    #[doc(hidden)]
    pub fn has_submenu(&self) -> bool {
        self.submenu.is_some()
    }

    /// Test/debug accessor: bounds of the active submenu, if any.
    #[doc(hidden)]
    pub fn submenu_bounds(&self) -> Option<Rect> {
        self.submenu.as_ref().map(|s| s.state.bounds)
    }

    /// Test/debug accessor: index of currently hovered parent item.
    #[doc(hidden)]
    pub fn hover_index(&self) -> Option<usize> {
        self.hover_index
    }

    /// Test/debug accessor: index of the parent item whose submenu is open.
    #[doc(hidden)]
    pub fn submenu_open_index(&self) -> Option<usize> {
        self.submenu_open
    }

    fn calculate_width(&self) -> i32 {
        let mut max_width = self.min_width;

        for item in &self.items {
            if !item.is_separator {
                // Bug cm20: count characters, not bytes — multi-byte UTF-8
                // labels were over-allocating width.
                let text_width = char_count(&item.text) * 7;
                let shortcut_width = if item.shortcut.is_empty() {
                    0
                } else {
                    char_count(&item.shortcut) * 7 + 20
                };
                let submenu_arrow = if item.submenu.is_some() { 20 } else { 0 };
                let total_width = 40 + text_width + shortcut_width + submenu_arrow;
                max_width = max_width.max(total_width);
            }
        }

        max_width
    }

    fn calculate_height(&self) -> i32 {
        let mut height = 0;
        for item in &self.items {
            height += if item.is_separator {
                SEPARATOR_HEIGHT
            } else {
                self.item_height
            };
        }
        height + 4 // padding
    }

    fn item_height_at(&self, index: usize) -> i32 {
        if self.items[index].is_separator {
            SEPARATOR_HEIGHT
        } else {
            self.item_height
        }
    }

    fn get_item_rect(&self, index: usize) -> Rect {
        // Bug cm21: linear walk, but O(n) per lookup is fine for menus —
        // the previous code did O(n²) only when *drawing*, which was the
        // worst place for it. Centralized here so callers don't repeat.
        let mut y = self.state.bounds.y() + 2;
        for i in 0..index {
            y += self.item_height_at(i);
        }
        let height = self.item_height_at(index);
        Rect::new(self.state.bounds.x(), y, self.state.bounds.width(), height)
    }

    /// Bug cm22: distinguish "outside" / "separator" / "disabled" /
    /// "actionable" so the caller can react correctly.
    fn classify_point(&self, point: Point) -> ItemHit {
        if !self.state.bounds.contains(point) {
            return ItemHit::Outside;
        }
        let mut y = self.state.bounds.y() + 2;
        for (i, item) in self.items.iter().enumerate() {
            let h = if item.is_separator {
                SEPARATOR_HEIGHT
            } else {
                self.item_height
            };
            if point.y >= y && point.y < y + h {
                if item.is_separator {
                    return ItemHit::Separator;
                }
                if !item.enabled {
                    return ItemHit::Disabled(i);
                }
                return ItemHit::Item(i);
            }
            y += h;
        }
        ItemHit::Outside
    }

    fn item_from_point(&self, point: Point) -> Option<usize> {
        match self.classify_point(point) {
            ItemHit::Item(i) => Some(i),
            _ => None,
        }
    }

    /// Cycle to the next focusable (non-separator, non-disabled) item.
    /// Returns the new hover index or None if the menu has no focusable
    /// items at all. Bug cm15.
    fn step_hover(&self, current: Option<usize>, forward: bool) -> Option<usize> {
        let n = self.items.len();
        if n == 0 {
            return None;
        }
        let start = current.unwrap_or(if forward { n - 1 } else { 0 });
        let mut i = start;
        for _ in 0..n {
            i = if forward {
                (i + 1) % n
            } else if i == 0 {
                n - 1
            } else {
                i - 1
            };
            let item = &self.items[i];
            if !item.is_separator && item.enabled {
                return Some(i);
            }
        }
        current
    }

    /// Test/debug accessor: number of items.
    #[doc(hidden)]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Result of `ContextMenu::classify_point`. Bug cm22.
#[derive(Clone, Copy)]
enum ItemHit {
    Outside,
    Separator,
    /// Disabled item — index unused for now, but exposed for callers
    /// that want to log/inspect the click target. Bug cm22.
    #[allow(dead_code)]
    Disabled(usize),
    Item(usize),
}

impl Widget for ContextMenu {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(self.calculate_width(), self.calculate_height())
    }

    fn layout(&mut self, _rect: Rect, _theme: &Theme) {
        // Context menu controls its own position
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible || !self.is_open {
            return;
        }

        // Draw shadow
        let shadow_offset = 2;
        let shadow_rect = Rect::new(
            self.state.bounds.x() + shadow_offset,
            self.state.bounds.y() + shadow_offset,
            self.state.bounds.width(),
            self.state.bounds.height(),
        );
        context.set_color(theme.colors.shadow);
        context.fill_rect(shadow_rect);

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);

        // Draw items
        for (i, item) in self.items.iter().enumerate() {
            let item_rect = self.get_item_rect(i);

            if item.is_separator {
                // Draw separator line
                context.set_color(theme.colors.border);
                let y = item_rect.center().y;
                context.draw_line(
                    Point::new(item_rect.x() + 5, y),
                    Point::new(item_rect.right() - 5, y),
                    1,
                );
            } else {
                // Draw item background if hovered
                if Some(i) == self.hover_index && item.enabled {
                    context.set_color(theme.colors.primary_hover);
                    context.fill_rect(item_rect);
                }

                // Draw item text
                let text_color = if item.enabled {
                    theme.colors.text
                } else {
                    theme.colors.text_disabled
                };
                context.set_color(text_color);

                let text_y = item_rect.center().y - theme.typography.font_size_base / 2;
                context.draw_text(
                    &item.text,
                    Point::new(item_rect.x() + 10, text_y),
                    theme.typography.font_size_base,
                );

                // Draw shortcut
                if !item.shortcut.is_empty() {
                    context.set_color(theme.colors.text_secondary);
                    let shortcut_width = item.shortcut.len() as i32 * 7;
                    let shortcut_x = item_rect.right() - shortcut_width - 10;
                    context.draw_text(
                        &item.shortcut,
                        Point::new(shortcut_x, text_y),
                        theme.typography.font_size_base,
                    );
                }

                // Draw submenu arrow
                if item.submenu.is_some() {
                    context.set_color(text_color);
                    let arrow_x = item_rect.right() - 15;
                    let arrow_y = item_rect.center().y;

                    // Draw right-pointing triangle
                    context.draw_line(
                        Point::new(arrow_x, arrow_y - 4),
                        Point::new(arrow_x + 4, arrow_y),
                        1,
                    );
                    context.draw_line(
                        Point::new(arrow_x + 4, arrow_y),
                        Point::new(arrow_x, arrow_y + 4),
                        1,
                    );
                }
            }
        }

        // Draw submenu if open
        if let Some(submenu) = &self.submenu {
            submenu.draw(context, theme);
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled || !self.is_open {
            return EventResult::Ignored;
        }

        // Handle submenu events first
        if let Some(submenu) = &mut self.submenu {
            if submenu.handle_event(event, _theme).is_consumed() {
                return EventResult::Consumed;
            }
        }

        match event {
            Event::MouseMove(mouse_event) => {
                // If the pointer is currently over an active submenu's
                // bounds, do NOT mutate parent hover state. Otherwise the
                // parent clears `hover_index` to None each time the mouse
                // is in the submenu, which both flickers the parent's
                // highlight and (worse) drives the destroy-when-not-over-
                // parent branch below into deciding to tear down the
                // submenu.
                if let Some(submenu) = &self.submenu {
                    if submenu.state.bounds.contains(mouse_event.position) {
                        return EventResult::Consumed;
                    }
                }

                let old_hover = self.hover_index;
                self.hover_index = self.item_from_point(mouse_event.position);

                // Handle submenu opening/closing
                if let Some(hover_idx) = self.hover_index {
                    if self.items[hover_idx].submenu.is_some() {
                        if self.submenu_open != Some(hover_idx) {
                            // Open new submenu — bound it to the same
                            // viewport as the parent if available, so
                            // edge-anchored submenus don't render off-
                            // screen. Bug cm16.
                            self.submenu_open = Some(hover_idx);
                            let item_rect = self.get_item_rect(hover_idx);
                            let submenu_pos = Point::new(item_rect.right(), item_rect.y());

                            let mut submenu = ContextMenu::new(WidgetId::default());
                            submenu.items = self.items[hover_idx].submenu.as_ref().unwrap().clone();
                            submenu.show_at_bounded(submenu_pos, self.viewport);
                            self.submenu = Some(Box::new(submenu));
                        }
                    } else if self.submenu_open.is_some() {
                        // Close submenu if hovering over non-submenu item
                        self.submenu_open = None;
                        self.submenu = None;
                    }
                } else if self.submenu_open.is_some()
                    && !self.state.bounds.contains(mouse_event.position)
                {
                    // Keep submenu open if mouse is outside but submenu is open
                    if let Some(submenu) = &self.submenu {
                        if !submenu.state.bounds.contains(mouse_event.position) {
                            self.submenu_open = None;
                            self.submenu = None;
                        }
                    }
                }

                // Bug cm17: when the pointer is inside the menu, always
                // consume MouseMove so it can't propagate to widgets
                // beneath the popup, even when the hover index didn't
                // change between events.
                let _ = old_hover;
                if self.state.bounds.contains(mouse_event.position) {
                    EventResult::Consumed
                } else if let Some(sub) = &self.submenu {
                    if sub.state.bounds.contains(mouse_event.position) {
                        EventResult::Consumed
                    } else {
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.pressed && mouse_event.button == MouseButton::Left {
                    // Bug cm18: classify the click so separator and
                    // disabled hits are handled distinctly (prior code
                    // fell through to the "outside" branch which was the
                    // wrong remedy).
                    match self.classify_point(mouse_event.position) {
                        ItemHit::Item(index) => {
                            let item = &self.items[index];
                            if item.submenu.is_none() {
                                if let Some(handler) = item.on_click {
                                    handler();
                                }
                                self.hide();
                                return EventResult::Consumed;
                            }
                            // Click on a submenu-bearing item: keep open.
                            return EventResult::Consumed;
                        }
                        ItemHit::Separator | ItemHit::Disabled(_) => {
                            // Swallow the click — don't action and don't
                            // close the menu. Bug cm18.
                            return EventResult::Consumed;
                        }
                        ItemHit::Outside => {
                            // Click outside parent — check submenu.
                            if let Some(submenu) = &self.submenu {
                                if submenu.state.bounds.contains(mouse_event.position) {
                                    // Inside submenu — let the submenu
                                    // handler at the top of this fn deal
                                    // with it.
                                    return EventResult::Ignored;
                                }
                            }
                            self.hide();
                            return EventResult::Consumed;
                        }
                    }
                }
                EventResult::Ignored
            }

            Event::KeyPress(KeyPressEvent { key, .. }) => {
                // Bug cm15: keyboard nav. Up/Down cycle items, Enter
                // activates, Esc closes, Right opens submenu, Left closes
                // submenu (or the whole menu if at top level).
                match key {
                    Key::Up => {
                        self.hover_index = self.step_hover(self.hover_index, false);
                        EventResult::Consumed
                    }
                    Key::Down => {
                        self.hover_index = self.step_hover(self.hover_index, true);
                        EventResult::Consumed
                    }
                    Key::Escape => {
                        self.hide();
                        EventResult::Consumed
                    }
                    Key::Enter | Key::Space => {
                        if let Some(idx) = self.hover_index {
                            let item = &self.items[idx];
                            if item.enabled && !item.is_separator {
                                if item.submenu.is_some() {
                                    // Open / focus submenu.
                                    if self.submenu_open != Some(idx) {
                                        self.submenu_open = Some(idx);
                                        let item_rect = self.get_item_rect(idx);
                                        let pos =
                                            Point::new(item_rect.right(), item_rect.y());
                                        let mut submenu =
                                            ContextMenu::new(WidgetId::default());
                                        submenu.items = self.items[idx]
                                            .submenu
                                            .as_ref()
                                            .unwrap()
                                            .clone();
                                        submenu.show_at_bounded(pos, self.viewport);
                                        // Pre-select first focusable item.
                                        submenu.hover_index = submenu.step_hover(None, true);
                                        self.submenu = Some(Box::new(submenu));
                                    }
                                } else {
                                    if let Some(handler) = item.on_click {
                                        handler();
                                    }
                                    self.hide();
                                }
                            }
                        }
                        EventResult::Consumed
                    }
                    Key::Right => {
                        if let Some(idx) = self.hover_index {
                            if self.items[idx].submenu.is_some()
                                && self.submenu_open != Some(idx)
                            {
                                self.submenu_open = Some(idx);
                                let item_rect = self.get_item_rect(idx);
                                let pos = Point::new(item_rect.right(), item_rect.y());
                                let mut submenu = ContextMenu::new(WidgetId::default());
                                submenu.items =
                                    self.items[idx].submenu.as_ref().unwrap().clone();
                                submenu.show_at_bounded(pos, self.viewport);
                                submenu.hover_index = submenu.step_hover(None, true);
                                self.submenu = Some(Box::new(submenu));
                            }
                        }
                        EventResult::Consumed
                    }
                    Key::Left => {
                        if self.submenu_open.is_some() {
                            self.submenu_open = None;
                            self.submenu = None;
                        }
                        EventResult::Consumed
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
        self.is_open
    }

    fn set_focused(&mut self, _focused: bool) {
        // Bug cm23: prior code closed the menu on any focus-loss. That
        // includes window resizes, which raise focus events that should
        // not dismiss popups. Outside-click handling is the correct
        // dismissal path; explicit `hide()` calls work as before.
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

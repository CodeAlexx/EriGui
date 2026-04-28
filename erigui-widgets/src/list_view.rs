use erigui_core::{
    DrawContext, Event, EventResult, Key, LayoutConstraints, MouseButton, MouseButtonEvent, Point,
    Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct ListItem {
    pub id: String,
    pub text: String,
    pub icon: Option<String>,
    pub selected: bool,
}

pub struct ListView {
    state: WidgetState,
    items: Vec<ListItem>,
    selected_index: Option<usize>,
    item_height: i32,
    on_selection_change: Option<Box<dyn FnMut(Option<usize>)>>,
    multi_select: bool,
}

impl ListView {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            selected_index: None,
            item_height: 24,
            on_selection_change: None,
            multi_select: false,
        }
    }

    pub fn with_items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }

    pub fn with_on_selection_change<F: FnMut(Option<usize>) + 'static>(mut self, f: F) -> Self {
        self.on_selection_change = Some(Box::new(f));
        self
    }

    pub fn with_multi_select(mut self, multi_select: bool) -> Self {
        self.multi_select = multi_select;
        self
    }

    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.selected_index = None;
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        if index.map(|i| i < self.items.len()).unwrap_or(true) {
            self.selected_index = index;

            if let Some(callback) = &mut self.on_selection_change {
                callback(self.selected_index);
            }
        }
    }

    pub fn selected_item(&self) -> Option<&ListItem> {
        self.selected_index.and_then(|i| self.items.get(i))
    }

    fn item_rect(&self, index: usize) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y() + (index as i32 * self.item_height),
            self.state.bounds.width(),
            self.item_height,
        )
    }

    fn item_at_position(&self, position: Point) -> Option<usize> {
        if !self.state.bounds.contains(position) {
            return None;
        }

        let relative_y = position.y - self.state.bounds.y();
        let index = (relative_y / self.item_height) as usize;

        if index < self.items.len() {
            Some(index)
        } else {
            None
        }
    }
}

impl Widget for ListView {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        let height = (self.items.len() as i32 * self.item_height)
            .min(constraints.max_height.unwrap_or(i32::MAX));

        Size::new(constraints.max_width.unwrap_or(200), height)
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

        // Draw items
        for (index, item) in self.items.iter().enumerate() {
            let item_rect = self.item_rect(index);

            // Skip items outside the visible area
            if item_rect.y() >= self.state.bounds.bottom() {
                break;
            }

            if item_rect.bottom() < self.state.bounds.y() {
                continue;
            }

            // Draw selection background
            if self.selected_index == Some(index) || (self.multi_select && item.selected) {
                context.set_color(theme.colors.selection);
                context.fill_rect(item_rect);
            }

            // Draw item text
            let text_color = if self.state.enabled {
                if self.selected_index == Some(index) {
                    theme.colors.surface
                } else {
                    theme.colors.text
                }
            } else {
                theme.colors.text_disabled
            };

            context.set_color(text_color);
            let text_pos = Point::new(
                item_rect.x() + theme.spacing.padding.left,
                item_rect.center().y - theme.typography.font_size_base / 2,
            );
            context.draw_text(&item.text, text_pos, theme.typography.font_size_base);
        }

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        if let Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) = event {
            if let Some(index) = self.item_at_position(*position) {
                if self.multi_select {
                    self.items[index].selected = !self.items[index].selected;
                } else {
                    self.set_selected_index(Some(index));
                }
                self.state.focused = true;
                return EventResult::Consumed;
            }
        }

        // Keyboard navigation when focused. Standard list behavior:
        // Up/Down move selection, Home/End jump, Space/Enter on the
        // selected item is host-meaningful (no internal action — the
        // host wires it through `on_selection_change` already firing
        // on `set_selected_index`).
        if let Event::KeyPress(ev) = event {
            if !self.state.focused {
                return EventResult::Ignored;
            }
            if self.items.is_empty() {
                return EventResult::Ignored;
            }
            let cur = self.selected_index.unwrap_or(0);
            let last = self.items.len() - 1;
            match ev.key {
                Key::Down => {
                    let next = (cur + 1).min(last);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::Up => {
                    let next = cur.saturating_sub(1);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::Home => {
                    self.set_selected_index(Some(0));
                    return EventResult::Consumed;
                }
                Key::End => {
                    self.set_selected_index(Some(last));
                    return EventResult::Consumed;
                }
                Key::PageDown => {
                    // 10 items as a reasonable page size since item height
                    // is fixed at 24 and viewport size isn't known at this
                    // layer.
                    let next = (cur + 10).min(last);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::PageUp => {
                    let next = cur.saturating_sub(10);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }

        EventResult::Ignored
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

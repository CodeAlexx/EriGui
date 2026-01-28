use erigui_core::{
    DrawContext, Event, EventResult, Key, LayoutConstraints, MouseButton, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct ComboBox {
    state: WidgetState,
    items: Vec<String>,
    selected_index: Option<usize>,
    is_open: bool,
    hover_index: Option<usize>,
    filter_text: String,
    filtered_indices: Vec<usize>,
    dropdown_height: i32,
    max_visible_items: usize,
    on_selection_changed: Option<Box<dyn Fn(Option<usize>, Option<&str>)>>,
}

impl ComboBox {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            selected_index: None,
            is_open: false,
            hover_index: None,
            filter_text: String::new(),
            filtered_indices: Vec::new(),
            dropdown_height: 200,
            max_visible_items: 8,
            on_selection_changed: None,
        }
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self.update_filter();
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        if index < self.items.len() {
            self.selected_index = Some(index);
        }
        self
    }

    pub fn with_on_selection_changed<F>(mut self, handler: F) -> Self
    where
        F: Fn(Option<usize>, Option<&str>) + 'static,
    {
        self.on_selection_changed = Some(Box::new(handler));
        self
    }

    pub fn add_item(&mut self, item: String) {
        self.items.push(item);
        self.update_filter();
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selected_index
            .and_then(|i| self.items.get(i).map(|s| s.as_str()))
    }

    pub fn set_selected(&mut self, index: Option<usize>) {
        if index.map_or(true, |i| i < self.items.len()) {
            self.selected_index = index;
            if let Some(handler) = &self.on_selection_changed {
                handler(self.selected_index, self.selected_text());
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn set_open(&mut self, open: bool) {
        self.is_open = open;
        if open {
            self.filter_text.clear();
            self.update_filter();
        }
    }

    fn update_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.to_lowercase().contains(&filter_lower))
                .map(|(i, _)| i)
                .collect();
        }
    }

    fn get_dropdown_rect(&self) -> Rect {
        let item_height = 25;
        let visible_items = self.filtered_indices.len().min(self.max_visible_items);
        let height = (visible_items as i32 * item_height).min(self.dropdown_height);

        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.bottom(),
            self.state.bounds.width(),
            height,
        )
    }

    #[allow(dead_code)]
    fn get_item_rect(&self, index: usize) -> Option<Rect> {
        let dropdown_rect = self.get_dropdown_rect();
        let item_height = 25;

        self.filtered_indices
            .iter()
            .position(|&i| i == index)
            .map(|pos| {
                Rect::new(
                    dropdown_rect.x(),
                    dropdown_rect.y() + pos as i32 * item_height,
                    dropdown_rect.width(),
                    item_height,
                )
            })
    }

    fn index_from_point(&self, point: Point) -> Option<usize> {
        let dropdown_rect = self.get_dropdown_rect();
        if !dropdown_rect.contains(point) {
            return None;
        }

        let item_height = 25;
        let relative_y = point.y - dropdown_rect.y();
        let pos = (relative_y / item_height) as usize;

        self.filtered_indices.get(pos).copied()
    }
}

impl Widget for ComboBox {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        Size::new(200, theme.typography.font_size_base + 12)
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw combo box background
        context.set_color(if self.state.enabled {
            theme.colors.surface
        } else {
            theme.colors.surface_variant
        });
        context.fill_rect(self.state.bounds);

        // Draw border
        context.set_color(if self.is_open {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(self.state.bounds);

        // Draw selected text or placeholder
        let text = self
            .selected_text()
            .unwrap_or(if self.filter_text.is_empty() {
                "Select..."
            } else {
                &self.filter_text
            });

        context.set_color(if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        });

        let text_y = self.state.bounds.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            text,
            Point::new(self.state.bounds.x() + 8, text_y),
            theme.typography.font_size_base,
        );

        // Draw dropdown arrow
        let arrow_size = 8;
        let arrow_x = self.state.bounds.right() - arrow_size - 8;
        let arrow_y = self.state.bounds.center().y;

        context.set_color(theme.colors.text);

        // Draw simple triangle arrow
        if self.is_open {
            // Up arrow
            context.draw_line(
                Point::new(arrow_x, arrow_y + arrow_size / 2),
                Point::new(arrow_x + arrow_size / 2, arrow_y - arrow_size / 2),
                2,
            );
            context.draw_line(
                Point::new(arrow_x + arrow_size / 2, arrow_y - arrow_size / 2),
                Point::new(arrow_x + arrow_size, arrow_y + arrow_size / 2),
                2,
            );
        } else {
            // Down arrow
            context.draw_line(
                Point::new(arrow_x, arrow_y - arrow_size / 2),
                Point::new(arrow_x + arrow_size / 2, arrow_y + arrow_size / 2),
                2,
            );
            context.draw_line(
                Point::new(arrow_x + arrow_size / 2, arrow_y + arrow_size / 2),
                Point::new(arrow_x + arrow_size, arrow_y - arrow_size / 2),
                2,
            );
        }

        // Draw dropdown if open
        if self.is_open {
            let dropdown_rect = self.get_dropdown_rect();

            // Draw dropdown background
            context.set_color(theme.colors.surface);
            context.fill_rect(dropdown_rect);

            // Draw dropdown border
            context.set_color(theme.colors.border);
            context.draw_rect(dropdown_rect);

            // Draw items
            let item_height = 25;
            for (pos, &index) in self.filtered_indices.iter().enumerate() {
                if pos >= self.max_visible_items {
                    break;
                }

                let item_rect = Rect::new(
                    dropdown_rect.x(),
                    dropdown_rect.y() + pos as i32 * item_height,
                    dropdown_rect.width(),
                    item_height,
                );

                // Draw item background if hovered or selected
                if Some(index) == self.hover_index {
                    context.set_color(theme.colors.primary_hover);
                    context.fill_rect(item_rect);
                } else if Some(index) == self.selected_index {
                    context.set_color(theme.colors.selection);
                    context.fill_rect(item_rect);
                }

                // Draw item text
                context.set_color(theme.colors.text);
                let text_y = item_rect.center().y - theme.typography.font_size_base / 2;
                context.draw_text(
                    &self.items[index],
                    Point::new(item_rect.x() + 8, text_y),
                    theme.typography.font_size_base,
                );
            }
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left && mouse_event.pressed {
                    if self.state.bounds.contains(mouse_event.position) {
                        self.is_open = !self.is_open;
                        if self.is_open {
                            self.filter_text.clear();
                            self.update_filter();
                        }
                        EventResult::Consumed
                    } else if self.is_open {
                        if let Some(index) = self.index_from_point(mouse_event.position) {
                            self.set_selected(Some(index));
                            self.is_open = false;
                            self.filter_text.clear();
                            EventResult::Consumed
                        } else {
                            self.is_open = false;
                            self.filter_text.clear();
                            EventResult::Consumed
                        }
                    } else {
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }

            Event::MouseMove(mouse_event) => {
                if self.is_open {
                    self.hover_index = self.index_from_point(mouse_event.position);
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            Event::KeyPress(key_event) => {
                if self.is_open {
                    match key_event.key {
                        Key::Escape => {
                            self.is_open = false;
                            self.filter_text.clear();
                            self.update_filter();
                            EventResult::Consumed
                        }
                        Key::Character(ch) => {
                            self.filter_text.push(ch);
                            self.update_filter();
                            EventResult::Consumed
                        }
                        Key::Backspace => {
                            self.filter_text.pop();
                            self.update_filter();
                            EventResult::Consumed
                        }
                        _ => EventResult::Ignored,
                    }
                } else {
                    EventResult::Ignored
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
        if !enabled {
            self.is_open = false;
        }
    }

    fn is_focused(&self) -> bool {
        self.is_open
    }

    fn set_focused(&mut self, focused: bool) {
        if !focused {
            self.is_open = false;
        }
    }

    fn can_focus(&self) -> bool {
        self.state.enabled
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

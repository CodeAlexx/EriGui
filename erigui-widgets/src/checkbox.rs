use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct Checkbox {
    state: WidgetState,
    text: String,
    checked: bool,
    is_hovering: bool,
    box_size: i32,
    on_toggle: Option<Box<dyn FnMut(bool)>>,
}

impl Checkbox {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            state: WidgetState::new(id),
            text: text.into(),
            checked: false,
            is_hovering: false,
            box_size: 16,
            on_toggle: None,
        }
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn with_on_toggle<F>(mut self, handler: F) -> Self
    where
        F: FnMut(bool) + 'static,
    {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            if let Some(handler) = &mut self.on_toggle {
                handler(self.checked);
            }
        }
    }

    pub fn toggle(&mut self) {
        self.set_checked(!self.checked);
    }

    fn get_box_rect(&self) -> Rect {
        let center_y = self.state.bounds.center().y - self.box_size / 2;
        Rect::new(
            self.state.bounds.x(),
            center_y,
            self.box_size,
            self.box_size,
        )
    }

    fn draw_checkmark(&self, context: &mut dyn DrawContext, box_rect: Rect, theme: &Theme) {
        if !self.checked {
            return;
        }

        // Draw checkmark
        context.set_color(theme.colors.background);

        // Draw a simple checkmark using lines
        let padding = 3;
        let x = box_rect.x() + padding;
        let y = box_rect.y() + padding;
        let w = box_rect.width() - 2 * padding;
        let h = box_rect.height() - 2 * padding;

        // Draw checkmark path
        context.draw_line(
            Point::new(x + w / 4, y + h / 2),
            Point::new(x + w / 2, y + 3 * h / 4),
            2,
        );
        context.draw_line(
            Point::new(x + w / 2, y + 3 * h / 4),
            Point::new(x + 3 * w / 4, y + h / 4),
            2,
        );
    }
}

impl Widget for Checkbox {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let text_width = self.text.len() as i32 * theme.typography.font_size_base / 2;
        let total_width = self.box_size + 8 + text_width; // 8px spacing
        Size::new(
            total_width,
            self.box_size.max(theme.typography.font_size_base),
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let box_rect = self.get_box_rect();

        // Draw checkbox background
        let bg_color = if !self.state.enabled {
            theme.colors.surface_variant
        } else if self.is_hovering {
            theme.colors.primary_hover
        } else if self.checked {
            theme.colors.primary
        } else {
            theme.colors.surface
        };

        context.set_color(bg_color);
        context.fill_rect(box_rect);

        // Draw checkbox border
        let border_color = if !self.state.enabled {
            theme.colors.border
        } else if self.checked {
            theme.colors.primary
        } else {
            theme.colors.border
        };

        context.set_color(border_color);
        context.draw_rect(box_rect);

        // Draw checkmark if checked
        self.draw_checkmark(context, box_rect, theme);

        // Draw text
        let text_color = if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };

        context.set_color(text_color);
        let text_x = box_rect.right() + 8;
        let text_y = self.state.bounds.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            &self.text,
            Point::new(text_x, text_y),
            theme.typography.font_size_base,
        );
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                let was_hovering = self.is_hovering;
                self.is_hovering = self.state.bounds.contains(mouse_event.position);

                if was_hovering != self.is_hovering {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left
                    && mouse_event.pressed
                    && self.state.bounds.contains(mouse_event.position)
                {
                    self.toggle();
                    self.state.focused = true;
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            // Keyboard toggle — Space and Enter, when focused.
            // Standard accessibility behavior.
            Event::KeyPress(ev) if self.state.focused => {
                use erigui_core::Key;
                match ev.key {
                    Key::Space | Key::Enter => {
                        self.toggle();
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
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
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

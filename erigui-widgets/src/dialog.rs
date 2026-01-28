use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, MouseButton, Point, Rect, Size,
    Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogType {
    Info,
    Warning,
    Error,
    Question,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogButton {
    Ok,
    Cancel,
    Yes,
    No,
    Close,
    Custom(u32),
}

pub struct Dialog {
    state: WidgetState,
    title: String,
    message: String,
    dialog_type: DialogType,
    buttons: Vec<(DialogButton, String)>,
    result: Option<DialogButton>,
    is_modal: bool,
    is_open: bool,
    is_dragging: bool,
    drag_offset: Point,
    hover_button: Option<usize>,
    title_height: i32,
    button_height: i32,
    button_width: i32,
    on_button_clicked: Option<Box<dyn Fn(DialogButton)>>,
}

impl Dialog {
    pub fn new(id: WidgetId, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            state: WidgetState::new(id),
            title: title.into(),
            message: message.into(),
            dialog_type: DialogType::Info,
            buttons: vec![(DialogButton::Ok, "OK".to_string())],
            result: None,
            is_modal: true,
            is_open: false,
            is_dragging: false,
            drag_offset: Point::ZERO,
            hover_button: None,
            title_height: 30,
            button_height: 30,
            button_width: 80,
            on_button_clicked: None,
        }
    }

    pub fn with_type(mut self, dialog_type: DialogType) -> Self {
        self.dialog_type = dialog_type;
        self
    }

    pub fn with_buttons(mut self, buttons: Vec<(DialogButton, String)>) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn with_standard_buttons(mut self, dialog_type: DialogType) -> Self {
        self.dialog_type = dialog_type;
        self.buttons = match dialog_type {
            DialogType::Info => vec![(DialogButton::Ok, "OK".to_string())],
            DialogType::Warning => vec![
                (DialogButton::Ok, "OK".to_string()),
                (DialogButton::Cancel, "Cancel".to_string()),
            ],
            DialogType::Error => vec![(DialogButton::Close, "Close".to_string())],
            DialogType::Question => vec![
                (DialogButton::Yes, "Yes".to_string()),
                (DialogButton::No, "No".to_string()),
            ],
            DialogType::Custom => vec![(DialogButton::Ok, "OK".to_string())],
        };
        self
    }

    pub fn with_on_button_clicked<F>(mut self, handler: F) -> Self
    where
        F: Fn(DialogButton) + 'static,
    {
        self.on_button_clicked = Some(Box::new(handler));
        self
    }

    pub fn show(&mut self) {
        self.is_open = true;
        self.result = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn result(&self) -> Option<DialogButton> {
        self.result
    }

    fn get_title_color(&self, theme: &Theme) -> Color {
        match self.dialog_type {
            DialogType::Info => theme.colors.info,
            DialogType::Warning => theme.colors.warning,
            DialogType::Error => theme.colors.error,
            DialogType::Question => theme.colors.success,
            DialogType::Custom => theme.colors.primary,
        }
    }

    fn get_title_rect(&self) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y(),
            self.state.bounds.width(),
            self.title_height,
        )
    }

    fn get_content_rect(&self) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y() + self.title_height,
            self.state.bounds.width(),
            self.state.bounds.height() - self.title_height - self.button_height - 20,
        )
    }

    fn get_button_area_rect(&self) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.bottom() - self.button_height - 10,
            self.state.bounds.width(),
            self.button_height + 10,
        )
    }

    fn get_button_rect(&self, index: usize) -> Rect {
        let button_area = self.get_button_area_rect();
        let total_width =
            self.buttons.len() as i32 * self.button_width + (self.buttons.len() as i32 - 1) * 10;
        let start_x = button_area.center().x - total_width / 2;

        Rect::new(
            start_x + index as i32 * (self.button_width + 10),
            button_area.y() + 5,
            self.button_width,
            self.button_height,
        )
    }
}

impl Widget for Dialog {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let text_width = self.message.len() as i32 * theme.typography.font_size_base / 2;
        let min_width = 300.max(text_width + 40);
        Size::new(min_width, 150)
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible || !self.is_open {
            return;
        }

        // Draw modal overlay if modal
        if self.is_modal {
            context.set_color(Color::rgba(0, 0, 0, 128));
            context.fill_rect(Rect::new(-5000, -5000, 10000, 10000)); // Large overlay
        }

        // Draw dialog shadow
        let shadow_offset = 4;
        let shadow_rect = Rect::new(
            self.state.bounds.x() + shadow_offset,
            self.state.bounds.y() + shadow_offset,
            self.state.bounds.width(),
            self.state.bounds.height(),
        );
        context.set_color(Color::rgba(0, 0, 0, 64));
        context.fill_rect(shadow_rect);

        // Draw dialog background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Draw dialog border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);

        // Draw title bar
        let title_rect = self.get_title_rect();
        context.set_color(self.get_title_color(theme));
        context.fill_rect(title_rect);

        // Draw title text
        context.set_color(theme.colors.background);
        let title_y = title_rect.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            &self.title,
            Point::new(title_rect.x() + 10, title_y),
            theme.typography.font_size_base,
        );

        // Draw close button
        let close_size = 20;
        let close_rect = Rect::new(
            title_rect.right() - close_size - 5,
            title_rect.y() + (title_rect.height() - close_size) / 2,
            close_size,
            close_size,
        );

        context.set_color(theme.colors.background);
        let padding = 5;
        context.draw_line(
            Point::new(close_rect.x() + padding, close_rect.y() + padding),
            Point::new(close_rect.right() - padding, close_rect.bottom() - padding),
            2,
        );
        context.draw_line(
            Point::new(close_rect.right() - padding, close_rect.y() + padding),
            Point::new(close_rect.x() + padding, close_rect.bottom() - padding),
            2,
        );

        // Draw message
        let content_rect = self.get_content_rect();
        context.set_color(theme.colors.text);
        let message_y = content_rect.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            &self.message,
            Point::new(content_rect.x() + 20, message_y),
            theme.typography.font_size_base,
        );

        // Draw buttons
        for (i, (_button, text)) in self.buttons.iter().enumerate() {
            let button_rect = self.get_button_rect(i);
            let is_hover = Some(i) == self.hover_button;

            // Draw button background
            context.set_color(if is_hover {
                theme.colors.primary_hover
            } else {
                theme.colors.primary
            });
            context.fill_rect(button_rect);

            // Draw button border
            context.set_color(theme.colors.border);
            context.draw_rect(button_rect);

            // Draw button text
            context.set_color(theme.colors.background);
            let text_width = text.len() as i32 * theme.typography.font_size_base / 2;
            let text_x = button_rect.center().x - text_width / 2;
            let text_y = button_rect.center().y - theme.typography.font_size_base / 2;
            context.draw_text(
                text,
                Point::new(text_x, text_y),
                theme.typography.font_size_base,
            );
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled || !self.is_open {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                if self.is_dragging {
                    self.state.bounds = Rect::new(
                        mouse_event.position.x - self.drag_offset.x,
                        mouse_event.position.y - self.drag_offset.y,
                        self.state.bounds.width(),
                        self.state.bounds.height(),
                    );
                    return EventResult::Consumed;
                }

                // Check button hover
                self.hover_button = None;
                for i in 0..self.buttons.len() {
                    if self.get_button_rect(i).contains(mouse_event.position) {
                        self.hover_button = Some(i);
                        break;
                    }
                }

                EventResult::Ignored
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    if mouse_event.pressed {
                        let title_rect = self.get_title_rect();

                        // Check close button
                        let close_size = 20;
                        let close_rect = Rect::new(
                            title_rect.right() - close_size - 5,
                            title_rect.y() + (title_rect.height() - close_size) / 2,
                            close_size,
                            close_size,
                        );

                        if close_rect.contains(mouse_event.position) {
                            self.close();
                            return EventResult::Consumed;
                        }

                        // Check title bar drag
                        if title_rect.contains(mouse_event.position) {
                            self.is_dragging = true;
                            self.drag_offset = Point::new(
                                mouse_event.position.x - self.state.bounds.x(),
                                mouse_event.position.y - self.state.bounds.y(),
                            );
                            return EventResult::Consumed;
                        }

                        // Check button clicks
                        for i in 0..self.buttons.len() {
                            if self.get_button_rect(i).contains(mouse_event.position) {
                                let (button, _) = &self.buttons[i];
                                self.result = Some(*button);
                                if let Some(handler) = &self.on_button_clicked {
                                    handler(*button);
                                }
                                self.close();
                                return EventResult::Consumed;
                            }
                        }
                    } else {
                        self.is_dragging = false;
                    }
                }

                if self.is_modal {
                    EventResult::Consumed
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
    }

    fn is_focused(&self) -> bool {
        self.is_open
    }

    fn set_focused(&mut self, focused: bool) {
        if !focused {
            self.is_dragging = false;
        }
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

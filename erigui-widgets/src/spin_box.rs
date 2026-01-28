use erigui_core::{
    DrawContext, Event, EventResult, Key, LayoutConstraints, MouseButton, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::time::{Duration, Instant};

pub struct SpinBox {
    state: WidgetState,
    min_value: f32,
    max_value: f32,
    current_value: f32,
    step: f32,
    decimals: usize,
    text_value: String,
    is_editing: bool,
    hover_button: Option<SpinButton>,
    pressed_button: Option<SpinButton>,
    button_repeat_start: Option<Instant>,
    last_repeat: Option<Instant>,
    on_value_changed: Option<Box<dyn Fn(f32)>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SpinButton {
    Up,
    Down,
}

impl SpinBox {
    pub fn new(id: WidgetId, min: f32, max: f32, initial: f32) -> Self {
        let mut spin_box = Self {
            state: WidgetState::new(id),
            min_value: min,
            max_value: max,
            current_value: initial.clamp(min, max),
            step: 1.0,
            decimals: 0,
            text_value: String::new(),
            is_editing: false,
            hover_button: None,
            pressed_button: None,
            button_repeat_start: None,
            last_repeat: None,
            on_value_changed: None,
        };
        spin_box.update_text_from_value();
        spin_box
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn with_decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self.update_text_from_value();
        self
    }

    pub fn with_on_value_changed<F>(mut self, handler: F) -> Self
    where
        F: Fn(f32) + 'static,
    {
        self.on_value_changed = Some(Box::new(handler));
        self
    }

    pub fn value(&self) -> f32 {
        self.current_value
    }

    pub fn set_value(&mut self, value: f32) {
        let new_value = value.clamp(self.min_value, self.max_value);
        if (new_value - self.current_value).abs() > f32::EPSILON {
            self.current_value = new_value;
            self.update_text_from_value();
            if let Some(handler) = &self.on_value_changed {
                handler(self.current_value);
            }
        }
    }

    pub fn increment(&mut self) {
        self.set_value(self.current_value + self.step);
    }

    pub fn decrement(&mut self) {
        self.set_value(self.current_value - self.step);
    }

    pub fn update(&mut self) {
        // Handle button repeat
        if let Some(button) = self.pressed_button {
            if let Some(start) = self.button_repeat_start {
                let elapsed = start.elapsed();
                if elapsed > Duration::from_millis(500) {
                    // Start repeating after 500ms
                    if let Some(last) = self.last_repeat {
                        if last.elapsed() > Duration::from_millis(50) {
                            match button {
                                SpinButton::Up => self.increment(),
                                SpinButton::Down => self.decrement(),
                            }
                            self.last_repeat = Some(Instant::now());
                        }
                    } else {
                        self.last_repeat = Some(Instant::now());
                    }
                }
            }
        }
    }

    fn update_text_from_value(&mut self) {
        self.text_value = format!("{:.prec$}", self.current_value, prec = self.decimals);
    }

    fn update_value_from_text(&mut self) {
        if let Ok(value) = self.text_value.parse::<f32>() {
            self.set_value(value);
        } else {
            self.update_text_from_value();
        }
    }

    fn get_text_rect(&self) -> Rect {
        let button_width = 20;
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y(),
            self.state.bounds.width() - button_width,
            self.state.bounds.height(),
        )
    }

    fn get_up_button_rect(&self) -> Rect {
        let button_width = 20;
        let button_height = self.state.bounds.height() / 2;
        Rect::new(
            self.state.bounds.right() - button_width,
            self.state.bounds.y(),
            button_width,
            button_height,
        )
    }

    fn get_down_button_rect(&self) -> Rect {
        let button_width = 20;
        let button_height = self.state.bounds.height() / 2;
        Rect::new(
            self.state.bounds.right() - button_width,
            self.state.bounds.y() + button_height,
            button_width,
            button_height,
        )
    }

    fn button_from_point(&self, point: Point) -> Option<SpinButton> {
        if self.get_up_button_rect().contains(point) {
            Some(SpinButton::Up)
        } else if self.get_down_button_rect().contains(point) {
            Some(SpinButton::Down)
        } else {
            None
        }
    }

    fn draw_arrow(&self, context: &mut dyn DrawContext, rect: Rect, up: bool) {
        let size = 6;
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        if up {
            // Up arrow
            context.draw_line(
                Point::new(center_x - size / 2, center_y + size / 2),
                Point::new(center_x, center_y - size / 2),
                1,
            );
            context.draw_line(
                Point::new(center_x, center_y - size / 2),
                Point::new(center_x + size / 2, center_y + size / 2),
                1,
            );
        } else {
            // Down arrow
            context.draw_line(
                Point::new(center_x - size / 2, center_y - size / 2),
                Point::new(center_x, center_y + size / 2),
                1,
            );
            context.draw_line(
                Point::new(center_x, center_y + size / 2),
                Point::new(center_x + size / 2, center_y - size / 2),
                1,
            );
        }
    }
}

impl Widget for SpinBox {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        Size::new(120, theme.typography.font_size_base + 12)
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw text area background
        let text_rect = self.get_text_rect();
        context.set_color(if self.state.enabled {
            theme.colors.surface
        } else {
            theme.colors.surface_variant
        });
        context.fill_rect(text_rect);

        // Draw text area border
        context.set_color(if self.is_editing {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(text_rect);

        // Draw text
        context.set_color(if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        });

        let text = if self.is_editing {
            &self.text_value
        } else {
            &self.text_value
        };
        let text_y = text_rect.center().y - theme.typography.font_size_base / 2;
        context.draw_text(
            text,
            Point::new(text_rect.x() + 8, text_y),
            theme.typography.font_size_base,
        );

        // Draw up button
        let up_rect = self.get_up_button_rect();
        let up_hover = self.hover_button == Some(SpinButton::Up);
        let up_pressed = self.pressed_button == Some(SpinButton::Up);

        context.set_color(if !self.state.enabled {
            theme.colors.surface_variant
        } else if up_pressed {
            theme.colors.primary_active
        } else if up_hover {
            theme.colors.primary_hover
        } else {
            theme.colors.surface_variant
        });
        context.fill_rect(up_rect);

        context.set_color(theme.colors.border);
        context.draw_rect(up_rect);

        context.set_color(if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        });
        self.draw_arrow(context, up_rect, true);

        // Draw down button
        let down_rect = self.get_down_button_rect();
        let down_hover = self.hover_button == Some(SpinButton::Down);
        let down_pressed = self.pressed_button == Some(SpinButton::Down);

        context.set_color(if !self.state.enabled {
            theme.colors.surface_variant
        } else if down_pressed {
            theme.colors.primary_active
        } else if down_hover {
            theme.colors.primary_hover
        } else {
            theme.colors.surface_variant
        });
        context.fill_rect(down_rect);

        context.set_color(theme.colors.border);
        context.draw_rect(down_rect);

        context.set_color(if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        });
        self.draw_arrow(context, down_rect, false);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                self.hover_button = self.button_from_point(mouse_event.position);
                EventResult::Ignored
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    if mouse_event.pressed {
                        if let Some(button) = self.button_from_point(mouse_event.position) {
                            self.pressed_button = Some(button);
                            self.button_repeat_start = Some(Instant::now());
                            self.last_repeat = None;

                            match button {
                                SpinButton::Up => self.increment(),
                                SpinButton::Down => self.decrement(),
                            }
                            EventResult::Consumed
                        } else if self.get_text_rect().contains(mouse_event.position) {
                            self.is_editing = true;
                            EventResult::Consumed
                        } else {
                            if self.is_editing {
                                self.is_editing = false;
                                self.update_value_from_text();
                            }
                            EventResult::Ignored
                        }
                    } else {
                        self.pressed_button = None;
                        self.button_repeat_start = None;
                        self.last_repeat = None;
                        EventResult::Ignored
                    }
                } else {
                    EventResult::Ignored
                }
            }

            Event::KeyPress(key_event) => {
                if self.is_editing {
                    match key_event.key {
                        Key::Character(ch) => {
                            if ch.is_numeric()
                                || ch == '.'
                                || (ch == '-' && self.text_value.is_empty())
                            {
                                self.text_value.push(ch);
                                EventResult::Consumed
                            } else {
                                EventResult::Ignored
                            }
                        }
                        Key::Backspace => {
                            self.text_value.pop();
                            EventResult::Consumed
                        }
                        Key::Enter => {
                            self.is_editing = false;
                            self.update_value_from_text();
                            EventResult::Consumed
                        }
                        Key::Escape => {
                            self.is_editing = false;
                            self.update_text_from_value();
                            EventResult::Consumed
                        }
                        _ => EventResult::Ignored,
                    }
                } else {
                    match key_event.key {
                        Key::Up => {
                            self.increment();
                            EventResult::Consumed
                        }
                        Key::Down => {
                            self.decrement();
                            EventResult::Consumed
                        }
                        _ => EventResult::Ignored,
                    }
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
            self.is_editing = false;
            self.pressed_button = None;
        }
    }

    fn is_focused(&self) -> bool {
        self.is_editing
    }

    fn set_focused(&mut self, focused: bool) {
        if !focused && self.is_editing {
            self.is_editing = false;
            self.update_value_from_text();
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

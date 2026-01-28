use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent, Point, Rect,
    Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPosition {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct RadioButton {
    state: WidgetState,
    text: String,
    group_id: u32,
    checked: bool,
    button_size: i32,
    label_position: LabelPosition,
    hover: bool,
    pressed: bool,
    animation_progress: f32,
    on_change: Option<Box<dyn FnMut(bool)>>,
}

impl RadioButton {
    pub fn new(id: WidgetId, text: impl Into<String>, group_id: u32) -> Self {
        // Register with group manager
        RadioGroupManager::instance()
            .lock()
            .unwrap()
            .register_button(id, group_id);

        Self {
            state: WidgetState::new(id),
            text: text.into(),
            group_id,
            checked: false,
            button_size: 16,
            label_position: LabelPosition::Right,
            hover: false,
            pressed: false,
            animation_progress: 0.0,
            on_change: None,
        }
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        if checked {
            RadioGroupManager::instance()
                .lock()
                .unwrap()
                .set_selected(self.group_id, self.state.id);
        }
        self
    }

    pub fn with_label_position(mut self, position: LabelPosition) -> Self {
        self.label_position = position;
        self
    }

    pub fn with_on_change<F: FnMut(bool) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            if checked {
                RadioGroupManager::instance()
                    .lock()
                    .unwrap()
                    .set_selected(self.group_id, self.state.id);
            }
            if let Some(callback) = &mut self.on_change {
                callback(checked);
            }
        }
    }

    fn update_animation(&mut self, delta_time: f32) {
        let target = if self.checked { 1.0 } else { 0.0 };
        let speed = 5.0; // Animation speed

        if self.animation_progress < target {
            self.animation_progress = (self.animation_progress + speed * delta_time).min(target);
        } else if self.animation_progress > target {
            self.animation_progress = (self.animation_progress - speed * delta_time).max(target);
        }
    }
}

impl Widget for RadioButton {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let text_size = if !self.text.is_empty() {
            // Approximate text size
            Size::new(
                self.text.len() as i32 * theme.typography.font_size_base * 3 / 5,
                theme.typography.font_size_base,
            )
        } else {
            Size::ZERO
        };

        let spacing = theme.spacing.gap_small;

        match self.label_position {
            LabelPosition::Left | LabelPosition::Right => Size::new(
                self.button_size + spacing + text_size.width,
                self.button_size.max(text_size.height),
            ),
            LabelPosition::Top | LabelPosition::Bottom => Size::new(
                self.button_size.max(text_size.width),
                self.button_size + spacing + text_size.height,
            ),
        }
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let spacing = theme.spacing.gap_small;

        // Calculate button position based on label position
        let (button_x, button_y) = match self.label_position {
            LabelPosition::Left => (
                self.state.bounds.right() - self.button_size,
                self.state.bounds.y() + (self.state.bounds.height() - self.button_size) / 2,
            ),
            LabelPosition::Right => (
                self.state.bounds.x(),
                self.state.bounds.y() + (self.state.bounds.height() - self.button_size) / 2,
            ),
            LabelPosition::Top => (
                self.state.bounds.x() + (self.state.bounds.width() - self.button_size) / 2,
                self.state.bounds.bottom() - self.button_size,
            ),
            LabelPosition::Bottom => (
                self.state.bounds.x() + (self.state.bounds.width() - self.button_size) / 2,
                self.state.bounds.y(),
            ),
        };

        let button_rect = Rect::new(button_x, button_y, self.button_size, self.button_size);
        let center = button_rect.center();
        let outer_radius = self.button_size / 2;
        let inner_radius = (outer_radius as f32 * 0.4 * self.animation_progress) as i32;

        // Draw outer circle
        let outer_color = if !self.state.enabled {
            theme.colors.text_disabled
        } else if self.pressed {
            theme.colors.primary_active
        } else if self.hover {
            theme.colors.primary_hover
        } else {
            theme.colors.border
        };

        context.set_color(outer_color);
        context.draw_circle(center, outer_radius, 16);

        // Draw inner filled circle when checked
        if self.animation_progress > 0.0 {
            let inner_color = if !self.state.enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.primary
            };

            context.set_color(inner_color);
            context.fill_circle(center, inner_radius, 12);
        }

        // Draw label
        if !self.text.is_empty() {
            let text_color = if !self.state.enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.text
            };

            context.set_color(text_color);

            let (text_x, text_y) = match self.label_position {
                LabelPosition::Left => (
                    self.state.bounds.x(),
                    button_rect.center().y + theme.typography.font_size_base / 2 - 2,
                ),
                LabelPosition::Right => (
                    button_rect.right() + spacing,
                    button_rect.center().y + theme.typography.font_size_base / 2 - 2,
                ),
                LabelPosition::Top => (
                    self.state.bounds.x()
                        + (self.state.bounds.width()
                            - self.text.len() as i32 * theme.typography.font_size_base * 3 / 5)
                            / 2,
                    self.state.bounds.y() + theme.typography.font_size_base,
                ),
                LabelPosition::Bottom => (
                    self.state.bounds.x()
                        + (self.state.bounds.width()
                            - self.text.len() as i32 * theme.typography.font_size_base * 3 / 5)
                            / 2,
                    button_rect.bottom() + spacing + theme.typography.font_size_base,
                ),
            };

            context.draw_text(
                &self.text,
                Point::new(text_x, text_y),
                theme.typography.font_size_base,
            );
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    if *pressed {
                        self.pressed = true;
                    } else if self.pressed {
                        self.pressed = false;
                        if !self.checked {
                            self.set_checked(true);
                        }
                    }
                    return EventResult::Consumed;
                } else {
                    self.pressed = false;
                }
            }
            Event::MouseMove(move_event) => {
                let was_hover = self.hover;
                self.hover = self.state.bounds.contains(move_event.position);
                if was_hover != self.hover {
                    return EventResult::Consumed;
                }
            }
            Event::Update => {
                // Update animation
                self.update_animation(0.016); // Assume 60 FPS

                // Check if we're still selected in the group
                let selected_id = RadioGroupManager::instance()
                    .lock()
                    .unwrap()
                    .get_selected(self.group_id);
                let should_be_checked = selected_id == Some(self.state.id);
                if self.checked != should_be_checked {
                    self.checked = should_be_checked;
                    if let Some(callback) = &mut self.on_change {
                        callback(self.checked);
                    }
                }

                if self.animation_progress > 0.0 && self.animation_progress < 1.0 {
                    return EventResult::Consumed;
                }
            }
            _ => {}
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

// Radio Group Manager - Singleton to manage radio button groups
pub struct RadioGroupManager {
    groups: HashMap<u32, Option<WidgetId>>,
    button_groups: HashMap<WidgetId, u32>,
}

impl RadioGroupManager {
    fn instance() -> Arc<Mutex<Self>> {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<Arc<Mutex<RadioGroupManager>>> = OnceLock::new();

        INSTANCE
            .get_or_init(|| {
                Arc::new(Mutex::new(RadioGroupManager {
                    groups: HashMap::new(),
                    button_groups: HashMap::new(),
                }))
            })
            .clone()
    }

    fn register_button(&mut self, button_id: WidgetId, group_id: u32) {
        self.button_groups.insert(button_id, group_id);
        if !self.groups.contains_key(&group_id) {
            self.groups.insert(group_id, None);
        }
    }

    fn set_selected(&mut self, group_id: u32, button_id: WidgetId) {
        self.groups.insert(group_id, Some(button_id));
    }

    fn get_selected(&self, group_id: u32) -> Option<WidgetId> {
        self.groups.get(&group_id).copied().flatten()
    }
}

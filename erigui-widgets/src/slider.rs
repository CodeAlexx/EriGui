use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

pub struct Slider {
    state: WidgetState,
    min_value: f32,
    max_value: f32,
    current_value: f32,
    orientation: SliderOrientation,
    is_dragging: bool,
    is_hovering: bool,
    thumb_size: i32,
    track_thickness: i32,
    on_value_changed: Option<Box<dyn Fn(f32)>>,
}

impl Slider {
    pub fn new(id: WidgetId, min: f32, max: f32, initial: f32) -> Self {
        Self {
            state: WidgetState::new(id),
            min_value: min,
            max_value: max,
            current_value: initial.clamp(min, max),
            orientation: SliderOrientation::Horizontal,
            is_dragging: false,
            is_hovering: false,
            thumb_size: 20,
            track_thickness: 6,
            on_value_changed: None,
        }
    }

    pub fn with_orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
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
            if let Some(handler) = &self.on_value_changed {
                handler(self.current_value);
            }
        }
    }

    pub fn normalized_value(&self) -> f32 {
        let range = self.max_value - self.min_value;
        if range.abs() < f32::EPSILON {
            return 0.0;
        }
        (self.current_value - self.min_value) / range
    }

    pub fn orientation(&self) -> SliderOrientation {
        self.orientation
    }

    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    fn get_track_rect(&self) -> Rect {
        match self.orientation {
            SliderOrientation::Horizontal => {
                let track_y = self.state.bounds.center().y - self.track_thickness / 2;
                Rect::new(
                    self.state.bounds.x() + self.thumb_size / 2,
                    track_y,
                    self.state.bounds.width() - self.thumb_size,
                    self.track_thickness,
                )
            }
            SliderOrientation::Vertical => {
                let track_x = self.state.bounds.center().x - self.track_thickness / 2;
                Rect::new(
                    track_x,
                    self.state.bounds.y() + self.thumb_size / 2,
                    self.track_thickness,
                    self.state.bounds.height() - self.thumb_size,
                )
            }
        }
    }

    fn get_thumb_rect(&self) -> Rect {
        let track = self.get_track_rect();
        let normalized = self.normalized_value();

        match self.orientation {
            SliderOrientation::Horizontal => {
                let thumb_x =
                    track.x() + (track.width() as f32 * normalized) as i32 - self.thumb_size / 2;
                let thumb_y = self.state.bounds.center().y - self.thumb_size / 2;
                Rect::new(thumb_x, thumb_y, self.thumb_size, self.thumb_size)
            }
            SliderOrientation::Vertical => {
                let thumb_x = self.state.bounds.center().x - self.thumb_size / 2;
                // Invert for vertical: bottom = min, top = max
                let thumb_y = track.bottom()
                    - (track.height() as f32 * normalized) as i32
                    - self.thumb_size / 2;
                Rect::new(thumb_x, thumb_y, self.thumb_size, self.thumb_size)
            }
        }
    }

    fn value_from_position(&self, pos: Point) -> f32 {
        let track = self.get_track_rect();

        let ratio = match self.orientation {
            SliderOrientation::Horizontal => {
                if track.width() <= 0 {
                    0.0
                } else {
                    ((pos.x - track.x()) as f32 / track.width() as f32).clamp(0.0, 1.0)
                }
            }
            SliderOrientation::Vertical => {
                if track.height() <= 0 {
                    0.0
                } else {
                    // Invert for vertical
                    1.0 - ((pos.y - track.y()) as f32 / track.height() as f32).clamp(0.0, 1.0)
                }
            }
        };

        self.min_value + (self.max_value - self.min_value) * ratio
    }
}

impl Widget for Slider {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        match self.orientation {
            SliderOrientation::Horizontal => Size::new(200, self.thumb_size + 4),
            SliderOrientation::Vertical => Size::new(self.thumb_size + 4, 200),
        }
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
        // Auto-detect orientation based on dimensions if not explicitly set
        if rect.width() > rect.height() * 2 {
            self.orientation = SliderOrientation::Horizontal;
        } else if rect.height() > rect.width() * 2 {
            self.orientation = SliderOrientation::Vertical;
        }
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let track_rect = self.get_track_rect();
        let thumb_rect = self.get_thumb_rect();

        // Draw track background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(track_rect);

        // Draw filled portion of track
        let filled_rect = match self.orientation {
            SliderOrientation::Horizontal => Rect::new(
                track_rect.x(),
                track_rect.y(),
                (thumb_rect.center().x - track_rect.x()).max(0),
                track_rect.height(),
            ),
            SliderOrientation::Vertical => {
                let filled_height = (track_rect.bottom() - thumb_rect.center().y).max(0);
                Rect::new(
                    track_rect.x(),
                    thumb_rect.center().y,
                    track_rect.width(),
                    filled_height,
                )
            }
        };

        context.set_color(theme.colors.primary);
        context.fill_rect(filled_rect);

        // Draw track border
        context.set_color(theme.colors.border);
        context.draw_rect(track_rect);

        // Draw thumb
        let thumb_color = if !self.state.enabled {
            theme.colors.surface_variant
        } else if self.is_dragging {
            theme.colors.primary_active
        } else if self.is_hovering {
            theme.colors.primary_hover
        } else {
            theme.colors.primary
        };

        context.set_color(thumb_color);
        context.fill_rect(thumb_rect);

        // Draw thumb border
        context.set_color(theme.colors.border);
        context.draw_rect(thumb_rect);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                let thumb_rect = self.get_thumb_rect();
                self.is_hovering = thumb_rect.contains(mouse_event.position);

                if self.is_dragging {
                    self.set_value(self.value_from_position(mouse_event.position));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    if mouse_event.pressed {
                        let thumb_rect = self.get_thumb_rect();
                        let track_rect = self.get_track_rect();

                        if thumb_rect.contains(mouse_event.position) {
                            self.is_dragging = true;
                            EventResult::Consumed
                        } else if track_rect.contains(mouse_event.position) {
                            // Click on track - jump to position
                            self.set_value(self.value_from_position(mouse_event.position));
                            self.is_dragging = true;
                            EventResult::Consumed
                        } else {
                            EventResult::Ignored
                        }
                    } else if self.is_dragging {
                        self.is_dragging = false;
                        EventResult::Consumed
                    } else {
                        EventResult::Ignored
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
            self.is_dragging = false;
            self.is_hovering = false;
        }
    }

    fn is_focused(&self) -> bool {
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

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

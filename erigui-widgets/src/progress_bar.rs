use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, Point, Rect, Size, Theme, Widget,
    WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressBarStyle {
    Solid,
    Striped,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressBarOrientation {
    Horizontal,
    Vertical,
}

pub struct ProgressBar {
    state: WidgetState,
    min_value: f32,
    max_value: f32,
    current_value: f32,
    orientation: ProgressBarOrientation,
    style: ProgressBarStyle,
    show_text: bool,
    show_percentage: bool,
    animation_offset: i32,
    custom_color: Option<Color>,
}

impl ProgressBar {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            min_value: 0.0,
            max_value: 100.0,
            current_value: 0.0,
            orientation: ProgressBarOrientation::Horizontal,
            style: ProgressBarStyle::Solid,
            show_text: true,
            show_percentage: true,
            animation_offset: 0,
            custom_color: None,
        }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.current_value = value.clamp(self.min_value, self.max_value);
        self
    }

    pub fn with_style(mut self, style: ProgressBarStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_orientation(mut self, orientation: ProgressBarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_text_visible(mut self, show: bool) -> Self {
        self.show_text = show;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.custom_color = Some(color);
        self
    }

    pub fn value(&self) -> f32 {
        self.current_value
    }

    pub fn set_value(&mut self, value: f32) {
        self.current_value = value.clamp(self.min_value, self.max_value);
    }

    pub fn percentage(&self) -> f32 {
        let range = self.max_value - self.min_value;
        if range.abs() < f32::EPSILON {
            return 100.0;
        }
        ((self.current_value - self.min_value) / range * 100.0).clamp(0.0, 100.0)
    }

    pub fn normalized_value(&self) -> f32 {
        let range = self.max_value - self.min_value;
        if range.abs() < f32::EPSILON {
            return 1.0;
        }
        ((self.current_value - self.min_value) / range).clamp(0.0, 1.0)
    }

    pub fn animate(&mut self) {
        if self.style == ProgressBarStyle::Striped {
            self.animation_offset = (self.animation_offset + 1) % 20;
        }
    }

    fn draw_stripes(&self, context: &mut dyn DrawContext, rect: Rect, color: Color) {
        context.set_color(color);
        context.fill_rect(rect);

        // Draw diagonal stripes
        let stripe_color = Color::rgba(255, 255, 255, 30); // Semi-transparent white
        context.set_color(stripe_color);

        let stripe_width = 20;
        let total_stripes = (rect.width() + rect.height()) / stripe_width + 2;

        for i in 0..total_stripes {
            let offset = i * stripe_width - self.animation_offset;

            // Draw diagonal lines using filled rectangles
            for j in 0..stripe_width / 2 {
                let x = rect.x() + offset + j;
                if x >= rect.x() && x < rect.right() {
                    let y_start = rect.y().max(rect.y() + offset + j - x + rect.x());
                    let y_end = rect
                        .bottom()
                        .min(rect.y() + offset + j + rect.height() - x + rect.x());

                    if y_end > y_start {
                        context.draw_line(Point::new(x, y_start), Point::new(x, y_end), 1);
                    }
                }
            }
        }
    }
}

impl Widget for ProgressBar {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        match self.orientation {
            ProgressBarOrientation::Horizontal => {
                Size::new(200, theme.typography.font_size_base + 8)
            }
            ProgressBarOrientation::Vertical => Size::new(theme.typography.font_size_base + 8, 200),
        }
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
        // Auto-detect orientation based on dimensions
        if rect.width() > rect.height() * 2 {
            self.orientation = ProgressBarOrientation::Horizontal;
        } else if rect.height() > rect.width() * 2 {
            self.orientation = ProgressBarOrientation::Vertical;
        }
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.state.bounds);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);

        // Calculate filled area
        let normalized = self.normalized_value();
        let filled_rect = match self.orientation {
            ProgressBarOrientation::Horizontal => {
                let filled_width = (self.state.bounds.width() as f32 * normalized) as i32;
                Rect::new(
                    self.state.bounds.x(),
                    self.state.bounds.y(),
                    filled_width,
                    self.state.bounds.height(),
                )
            }
            ProgressBarOrientation::Vertical => {
                let filled_height = (self.state.bounds.height() as f32 * normalized) as i32;
                Rect::new(
                    self.state.bounds.x(),
                    self.state.bounds.bottom() - filled_height,
                    self.state.bounds.width(),
                    filled_height,
                )
            }
        };

        // Draw filled area
        let fill_color = self.custom_color.unwrap_or(theme.colors.primary);

        match self.style {
            ProgressBarStyle::Solid => {
                context.set_color(fill_color);
                context.fill_rect(filled_rect);
            }
            ProgressBarStyle::Striped => {
                self.draw_stripes(context, filled_rect, fill_color);
            }
        }

        // Draw text if enabled
        if self.show_text {
            let text = if self.show_percentage {
                format!("{:.0}%", self.percentage())
            } else {
                format!("{:.1}/{:.1}", self.current_value, self.max_value)
            };

            let text_size = context.measure_text(&text, theme.typography.font_size_base);
            let text_pos = Point::new(
                self.state.bounds.center().x - text_size.width / 2,
                self.state.bounds.center().y - text_size.height / 2,
            );

            // Draw text with contrasting color
            let text_color = if normalized > 0.5 {
                theme.colors.background
            } else {
                theme.colors.text
            };

            context.set_color(text_color);
            context.draw_text(&text, text_pos, theme.typography.font_size_base);
        }
    }

    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
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
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn can_focus(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

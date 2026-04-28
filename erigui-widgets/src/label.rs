use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, Point, Rect, Size, Theme, Widget,
    WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

pub struct Label {
    state: WidgetState,
    text: String,
    color: Option<Color>,
    align: TextAlign,
}

impl Label {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            state: WidgetState::new(id),
            text: text.into(),
            color: None,
            align: TextAlign::Left,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    pub fn set_align(&mut self, align: TextAlign) {
        self.align = align;
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        Size::new(
            self.text.len() as i32 * theme.typography.font_size_base / 2,
            theme.typography.font_size_base,
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let color = self.color.unwrap_or({
            if self.state.enabled {
                theme.colors.text
            } else {
                theme.colors.text_disabled
            }
        });

        context.set_color(color);

        let text_size = context.measure_text(&self.text, theme.typography.font_size_base);
        let y_pos = self.state.bounds.center().y - text_size.height / 2;

        let x_pos = match self.align {
            TextAlign::Left => self.state.bounds.x(),
            TextAlign::Center => self.state.bounds.center().x - text_size.width / 2,
            TextAlign::Right => self.state.bounds.right() - text_size.width,
        };

        context.draw_text(
            &self.text,
            Point::new(x_pos, y_pos),
            theme.typography.font_size_base,
        );
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

use crate::{Event, LayoutConstraints, Point, Rect, Size, Theme, WidgetId};
use std::any::Any;

pub trait Widget: Any {
    fn id(&self) -> WidgetId;

    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size;

    fn layout(&mut self, rect: Rect, theme: &Theme);

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme);

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult;

    fn children(&self) -> &[WidgetId] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [WidgetId] {
        &mut []
    }

    fn hit_test(&self, point: Point) -> Option<WidgetId> {
        if self.bounds().contains(point) {
            Some(self.id())
        } else {
            None
        }
    }

    fn bounds(&self) -> Rect;

    fn set_bounds(&mut self, bounds: Rect);

    fn is_visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, visible: bool);

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, enabled: bool);

    fn is_focused(&self) -> bool {
        false
    }

    fn set_focused(&mut self, focused: bool);

    fn can_focus(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Ignored,
    Consumed,
}

impl EventResult {
    pub fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }

    pub fn is_ignored(&self) -> bool {
        matches!(self, Self::Ignored)
    }
}

pub trait DrawContext {
    fn set_color(&mut self, color: crate::Color);

    fn draw_rect(&mut self, rect: Rect);

    fn fill_rect(&mut self, rect: Rect);

    fn draw_circle(&mut self, center: Point, radius: i32, segments: i32);

    fn fill_circle(&mut self, center: Point, radius: i32, segments: i32);

    fn draw_line(&mut self, start: Point, end: Point, thickness: i32);

    fn draw_text(&mut self, text: &str, position: Point, size: i32);

    fn measure_text(&self, text: &str, size: i32) -> Size;

    fn push_clip_rect(&mut self, rect: Rect);

    fn pop_clip_rect(&mut self);

    fn viewport_size(&self) -> Size;

    // Advanced rendering features
    fn draw_rounded_rect(&mut self, rect: Rect, corner_radius: i32);

    fn fill_rounded_rect(&mut self, rect: Rect, corner_radius: i32);

    fn draw_gradient_rect(
        &mut self,
        rect: Rect,
        top_color: crate::Color,
        bottom_color: crate::Color,
        horizontal: bool,
    );

    fn draw_shadow(
        &mut self,
        rect: Rect,
        shadow_color: crate::Color,
        blur_radius: i32,
        offset: Point,
    );

    fn draw_ellipse(&mut self, center: Point, radius_x: i32, radius_y: i32, segments: i32);

    fn fill_ellipse(&mut self, center: Point, radius_x: i32, radius_y: i32, segments: i32);

    fn draw_polygon(&mut self, points: &[Point]);

    fn fill_polygon(&mut self, points: &[Point]);

    fn set_line_width(&mut self, width: i32);

    /// Draw an RGBA image into the given rect. Pixel data is row-major RGBA8.
    fn draw_image_rgba(&mut self, rect: Rect, width: i32, height: i32, data: &[u8]);
}

pub struct WidgetState {
    pub id: WidgetId,
    pub bounds: Rect,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
    pub tooltip: Option<String>,
}

impl WidgetState {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            bounds: Rect::default(),
            visible: true,
            enabled: true,
            focused: false,
            tooltip: None,
        }
    }
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            id: WidgetId::default(),
            bounds: Rect::default(),
            visible: true,
            enabled: true,
            focused: false,
            tooltip: None,
        }
    }
}

use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseWheelEvent, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct ScrollView {
    state: WidgetState,
    child: Option<WidgetId>,
    scroll_offset: Point,
    content_size: Size,
    show_horizontal_scrollbar: bool,
    show_vertical_scrollbar: bool,
    scrollbar_width: i32,
}

impl ScrollView {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            child: None,
            scroll_offset: Point::ZERO,
            content_size: Size::ZERO,
            show_horizontal_scrollbar: true,
            show_vertical_scrollbar: true,
            scrollbar_width: 12,
        }
    }

    pub fn with_child(mut self, child: WidgetId) -> Self {
        self.child = Some(child);
        self
    }

    pub fn set_child(&mut self, child: Option<WidgetId>) {
        self.child = child;
    }

    pub fn set_scroll_offset(&mut self, offset: Point) {
        self.scroll_offset = offset;
        self.clamp_scroll_offset();
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset.y = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        let max_y = (self.content_size.height - self.viewport_size().height).max(0);
        self.scroll_offset.y = max_y;
    }

    fn viewport_size(&self) -> Size {
        let mut size = self.state.bounds.size;

        if self.show_vertical_scrollbar && self.needs_vertical_scrollbar() {
            size.width -= self.scrollbar_width;
        }

        if self.show_horizontal_scrollbar && self.needs_horizontal_scrollbar() {
            size.height -= self.scrollbar_width;
        }

        size
    }

    fn needs_horizontal_scrollbar(&self) -> bool {
        self.content_size.width > self.state.bounds.width()
    }

    fn needs_vertical_scrollbar(&self) -> bool {
        self.content_size.height > self.state.bounds.height()
    }

    fn clamp_scroll_offset(&mut self) {
        let viewport = self.viewport_size();

        self.scroll_offset.x = self
            .scroll_offset
            .x
            .max(0)
            .min((self.content_size.width - viewport.width).max(0));

        self.scroll_offset.y = self
            .scroll_offset
            .y
            .max(0)
            .min((self.content_size.height - viewport.height).max(0));
    }

    fn vertical_scrollbar_rect(&self) -> Rect {
        let viewport = self.viewport_size();
        Rect::new(
            self.state.bounds.right() - self.scrollbar_width,
            self.state.bounds.y(),
            self.scrollbar_width,
            viewport.height,
        )
    }

    fn horizontal_scrollbar_rect(&self) -> Rect {
        let viewport = self.viewport_size();
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.bottom() - self.scrollbar_width,
            viewport.width,
            self.scrollbar_width,
        )
    }

    fn vertical_thumb_rect(&self) -> Rect {
        let scrollbar = self.vertical_scrollbar_rect();
        let viewport = self.viewport_size();

        let thumb_height = ((viewport.height as f32 / self.content_size.height as f32)
            * scrollbar.height() as f32) as i32;
        let thumb_height = thumb_height.max(20);

        let thumb_y = ((self.scroll_offset.y as f32 / self.content_size.height as f32)
            * (scrollbar.height() - thumb_height) as f32) as i32;

        Rect::new(
            scrollbar.x() + 2,
            scrollbar.y() + thumb_y,
            scrollbar.width() - 4,
            thumb_height,
        )
    }

    fn horizontal_thumb_rect(&self) -> Rect {
        let scrollbar = self.horizontal_scrollbar_rect();
        let viewport = self.viewport_size();

        let thumb_width = ((viewport.width as f32 / self.content_size.width as f32)
            * scrollbar.width() as f32) as i32;
        let thumb_width = thumb_width.max(20);

        let thumb_x = ((self.scroll_offset.x as f32 / self.content_size.width as f32)
            * (scrollbar.width() - thumb_width) as f32) as i32;

        Rect::new(
            scrollbar.x() + thumb_x,
            scrollbar.y() + 2,
            thumb_width,
            scrollbar.height() - 4,
        )
    }
}

impl Widget for ScrollView {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(
            constraints.max_width.unwrap_or(400),
            constraints.max_height.unwrap_or(300),
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;

        // Layout child with unbounded constraints to get content size
        if let Some(_child_id) = self.child {
            // In a real implementation, we'd need access to the widget manager here
            // to layout the child widget
            self.content_size = Size::new(800, 1200); // Placeholder
        }

        self.clamp_scroll_offset();
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw content viewport
        let viewport = Rect::from_origin_size(self.state.bounds.origin, self.viewport_size());

        context.push_clip_rect(viewport);

        // In a real implementation, we'd translate the drawing context by -scroll_offset
        // and draw the child widget

        context.pop_clip_rect();

        // Draw scrollbars
        if self.show_vertical_scrollbar && self.needs_vertical_scrollbar() {
            // Scrollbar track
            context.set_color(theme.colors.surface_variant);
            context.fill_rect(self.vertical_scrollbar_rect());

            // Scrollbar thumb
            context.set_color(theme.colors.border);
            context.fill_rect(self.vertical_thumb_rect());
        }

        if self.show_horizontal_scrollbar && self.needs_horizontal_scrollbar() {
            // Scrollbar track
            context.set_color(theme.colors.surface_variant);
            context.fill_rect(self.horizontal_scrollbar_rect());

            // Scrollbar thumb
            context.set_color(theme.colors.border);
            context.fill_rect(self.horizontal_thumb_rect());
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        if let Event::MouseWheel(MouseWheelEvent {
                delta, position, ..
            }) = event {
            if self.state.bounds.contains(*position) {
                self.scroll_offset.x -= delta.x * 20;
                self.scroll_offset.y -= delta.y * 20;
                self.clamp_scroll_offset();
                return EventResult::Consumed;
            }
        }

        EventResult::Ignored
    }

    fn children(&self) -> &[WidgetId] {
        if let Some(ref child) = self.child {
            std::slice::from_ref(child)
        } else {
            &[]
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

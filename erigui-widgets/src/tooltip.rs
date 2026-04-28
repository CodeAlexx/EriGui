use erigui_core::{
    Color, DrawContext, Event, MouseButtonEvent, Point, Rect, Size, Theme, WidgetId,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
    Auto,
}

/// Per-widget tooltip helper. Widgets compose this and call
/// `update_on_event` from their own `handle_event`, then `draw`
/// after their own draw. This replaces the global `TooltipManager`
/// singleton — every widget owns its tooltip directly.
///
/// Hover behaviour: on `Event::MouseMove`, hover-start records when
/// the cursor first entered `widget_bounds`. Once `delay_ms` elapses
/// while still over the widget, `visible` flips true. Any mouse press
/// hides the tooltip; moving outside the bounds clears the timer.
pub struct TooltipState {
    text: String,
    delay_ms: u32,
    position: TooltipPosition,
    hover_start: Option<Instant>,
    visible: bool,
    offset: i32,
    padding: i32,
    max_width: i32,
}

impl TooltipState {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            delay_ms: 500,
            position: TooltipPosition::Auto,
            hover_start: None,
            visible: false,
            offset: 4,
            padding: 8,
            max_width: 200,
        }
    }

    pub fn with_delay_ms(mut self, ms: u32) -> Self {
        self.delay_ms = ms;
        self
    }

    pub fn with_position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Clear hover timing and visibility. Call from a widget's
    /// `set_visible(false)` / `set_enabled(false)` if it wants the
    /// tooltip suppressed when the host hides the widget.
    pub fn hide(&mut self) {
        self.hover_start = None;
        self.visible = false;
    }

    /// Drive hover/visibility from a Widget's normal event stream.
    /// Pass the widget's own bounds — the helper tests
    /// `bounds.contains(position)` itself, no per-frame poll needed.
    pub fn update_on_event(&mut self, event: &Event, widget_bounds: Rect) {
        match event {
            Event::MouseMove(ev) => {
                if widget_bounds.contains(ev.position) {
                    // Started hovering. Latch the start instant so the
                    // delay countdown is computed in `draw`/checks.
                    if self.hover_start.is_none() {
                        self.hover_start = Some(Instant::now());
                    }
                    // Promote to visible once delay has elapsed. Doing
                    // this here (not just in draw) means `is_visible`
                    // returns the right answer mid-frame.
                    if let Some(start) = self.hover_start {
                        if Instant::now().duration_since(start)
                            >= Duration::from_millis(self.delay_ms as u64)
                        {
                            self.visible = true;
                        }
                    }
                } else {
                    // Mouse left bounds — kill any pending show. Don't
                    // wait for a click to dismiss.
                    self.hide();
                }
            }
            // Any mouse button press dismisses — matches typical OS
            // tooltip semantics. Press happens before the click handler
            // runs, which is fine: tooltip should disappear on press.
            Event::MouseButton(MouseButtonEvent { pressed: true, .. }) => {
                self.hide();
            }
            _ => {}
        }
    }

    fn calculate_position(
        &self,
        anchor: Rect,
        tooltip_size: Size,
        viewport_size: Size,
    ) -> Point {
        let mut final_position = self.position;

        if final_position == TooltipPosition::Auto {
            if anchor.y() - tooltip_size.height - self.offset >= 0 {
                final_position = TooltipPosition::Above;
            } else if anchor.bottom() + tooltip_size.height + self.offset
                <= viewport_size.height
            {
                final_position = TooltipPosition::Below;
            } else if anchor.right() + tooltip_size.width + self.offset
                <= viewport_size.width
            {
                final_position = TooltipPosition::Right;
            } else if anchor.x() - tooltip_size.width - self.offset >= 0 {
                final_position = TooltipPosition::Left;
            } else {
                final_position = TooltipPosition::Above;
            }
        }

        let pos = match final_position {
            TooltipPosition::Above | TooltipPosition::Auto => Point::new(
                anchor.x() + (anchor.width() - tooltip_size.width) / 2,
                anchor.y() - tooltip_size.height - self.offset,
            ),
            TooltipPosition::Below => Point::new(
                anchor.x() + (anchor.width() - tooltip_size.width) / 2,
                anchor.bottom() + self.offset,
            ),
            TooltipPosition::Left => Point::new(
                anchor.x() - tooltip_size.width - self.offset,
                anchor.y() + (anchor.height() - tooltip_size.height) / 2,
            ),
            TooltipPosition::Right => Point::new(
                anchor.right() + self.offset,
                anchor.y() + (anchor.height() - tooltip_size.height) / 2,
            ),
        };

        // Clamp into the viewport so off-screen anchor rects still draw.
        Point::new(
            pos.x.max(0).min(viewport_size.width - tooltip_size.width),
            pos.y.max(0).min(viewport_size.height - tooltip_size.height),
        )
    }

    /// Paint the tooltip if currently visible. `anchor` is the widget's
    /// bounds — positioning is computed from it. Call after the widget
    /// has drawn itself so the tooltip renders on top.
    pub fn draw(&self, context: &mut dyn DrawContext, theme: &Theme, anchor: Rect) {
        if !self.visible || self.text.is_empty() {
            return;
        }

        let text_width = self.text.len() as i32 * theme.typography.font_size_base * 3 / 5;
        let text_height = theme.typography.font_size_base;

        let tooltip_width = (text_width + self.padding * 2).min(self.max_width);
        let tooltip_height = text_height + self.padding * 2;
        let tooltip_size = Size::new(tooltip_width, tooltip_height);

        let viewport_size = context.viewport_size();
        let position = self.calculate_position(anchor, tooltip_size, viewport_size);

        let tooltip_rect = Rect::new(position.x, position.y, tooltip_width, tooltip_height);

        // Shadow.
        context.set_color(Color::rgba(0, 0, 0, 64));
        context.fill_rect(Rect::new(
            tooltip_rect.x() + 2,
            tooltip_rect.y() + 2,
            tooltip_rect.width(),
            tooltip_rect.height(),
        ));

        // Background.
        context.set_color(Color::rgba(48, 48, 48, 240));
        context.fill_rect(tooltip_rect);

        // Border.
        context.set_color(theme.colors.border);
        context.draw_rect(tooltip_rect);

        // Text.
        context.set_color(Color::WHITE);
        context.draw_text(
            &self.text,
            Point::new(
                tooltip_rect.x() + self.padding,
                tooltip_rect.center().y + theme.typography.font_size_base / 2 - 2,
            ),
            theme.typography.font_size_base,
        );
    }
}

pub struct Tooltip {
    _id: WidgetId,
    text: String,
    target_bounds: Rect,
    position: TooltipPosition,
    delay_ms: u32,
    visible: bool,
    hover_start: Option<Instant>,
    offset: i32,
    padding: i32,
    max_width: i32,
}

impl Tooltip {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            _id: id,
            text: text.into(),
            target_bounds: Rect::default(),
            position: TooltipPosition::Auto,
            delay_ms: 500,
            visible: false,
            hover_start: None,
            offset: 4,
            padding: 8,
            max_width: 200,
        }
    }

    pub fn with_position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn set_target_bounds(&mut self, bounds: Rect) {
        self.target_bounds = bounds;
    }

    fn calculate_position(
        &self,
        tooltip_size: Size,
        viewport_size: Size,
    ) -> (Point, TooltipPosition) {
        let mut final_position = self.position;

        if final_position == TooltipPosition::Auto {
            // Try above first
            if self.target_bounds.y() - tooltip_size.height - self.offset >= 0 {
                final_position = TooltipPosition::Above;
            }
            // Try below
            else if self.target_bounds.bottom() + tooltip_size.height + self.offset
                <= viewport_size.height
            {
                final_position = TooltipPosition::Below;
            }
            // Try right
            else if self.target_bounds.right() + tooltip_size.width + self.offset
                <= viewport_size.width
            {
                final_position = TooltipPosition::Right;
            }
            // Try left
            else if self.target_bounds.x() - tooltip_size.width - self.offset >= 0 {
                final_position = TooltipPosition::Left;
            }
            // Default to above
            else {
                final_position = TooltipPosition::Above;
            }
        }

        let pos = match final_position {
            TooltipPosition::Above | TooltipPosition::Auto => Point::new(
                self.target_bounds.x() + (self.target_bounds.width() - tooltip_size.width) / 2,
                self.target_bounds.y() - tooltip_size.height - self.offset,
            ),
            TooltipPosition::Below => Point::new(
                self.target_bounds.x() + (self.target_bounds.width() - tooltip_size.width) / 2,
                self.target_bounds.bottom() + self.offset,
            ),
            TooltipPosition::Left => Point::new(
                self.target_bounds.x() - tooltip_size.width - self.offset,
                self.target_bounds.y() + (self.target_bounds.height() - tooltip_size.height) / 2,
            ),
            TooltipPosition::Right => Point::new(
                self.target_bounds.right() + self.offset,
                self.target_bounds.y() + (self.target_bounds.height() - tooltip_size.height) / 2,
            ),
        };

        // Clamp to viewport
        let clamped_pos = Point::new(
            pos.x.max(0).min(viewport_size.width - tooltip_size.width),
            pos.y.max(0).min(viewport_size.height - tooltip_size.height),
        );

        (clamped_pos, final_position)
    }

    pub fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.visible || self.text.is_empty() {
            return;
        }

        // Calculate text size
        let text_width = self.text.len() as i32 * theme.typography.font_size_base * 3 / 5;
        let text_height = theme.typography.font_size_base;

        // Calculate tooltip size
        let tooltip_width = (text_width + self.padding * 2).min(self.max_width);
        let tooltip_height = text_height + self.padding * 2;
        let tooltip_size = Size::new(tooltip_width, tooltip_height);

        let viewport_size = context.viewport_size();
        let (position, _) = self.calculate_position(tooltip_size, viewport_size);

        let tooltip_rect = Rect::new(position.x, position.y, tooltip_width, tooltip_height);

        // Draw shadow
        context.set_color(Color::rgba(0, 0, 0, 64));
        context.fill_rect(Rect::new(
            tooltip_rect.x() + 2,
            tooltip_rect.y() + 2,
            tooltip_rect.width(),
            tooltip_rect.height(),
        ));

        // Draw background
        context.set_color(Color::rgba(48, 48, 48, 240));
        context.fill_rect(tooltip_rect);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(tooltip_rect);

        // Draw text
        context.set_color(Color::WHITE);
        context.draw_text(
            &self.text,
            Point::new(
                tooltip_rect.x() + self.padding,
                tooltip_rect.center().y + theme.typography.font_size_base / 2 - 2,
            ),
            theme.typography.font_size_base,
        );
    }
}

// Global tooltip manager
pub struct TooltipManager {
    tooltips: HashMap<WidgetId, Tooltip>,
    active_widget: Option<WidgetId>,
    mouse_position: Point,
    last_update: Instant,
}

impl Default for TooltipManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TooltipManager {
    pub fn new() -> Self {
        Self {
            tooltips: HashMap::new(),
            active_widget: None,
            mouse_position: Point::ZERO,
            last_update: Instant::now(),
        }
    }

    pub fn register_tooltip(&mut self, widget_id: WidgetId, tooltip: Tooltip) {
        self.tooltips.insert(widget_id, tooltip);
    }

    pub fn unregister_tooltip(&mut self, widget_id: WidgetId) {
        self.tooltips.remove(&widget_id);
        if self.active_widget == Some(widget_id) {
            self.active_widget = None;
        }
    }

    pub fn set_tooltip_text(&mut self, widget_id: WidgetId, text: impl Into<String>) {
        if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
            tooltip.set_text(text);
        }
    }

    pub fn update(
        &mut self,
        mouse_position: Point,
        hovered_widget: Option<WidgetId>,
        widget_bounds: Option<Rect>,
    ) {
        self.mouse_position = mouse_position;
        let now = Instant::now();

        // Check if we're hovering over a new widget
        if hovered_widget != self.active_widget {
            // Hide current tooltip
            if let Some(current_id) = self.active_widget {
                if let Some(tooltip) = self.tooltips.get_mut(&current_id) {
                    tooltip.visible = false;
                    tooltip.hover_start = None;
                }
            }

            // Start tracking new widget
            self.active_widget = hovered_widget;
            if let Some(widget_id) = hovered_widget {
                if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
                    tooltip.hover_start = Some(now);
                    if let Some(bounds) = widget_bounds {
                        tooltip.set_target_bounds(bounds);
                    }
                }
            }
        }

        // Update visibility based on hover duration
        if let Some(widget_id) = self.active_widget {
            if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
                if let Some(hover_start) = tooltip.hover_start {
                    let elapsed = now.duration_since(hover_start);
                    if elapsed >= Duration::from_millis(tooltip.delay_ms as u64) {
                        tooltip.visible = true;
                    }
                }
            }
        }

        self.last_update = now;
    }

    pub fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if let Some(widget_id) = self.active_widget {
            if let Some(tooltip) = self.tooltips.get(&widget_id) {
                tooltip.draw(context, theme);
            }
        }
    }

    pub fn hide_all(&mut self) {
        for tooltip in self.tooltips.values_mut() {
            tooltip.visible = false;
            tooltip.hover_start = None;
        }
        self.active_widget = None;
    }
}

// Singleton instance
use std::sync::{Mutex, OnceLock};

static TOOLTIP_MANAGER: OnceLock<Mutex<TooltipManager>> = OnceLock::new();

pub fn tooltip_manager() -> std::sync::MutexGuard<'static, TooltipManager> {
    TOOLTIP_MANAGER
        .get_or_init(|| Mutex::new(TooltipManager::new()))
        .lock()
        .unwrap()
}

// Helper trait to add tooltip support to widgets
pub trait TooltipExt {
    fn set_tooltip(&mut self, text: impl Into<String>);
    fn clear_tooltip(&mut self);
}

// Widget extension to support tooltips
pub struct TooltipWidget {
    pub widget_id: WidgetId,
    pub tooltip_text: Option<String>,
}

impl TooltipWidget {
    pub fn new(widget_id: WidgetId) -> Self {
        Self {
            widget_id,
            tooltip_text: None,
        }
    }

    pub fn set_tooltip(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.tooltip_text = Some(text.clone());
        let tooltip = Tooltip::new(self.widget_id, text);
        tooltip_manager().register_tooltip(self.widget_id, tooltip);
    }

    pub fn clear_tooltip(&mut self) {
        self.tooltip_text = None;
        tooltip_manager().unregister_tooltip(self.widget_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erigui_core::{
        Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point, Rect,
    };
    use std::thread::sleep;
    use std::time::Duration;

    fn move_to(p: Point) -> Event {
        Event::MouseMove(MouseMoveEvent {
            position: p,
            delta: Point::ZERO,
            modifiers: Modifiers::empty(),
        })
    }

    fn left_press(p: Point) -> Event {
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: p,
            pressed: true,
            modifiers: Modifiers::empty(),
        })
    }

    #[test]
    fn tooltip_state_starts_hidden() {
        let s = TooltipState::new("hello");
        assert!(!s.is_visible());
        assert_eq!(s.text(), "hello");
    }

    #[test]
    fn tooltip_state_shows_after_delay_while_hovered() {
        let bounds = Rect::new(0, 0, 100, 30);
        let mut s = TooltipState::new("hi").with_delay_ms(20);

        // Initial hover starts the timer but does not show yet.
        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        assert!(!s.is_visible());

        // Wait past the delay, then a subsequent move-while-hovered
        // promotes to visible. Real apps generate move events
        // continuously so this models reality.
        sleep(Duration::from_millis(30));
        s.update_on_event(&move_to(Point::new(11, 10)), bounds);
        assert!(s.is_visible());
    }

    #[test]
    fn tooltip_state_hides_when_mouse_leaves_bounds() {
        let bounds = Rect::new(0, 0, 100, 30);
        let mut s = TooltipState::new("hi").with_delay_ms(0);

        // Show first.
        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        assert!(s.is_visible());

        // Move outside — must clear visibility and hover-start.
        s.update_on_event(&move_to(Point::new(500, 500)), bounds);
        assert!(!s.is_visible());
    }

    #[test]
    fn tooltip_state_hides_on_mouse_press() {
        let bounds = Rect::new(0, 0, 100, 30);
        let mut s = TooltipState::new("hi").with_delay_ms(0);

        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        assert!(s.is_visible());

        s.update_on_event(&left_press(Point::new(10, 10)), bounds);
        assert!(!s.is_visible());
    }

    #[test]
    fn tooltip_state_explicit_hide_clears_state() {
        let bounds = Rect::new(0, 0, 100, 30);
        let mut s = TooltipState::new("hi").with_delay_ms(0);

        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        assert!(s.is_visible());

        s.hide();
        assert!(!s.is_visible());
    }

    #[test]
    fn tooltip_state_set_text_replaces_content() {
        let mut s = TooltipState::new("first");
        assert_eq!(s.text(), "first");
        s.set_text("second");
        assert_eq!(s.text(), "second");
    }

    #[test]
    fn tooltip_state_with_delay_ms_overrides_default() {
        let s = TooltipState::new("hi").with_delay_ms(123);
        // Indirectly check the delay: zero-bounds means no hover, so
        // we re-create with delay 0 and confirm immediate show.
        // (Not a perfect probe, but we don't expose `delay_ms`.)
        let bounds = Rect::new(0, 0, 10, 10);
        let mut s2 = TooltipState::new("hi").with_delay_ms(0);
        s2.update_on_event(&move_to(Point::new(1, 1)), bounds);
        assert!(s2.is_visible());
        // The other instance with non-zero delay should not show
        // immediately on a single event.
        let mut s3 = s;
        s3.update_on_event(&move_to(Point::new(1, 1)), bounds);
        assert!(!s3.is_visible());
    }

    #[test]
    fn tooltip_state_re_enter_after_leave_restarts_timer() {
        let bounds = Rect::new(0, 0, 100, 30);
        let mut s = TooltipState::new("hi").with_delay_ms(50);

        // Hover, leave before delay, re-enter — must not show
        // instantly because the timer restarts.
        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        s.update_on_event(&move_to(Point::new(500, 500)), bounds);
        s.update_on_event(&move_to(Point::new(10, 10)), bounds);
        assert!(!s.is_visible());
    }
}

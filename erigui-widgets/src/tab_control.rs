use erigui_core::{
    DrawContext, Event, EventResult, Key, LayoutConstraints, Modifiers, MouseButton, Point, Rect,
    Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct TabPage {
    pub title: String,
    pub content: Vec<WidgetId>,
    pub enabled: bool,
    pub closeable: bool,
}

pub struct TabItem {
    pub title: String,
    pub id: String,
}

impl TabItem {
    pub fn new(title: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
        }
    }
}

impl TabPage {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
            enabled: true,
            closeable: false,
        }
    }

    pub fn with_closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    pub fn add_widget(&mut self, widget_id: WidgetId) {
        self.content.push(widget_id);
    }
}

pub struct TabControl {
    state: WidgetState,
    tabs: Vec<TabPage>,
    active_tab: usize,
    hover_tab: Option<usize>,
    hover_close: Option<usize>,
    tab_height: i32,
    min_tab_width: i32,
    max_tab_width: i32,
    on_tab_changed: Option<Box<dyn Fn(usize)>>,
    on_tab_closed: Option<Box<dyn Fn(usize)>>,
}

impl TabControl {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            tabs: Vec::new(),
            active_tab: 0,
            hover_tab: None,
            hover_close: None,
            tab_height: 30,
            min_tab_width: 80,
            max_tab_width: 200,
            on_tab_changed: None,
            on_tab_closed: None,
        }
    }

    pub fn with_on_tab_changed<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize) + 'static,
    {
        self.on_tab_changed = Some(Box::new(handler));
        self
    }

    pub fn with_on_tab_closed<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize) + 'static,
    {
        self.on_tab_closed = Some(Box::new(handler));
        self
    }

    pub fn add_tab(&mut self, tab: TabPage) {
        self.tabs.push(tab);
        if self.tabs.len() == 1 {
            self.active_tab = 0;
        }
    }

    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() && self.tabs[index].enabled {
            self.active_tab = index;
            if let Some(handler) = &self.on_tab_changed {
                handler(index);
            }
        }
    }

    pub fn active_content(&self) -> &[WidgetId] {
        self.tabs
            .get(self.active_tab)
            .map(|tab| tab.content.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_tabs(&mut self, items: Vec<TabItem>) {
        self.tabs.clear();
        for item in items {
            let tab = TabPage::new(item.title);
            // Store the ID in the tab somehow - for now just use title
            self.tabs.push(tab);
        }
        if !self.tabs.is_empty() && self.active_tab >= self.tabs.len() {
            self.active_tab = 0;
        }
    }

    pub fn get_active_tab(&self) -> Option<usize> {
        if self.active_tab < self.tabs.len() {
            Some(self.active_tab)
        } else {
            None
        }
    }

    fn get_tab_rect(&self, index: usize) -> Rect {
        let total_tabs = self.tabs.len() as i32;
        let available_width = self.state.bounds.width();
        let tab_width = (available_width / total_tabs)
            .max(self.min_tab_width)
            .min(self.max_tab_width);

        Rect::new(
            self.state.bounds.x() + index as i32 * tab_width,
            self.state.bounds.y(),
            tab_width,
            self.tab_height,
        )
    }

    fn get_close_button_rect(&self, tab_rect: Rect) -> Rect {
        let size = 16;
        let margin = 4;
        Rect::new(
            tab_rect.right() - size - margin,
            tab_rect.y() + (tab_rect.height() - size) / 2,
            size,
            size,
        )
    }

    fn get_content_rect(&self) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y() + self.tab_height,
            self.state.bounds.width(),
            self.state.bounds.height() - self.tab_height,
        )
    }

    fn tab_from_point(&self, point: Point) -> Option<usize> {
        if point.y < self.state.bounds.y() || point.y > self.state.bounds.y() + self.tab_height {
            return None;
        }

        (0..self.tabs.len()).find(|&i| self.get_tab_rect(i).contains(point))
    }

    /// Find the next enabled tab from `start`, going `forward` or backward.
    /// Wraps around. Skips disabled tabs so Ctrl+Tab can't get stuck.
    /// Returns `None` if no enabled tab exists at all (or only `start`
    /// itself is enabled, in which case there's nowhere new to go).
    fn next_enabled_tab(&self, start: usize, forward: bool) -> Option<usize> {
        let n = self.tabs.len();
        if n <= 1 {
            return None;
        }
        for step in 1..n {
            let i = if forward {
                (start + step) % n
            } else {
                // step < n so (start + n - step) is in [start+1, start+n-1]
                (start + n - step) % n
            };
            if self.tabs[i].enabled {
                return Some(i);
            }
        }
        None
    }

    /// Map `Key::Num1..Key::Num9` to indices 0..8. Returns `None` for any
    /// other key or for digits past the tab count.
    fn tab_index_from_digit(key: &Key) -> Option<usize> {
        match key {
            Key::Num1 => Some(0),
            Key::Num2 => Some(1),
            Key::Num3 => Some(2),
            Key::Num4 => Some(3),
            Key::Num5 => Some(4),
            Key::Num6 => Some(5),
            Key::Num7 => Some(6),
            Key::Num8 => Some(7),
            Key::Num9 => Some(8),
            _ => None,
        }
    }
}

impl Widget for TabControl {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(400, 300)
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw tab bar background
        let tab_bar_rect = Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y(),
            self.state.bounds.width(),
            self.tab_height,
        );
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(tab_bar_rect);

        // Draw tabs
        for (i, tab) in self.tabs.iter().enumerate() {
            let tab_rect = self.get_tab_rect(i);
            let is_active = i == self.active_tab;
            let is_hover = Some(i) == self.hover_tab;

            // Draw tab background
            let tab_color = if is_active {
                theme.colors.surface
            } else if is_hover && tab.enabled {
                theme.colors.primary_hover
            } else {
                theme.colors.surface_variant
            };

            context.set_color(tab_color);
            context.fill_rect(tab_rect);

            // Draw tab border
            if is_active {
                context.set_color(theme.colors.primary);
                // Top border
                context.draw_line(
                    Point::new(tab_rect.x(), tab_rect.y()),
                    Point::new(tab_rect.right(), tab_rect.y()),
                    2,
                );
                // Left border
                context.draw_line(
                    Point::new(tab_rect.x(), tab_rect.y()),
                    Point::new(tab_rect.x(), tab_rect.bottom()),
                    1,
                );
                // Right border
                context.draw_line(
                    Point::new(tab_rect.right() - 1, tab_rect.y()),
                    Point::new(tab_rect.right() - 1, tab_rect.bottom()),
                    1,
                );
            }

            // Draw tab text
            let text_color = if tab.enabled {
                if is_active {
                    theme.colors.text
                } else {
                    theme.colors.text_secondary
                }
            } else {
                theme.colors.text_disabled
            };

            context.set_color(text_color);
            let text_y = tab_rect.center().y - theme.typography.font_size_base / 2;
            context.draw_text(
                &tab.title,
                Point::new(tab_rect.x() + 8, text_y),
                theme.typography.font_size_base,
            );

            // Draw close button if closeable
            if tab.closeable {
                let close_rect = self.get_close_button_rect(tab_rect);
                let is_hover_close = Some(i) == self.hover_close;

                context.set_color(if is_hover_close {
                    theme.colors.error
                } else {
                    theme.colors.text_secondary
                });

                // Draw X
                let padding = 4;
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
            }
        }

        // Draw content area
        let content_rect = self.get_content_rect();
        context.set_color(theme.colors.surface);
        context.fill_rect(content_rect);

        // Draw content area border
        context.set_color(theme.colors.border);
        context.draw_rect(content_rect);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                self.hover_tab = self.tab_from_point(mouse_event.position);

                // Check for close button hover
                self.hover_close = None;
                if let Some(tab_index) = self.hover_tab {
                    if self.tabs[tab_index].closeable {
                        let tab_rect = self.get_tab_rect(tab_index);
                        let close_rect = self.get_close_button_rect(tab_rect);
                        if close_rect.contains(mouse_event.position) {
                            self.hover_close = Some(tab_index);
                        }
                    }
                }

                EventResult::Ignored
            }

            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left && mouse_event.pressed {
                    // Check close button click
                    if let Some(close_index) = self.hover_close {
                        if let Some(handler) = &self.on_tab_closed {
                            handler(close_index);
                        }
                        self.remove_tab(close_index);
                        return EventResult::Consumed;
                    }

                    // Check tab click
                    if let Some(tab_index) = self.tab_from_point(mouse_event.position) {
                        if self.tabs[tab_index].enabled {
                            self.set_active_tab(tab_index);
                            // Match ListView/Slider: focus on click so
                            // subsequent Ctrl+Tab keys are accepted.
                            self.state.focused = true;
                            return EventResult::Consumed;
                        }
                    }
                }
                EventResult::Ignored
            }

            Event::KeyPress(ev) => {
                // All shortcuts require focus — host hands focus via Tab
                // traversal or click. Without this, every Ctrl+Tab in the
                // app would steal from whichever tab control was nearby.
                if !self.state.focused {
                    return EventResult::Ignored;
                }
                if !ev.modifiers.contains(Modifiers::CTRL) {
                    return EventResult::Ignored;
                }
                // Ctrl+Tab / Ctrl+Shift+Tab cycles enabled tabs.
                if ev.key == Key::Tab {
                    let forward = !ev.modifiers.contains(Modifiers::SHIFT);
                    if let Some(next) = self.next_enabled_tab(self.active_tab, forward) {
                        self.set_active_tab(next);
                    }
                    return EventResult::Consumed;
                }
                // Ctrl+1..9 jumps directly. set_active_tab no-ops on
                // disabled or out-of-range index — both correct here.
                if ev.modifiers.contains(Modifiers::SHIFT) {
                    return EventResult::Ignored;
                }
                if let Some(idx) = Self::tab_index_from_digit(&ev.key) {
                    if idx < self.tabs.len() {
                        self.set_active_tab(idx);
                        return EventResult::Consumed;
                    }
                }
                EventResult::Ignored
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
        self.state.enabled && self.state.visible
    }

    fn children(&self) -> &[WidgetId] {
        self.active_content()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

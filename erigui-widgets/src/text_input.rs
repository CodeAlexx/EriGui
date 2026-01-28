use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    Point, Rect, Size, TextInputEvent, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct TextInput {
    state: WidgetState,
    text: String,
    placeholder: String,
    cursor_pos: usize,
    selection_start: Option<usize>,
    scroll_offset: i32,
    on_change: Option<Box<dyn FnMut(&str)>>,
    on_submit: Option<Box<dyn FnMut(&str)>>,
}

impl TextInput {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            text: String::new(),
            placeholder: String::new(),
            cursor_pos: 0,
            selection_start: None,
            scroll_offset: 0,
            on_change: None,
            on_submit: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.cursor_pos = self.text.len();
        self
    }

    pub fn with_on_change<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn with_on_submit<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_submit = Some(Box::new(f));
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_pos = self.text.len();
        self.selection_start = None;
        self.scroll_offset = 0;
        if let Some(callback) = &mut self.on_change {
            callback(&self.text);
        }
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    pub fn get_text(&self) -> String {
        self.text.clone()
    }

    fn approx_char_width(&self, theme: &Theme) -> i32 {
        (theme.typography.font_size_base as f32 * 0.55).round() as i32
    }

    fn ensure_cursor_visible(&mut self, theme: &Theme) {
        let inner = self.state.bounds.inset(4);
        let cursor_x = self.cursor_pixel(theme);
        let visible_start = self.scroll_offset;
        let visible_end = self.scroll_offset + inner.width() - 4;
        if cursor_x < visible_start {
            self.scroll_offset = cursor_x.max(0);
        } else if cursor_x > visible_end {
            self.scroll_offset = (cursor_x - inner.width() + 8).max(0);
        }
    }

    fn cursor_pixel(&self, theme: &Theme) -> i32 {
        let char_w = self.approx_char_width(theme);
        (self.text[..self.cursor_pos].chars().count() as i32) * char_w
    }

    fn insert_char(&mut self, ch: char, theme: &Theme) {
        if self.selection_start.is_some() {
            self.delete_selection();
        }
        self.text.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        if let Some(callback) = &mut self.on_change {
            callback(&self.text);
        }
        self.ensure_cursor_visible(theme);
    }

    fn delete_selection(&mut self) {
        if let Some(start) = self.selection_start {
            let (start, end) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };
            self.text.drain(start..end);
            self.cursor_pos = start;
            self.selection_start = None;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.cursor_pos = prev_boundary(&self.text, self.cursor_pos);
        self.selection_start = None;
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos >= self.text.len() {
            return;
        }
        self.cursor_pos = next_boundary(&self.text, self.cursor_pos);
        self.selection_start = None;
    }
}

impl Widget for TextInput {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size {
        Size::new(
            constraints
                .max_width
                .unwrap_or(160)
                .max(theme.typography.font_size_base * 4),
            theme.typography.font_size_base * 2 + 8,
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);
        context.set_color(if self.state.focused {
            theme.colors.border_focus
        } else {
            theme.colors.border
        });
        context.draw_rect(self.state.bounds);

        let inner = self.state.bounds.inset(4);
        context.push_clip_rect(inner);

        let display_text = if self.text.is_empty() && !self.state.focused {
            self.placeholder.as_str()
        } else {
            &self.text
        };

        let text_color = if self.text.is_empty() && !self.state.focused {
            theme.colors.text_secondary
        } else if self.state.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };

        context.set_color(text_color);
        let text_pos = Point::new(
            inner.x() - self.scroll_offset,
            inner.center().y - theme.typography.font_size_base / 2,
        );
        context.draw_text(display_text, text_pos, theme.typography.font_size_base);

        if self.state.focused && self.state.enabled {
            let before = &self.text[..self.cursor_pos];
            let cursor_x = inner.x()
                + context
                    .measure_text(before, theme.typography.font_size_base)
                    .width
                - self.scroll_offset;
            let top = inner.y() + 2;
            let bottom = inner.bottom() - 2;
            context.set_color(theme.colors.text);
            context.draw_line(Point::new(cursor_x, top), Point::new(cursor_x, bottom), 1);
        }

        context.pop_clip_rect();
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        match event {
            Event::MouseButton(ev) => {
                if ev.button == MouseButton::Left && ev.pressed {
                    if self.state.bounds.contains(ev.position) {
                        self.state.focused = true;
                        // set cursor at end for simplicity
                        self.cursor_pos = self.text.len();
                        self.selection_start = None;
                        return EventResult::Consumed;
                    } else {
                        self.state.focused = false;
                    }
                }
            }
            Event::KeyPress(KeyPressEvent { key, .. }) => {
                if !self.state.focused || !self.state.enabled {
                    return EventResult::Ignored;
                }
                match key {
                    Key::Backspace => {
                        if self.cursor_pos > 0 {
                            self.move_cursor_left();
                            self.text.remove(self.cursor_pos);
                            if let Some(callback) = &mut self.on_change {
                                callback(&self.text);
                            }
                        }
                        return EventResult::Consumed;
                    }
                    Key::Left => {
                        self.move_cursor_left();
                        return EventResult::Consumed;
                    }
                    Key::Right => {
                        self.move_cursor_right();
                        return EventResult::Consumed;
                    }
                    Key::Enter => {
                        if let Some(cb) = &mut self.on_submit {
                            cb(&self.text);
                        }
                        return EventResult::Consumed;
                    }
                    _ => {}
                }
            }
            Event::TextInput(TextInputEvent { text }) => {
                if !self.state.focused || !self.state.enabled {
                    return EventResult::Ignored;
                }
                for ch in text.chars() {
                    self.insert_char(ch, theme);
                }
                return EventResult::Consumed;
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
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn next_boundary(text: &str, idx: usize) -> usize {
    let mut iter = text[idx..].char_indices();
    iter.next();
    iter.next().map(|(i, _)| idx + i).unwrap_or(text.len())
}

fn prev_boundary(text: &str, idx: usize) -> usize {
    if idx == 0 {
        0
    } else {
        text[..idx].char_indices().last().map(|(i, _)| i).unwrap_or(0)
    }
}

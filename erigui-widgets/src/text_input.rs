use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints,
    MouseButtonEvent, MouseButton, Point, Rect, Size, TextInputEvent, Theme, Widget,
    WidgetId, WidgetState,
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
    
    fn insert_char(&mut self, ch: char) {
        if self.selection_start.is_some() {
            self.delete_selection();
        }
        
        self.text.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        
        if let Some(callback) = &mut self.on_change {
            callback(&self.text);
        }
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
        if self.cursor_pos > 0 {
            // Move to previous character boundary
            while self.cursor_pos > 0 && !self.text.is_char_boundary(self.cursor_pos - 1) {
                self.cursor_pos -= 1;
            }
            if self.cursor_pos > 0 {
                self.cursor_pos -= 1;
            }
        }
    }
    
    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.text.len() {
            // Move to next character boundary
            self.cursor_pos += 1;
            while self.cursor_pos < self.text.len() && !self.text.is_char_boundary(self.cursor_pos) {
                self.cursor_pos += 1;
            }
        }
    }
}

impl Widget for TextInput {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let padding = theme.spacing.padding;
        let height = theme.typography.font_size_base + padding.vertical();
        
        Size::new(
            constraints.max_width.unwrap_or(200),
            height,
        )
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }
    
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }
        
        let bounds = self.state.bounds;
        let padding = theme.spacing.padding;
        
        // Background
        let bg_color = if self.state.focused {
            theme.colors.surface
        } else {
            theme.colors.surface_variant
        };
        context.set_color(bg_color);
        context.fill_rect(bounds);
        
        // Border
        let border_color = if self.state.focused {
            theme.colors.border_focus
        } else if self.state.enabled {
            theme.colors.border
        } else {
            theme.colors.border
        };
        context.set_color(border_color);
        context.draw_rect(bounds);
        
        // Text or placeholder
        let text_bounds = bounds.inset(padding.left);
        context.push_clip_rect(text_bounds);
        
        let display_text = if self.text.is_empty() && !self.state.focused {
            &self.placeholder
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
            text_bounds.x() - self.scroll_offset,
            text_bounds.center().y - theme.typography.font_size_base / 2,
        );
        context.draw_text(display_text, text_pos, theme.typography.font_size_base);
        
        // Cursor
        if self.state.focused && self.state.enabled {
            let cursor_x = if self.cursor_pos == 0 {
                text_bounds.x() - self.scroll_offset
            } else {
                let text_before = &self.text[..self.cursor_pos];
                let width = context.measure_text(text_before, theme.typography.font_size_base).width;
                text_bounds.x() + width - self.scroll_offset
            };
            
            context.set_color(theme.colors.text);
            context.draw_line(
                Point::new(cursor_x, text_bounds.y() + 2),
                Point::new(cursor_x, text_bounds.bottom() - 2),
                1,
            );
        }
        
        context.pop_clip_rect();
    }
    
    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }
        
        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    self.state.focused = true;
                    // TODO: Calculate cursor position from mouse
                    return EventResult::Consumed;
                } else {
                    self.state.focused = false;
                }
            }
            
            Event::TextInput(TextInputEvent { text }) if self.state.focused => {
                for ch in text.chars() {
                    self.insert_char(ch);
                }
                return EventResult::Consumed;
            }
            
            Event::KeyPress(KeyPressEvent { key, .. }) if self.state.focused => {
                match key {
                    Key::Backspace => {
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        } else if self.cursor_pos > 0 {
                            self.move_cursor_left();
                            self.text.remove(self.cursor_pos);
                        }
                        
                        if let Some(callback) = &mut self.on_change {
                            callback(&self.text);
                        }
                        return EventResult::Consumed;
                    }
                    
                    Key::Delete => {
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        } else if self.cursor_pos < self.text.len() {
                            self.text.remove(self.cursor_pos);
                        }
                        
                        if let Some(callback) = &mut self.on_change {
                            callback(&self.text);
                        }
                        return EventResult::Consumed;
                    }
                    
                    Key::Left => {
                        self.move_cursor_left();
                        self.selection_start = None;
                        return EventResult::Consumed;
                    }
                    
                    Key::Right => {
                        self.move_cursor_right();
                        self.selection_start = None;
                        return EventResult::Consumed;
                    }
                    
                    Key::Home => {
                        self.cursor_pos = 0;
                        self.selection_start = None;
                        return EventResult::Consumed;
                    }
                    
                    Key::End => {
                        self.cursor_pos = self.text.len();
                        self.selection_start = None;
                        return EventResult::Consumed;
                    }
                    
                    Key::Enter => {
                        if let Some(callback) = &mut self.on_submit {
                            callback(&self.text);
                        }
                        return EventResult::Consumed;
                    }
                    
                    _ => {}
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
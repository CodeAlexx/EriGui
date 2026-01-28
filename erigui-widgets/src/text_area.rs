use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, Point, Rect, Size, TextInputEvent, Theme, Widget, WidgetId,
    WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Selection {
    fn new(line: usize, col: usize) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }

    fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }

    fn normalize(&self) -> (usize, usize, usize, usize) {
        if self.start_line < self.end_line
            || (self.start_line == self.end_line && self.start_col <= self.end_col)
        {
            (self.start_line, self.start_col, self.end_line, self.end_col)
        } else {
            (self.end_line, self.end_col, self.start_line, self.start_col)
        }
    }
}

pub struct TextArea {
    state: WidgetState,
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    selection: Option<Selection>,
    scroll_offset: Point,
    pub line_numbers: bool,
    pub word_wrap: bool,
    tab_size: usize,

    // Visual properties
    line_height: i32,
    char_width: i32,
    gutter_width: i32,

    // Interaction state
    selecting: bool,

    // Undo/redo system
    undo_stack: Vec<TextState>,
    redo_stack: Vec<TextState>,
    // Callbacks
    on_change: Option<Box<dyn FnMut(&str)>>,
}

#[derive(Clone)]
struct TextState {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
}

impl TextArea {
    fn line_char_len(line: &str) -> usize {
        line.chars().count()
    }

    fn char_to_byte_index(line: &str, char_col: usize) -> usize {
        if char_col == 0 {
            return 0;
        }
        let mut count = 0;
        for (byte_idx, _) in line.char_indices() {
            if count == char_col {
                return byte_idx;
            }
            count += 1;
        }
        line.len()
    }

    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            scroll_offset: Point::ZERO,
            line_numbers: true,
            word_wrap: false,
            tab_size: 4,
            line_height: 20,
            char_width: 8,
            gutter_width: 50,
            selecting: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            on_change: None,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.set_text(text);
        self
    }

    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    pub fn with_word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    pub fn with_on_change<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.save_state();
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = None;
        self.notify_change();
    }

    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn get_selected_text(&self) -> Option<String> {
        if let Some(sel) = &self.selection {
            if sel.is_empty() {
                return None;
            }

            let (start_line, start_col, end_line, end_col) = sel.normalize();

            if start_line == end_line {
                Some(self.lines[start_line][start_col..end_col].to_string())
            } else {
                let mut result = String::new();
                result.push_str(&self.lines[start_line][start_col..]);

                for line in (start_line + 1)..end_line {
                    result.push('\n');
                    result.push_str(&self.lines[line]);
                }

                if end_line < self.lines.len() {
                    result.push('\n');
                    result.push_str(&self.lines[end_line][..end_col]);
                }

                Some(result)
            }
        } else {
            None
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection
            .as_ref()
            .map(|sel| !sel.is_empty())
            .unwrap_or(false)
    }

    pub fn copy_selection(&self) -> Option<String> {
        self.get_selected_text()
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        if self.has_selection() {
            let text = self.get_selected_text();
            if text.is_some() {
                self.save_state();
                self.delete_selection();
                self.notify_change();
            }
            text
        } else {
            None
        }
    }

    pub fn paste_text(&mut self, text: &str) {
        if !text.is_empty() {
            self.insert_text(text);
        }
    }

    fn save_state(&mut self) {
        self.undo_stack.push(TextState {
            lines: self.lines.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        });

        // Limit undo stack size
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }

        self.redo_stack.clear();
    }

    fn notify_change(&mut self) {
        let text = self.get_text();
        if let Some(callback) = &mut self.on_change {
            callback(&text);
        }
    }

    fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_state();
        self.delete_selection();
        for ch in text.chars() {
            if ch == '\n' {
                let current_line = self.lines[self.cursor_line].clone();
                let split_idx = Self::char_to_byte_index(&current_line, self.cursor_col);
                let (before, after) = current_line.split_at(split_idx);
                self.lines[self.cursor_line] = before.to_string();
                self.lines.insert(self.cursor_line + 1, after.to_string());
                self.cursor_line += 1;
                self.cursor_col = 0;
            } else {
                let insert_idx =
                    Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
                self.lines[self.cursor_line].insert(insert_idx, ch);
                self.cursor_col += 1;
            }
        }
        self.notify_change();
    }

    fn insert_char(&mut self, ch: char) {
        self.save_state();
        self.delete_selection();

        if ch == '\n' {
            let current_line = self.lines[self.cursor_line].clone();
            let split_idx = Self::char_to_byte_index(&current_line, self.cursor_col);
            let (before, after) = current_line.split_at(split_idx);
            self.lines[self.cursor_line] = before.to_string();
            self.lines.insert(self.cursor_line + 1, after.to_string());
            self.cursor_line += 1;
            self.cursor_col = 0;
        } else if ch == '\t' {
            let spaces = " ".repeat(self.tab_size - (self.cursor_col % self.tab_size));
            let insert_idx =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            self.lines[self.cursor_line].insert_str(insert_idx, &spaces);
            self.cursor_col += spaces.chars().count();
        } else {
            let insert_idx =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            self.lines[self.cursor_line].insert(insert_idx, ch);
            self.cursor_col += 1;
        }

        self.notify_change();
    }

    fn delete_char_backward(&mut self) {
        if self.selection.is_some() {
            self.save_state();
            self.delete_selection();
            self.notify_change();
        } else if self.cursor_col > 0 {
            self.save_state();
            let remove_end =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            let remove_start =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col - 1);
            self.cursor_col -= 1;
            self.lines[self.cursor_line].drain(remove_start..remove_end);
            self.notify_change();
        } else if self.cursor_line > 0 {
            self.save_state();
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            let prev_len = Self::line_char_len(&self.lines[self.cursor_line]);
            self.lines[self.cursor_line].push_str(&current_line);
            self.cursor_col = prev_len;
            self.notify_change();
        }
    }

    fn delete_char_forward(&mut self) {
        if self.selection.is_some() {
            self.save_state();
            self.delete_selection();
            self.notify_change();
        } else if self.cursor_col < Self::line_char_len(&self.lines[self.cursor_line]) {
            self.save_state();
            let start = Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            let end = Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col + 1);
            self.lines[self.cursor_line].drain(start..end);
            self.notify_change();
        } else if self.cursor_line < self.lines.len() - 1 {
            self.save_state();
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
            self.notify_change();
        }
    }

    fn delete_selection(&mut self) {
        if let Some(sel) = &self.selection {
            if !sel.is_empty() {
                let (start_line, start_col, end_line, end_col) = sel.normalize();

                if start_line == end_line {
                    let start_byte = Self::char_to_byte_index(&self.lines[start_line], start_col);
                    let end_byte = Self::char_to_byte_index(&self.lines[start_line], end_col);
                    self.lines[start_line].drain(start_byte..end_byte);
                } else {
                    // Delete from start position to end of first line
                    let start_byte = Self::char_to_byte_index(&self.lines[start_line], start_col);
                    self.lines[start_line].truncate(start_byte);

                    // Append remainder of last line to first line
                    let end_byte = Self::char_to_byte_index(&self.lines[end_line], end_col);
                    let remainder = self.lines[end_line][end_byte..].to_string();
                    self.lines[start_line].push_str(&remainder);

                    // Remove intermediate lines
                    for _ in 0..(end_line - start_line) {
                        self.lines.remove(start_line + 1);
                    }
                }

                self.cursor_line = start_line;
                self.cursor_col = start_col;
                self.selection = None;
            }
        }
    }

    fn move_cursor(&mut self, line: usize, col: usize, extend_selection: bool) {
        let line = line.min(self.lines.len() - 1);
        let col = col.min(Self::line_char_len(&self.lines[line]));

        if extend_selection {
            if self.selection.is_none() {
                self.selection = Some(Selection::new(self.cursor_line, self.cursor_col));
            }
            if let Some(sel) = &mut self.selection {
                sel.end_line = line;
                sel.end_col = col;
            }
        } else {
            self.selection = None;
        }

        self.cursor_line = line;
        self.cursor_col = col;
        self.ensure_cursor_visible();
    }

    fn ensure_cursor_visible(&mut self) {
        let cursor_x = self.cursor_col as i32 * self.char_width;
        let cursor_y = self.cursor_line as i32 * self.line_height;
        let content_x = if self.line_numbers {
            self.gutter_width
        } else {
            0
        };

        // Horizontal scrolling
        if cursor_x < self.scroll_offset.x {
            self.scroll_offset.x = cursor_x;
        } else if cursor_x > self.scroll_offset.x + self.state.bounds.width() - content_x - 20 {
            self.scroll_offset.x = cursor_x - self.state.bounds.width() + content_x + 20;
        }

        // Vertical scrolling
        if cursor_y < self.scroll_offset.y {
            self.scroll_offset.y = cursor_y;
        } else if cursor_y + self.line_height
            > self.scroll_offset.y + self.state.bounds.height() - 20
        {
            self.scroll_offset.y = cursor_y + self.line_height - self.state.bounds.height() + 20;
        }
    }

    fn position_to_cursor(&self, pos: Point) -> (usize, usize) {
        let content_x = if self.line_numbers {
            self.gutter_width
        } else {
            0
        };
        let x = pos.x - self.state.bounds.x() - content_x + self.scroll_offset.x;
        let y = pos.y - self.state.bounds.y() + self.scroll_offset.y;

        let line = (y / self.line_height).max(0) as usize;
        let line = line.min(self.lines.len() - 1);

        let col = (x / self.char_width).max(0) as usize;
        let col = col.min(Self::line_char_len(&self.lines[line]));

        (line, col)
    }

    pub fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            self.redo_stack.push(TextState {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });

            self.lines = state.lines;
            self.cursor_line = state.cursor_line;
            self.cursor_col = state.cursor_col;
            self.selection = None;
            self.notify_change();
        }
    }

    pub fn redo(&mut self) {
        if let Some(state) = self.redo_stack.pop() {
            self.undo_stack.push(TextState {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });

            self.lines = state.lines;
            self.cursor_line = state.cursor_line;
            self.cursor_col = state.cursor_col;
            self.selection = None;
            self.notify_change();
        }
    }
}

impl Widget for TextArea {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        let width = constraints.max_width.unwrap_or(600);
        let height = constraints.max_height.unwrap_or(400);

        Size::new(width, height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        self.line_height = theme.typography.font_size_base + 4;
        self.char_width = theme.typography.font_size_base * 3 / 5;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Calculate content area
        let content_x = self.state.bounds.x()
            + if self.line_numbers {
                self.gutter_width
            } else {
                0
            };
        let content_rect = Rect::new(
            content_x,
            self.state.bounds.y(),
            self.state.bounds.width()
                - if self.line_numbers {
                    self.gutter_width
                } else {
                    0
                },
            self.state.bounds.height(),
        );

        // Set clipping
        context.push_clip_rect(content_rect);

        // Draw selection
        if let Some(sel) = &self.selection {
            if !sel.is_empty() {
                let (start_line, start_col, end_line, end_col) = sel.normalize();
                context.set_color(theme.colors.selection);

                if start_line == end_line {
                    // Single line selection
                    let x = content_x + start_col as i32 * self.char_width - self.scroll_offset.x;
                    let y = self.state.bounds.y() + start_line as i32 * self.line_height
                        - self.scroll_offset.y;
                    let width = (end_col - start_col) as i32 * self.char_width;
                    context.fill_rect(Rect::new(x, y, width, self.line_height));
                } else {
                    // Multi-line selection
                    // First line
                    let x = content_x + start_col as i32 * self.char_width - self.scroll_offset.x;
                    let y = self.state.bounds.y() + start_line as i32 * self.line_height
                        - self.scroll_offset.y;
                    let width = (Self::line_char_len(&self.lines[start_line]) - start_col) as i32
                        * self.char_width;
                    context.fill_rect(Rect::new(x, y, width, self.line_height));

                    // Middle lines
                    for line in (start_line + 1)..end_line {
                        let y = self.state.bounds.y() + line as i32 * self.line_height
                            - self.scroll_offset.y;
                        let width = Self::line_char_len(&self.lines[line]) as i32 * self.char_width;
                        context.fill_rect(Rect::new(
                            content_x - self.scroll_offset.x,
                            y,
                            width,
                            self.line_height,
                        ));
                    }

                    // Last line
                    if end_line < self.lines.len() {
                        let y = self.state.bounds.y() + end_line as i32 * self.line_height
                            - self.scroll_offset.y;
                        let width = end_col as i32 * self.char_width;
                        context.fill_rect(Rect::new(
                            content_x - self.scroll_offset.x,
                            y,
                            width,
                            self.line_height,
                        ));
                    }
                }
            }
        }

        // Draw text
        context.set_color(theme.colors.text);
        for (i, line) in self.lines.iter().enumerate() {
            let y = self.state.bounds.y() + i as i32 * self.line_height - self.scroll_offset.y;

            if y + self.line_height >= self.state.bounds.y() && y < self.state.bounds.bottom() {
                context.draw_text(
                    line,
                    Point::new(content_x - self.scroll_offset.x, y + self.line_height - 4),
                    theme.typography.font_size_base,
                );
            }
        }

        // Draw cursor
        if self.state.focused {
            context.set_color(theme.colors.text);
            let cursor_x =
                content_x + self.cursor_col as i32 * self.char_width - self.scroll_offset.x;
            let cursor_y = self.state.bounds.y() + self.cursor_line as i32 * self.line_height
                - self.scroll_offset.y;
            context.fill_rect(Rect::new(cursor_x, cursor_y, 2, self.line_height));
        }

        context.pop_clip_rect();

        // Draw line numbers
        if self.line_numbers {
            let gutter_rect = Rect::new(
                self.state.bounds.x(),
                self.state.bounds.y(),
                self.gutter_width - 5,
                self.state.bounds.height(),
            );

            context.set_color(theme.colors.surface_variant);
            context.fill_rect(gutter_rect);

            context.push_clip_rect(gutter_rect);
            context.set_color(theme.colors.text_secondary);

            for i in 0..self.lines.len() {
                let y = self.state.bounds.y() + i as i32 * self.line_height - self.scroll_offset.y;

                if y + self.line_height >= self.state.bounds.y() && y < self.state.bounds.bottom() {
                    let line_num = format!("{:>4}", i + 1);
                    context.draw_text(
                        &line_num,
                        Point::new(self.state.bounds.x() + 5, y + self.line_height - 4),
                        theme.typography.font_size_small,
                    );
                }
            }

            context.pop_clip_rect();

            // Draw gutter separator
            context.set_color(theme.colors.border);
            context.fill_rect(Rect::new(
                self.state.bounds.x() + self.gutter_width - 1,
                self.state.bounds.y(),
                1,
                self.state.bounds.height(),
            ));
        }

        // Draw border
        context.set_color(if self.state.focused {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(self.state.bounds);
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
                modifiers,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    if *pressed {
                        self.state.focused = true;
                        let (line, col) = self.position_to_cursor(*position);
                        self.move_cursor(line, col, modifiers.contains(Modifiers::SHIFT));
                        self.selecting = true;
                    } else {
                        self.selecting = false;
                    }
                    return EventResult::Consumed;
                } else if *pressed {
                    self.state.focused = false;
                }
            }
            Event::MouseMove(MouseMoveEvent { position, .. }) => {
                if self.selecting {
                    let (line, col) = self.position_to_cursor(*position);
                    self.move_cursor(line, col, true);
                    return EventResult::Consumed;
                }
            }
            Event::KeyPress(KeyPressEvent { key, modifiers, .. }) => {
                if !self.state.focused {
                    return EventResult::Ignored;
                }

                match key {
                    Key::Left => {
                        if self.cursor_col > 0 {
                            self.move_cursor(
                                self.cursor_line,
                                self.cursor_col - 1,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        } else if self.cursor_line > 0 {
                            self.move_cursor(
                                self.cursor_line - 1,
                                Self::line_char_len(&self.lines[self.cursor_line - 1]),
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Right => {
                        if self.cursor_col < Self::line_char_len(&self.lines[self.cursor_line]) {
                            self.move_cursor(
                                self.cursor_line,
                                self.cursor_col + 1,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        } else if self.cursor_line < self.lines.len() - 1 {
                            self.move_cursor(
                                self.cursor_line + 1,
                                0,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Up => {
                        if self.cursor_line > 0 {
                            self.move_cursor(
                                self.cursor_line - 1,
                                self.cursor_col,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Down => {
                        if self.cursor_line < self.lines.len() - 1 {
                            self.move_cursor(
                                self.cursor_line + 1,
                                self.cursor_col,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Home => {
                        if modifiers.contains(Modifiers::CTRL) {
                            self.move_cursor(0, 0, modifiers.contains(Modifiers::SHIFT));
                        } else {
                            self.move_cursor(
                                self.cursor_line,
                                0,
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::End => {
                        if modifiers.contains(Modifiers::CTRL) {
                            let last_line = self.lines.len() - 1;
                            self.move_cursor(
                                last_line,
                                Self::line_char_len(&self.lines[last_line]),
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        } else {
                            self.move_cursor(
                                self.cursor_line,
                                Self::line_char_len(&self.lines[self.cursor_line]),
                                modifiers.contains(Modifiers::SHIFT),
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Backspace => {
                        self.delete_char_backward();
                        return EventResult::Consumed;
                    }
                    Key::Delete => {
                        self.delete_char_forward();
                        return EventResult::Consumed;
                    }
                    Key::Enter => {
                        self.insert_char('\n');
                        return EventResult::Consumed;
                    }
                    Key::Tab => {
                        self.insert_char('\t');
                        return EventResult::Consumed;
                    }
                    Key::Character(ch) => {
                        if modifiers.contains(Modifiers::CTRL) {
                            match ch {
                                'z' => self.undo(),
                                'y' => self.redo(),
                                'a' => {
                                    self.selection = Some(Selection {
                                        start_line: 0,
                                        start_col: 0,
                                        end_line: self.lines.len() - 1,
                                        end_col: Self::line_char_len(
                                            &self.lines[self.lines.len() - 1],
                                        ),
                                    });
                                }
                                _ => {}
                            }
                        } else {
                            self.insert_char(*ch);
                        }
                        return EventResult::Consumed;
                    }
                    _ => {}
                }
            }
            Event::MouseWheel(wheel_event) => {
                if self.state.bounds.contains(wheel_event.position) {
                    self.scroll_offset.y =
                        (self.scroll_offset.y - wheel_event.delta.y * 3).max(0).min(
                            (self.lines.len() as i32 * self.line_height)
                                .saturating_sub(self.state.bounds.height()),
                        );

                    if wheel_event.modifiers.contains(Modifiers::SHIFT) {
                        self.scroll_offset.x =
                            (self.scroll_offset.x - wheel_event.delta.x * 3).max(0);
                    }

                    return EventResult::Consumed;
                }
            }
            Event::TextInput(TextInputEvent { text }) => {
                if !self.state.focused {
                    return EventResult::Ignored;
                }

                for ch in text.chars() {
                    // Ignore control characters that shouldn't be inserted directly
                    if ch == '\u{8}' || ch == '\u{7f}' || ch.is_control() {
                        continue;
                    }
                    if ch == '\r' {
                        self.insert_char('\n');
                    } else {
                        self.insert_char(ch);
                    }
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
        self.state.enabled && self.state.visible
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

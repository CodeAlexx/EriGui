use crate::clipboard;
use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, Point, Rect, Size, TextInputEvent, Theme, Widget, WidgetId,
    WidgetState,
};
use std::any::Any;
use std::collections::VecDeque;
use std::time::Instant;

/// Callback fired when the text area's text changes. Argument: current text.
type ChangeCallback = Box<dyn FnMut(&str)>;

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
    /// Preferred column for vertical motion (Up/Down). Restores the
    /// "intent" column when navigating across short lines.
    preferred_col: Option<usize>,
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

    // Multi-click word/line selection state.
    last_click_time: Option<Instant>,
    last_click_pos: Point,
    click_count: u32,

    // Undo/redo system
    undo_stack: VecDeque<TextState>,
    redo_stack: VecDeque<TextState>,
    /// Marker for grouping consecutive single-char inserts into a single
    /// undo entry. Reset whenever a structural edit (newline, delete,
    /// paste, selection delete) happens, or when too much time passes.
    last_typing_state: Option<TypingState>,
    // Callbacks
    on_change: Option<ChangeCallback>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TypingState {
    /// Last edit was a single-char insertion. Subsequent single-char
    /// inserts coalesce into the same undo step until something else
    /// happens.
    SingleChar,
}

#[derive(Clone)]
struct TextState {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    selection: Option<Selection>,
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
            preferred_col: None,
            selection: None,
            scroll_offset: Point::ZERO,
            line_numbers: true,
            word_wrap: false,
            tab_size: 4,
            line_height: 20,
            char_width: 8,
            gutter_width: 50,
            selecting: false,
            last_click_time: None,
            last_click_pos: Point::ZERO,
            click_count: 0,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            last_typing_state: None,
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
        // Bug ta11: `text.lines()` collapses trailing `\n` and bare `\r`.
        // Walk the raw string and split on `\n`/`\r\n`/`\r` while preserving
        // the trailing-empty-line semantics.
        self.lines = Self::split_lines(text);
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.preferred_col = None;
        self.selection = None;
        self.last_typing_state = None;
        self.notify_change();
    }

    fn split_lines(text: &str) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut iter = text.chars().peekable();
        while let Some(ch) = iter.next() {
            match ch {
                '\n' => {
                    out.push(std::mem::take(&mut cur));
                }
                '\r' => {
                    out.push(std::mem::take(&mut cur));
                    if iter.peek() == Some(&'\n') {
                        iter.next();
                    }
                }
                _ => cur.push(ch),
            }
        }
        out.push(cur);
        out
    }

    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Test/debug accessor: number of lines in the buffer.
    #[doc(hidden)]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Test/debug accessor: whether a drag-select is currently active.
    #[doc(hidden)]
    pub fn is_selecting(&self) -> bool {
        self.selecting
    }

    /// Test/debug accessor: (cursor_line, cursor_col).
    #[doc(hidden)]
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_line, self.cursor_col)
    }

    /// Test/debug accessor: current selection, if any.
    #[doc(hidden)]
    pub fn selection_for_test(&self) -> Option<Selection> {
        self.selection
    }

    /// Test/debug accessor: undo stack depth.
    #[doc(hidden)]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Test/debug accessor: number of horizontal scroll-pixels currently.
    #[doc(hidden)]
    pub fn scroll_x_for_test(&self) -> i32 {
        self.scroll_offset.x
    }

    /// Test/debug accessor: max horizontal scroll based on widest line.
    #[doc(hidden)]
    pub fn max_scroll_x_for_test(&self) -> i32 {
        self.max_scroll_x()
    }

    /// Test-only helper: force the lines buffer into an empty state to
    /// validate defensive handling. Production code never produces this state.
    #[doc(hidden)]
    pub fn force_empty_lines_for_test(&mut self) {
        self.lines.clear();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = None;
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
        // Structural edits drop any in-progress typing coalesce window.
        self.last_typing_state = None;
        self.push_undo_entry();
        self.redo_stack.clear();
    }

    /// Push a fresh undo entry without resetting `last_typing_state`. Used
    /// by typing-coalesced single-char inserts that want to record a
    /// single entry covering the whole run of keystrokes.
    fn push_undo_entry(&mut self) {
        // Bug ta20: O(1) front-pop via VecDeque instead of Vec::remove(0).
        self.undo_stack.push_back(TextState {
            lines: self.lines.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
            selection: self.selection,
        });
        // Bound the stack at 100 entries.
        while self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
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
        // Bug ta21: coalesce consecutive single-character text inserts into
        // one undo entry. Any structural change (newline, tab expansion,
        // selection-replace, paste, delete) breaks the coalesce window.
        let is_simple_char = ch != '\n' && ch != '\t' && self.selection.is_none();
        if is_simple_char && self.last_typing_state == Some(TypingState::SingleChar) {
            // Don't snapshot — extend the existing entry. redo_stack still
            // needs to clear because we're branching from the prior history.
            self.redo_stack.clear();
        } else {
            self.save_state();
            self.last_typing_state = if is_simple_char {
                Some(TypingState::SingleChar)
            } else {
                None
            };
        }
        self.delete_selection();

        if ch == '\n' {
            let current_line = self.lines[self.cursor_line].clone();
            let split_idx = Self::char_to_byte_index(&current_line, self.cursor_col);
            let (before, after) = current_line.split_at(split_idx);
            self.lines[self.cursor_line] = before.to_string();
            self.lines.insert(self.cursor_line + 1, after.to_string());
            self.cursor_line += 1;
            self.cursor_col = 0;
            self.preferred_col = None;
        } else if ch == '\t' {
            let spaces = " ".repeat(self.tab_size - (self.cursor_col % self.tab_size));
            let insert_idx =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            self.lines[self.cursor_line].insert_str(insert_idx, &spaces);
            self.cursor_col += spaces.chars().count();
            self.preferred_col = None;
        } else {
            let insert_idx =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            self.lines[self.cursor_line].insert(insert_idx, ch);
            self.cursor_col += 1;
            self.preferred_col = None;
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
        self.move_cursor_internal(line, col, extend_selection, /*reset_preferred=*/ true);
        // Any movement breaks the typing coalesce window so the next char
        // starts a fresh undo entry.
        self.last_typing_state = None;
    }

    /// Same as `move_cursor` but lets vertical motion preserve the
    /// "preferred column" so Up/Down across short lines restores intent.
    /// Bug ta17.
    fn move_cursor_keep_preferred(&mut self, line: usize, col: usize, extend_selection: bool) {
        self.move_cursor_internal(line, col, extend_selection, /*reset_preferred=*/ false);
        self.last_typing_state = None;
    }

    fn move_cursor_internal(
        &mut self,
        line: usize,
        col: usize,
        extend_selection: bool,
        reset_preferred: bool,
    ) {
        // Guard against an empty `lines` buffer. Production code maintains
        // the invariant that `lines.len() >= 1`, but we don't want a
        // misuse to crash the host.
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = line.min(self.lines.len().saturating_sub(1));
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
        if reset_preferred {
            self.preferred_col = None;
        }
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
        if self.lines.is_empty() {
            return (0, 0);
        }
        let content_x = if self.line_numbers {
            self.gutter_width
        } else {
            0
        };
        let x = pos.x - self.state.bounds.x() - content_x + self.scroll_offset.x;
        let y = pos.y - self.state.bounds.y() + self.scroll_offset.y;

        let line = (y / self.line_height).max(0) as usize;
        let line = line.min(self.lines.len().saturating_sub(1));

        let col = (x / self.char_width).max(0) as usize;
        let col = col.min(Self::line_char_len(&self.lines[line]));

        (line, col)
    }

    pub fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop_back() {
            // Bug ta22: restore selection, not just text+cursor.
            self.redo_stack.push_back(TextState {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
                selection: self.selection,
            });

            self.lines = state.lines;
            self.cursor_line = state.cursor_line;
            self.cursor_col = state.cursor_col;
            self.selection = state.selection;
            self.last_typing_state = None;
            self.notify_change();
        }
    }

    /// Find the start of the previous word boundary at (line, col). Used
    /// by Ctrl+Left and Ctrl+Backspace.
    fn prev_word_pos(&self, line: usize, col: usize) -> (usize, usize) {
        if col > 0 {
            let chars: Vec<char> = self.lines[line].chars().collect();
            let mut i = col;
            // Skip whitespace immediately to the left of cursor.
            while i > 0 && chars[i - 1].is_whitespace() {
                i -= 1;
            }
            // Then skip word chars.
            while i > 0 && !chars[i - 1].is_whitespace() {
                i -= 1;
            }
            (line, i)
        } else if line > 0 {
            // At start of line: jump to end of previous line.
            (line - 1, Self::line_char_len(&self.lines[line - 1]))
        } else {
            (line, col)
        }
    }

    /// Find the start of the next word boundary at (line, col). Used by
    /// Ctrl+Right.
    fn next_word_pos(&self, line: usize, col: usize) -> (usize, usize) {
        let line_chars: Vec<char> = self.lines[line].chars().collect();
        let line_len = line_chars.len();
        if col < line_len {
            let mut i = col;
            // Skip current word.
            while i < line_len && !line_chars[i].is_whitespace() {
                i += 1;
            }
            // Skip whitespace after.
            while i < line_len && line_chars[i].is_whitespace() {
                i += 1;
            }
            (line, i)
        } else if line + 1 < self.lines.len() {
            (line + 1, 0)
        } else {
            (line, col)
        }
    }

    /// Compute the visible line capacity for PageUp/PageDown jumps.
    fn page_lines(&self) -> usize {
        if self.line_height <= 0 {
            return 1;
        }
        ((self.state.bounds.height() / self.line_height).max(1)) as usize
    }

    /// Compute max horizontal scroll offset based on the longest line.
    fn max_scroll_x(&self) -> i32 {
        let widest = self
            .lines
            .iter()
            .map(|l| Self::line_char_len(l) as i32)
            .max()
            .unwrap_or(0);
        let pixel_width = widest * self.char_width;
        let content_width = self.state.bounds.width()
            - if self.line_numbers {
                self.gutter_width
            } else {
                0
            };
        (pixel_width - content_width).max(0)
    }

    /// Select the word containing (line, col). Bug ta18.
    fn select_word_at(&mut self, line: usize, col: usize) {
        if line >= self.lines.len() {
            return;
        }
        let chars: Vec<char> = self.lines[line].chars().collect();
        if chars.is_empty() {
            self.selection = None;
            return;
        }
        let n = chars.len();
        let mut start = col.min(n);
        let mut end = col.min(n);
        let is_word = |c: char| !c.is_whitespace();
        // Move start left while previous char is in same class as start.
        let class_at = |i: usize| -> bool {
            if i < n { is_word(chars[i]) } else { false }
        };
        let click_class = if start < n {
            class_at(start)
        } else if start > 0 {
            class_at(start - 1)
        } else {
            false
        };
        while start > 0 && class_at(start - 1) == click_class {
            start -= 1;
        }
        while end < n && class_at(end) == click_class {
            end += 1;
        }
        if start == end {
            // Click hit whitespace boundary: just collapse.
            self.selection = None;
            return;
        }
        self.selection = Some(Selection {
            start_line: line,
            start_col: start,
            end_line: line,
            end_col: end,
        });
        self.cursor_line = line;
        self.cursor_col = end;
    }

    /// Select the entire line. Bug ta18.
    fn select_line_at(&mut self, line: usize) {
        if line >= self.lines.len() {
            return;
        }
        let line_len = Self::line_char_len(&self.lines[line]);
        self.selection = Some(Selection {
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: line_len,
        });
        self.cursor_line = line;
        self.cursor_col = line_len;
    }

    /// Replace the current selection (or insert at cursor) with `text`,
    /// taking care of newlines and breaking the typing-coalesce window.
    fn replace_selection_or_insert(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_state();
        self.delete_selection();
        for ch in text.chars() {
            if ch == '\n' || ch == '\r' {
                let current_line = self.lines[self.cursor_line].clone();
                let split_idx = Self::char_to_byte_index(&current_line, self.cursor_col);
                let (before, after) = current_line.split_at(split_idx);
                self.lines[self.cursor_line] = before.to_string();
                self.lines.insert(self.cursor_line + 1, after.to_string());
                self.cursor_line += 1;
                self.cursor_col = 0;
                continue;
            }
            if ch.is_control() {
                continue;
            }
            let insert_idx =
                Self::char_to_byte_index(&self.lines[self.cursor_line], self.cursor_col);
            self.lines[self.cursor_line].insert(insert_idx, ch);
            self.cursor_col += 1;
        }
        self.preferred_col = None;
        self.notify_change();
    }

    pub fn redo(&mut self) {
        if let Some(state) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(TextState {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
                selection: self.selection,
            });

            self.lines = state.lines;
            self.cursor_line = state.cursor_line;
            self.cursor_col = state.cursor_col;
            self.selection = state.selection;
            self.last_typing_state = None;
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

        // Draw selection. Bug ta13/ta15: use measure_text on the prefix
        // for each selection edge so proportional fonts align cleanly with
        // rendered text. Empty lines in a multi-line selection render with
        // a small minimum width so the user can see they're selected.
        if let Some(sel) = &self.selection {
            if !sel.is_empty() {
                let (start_line, start_col, end_line, end_col) = sel.normalize();
                context.set_color(theme.colors.selection);
                let font = theme.typography.font_size_base;
                let empty_line_marker = self.char_width.max(4);

                if start_line == end_line {
                    let line = &self.lines[start_line];
                    let start_byte = Self::char_to_byte_index(line, start_col);
                    let end_byte = Self::char_to_byte_index(line, end_col);
                    let x_start = context.measure_text(&line[..start_byte], font).width;
                    let x_end = context.measure_text(&line[..end_byte], font).width;
                    let x = content_x + x_start - self.scroll_offset.x;
                    let y = self.state.bounds.y()
                        + start_line as i32 * self.line_height
                        - self.scroll_offset.y;
                    let width = (x_end - x_start).max(1);
                    context.fill_rect(Rect::new(x, y, width, self.line_height));
                } else {
                    // First line: from start_col to EOL.
                    let first = &self.lines[start_line];
                    let start_byte = Self::char_to_byte_index(first, start_col);
                    let x_start = context.measure_text(&first[..start_byte], font).width;
                    let x_end = context.measure_text(first, font).width;
                    let y = self.state.bounds.y()
                        + start_line as i32 * self.line_height
                        - self.scroll_offset.y;
                    let mut width = x_end - x_start;
                    if width <= 0 {
                        width = empty_line_marker;
                    }
                    context.fill_rect(Rect::new(
                        content_x + x_start - self.scroll_offset.x,
                        y,
                        width,
                        self.line_height,
                    ));

                    // Middle lines: full line width, with a marker for
                    // empty lines so the selection stays visible. Bug ta15.
                    for line in (start_line + 1)..end_line {
                        let y = self.state.bounds.y() + line as i32 * self.line_height
                            - self.scroll_offset.y;
                        let mut width = context.measure_text(&self.lines[line], font).width;
                        if width <= 0 {
                            width = empty_line_marker;
                        }
                        context.fill_rect(Rect::new(
                            content_x - self.scroll_offset.x,
                            y,
                            width,
                            self.line_height,
                        ));
                    }

                    // Last line: from BOL to end_col.
                    if end_line < self.lines.len() {
                        let last = &self.lines[end_line];
                        let end_byte = Self::char_to_byte_index(last, end_col);
                        let mut width =
                            context.measure_text(&last[..end_byte], font).width;
                        if width <= 0 {
                            width = empty_line_marker;
                        }
                        let y = self.state.bounds.y() + end_line as i32 * self.line_height
                            - self.scroll_offset.y;
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

        // Draw cursor. Bug ta14: measure text up to cursor for proportional
        // fonts so cursor sits flush with rendered glyph edges.
        if self.state.focused {
            context.set_color(theme.colors.text);
            let line = self
                .lines
                .get(self.cursor_line)
                .map(String::as_str)
                .unwrap_or("");
            let byte_idx = Self::char_to_byte_index(line, self.cursor_col);
            let prefix_w = context
                .measure_text(&line[..byte_idx], theme.typography.font_size_base)
                .width;
            let cursor_x = content_x + prefix_w - self.scroll_offset.x;
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
                // A button-release must always end an in-progress drag-
                // select, even when the release happens outside our bounds
                // (e.g. user dragged off the edge before releasing).
                if !*pressed && self.selecting {
                    self.selecting = false;
                    return EventResult::Consumed;
                }
                if self.state.bounds.contains(*position) {
                    if *pressed {
                        self.state.focused = true;
                        let (line, col) = self.position_to_cursor(*position);

                        // Bug ta18: track click count for double/triple-click
                        // selection. 350 ms threshold and a 4 px proximity
                        // gate, matching common toolkits.
                        let now = Instant::now();
                        let near = (position.x - self.last_click_pos.x).abs() < 4
                            && (position.y - self.last_click_pos.y).abs() < 4;
                        let recent = self
                            .last_click_time
                            .map(|t| now.duration_since(t).as_millis() < 350)
                            .unwrap_or(false);
                        if near && recent {
                            self.click_count = (self.click_count + 1).min(3);
                        } else {
                            self.click_count = 1;
                        }
                        self.last_click_time = Some(now);
                        self.last_click_pos = *position;

                        match self.click_count {
                            2 => {
                                // Word selection at click position.
                                self.move_cursor(line, col, false);
                                self.select_word_at(line, col);
                                self.selecting = false;
                            }
                            3 => {
                                // Line selection.
                                self.select_line_at(line);
                                self.selecting = false;
                            }
                            _ => {
                                self.move_cursor(
                                    line,
                                    col,
                                    modifiers.contains(Modifiers::SHIFT),
                                );
                                self.selecting = true;
                            }
                        }
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

                let shift = modifiers.contains(Modifiers::SHIFT);
                // Linux AltGr generates Ctrl+Alt; treating that as a
                // Ctrl-shortcut press would steal AltGr-typed characters
                // (@, €, etc.). Require Ctrl WITHOUT Alt.
                let ctrl = modifiers.contains(Modifiers::CTRL)
                    && !modifiers.contains(Modifiers::ALT);
                match key {
                    Key::Left => {
                        if ctrl {
                            // Bug ta19: Ctrl+Left = previous word.
                            let (l, c) = self.prev_word_pos(self.cursor_line, self.cursor_col);
                            self.move_cursor(l, c, shift);
                        } else if self.cursor_col > 0 {
                            self.move_cursor(self.cursor_line, self.cursor_col - 1, shift);
                        } else if self.cursor_line > 0 {
                            self.move_cursor(
                                self.cursor_line - 1,
                                Self::line_char_len(&self.lines[self.cursor_line - 1]),
                                shift,
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Right => {
                        if ctrl {
                            // Bug ta19: Ctrl+Right = next word.
                            let (l, c) = self.next_word_pos(self.cursor_line, self.cursor_col);
                            self.move_cursor(l, c, shift);
                        } else if self.cursor_col < Self::line_char_len(&self.lines[self.cursor_line]) {
                            self.move_cursor(self.cursor_line, self.cursor_col + 1, shift);
                        } else if self.cursor_line < self.lines.len() - 1 {
                            self.move_cursor(self.cursor_line + 1, 0, shift);
                        }
                        return EventResult::Consumed;
                    }
                    Key::Up => {
                        // Bug ta17: preserve preferred column for vertical motion.
                        let target_col =
                            self.preferred_col.unwrap_or(self.cursor_col);
                        if self.preferred_col.is_none() {
                            self.preferred_col = Some(self.cursor_col);
                        }
                        if self.cursor_line > 0 {
                            self.move_cursor_keep_preferred(
                                self.cursor_line - 1,
                                target_col,
                                shift,
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Down => {
                        let target_col =
                            self.preferred_col.unwrap_or(self.cursor_col);
                        if self.preferred_col.is_none() {
                            self.preferred_col = Some(self.cursor_col);
                        }
                        if self.cursor_line < self.lines.len().saturating_sub(1) {
                            self.move_cursor_keep_preferred(
                                self.cursor_line + 1,
                                target_col,
                                shift,
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::PageUp => {
                        // Bug ta19: PageUp = jump up by visible-line count.
                        let target_col =
                            self.preferred_col.unwrap_or(self.cursor_col);
                        if self.preferred_col.is_none() {
                            self.preferred_col = Some(self.cursor_col);
                        }
                        let page = self.page_lines();
                        let new_line = self.cursor_line.saturating_sub(page);
                        self.move_cursor_keep_preferred(new_line, target_col, shift);
                        return EventResult::Consumed;
                    }
                    Key::PageDown => {
                        let target_col =
                            self.preferred_col.unwrap_or(self.cursor_col);
                        if self.preferred_col.is_none() {
                            self.preferred_col = Some(self.cursor_col);
                        }
                        let page = self.page_lines();
                        let last = self.lines.len().saturating_sub(1);
                        let new_line = (self.cursor_line + page).min(last);
                        self.move_cursor_keep_preferred(new_line, target_col, shift);
                        return EventResult::Consumed;
                    }
                    Key::Home => {
                        if ctrl {
                            self.move_cursor(0, 0, shift);
                        } else {
                            self.move_cursor(self.cursor_line, 0, shift);
                        }
                        return EventResult::Consumed;
                    }
                    Key::End => {
                        if ctrl {
                            let last_line = self.lines.len().saturating_sub(1);
                            self.move_cursor(
                                last_line,
                                Self::line_char_len(&self.lines[last_line]),
                                shift,
                            );
                        } else {
                            self.move_cursor(
                                self.cursor_line,
                                Self::line_char_len(&self.lines[self.cursor_line]),
                                shift,
                            );
                        }
                        return EventResult::Consumed;
                    }
                    Key::Backspace => {
                        if ctrl && !self.has_selection() {
                            // Bug ta19: Ctrl+Backspace = delete previous word.
                            let (target_line, target_col) =
                                self.prev_word_pos(self.cursor_line, self.cursor_col);
                            // Set up a synthetic selection from target -> cursor and
                            // delete it.
                            self.save_state();
                            self.selection = Some(Selection {
                                start_line: target_line,
                                start_col: target_col,
                                end_line: self.cursor_line,
                                end_col: self.cursor_col,
                            });
                            self.delete_selection();
                            self.notify_change();
                        } else {
                            self.delete_char_backward();
                        }
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
                        // Plain Tab = focus navigation (let parent handle).
                        // Ctrl+Tab = insert literal tab character.
                        if ctrl {
                            self.insert_char('\t');
                            return EventResult::Consumed;
                        }
                        return EventResult::Ignored;
                    }
                    // Bug ta22: case-insensitive Ctrl+ shortcuts. Match
                    // both lowercase and uppercase variants so CapsLock /
                    // Shift don't break the bindings.
                    Key::Character(ch) => {
                        if ctrl {
                            match ch.to_ascii_lowercase() {
                                'z' => {
                                    if shift {
                                        self.redo();
                                    } else {
                                        self.undo();
                                    }
                                }
                                'y' => self.redo(),
                                'a' => {
                                    // Bug ta12: select-all must also place
                                    // the cursor at the end of the selection
                                    // so subsequent navigation is sensible.
                                    let last_line = self.lines.len().saturating_sub(1);
                                    let last_col =
                                        Self::line_char_len(&self.lines[last_line]);
                                    self.selection = Some(Selection {
                                        start_line: 0,
                                        start_col: 0,
                                        end_line: last_line,
                                        end_col: last_col,
                                    });
                                    self.cursor_line = last_line;
                                    self.cursor_col = last_col;
                                    self.preferred_col = None;
                                    self.last_typing_state = None;
                                }
                                'c' => {
                                    // Bug ta19: Ctrl+C copy.
                                    if let Some(text) = self.copy_selection() {
                                        clipboard::set_clipboard(&text);
                                    }
                                }
                                'x' => {
                                    // Bug ta19: Ctrl+X cut.
                                    if let Some(text) = self.cut_selection() {
                                        clipboard::set_clipboard(&text);
                                    }
                                }
                                'v' => {
                                    // Bug ta19: Ctrl+V paste.
                                    let pasted = clipboard::get_clipboard();
                                    if !pasted.is_empty() {
                                        self.replace_selection_or_insert(&pasted);
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            self.insert_char(*ch);
                        }
                        return EventResult::Consumed;
                    }
                    Key::Z if ctrl => {
                        if shift {
                            self.redo();
                        } else {
                            self.undo();
                        }
                        return EventResult::Consumed;
                    }
                    Key::Y if ctrl => {
                        self.redo();
                        return EventResult::Consumed;
                    }
                    Key::A if ctrl => {
                        let last_line = self.lines.len().saturating_sub(1);
                        let last_col = Self::line_char_len(&self.lines[last_line]);
                        self.selection = Some(Selection {
                            start_line: 0,
                            start_col: 0,
                            end_line: last_line,
                            end_col: last_col,
                        });
                        self.cursor_line = last_line;
                        self.cursor_col = last_col;
                        self.preferred_col = None;
                        self.last_typing_state = None;
                        return EventResult::Consumed;
                    }
                    Key::C if ctrl => {
                        if let Some(text) = self.copy_selection() {
                            clipboard::set_clipboard(&text);
                        }
                        return EventResult::Consumed;
                    }
                    Key::X if ctrl => {
                        if let Some(text) = self.cut_selection() {
                            clipboard::set_clipboard(&text);
                        }
                        return EventResult::Consumed;
                    }
                    Key::V if ctrl => {
                        let pasted = clipboard::get_clipboard();
                        if !pasted.is_empty() {
                            self.replace_selection_or_insert(&pasted);
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
                        // Bug ta16: clamp horizontal scroll to widest line
                        // so users can't scroll right indefinitely.
                        let max_x = self.max_scroll_x();
                        self.scroll_offset.x =
                            (self.scroll_offset.x - wheel_event.delta.x * 3)
                                .max(0)
                                .min(max_x);
                    }

                    return EventResult::Consumed;
                }
            }
            Event::TextInput(TextInputEvent { text }) => {
                if !self.state.focused {
                    return EventResult::Ignored;
                }

                for ch in text.chars() {
                    // Convert carriage return / line feed to newline FIRST.
                    // `\r` is a control character, so the is_control() filter
                    // below would otherwise drop it before the conversion ran.
                    if ch == '\r' || ch == '\n' {
                        self.insert_char('\n');
                        continue;
                    }
                    // Ignore other control characters that shouldn't be
                    // inserted directly (backspace, DEL, escape, ...).
                    if ch == '\u{8}' || ch == '\u{7f}' || ch.is_control() {
                        continue;
                    }
                    self.insert_char(ch);
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

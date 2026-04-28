use crate::clipboard;
use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    Point, Rect, Size, TextInputEvent, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

/// Callback fired when the text input's text changes. Argument: current text.
type ChangeCallback = Box<dyn FnMut(&str)>;
/// Callback fired when the text input is submitted (Enter pressed). Argument: current text.
type SubmitCallback = Box<dyn FnMut(&str)>;

pub struct TextInput {
    state: WidgetState,
    text: String,
    placeholder: String,
    cursor_pos: usize,
    selection_start: Option<usize>,
    scroll_offset: i32,
    /// Bug ti18: optional cap on character count.
    max_length: Option<usize>,
    /// Bug ti19: render text as masked dots.
    password_mode: bool,
    /// Bug ti17: cached width so `layout` can detect resize and clamp scroll.
    last_layout_width: i32,
    on_change: Option<ChangeCallback>,
    on_submit: Option<SubmitCallback>,
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
            max_length: None,
            password_mode: false,
            last_layout_width: 0,
            on_change: None,
            on_submit: None,
        }
    }

    /// Bug ti18: cap the character count of inserted/pasted text.
    pub fn with_max_length(mut self, n: usize) -> Self {
        self.max_length = Some(n);
        self
    }

    pub fn set_max_length(&mut self, n: Option<usize>) {
        self.max_length = n;
    }

    pub fn max_length(&self) -> Option<usize> {
        self.max_length
    }

    /// Bug ti19: render `*` for each char.
    pub fn with_password_mode(mut self, on: bool) -> Self {
        self.password_mode = on;
        self
    }

    pub fn set_password_mode(&mut self, on: bool) {
        self.password_mode = on;
    }

    pub fn is_password_mode(&self) -> bool {
        self.password_mode
    }

    fn current_char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn at_max_length(&self) -> bool {
        match self.max_length {
            Some(max) => self.current_char_count() >= max,
            None => false,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        if let Some(max) = self.max_length {
            // Bug ti18: clamp at construction to honor max_length.
            let chars: String = self.text.chars().take(max).collect();
            self.text = chars;
        }
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
        if let Some(max) = self.max_length {
            // Bug ti18.
            let chars: String = self.text.chars().take(max).collect();
            self.text = chars;
        }
        self.cursor_pos = self.text.len();
        self.selection_start = None;
        // Bug ti16: long preset string: defer scroll-into-view to the next
        // frame where we have a theme. Until then start at 0; the draw
        // path uses an ensure-visible helper that recomputes on demand.
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

    /// Test/debug accessor: current cursor position (byte index).
    #[doc(hidden)]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Test/debug accessor: selection start position, if any.
    #[doc(hidden)]
    pub fn selection_start(&self) -> Option<usize> {
        self.selection_start
    }

    /// Test/debug helper: explicitly set cursor position (snapped to a
    /// valid UTF-8 boundary). Intended for tests that need a known cursor
    /// before simulating events.
    #[doc(hidden)]
    pub fn set_cursor_pos_for_test(&mut self, pos: usize) {
        self.cursor_pos = pos.min(self.text.len());
        self.cursor_pos = self.safe_cursor_pos();
    }

    #[doc(hidden)]
    pub fn scroll_offset_for_test(&self) -> i32 {
        self.scroll_offset
    }

    fn approx_char_width(&self, theme: &Theme) -> i32 {
        (theme.typography.font_size_base as f32 * 0.55).round() as i32
    }

    /// Ensures cursor_pos is at a valid UTF-8 character boundary.
    /// If not, snaps to the nearest valid boundary (preferring backward).
    fn safe_cursor_pos(&self) -> usize {
        if self.text.is_char_boundary(self.cursor_pos) {
            self.cursor_pos
        } else {
            // Find the previous valid boundary
            self.text[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0)
        }
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

    /// Returns the cursor x-pixel (relative to the inner content origin).
    /// Bug ti13: this matches the same per-char metric the draw path falls
    /// back to when no `DrawContext` is available, so scroll math stays
    /// consistent with rendered output.
    fn cursor_pixel(&self, theme: &Theme) -> i32 {
        let char_w = self.approx_char_width(theme);
        let safe_pos = self.safe_cursor_pos();
        (self.text[..safe_pos].chars().count() as i32) * char_w
    }

    fn insert_char(&mut self, ch: char, theme: &Theme) {
        if self.selection_start.is_some() {
            self.delete_selection();
        }
        // Bug ti18: enforce max_length on each insert.
        if self.at_max_length() {
            self.ensure_cursor_visible(theme);
            return;
        }
        // Ensure we insert at a valid UTF-8 boundary
        let safe_pos = self.safe_cursor_pos();
        self.cursor_pos = safe_pos;
        self.text.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        if let Some(callback) = &mut self.on_change {
            callback(&self.text);
        }
        self.ensure_cursor_visible(theme);
    }

    /// Helper for word-jump. Returns the byte index of the previous word
    /// boundary, or 0 if at start.
    fn prev_word_boundary(&self) -> usize {
        let idx = self.cursor_pos;
        if idx == 0 {
            return 0;
        }
        let chars: Vec<(usize, char)> = self.text.char_indices().collect();
        let mut i = chars.len();
        while i > 0 {
            let (b, _) = chars[i - 1];
            if b < idx {
                break;
            }
            i -= 1;
        }
        // Skip whitespace.
        while i > 0 && chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        // Skip word.
        while i > 0 && !chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        if i == 0 {
            0
        } else {
            chars[i].0
        }
    }

    fn next_word_boundary(&self) -> usize {
        let len = self.text.len();
        let idx = self.cursor_pos;
        if idx >= len {
            return len;
        }
        let chars: Vec<(usize, char)> = self.text.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].0 >= idx {
                break;
            }
            i += 1;
        }
        // Skip word.
        while i < chars.len() && !chars[i].1.is_whitespace() {
            i += 1;
        }
        // Skip whitespace.
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        if i == chars.len() {
            len
        } else {
            chars[i].0
        }
    }

    /// Return the currently-selected text, if any.
    fn selected_text(&self) -> Option<String> {
        let sel_start = self.selection_start?;
        let safe_sel = if self.text.is_char_boundary(sel_start) {
            sel_start
        } else {
            self.text[..sel_start]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        let safe_cur = self.safe_cursor_pos();
        let (lo, hi) = if safe_sel < safe_cur {
            (safe_sel, safe_cur)
        } else {
            (safe_cur, safe_sel)
        };
        if lo == hi {
            None
        } else {
            Some(self.text[lo..hi].to_string())
        }
    }

    fn delete_selection(&mut self) {
        if let Some(sel_start) = self.selection_start {
            // Ensure both positions are at valid UTF-8 boundaries
            let safe_sel_start = if self.text.is_char_boundary(sel_start) {
                sel_start
            } else {
                self.text[..sel_start].char_indices().last().map(|(i, _)| i).unwrap_or(0)
            };
            let safe_cursor = self.safe_cursor_pos();

            let (start, end) = if safe_sel_start < safe_cursor {
                (safe_sel_start, safe_cursor)
            } else {
                (safe_cursor, safe_sel_start)
            };
            self.text.drain(start..end);
            self.cursor_pos = start;
            self.selection_start = None;
        }
    }

    fn move_cursor_left(&mut self, extend_selection: bool) {
        if self.cursor_pos == 0 {
            return;
        }
        if extend_selection {
            // Anchor selection at the pre-move cursor position the first
            // time we extend. Subsequent shifted moves keep the anchor.
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
            self.cursor_pos = prev_boundary(&self.text, self.cursor_pos);
        } else {
            self.cursor_pos = prev_boundary(&self.text, self.cursor_pos);
            self.selection_start = None;
        }
    }

    fn move_cursor_right(&mut self, extend_selection: bool) {
        if self.cursor_pos >= self.text.len() {
            return;
        }
        if extend_selection {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
            self.cursor_pos = next_boundary(&self.text, self.cursor_pos);
        } else {
            self.cursor_pos = next_boundary(&self.text, self.cursor_pos);
            self.selection_start = None;
        }
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

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        let prev_width = self.last_layout_width;
        let first_layout = prev_width == 0;
        self.state.bounds = rect;
        self.last_layout_width = rect.width();
        // Bug ti16: first layout after `with_text` / `set_text` has cursor
        // at text.len() but scroll_offset==0; if the text is too long, the
        // cursor is off-screen. Re-run ensure_cursor_visible.
        // Bug ti17: same on subsequent resizes.
        if first_layout || prev_width != rect.width() {
            self.ensure_cursor_visible(theme);
            if self.scroll_offset < 0 {
                self.scroll_offset = 0;
            }
        }
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

        // Bug ti19: render password mode as masked dots.
        let masked: String;
        let display_text: &str = if self.text.is_empty() && !self.state.focused {
            self.placeholder.as_str()
        } else if self.password_mode {
            masked = "•".repeat(self.text.chars().count());
            masked.as_str()
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

        // Bug ti14: render selection background under text.
        if let Some(sel_start) = self.selection_start {
            let safe_sel = if self.text.is_char_boundary(sel_start) {
                sel_start
            } else {
                self.text[..sel_start]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            };
            let safe_cur = self.safe_cursor_pos();
            let (lo, hi) = if safe_sel < safe_cur {
                (safe_sel, safe_cur)
            } else {
                (safe_cur, safe_sel)
            };
            if lo != hi {
                let char_w = self.approx_char_width(theme);
                let lo_chars = self.text[..lo].chars().count() as i32;
                let hi_chars = self.text[..hi].chars().count() as i32;
                let x = inner.x() + lo_chars * char_w - self.scroll_offset;
                let w = (hi_chars - lo_chars) * char_w;
                let y = inner.y() + 2;
                let h = inner.height() - 4;
                context.set_color(theme.colors.selection);
                context.fill_rect(Rect::new(x, y, w, h));
            }
        }

        context.set_color(text_color);
        let text_pos = Point::new(
            inner.x() - self.scroll_offset,
            inner.center().y - theme.typography.font_size_base / 2,
        );
        context.draw_text(display_text, text_pos, theme.typography.font_size_base);

        if self.state.focused && self.state.enabled {
            // Bug ti13: keep cursor x in lock-step with the metric used by
            // `ensure_cursor_visible` so scroll never under/overshoots.
            let cursor_x = inner.x() + self.cursor_pixel(theme) - self.scroll_offset;
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
                        // Map the click x-coordinate to the nearest character
                        // column. We use `approx_char_width` (same metric the
                        // draw path falls back on when measure_text is not
                        // available in this code path).
                        let inner = self.state.bounds.inset(4);
                        let char_w = self.approx_char_width(theme).max(1);
                        let local_x = ev.position.x - inner.x() + self.scroll_offset;
                        let target_col = ((local_x + char_w / 2) / char_w).max(0) as usize;

                        // Walk the text using prev_boundary / next_boundary to
                        // land on a UTF-8 boundary and clamp to text length.
                        let mut new_pos = 0usize;
                        let total_chars = self.text.chars().count();
                        let target_col = target_col.min(total_chars);
                        for _ in 0..target_col {
                            new_pos = next_boundary(&self.text, new_pos);
                            if new_pos >= self.text.len() {
                                break;
                            }
                        }
                        self.cursor_pos = new_pos;
                        self.selection_start = None;
                        return EventResult::Consumed;
                    } else {
                        self.state.focused = false;
                    }
                }
            }
            Event::KeyPress(KeyPressEvent { key, modifiers, .. }) => {
                if !self.state.focused || !self.state.enabled {
                    return EventResult::Ignored;
                }
                let shift = modifiers.contains(Modifiers::SHIFT);
                // Linux AltGr generates Ctrl+Alt; treating that as a
                // Ctrl-shortcut press would steal AltGr-typed characters
                // (@, €, etc.). Require Ctrl WITHOUT Alt.
                let ctrl = modifiers.contains(Modifiers::CTRL)
                    && !modifiers.contains(Modifiers::ALT);
                match key {
                    Key::Backspace => {
                        // Bug ti12: if there's a selection, delete it.
                        if self.selection_start.is_some() {
                            self.delete_selection();
                            if let Some(callback) = &mut self.on_change {
                                callback(&self.text);
                            }
                            self.ensure_cursor_visible(theme);
                        } else if ctrl {
                            // Bug ti15: Ctrl+Backspace = delete previous word.
                            let target = self.prev_word_boundary();
                            if target < self.cursor_pos {
                                self.text.drain(target..self.cursor_pos);
                                self.cursor_pos = target;
                                if let Some(callback) = &mut self.on_change {
                                    callback(&self.text);
                                }
                                self.ensure_cursor_visible(theme);
                            }
                        } else if self.cursor_pos > 0 {
                            self.move_cursor_left(false);
                            self.text.remove(self.cursor_pos);
                            if let Some(callback) = &mut self.on_change {
                                callback(&self.text);
                            }
                            self.ensure_cursor_visible(theme);
                        }
                        return EventResult::Consumed;
                    }
                    Key::Delete => {
                        // Bug ti15: Delete forward.
                        if self.selection_start.is_some() {
                            self.delete_selection();
                            if let Some(callback) = &mut self.on_change {
                                callback(&self.text);
                            }
                        } else if self.cursor_pos < self.text.len() {
                            let next = next_boundary(&self.text, self.cursor_pos);
                            self.text.drain(self.cursor_pos..next);
                            if let Some(callback) = &mut self.on_change {
                                callback(&self.text);
                            }
                        }
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::Left => {
                        if ctrl {
                            // Bug ti15: Ctrl+Left = previous word.
                            let target = self.prev_word_boundary();
                            if shift && self.selection_start.is_none() {
                                self.selection_start = Some(self.cursor_pos);
                            }
                            if !shift {
                                self.selection_start = None;
                            }
                            self.cursor_pos = target;
                        } else {
                            self.move_cursor_left(shift);
                        }
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::Right => {
                        if ctrl {
                            // Bug ti15: Ctrl+Right = next word.
                            let target = self.next_word_boundary();
                            if shift && self.selection_start.is_none() {
                                self.selection_start = Some(self.cursor_pos);
                            }
                            if !shift {
                                self.selection_start = None;
                            }
                            self.cursor_pos = target;
                        } else {
                            self.move_cursor_right(shift);
                        }
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::Home => {
                        // Bug ti15: Home / Shift+Home.
                        if shift && self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                        if !shift {
                            self.selection_start = None;
                        }
                        self.cursor_pos = 0;
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::End => {
                        // Bug ti15: End / Shift+End.
                        if shift && self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                        if !shift {
                            self.selection_start = None;
                        }
                        self.cursor_pos = self.text.len();
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::Tab => {
                        // Bug ti15: Tab is focus navigation. Pass through.
                        return EventResult::Ignored;
                    }
                    Key::Enter => {
                        if let Some(cb) = &mut self.on_submit {
                            cb(&self.text);
                        }
                        return EventResult::Consumed;
                    }
                    Key::Character(ch) => {
                        if ctrl {
                            // Bug ti15 / ta22: case-insensitive Ctrl+ shortcuts.
                            match ch.to_ascii_lowercase() {
                                'a' => {
                                    // Select all.
                                    self.selection_start = Some(0);
                                    self.cursor_pos = self.text.len();
                                    self.ensure_cursor_visible(theme);
                                    return EventResult::Consumed;
                                }
                                'c' => {
                                    if !self.password_mode {
                                        if let Some(text) = self.selected_text() {
                                            clipboard::set_clipboard(&text);
                                        }
                                    }
                                    return EventResult::Consumed;
                                }
                                'x' => {
                                    if self.password_mode {
                                        return EventResult::Consumed;
                                    }
                                    if let Some(text) = self.selected_text() {
                                        clipboard::set_clipboard(&text);
                                        self.delete_selection();
                                        if let Some(cb) = &mut self.on_change {
                                            cb(&self.text);
                                        }
                                        self.ensure_cursor_visible(theme);
                                    }
                                    return EventResult::Consumed;
                                }
                                'v' => {
                                    let pasted = clipboard::get_clipboard();
                                    if !pasted.is_empty() {
                                        if self.selection_start.is_some() {
                                            self.delete_selection();
                                        }
                                        for ch in pasted.chars() {
                                            if ch.is_control() {
                                                continue;
                                            }
                                            self.insert_char(ch, theme);
                                        }
                                    }
                                    return EventResult::Consumed;
                                }
                                _ => {}
                            }
                            // Other Ctrl+chars: swallow so they don't insert.
                            return EventResult::Consumed;
                        }
                    }
                    Key::A if ctrl => {
                        self.selection_start = Some(0);
                        self.cursor_pos = self.text.len();
                        self.ensure_cursor_visible(theme);
                        return EventResult::Consumed;
                    }
                    Key::C if ctrl => {
                        if !self.password_mode {
                            if let Some(text) = self.selected_text() {
                                clipboard::set_clipboard(&text);
                            }
                        }
                        return EventResult::Consumed;
                    }
                    Key::X if ctrl => {
                        if self.password_mode {
                            return EventResult::Consumed;
                        }
                        if let Some(text) = self.selected_text() {
                            clipboard::set_clipboard(&text);
                            self.delete_selection();
                            if let Some(cb) = &mut self.on_change {
                                cb(&self.text);
                            }
                            self.ensure_cursor_visible(theme);
                        }
                        return EventResult::Consumed;
                    }
                    Key::V if ctrl => {
                        let pasted = clipboard::get_clipboard();
                        if !pasted.is_empty() {
                            if self.selection_start.is_some() {
                                self.delete_selection();
                            }
                            for ch in pasted.chars() {
                                if ch.is_control() {
                                    continue;
                                }
                                self.insert_char(ch, theme);
                            }
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

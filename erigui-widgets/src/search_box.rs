use crate::{Icon, TextInput};
use erigui_core::{
    Color, DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::time::{Duration, Instant};

/// Callback fired when a search is submitted. Argument: search text.
type SearchCallback = Box<dyn FnMut(&str)>;
/// Callback fired when the search box text changes. Argument: current text.
type ChangeCallback = Box<dyn FnMut(&str)>;
/// Callback used to fetch suggestions for the current query.
/// Argument: current text. Returns: list of suggestion strings.
type SuggestionProvider = Box<dyn FnMut(&str) -> Vec<String>>;

pub struct SearchBox {
    state: WidgetState,
    input: TextInput,
    placeholder: String,
    search_history: Vec<String>,
    show_suggestions: bool,
    suggestions: Vec<String>,
    selected_suggestion: Option<usize>,

    // Debouncing
    last_change: Option<Instant>,
    debounce_delay: Duration,

    // Visual state
    clear_button_hovered: bool,
    search_icon_rect: Rect,
    clear_button_rect: Rect,
    suggestions_rect: Rect,

    // Options
    _case_sensitive: bool,
    _use_regex: bool,
    _whole_word: bool,

    // Callbacks
    on_search: Option<SearchCallback>,
    on_change: Option<ChangeCallback>,
    suggestion_provider: Option<SuggestionProvider>,
}

impl SearchBox {
    pub fn new(id: WidgetId) -> Self {
        let mut input = TextInput::new(WidgetId::default());
        input.set_placeholder("Search...");

        Self {
            state: WidgetState::new(id),
            input,
            placeholder: "Search...".to_string(),
            search_history: Vec::new(),
            show_suggestions: false,
            suggestions: Vec::new(),
            selected_suggestion: None,
            last_change: None,
            debounce_delay: Duration::from_millis(300),
            clear_button_hovered: false,
            search_icon_rect: Rect::default(),
            clear_button_rect: Rect::default(),
            suggestions_rect: Rect::default(),
            _case_sensitive: false,
            _use_regex: false,
            _whole_word: false,
            on_search: None,
            on_change: None,
            suggestion_provider: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self.input.set_placeholder(&self.placeholder);
        self
    }

    pub fn with_debounce_delay(mut self, millis: u64) -> Self {
        self.debounce_delay = Duration::from_millis(millis);
        self
    }

    pub fn with_on_search<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_search = Some(Box::new(f));
        self
    }

    pub fn with_on_change<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn with_suggestion_provider<F: FnMut(&str) -> Vec<String> + 'static>(
        mut self,
        f: F,
    ) -> Self {
        self.suggestion_provider = Some(Box::new(f));
        self
    }

    pub fn get_text(&self) -> String {
        self.input.get_text()
    }

    /// Test/debug accessor: whether a debounced text change is pending
    /// (i.e. typing has occurred but the debounce timer has not yet expired).
    #[doc(hidden)]
    pub fn has_pending_change(&self) -> bool {
        self.last_change.is_some()
    }

    pub fn set_text(&mut self, text: &str) {
        self.input.set_text(text);
        self.trigger_change();
    }

    pub fn clear(&mut self) {
        self.input.set_text("");
        self.show_suggestions = false;
        self.suggestions.clear();
        self.selected_suggestion = None;
        // Bug sb19: button is gone after clear; reset hover state.
        self.clear_button_hovered = false;
        self.trigger_change();
    }

    pub fn add_to_history(&mut self, query: String) {
        if query.is_empty() {
            return;
        }
        // Bug sb20: LRU-promote existing entries instead of dropping the
        // re-search silently. Move the matching entry to position 0.
        if let Some(pos) = self.search_history.iter().position(|q| q == &query) {
            if pos != 0 {
                let existing = self.search_history.remove(pos);
                self.search_history.insert(0, existing);
            }
            return;
        }
        self.search_history.insert(0, query);
        if self.search_history.len() > 20 {
            self.search_history.pop();
        }
    }

    /// Bug sb11: poll the debounced change pipeline. This is intended to
    /// be called by the host's main loop (e.g. on Event::Update) so that
    /// debounced searches fire even when no further events arrive after
    /// the last keystroke.
    pub fn poll_debounce(&mut self) {
        if let Some(last) = self.last_change {
            if last.elapsed() >= self.debounce_delay {
                self.last_change = None;
                self.trigger_change();
            }
        }
    }

    /// Bug sb12: callers that have a layered/over-painted draw context can
    /// invoke this *after* drawing the rest of the surface so the
    /// suggestion list renders on top of any overlapping widgets.
    pub fn draw_suggestions_overlay(&self, context: &mut dyn DrawContext, theme: &Theme) {
        // Re-uses the private suggestion-rect path. Recomputes the bounds
        // from current suggestion count (Bug sb13).
        if !self.show_suggestions || self.suggestions.is_empty() {
            return;
        }
        self.draw_suggestions(context, theme);
    }

    /// Test/debug accessor: current suggestion count.
    #[doc(hidden)]
    pub fn suggestion_count(&self) -> usize {
        self.suggestions.len()
    }

    /// Test/debug accessor: index of the currently-selected suggestion.
    #[doc(hidden)]
    pub fn selected_suggestion(&self) -> Option<usize> {
        self.selected_suggestion
    }

    /// Test/debug accessor: whether the suggestion dropdown is showing.
    #[doc(hidden)]
    pub fn is_showing_suggestions(&self) -> bool {
        self.show_suggestions
    }

    /// Test/debug accessor: clear-button hover state.
    #[doc(hidden)]
    pub fn is_clear_button_hovered(&self) -> bool {
        self.clear_button_hovered
    }

    /// Test/debug accessor: search history snapshot.
    #[doc(hidden)]
    pub fn history_snapshot(&self) -> Vec<String> {
        self.search_history.clone()
    }

    fn trigger_search(&mut self) {
        let text = self.input.get_text();
        if !text.is_empty() {
            self.add_to_history(text.clone());
        }

        if let Some(callback) = &mut self.on_search {
            callback(&text);
        }

        self.show_suggestions = false;
    }

    /// Refresh the suggestion list synchronously (not debounced). Suggestions
    /// drive the dropdown's visibility, so they should reflect the current
    /// query immediately.
    fn refresh_suggestions(&mut self) {
        let text = self.input.get_text();
        if text.is_empty() {
            self.suggestions = self.search_history.clone();
        } else if let Some(provider) = &mut self.suggestion_provider {
            self.suggestions = provider(&text);
        } else {
            self.suggestions = self
                .search_history
                .iter()
                .filter(|h| h.to_lowercase().contains(&text.to_lowercase()))
                .cloned()
                .collect();
        }
        self.show_suggestions = !self.suggestions.is_empty();
        self.selected_suggestion = None;
    }

    fn trigger_change(&mut self) {
        // Bug sb11: this is the *debounced* call. Don't re-set
        // `last_change` here — that would loop forever. We refresh
        // suggestions and fire the on_change callback, then leave
        // `last_change` as None so the debounce window stays closed
        // until the user types again.
        self.refresh_suggestions();
        let text = self.input.get_text();
        if let Some(callback) = &mut self.on_change {
            callback(&text);
        }
    }

    fn select_suggestion(&mut self, index: usize) {
        if index < self.suggestions.len() {
            let suggestion = self.suggestions[index].clone();
            self.input.set_text(&suggestion);
            self.show_suggestions = false;
            self.trigger_search();
        }
    }

    fn draw_search_icon(&self, context: &mut dyn DrawContext, theme: &Theme) {
        let color = theme.colors.text_secondary;
        Icon::draw_search(context, self.search_icon_rect, color);
    }

    fn draw_clear_button(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if self.input.get_text().is_empty() {
            return;
        }

        let color = if self.clear_button_hovered {
            theme.colors.text
        } else {
            theme.colors.text_secondary
        };

        // Draw X
        let center = self.clear_button_rect.center();
        let size = 6;

        context.set_color(color);
        context.set_line_width(2);
        context.draw_line(
            Point::new(center.x - size, center.y - size),
            Point::new(center.x + size, center.y + size),
            2,
        );
        context.draw_line(
            Point::new(center.x - size, center.y + size),
            Point::new(center.x + size, center.y - size),
            2,
        );
    }

    /// Bug sb13: recompute the suggestions rect from the *current*
    /// suggestion count. The cached `suggestions_rect` from layout time
    /// is sized for whatever was in `suggestions` then, which is stale
    /// once suggestions are populated.
    fn current_suggestions_rect(&self, theme: &Theme) -> Rect {
        let item_height = theme.typography.font_size_base + 8;
        let max_suggestions = 8;
        let height = (self.suggestions.len().min(max_suggestions) as i32) * item_height;
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.bottom() + 2,
            self.state.bounds.width(),
            height,
        )
    }

    fn draw_suggestions(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.show_suggestions || self.suggestions.is_empty() {
            return;
        }
        let rect = self.current_suggestions_rect(theme);

        // Draw dropdown background
        context.set_color(theme.colors.surface);
        context.fill_rect(rect);

        // Draw shadow
        context.set_color(Color::rgba(0, 0, 0, 32));
        context.fill_rect(Rect::new(
            rect.x() + 2,
            rect.y() + 2,
            rect.width(),
            rect.height(),
        ));

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(rect);

        // Draw suggestions
        let item_height = theme.typography.font_size_base + 8;
        let padding = 8;

        context.push_clip_rect(rect);

        for (i, suggestion) in self.suggestions.iter().enumerate() {
            let y = rect.y() + i as i32 * item_height;
            let item_rect = Rect::new(rect.x(), y, rect.width(), item_height);

            // Draw selection background
            if Some(i) == self.selected_suggestion {
                context.set_color(theme.colors.primary_hover);
                context.fill_rect(item_rect);
            }

            // Draw text
            context.set_color(if Some(i) == self.selected_suggestion {
                theme.colors.background
            } else {
                theme.colors.text
            });

            context.draw_text(
                suggestion,
                Point::new(
                    item_rect.x() + padding,
                    item_rect.center().y - theme.typography.font_size_base / 2,
                ),
                theme.typography.font_size_base,
            );
        }

        context.pop_clip_rect();
    }
}

impl Widget for SearchBox {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let base_size = self.input.measure(constraints, theme);
        Size::new(base_size.width.max(200), base_size.height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;

        // Layout components
        let icon_size = 20;
        let padding = 8;

        // Search icon on left
        self.search_icon_rect = Rect::new(
            rect.x() + padding,
            rect.y() + (rect.height() - icon_size) / 2,
            icon_size,
            icon_size,
        );

        // Clear button on right
        self.clear_button_rect = Rect::new(
            rect.right() - icon_size - padding,
            rect.y() + (rect.height() - icon_size) / 2,
            icon_size,
            icon_size,
        );

        // Input in the middle
        let input_x = rect.x() + icon_size + padding * 2;
        let input_width = rect.width() - icon_size * 2 - padding * 4;
        self.input.layout(
            Rect::new(input_x, rect.y(), input_width, rect.height()),
            theme,
        );

        // Suggestions dropdown
        let max_suggestions = 8;
        let item_height = theme.typography.font_size_base + 8;
        let suggestions_height = (self.suggestions.len().min(max_suggestions) as i32) * item_height;

        self.suggestions_rect = Rect::new(
            rect.x(),
            rect.bottom() + 2,
            rect.width(),
            suggestions_height,
        );
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.state.bounds);

        // Draw border
        context.set_color(if self.state.focused {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(self.state.bounds);

        // Draw search icon
        self.draw_search_icon(context, theme);

        // Draw input (without its own border)
        self.input.draw(context, theme);

        // Draw clear button
        self.draw_clear_button(context, theme);

        // Draw suggestions dropdown
        self.draw_suggestions(context, theme);
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Check for debounced change trigger
        if let Some(last_change) = self.last_change {
            if last_change.elapsed() >= self.debounce_delay {
                self.last_change = None;
                self.trigger_change();
            }
        }

        // Bug sb13: use the *current* suggestions rect, not the one cached
        // at layout time (which was sized for the suggestion count when
        // layout ran).
        let sug_rect = self.current_suggestions_rect(theme);

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    self.state.focused = true;

                    // Check clear button
                    if self.clear_button_rect.contains(*position)
                        && !self.input.get_text().is_empty()
                    {
                        self.clear();
                        // Bug sb18: also focus the inner input so subsequent
                        // typing reaches it.
                        self.input.set_focused(true);
                        return EventResult::Consumed;
                    }

                    // Pass to input
                    self.input.handle_event(event, theme);
                    EventResult::Consumed
                } else if sug_rect.contains(*position) && self.show_suggestions {
                    // Click on suggestion. Bug sb14: clamp to valid range.
                    let item_height = theme.typography.font_size_base + 8;
                    let raw = position.y - sug_rect.y();
                    if raw < 0 {
                        return EventResult::Consumed;
                    }
                    let index = (raw / item_height.max(1)) as usize;
                    if index < self.suggestions.len() {
                        self.select_suggestion(index);
                    }
                    EventResult::Consumed
                } else {
                    self.state.focused = false;
                    self.show_suggestions = false;
                    self.selected_suggestion = None;
                    self.clear_button_hovered = false;
                    EventResult::Consumed
                }
            }
            Event::MouseMove(move_event) => {
                let pos = move_event.position;
                self.clear_button_hovered = self.clear_button_rect.contains(pos);

                if self.show_suggestions && sug_rect.contains(pos) {
                    let item_height = theme.typography.font_size_base + 8;
                    let raw = pos.y - sug_rect.y();
                    if raw < 0 {
                        return EventResult::Consumed;
                    }
                    let index = (raw / item_height.max(1)) as usize;
                    if index < self.suggestions.len() {
                        self.selected_suggestion = Some(index);
                    }
                    return EventResult::Consumed;
                }
                // Bug sb15: clear hover-driven selection when the pointer
                // leaves the suggestion list, so subsequent keyboard
                // navigation starts from a clean state.
                if self.show_suggestions && !sug_rect.contains(pos) {
                    self.selected_suggestion = None;
                }
                EventResult::Ignored
            }
            Event::KeyPress(KeyPressEvent {
                key,
                modifiers: _modifiers,
                ..
            }) => {
                if !self.state.focused {
                    return EventResult::Ignored;
                }

                match key {
                    Key::Enter => {
                        if let Some(index) = self.selected_suggestion {
                            self.select_suggestion(index);
                        } else {
                            self.trigger_search();
                        }
                        return EventResult::Consumed;
                    }
                    Key::Escape => {
                        // Bug sb16: full reset on Esc — close suggestions,
                        // clear selection, and return Consumed regardless
                        // of suggestion-state so the host knows we ate it.
                        self.show_suggestions = false;
                        self.selected_suggestion = None;
                        return EventResult::Consumed;
                    }
                    Key::Up => {
                        // Bug sb17: when suggestions are present, Up cycles
                        // them and is consumed. When no suggestions exist,
                        // Up is consumed (so the inner input doesn't move
                        // its caret) but returned as Consumed by the
                        // SearchBox to keep behavior stable.
                        if self.show_suggestions && !self.suggestions.is_empty() {
                            self.selected_suggestion = Some(
                                self.selected_suggestion
                                    .map(|i| {
                                        if i > 0 {
                                            i - 1
                                        } else {
                                            self.suggestions.len() - 1
                                        }
                                    })
                                    .unwrap_or(self.suggestions.len() - 1),
                            );
                            return EventResult::Consumed;
                        }
                        return EventResult::Consumed;
                    }
                    Key::Down => {
                        if self.show_suggestions && !self.suggestions.is_empty() {
                            self.selected_suggestion = Some(
                                self.selected_suggestion
                                    .map(|i| (i + 1) % self.suggestions.len())
                                    .unwrap_or(0),
                            );
                            return EventResult::Consumed;
                        }
                        return EventResult::Consumed;
                    }
                    _ => {}
                }

                // Pass to input and check for changes.
                let old_text = self.input.get_text();
                let result = self.input.handle_event(event, theme);

                if self.input.get_text() != old_text {
                    self.last_change = Some(Instant::now());
                    // Refresh suggestions immediately so the dropdown
                    // reflects the current query without waiting for
                    // the debounced on_change callback. Bug sb13/sb17.
                    self.refresh_suggestions();
                }

                result
            }
            _ => {
                // Forward to the inner input. Capture text before/after so
                // that TextInput / IME events also trigger the debounced
                // change pipeline — the explicit KeyPress arm above already
                // does this for printable keys, but the default arm is the
                // path TextInputEvent travels through.
                let old_text = self.input.get_text();
                let result = self.input.handle_event(event, theme);
                if self.input.get_text() != old_text {
                    self.last_change = Some(Instant::now());
                    self.refresh_suggestions();
                }
                result
            }
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
        self.input.set_focused(focused);
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

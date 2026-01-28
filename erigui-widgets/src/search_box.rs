use crate::{Icon, TextInput};
use erigui_core::{
    Color, DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::time::{Duration, Instant};

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
    on_search: Option<Box<dyn FnMut(&str)>>,
    on_change: Option<Box<dyn FnMut(&str)>>,
    suggestion_provider: Option<Box<dyn FnMut(&str) -> Vec<String>>>,
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

    pub fn set_text(&mut self, text: &str) {
        self.input.set_text(text);
        self.trigger_change();
    }

    pub fn clear(&mut self) {
        self.input.set_text("");
        self.show_suggestions = false;
        self.suggestions.clear();
        self.selected_suggestion = None;
        self.trigger_change();
    }

    pub fn add_to_history(&mut self, query: String) {
        if !query.is_empty() && !self.search_history.contains(&query) {
            self.search_history.insert(0, query);
            if self.search_history.len() > 20 {
                self.search_history.pop();
            }
        }
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

    fn trigger_change(&mut self) {
        self.last_change = Some(Instant::now());

        let text = self.input.get_text();

        // Update suggestions
        if text.is_empty() {
            // Show history when empty
            self.suggestions = self.search_history.clone();
        } else if let Some(provider) = &mut self.suggestion_provider {
            self.suggestions = provider(&text);
        } else {
            // Default: filter history
            self.suggestions = self
                .search_history
                .iter()
                .filter(|h| h.to_lowercase().contains(&text.to_lowercase()))
                .cloned()
                .collect();
        }

        self.show_suggestions = !self.suggestions.is_empty();
        self.selected_suggestion = None;

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

    fn draw_suggestions(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.show_suggestions || self.suggestions.is_empty() {
            return;
        }

        // Draw dropdown background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.suggestions_rect);

        // Draw shadow
        context.set_color(Color::rgba(0, 0, 0, 32));
        context.fill_rect(Rect::new(
            self.suggestions_rect.x() + 2,
            self.suggestions_rect.y() + 2,
            self.suggestions_rect.width(),
            self.suggestions_rect.height(),
        ));

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.suggestions_rect);

        // Draw suggestions
        let item_height = theme.typography.font_size_base + 8;
        let padding = 8;

        context.push_clip_rect(self.suggestions_rect);

        for (i, suggestion) in self.suggestions.iter().enumerate() {
            let y = self.suggestions_rect.y() + i as i32 * item_height;
            let item_rect = Rect::new(
                self.suggestions_rect.x(),
                y,
                self.suggestions_rect.width(),
                item_height,
            );

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
                    item_rect.center().y + theme.typography.font_size_base / 2 - 2,
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
                        return EventResult::Consumed;
                    }

                    // Pass to input
                    self.input.handle_event(event, theme);
                    return EventResult::Consumed;
                } else if self.suggestions_rect.contains(*position) && self.show_suggestions {
                    // Click on suggestion
                    let item_height = theme.typography.font_size_base + 8;
                    let index = ((position.y - self.suggestions_rect.y()) / item_height) as usize;
                    self.select_suggestion(index);
                    return EventResult::Consumed;
                } else {
                    self.state.focused = false;
                    self.show_suggestions = false;
                    return EventResult::Ignored;
                }
            }
            Event::MouseMove(move_event) => {
                self.clear_button_hovered = self.clear_button_rect.contains(move_event.position);

                if self.show_suggestions && self.suggestions_rect.contains(move_event.position) {
                    let item_height = theme.typography.font_size_base + 8;
                    let index = ((move_event.position.y - self.suggestions_rect.y()) / item_height)
                        as usize;
                    if index < self.suggestions.len() {
                        self.selected_suggestion = Some(index);
                    }
                    return EventResult::Consumed;
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
                        self.show_suggestions = false;
                        self.selected_suggestion = None;
                        return EventResult::Consumed;
                    }
                    Key::Up => {
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
                    }
                    _ => {}
                }

                // Pass to input and check for changes
                let old_text = self.input.get_text();
                let result = self.input.handle_event(event, theme);

                if self.input.get_text() != old_text {
                    self.last_change = Some(Instant::now());
                }

                result
            }
            _ => self.input.handle_event(event, theme),
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

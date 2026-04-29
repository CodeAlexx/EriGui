use erigui_core::{
    DrawContext, Event, EventResult, Key, LayoutConstraints, MouseButton, MouseButtonEvent,
    MouseWheelEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct ListItem {
    pub id: String,
    pub text: String,
    pub icon: Option<String>,
    pub selected: bool,
}

pub struct ListView {
    state: WidgetState,
    items: Vec<ListItem>,
    selected_index: Option<usize>,
    item_height: i32,
    /// Vertical scroll offset in pixels. Always >= 0; clamped so the last
    /// item never scrolls past the bottom of the viewport (unless the list
    /// is shorter than the viewport, in which case it's pinned to 0).
    scroll_offset: i32,
    on_selection_change: Option<Box<dyn FnMut(Option<usize>)>>,
    multi_select: bool,
    /// `Some(_)` while the user is mid-drag on the scrollbar thumb.
    drag: Option<ScrollbarDrag>,
}

/// Captured at MouseButton press inside the scrollbar thumb. We track the
/// mouse position and scroll_offset at press time so MouseMove deltas map
/// to scroll deltas independent of where on the thumb the click landed.
#[derive(Clone, Copy)]
struct ScrollbarDrag {
    start_mouse_y: i32,
    start_offset: i32,
}

/// Geometry of the visible scrollbar. `None` when content fits the viewport
/// (no scrollbar drawn, no hit target).
struct ScrollbarGeometry {
    track: Rect,
    thumb: Rect,
    /// `track.height() - thumb.height()`. Mouse can move the top of the
    /// thumb anywhere in `[track.y, track.y + max_thumb_y]`. Used to scale
    /// mouse Y delta into scroll-offset delta during drag.
    max_thumb_y: i32,
}

impl ListView {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            selected_index: None,
            item_height: 24,
            scroll_offset: 0,
            on_selection_change: None,
            multi_select: false,
            drag: None,
        }
    }

    /// Compute the scrollbar track + thumb rects. Returns `None` when no
    /// scrollbar is needed (content fits viewport). Single source of truth
    /// shared between `draw` and the drag hit-test.
    fn scrollbar_geometry(&self) -> Option<ScrollbarGeometry> {
        let max = self.max_scroll();
        if max <= 0 {
            return None;
        }
        const TRACK_W: i32 = 8;
        let track = Rect::new(
            self.state.bounds.right() - TRACK_W,
            self.state.bounds.y(),
            TRACK_W,
            self.state.bounds.height(),
        );
        let viewport_h = self.state.bounds.height();
        let content_h = self.content_height();
        let thumb_h_raw = ((viewport_h as f32 / content_h as f32) * track.height() as f32) as i32;
        let thumb_h = thumb_h_raw.max(20).min(track.height());
        let max_thumb_y = track.height() - thumb_h;
        let thumb_y_offset = if max > 0 && max_thumb_y > 0 {
            ((self.scroll_offset as f32 / max as f32) * max_thumb_y as f32) as i32
        } else {
            0
        };
        let thumb = Rect::new(
            track.x() + 1,
            track.y() + thumb_y_offset,
            TRACK_W - 2,
            thumb_h,
        );
        Some(ScrollbarGeometry { track, thumb, max_thumb_y })
    }

    /// Total content height (items × item_height). May exceed bounds.height.
    fn content_height(&self) -> i32 {
        self.items.len() as i32 * self.item_height
    }

    /// Maximum legal `scroll_offset` value (so the last item bottom-aligns
    /// with the viewport bottom). Returns 0 if list is shorter than viewport.
    fn max_scroll(&self) -> i32 {
        (self.content_height() - self.state.bounds.height()).max(0)
    }

    /// Clamp scroll_offset to [0, max_scroll].
    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset < 0 {
            self.scroll_offset = 0;
        } else if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// Adjust scroll_offset so the selected item is fully visible.
    fn scroll_to_selected(&mut self) {
        let Some(i) = self.selected_index else {
            return;
        };
        let item_top = i as i32 * self.item_height;
        let item_bottom = item_top + self.item_height;
        let viewport_top = self.scroll_offset;
        let viewport_bottom = viewport_top + self.state.bounds.height();
        if item_top < viewport_top {
            self.scroll_offset = item_top;
        } else if item_bottom > viewport_bottom {
            self.scroll_offset = item_bottom - self.state.bounds.height();
        }
        self.clamp_scroll();
    }

    /// Read accessor for tests/host integration.
    pub fn scroll_offset(&self) -> i32 {
        self.scroll_offset
    }

    pub fn with_items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }

    pub fn with_on_selection_change<F: FnMut(Option<usize>) + 'static>(mut self, f: F) -> Self {
        self.on_selection_change = Some(Box::new(f));
        self
    }

    pub fn with_multi_select(mut self, multi_select: bool) -> Self {
        self.multi_select = multi_select;
        self
    }

    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.selected_index = None;
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        if index.map(|i| i < self.items.len()).unwrap_or(true) {
            self.selected_index = index;
            self.scroll_to_selected();

            if let Some(callback) = &mut self.on_selection_change {
                callback(self.selected_index);
            }
        }
    }

    pub fn selected_item(&self) -> Option<&ListItem> {
        self.selected_index.and_then(|i| self.items.get(i))
    }

    fn item_rect(&self, index: usize) -> Rect {
        Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y() + (index as i32 * self.item_height) - self.scroll_offset,
            self.state.bounds.width(),
            self.item_height,
        )
    }

    fn item_at_position(&self, position: Point) -> Option<usize> {
        if !self.state.bounds.contains(position) {
            return None;
        }

        let relative_y = position.y - self.state.bounds.y() + self.scroll_offset;
        let index = (relative_y / self.item_height) as usize;

        if index < self.items.len() {
            Some(index)
        } else {
            None
        }
    }
}

impl Widget for ListView {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        let height = (self.items.len() as i32 * self.item_height)
            .min(constraints.max_height.unwrap_or(i32::MAX));

        Size::new(constraints.max_width.unwrap_or(200), height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        // Theme-scale row height. Mojo-port hardcoded 24 left rows
        // squashed at HiDPI, causing the next row's text to bleed into
        // the previous row. Pattern: font + padding gives a row tall
        // enough for a glyph plus chrome at any scale.
        let font = theme.typography.font_size_base;
        let pad = theme.spacing.padding.top.max(4);
        self.item_height = (font + pad * 2).max(24);
        // Re-clamp in case the bounds shrank below current scroll offset.
        self.clamp_scroll();
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Draw items
        for (index, item) in self.items.iter().enumerate() {
            let item_rect = self.item_rect(index);

            // Skip items outside the visible area
            if item_rect.y() >= self.state.bounds.bottom() {
                break;
            }

            if item_rect.bottom() < self.state.bounds.y() {
                continue;
            }

            // Draw selection background
            if self.selected_index == Some(index) || (self.multi_select && item.selected) {
                context.set_color(theme.colors.selection);
                context.fill_rect(item_rect);
            }

            // Draw item text
            let text_color = if self.state.enabled {
                if self.selected_index == Some(index) {
                    theme.colors.surface
                } else {
                    theme.colors.text
                }
            } else {
                theme.colors.text_disabled
            };

            context.set_color(text_color);
            let text_pos = Point::new(
                item_rect.x() + theme.spacing.padding.left,
                item_rect.center().y - theme.typography.font_size_base / 2,
            );
            context.draw_text(&item.text, text_pos, theme.typography.font_size_base);
        }

        // Vertical scrollbar — only drawn when content exceeds viewport.
        if let Some(g) = self.scrollbar_geometry() {
            // Track in the muted surface variant; thumb in text_secondary
            // for clear contrast (theme.colors.border was too close to the
            // track color on the dark theme).
            context.set_color(theme.colors.surface_variant);
            context.fill_rect(g.track);
            context.set_color(theme.colors.text_secondary);
            context.fill_rect(g.thumb);
        }

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Scrollbar drag: thumb hit-test runs BEFORE item-click so a press
        // on the thumb doesn't double-fire as an item selection underneath.
        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                if let Some(g) = self.scrollbar_geometry() {
                    if g.thumb.contains(*position) {
                        self.drag = Some(ScrollbarDrag {
                            start_mouse_y: position.y,
                            start_offset: self.scroll_offset,
                        });
                        self.state.focused = true;
                        return EventResult::Consumed;
                    }
                }
            }
            Event::MouseMove(ev) => {
                if let Some(d) = self.drag {
                    if let Some(g) = self.scrollbar_geometry() {
                        let dy_mouse = ev.position.y - d.start_mouse_y;
                        // Map thumb-y delta back to scroll-offset delta.
                        // max_thumb_y > 0 because geometry returned Some
                        // (max_scroll > 0) and `min(track.height())` only
                        // collapses when track.height() is tiny — guard
                        // anyway to avoid div-by-zero when ranges align.
                        if g.max_thumb_y > 0 {
                            let max = self.max_scroll();
                            let dy_scroll = (dy_mouse as f32 / g.max_thumb_y as f32
                                * max as f32) as i32;
                            self.scroll_offset = d.start_offset.saturating_add(dy_scroll);
                            self.clamp_scroll();
                        }
                        return EventResult::Consumed;
                    }
                }
            }
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                pressed: false,
                ..
            }) => {
                if self.drag.is_some() {
                    self.drag = None;
                    return EventResult::Consumed;
                }
            }
            _ => {}
        }

        if let Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) = event {
            if let Some(index) = self.item_at_position(*position) {
                if self.multi_select {
                    self.items[index].selected = !self.items[index].selected;
                } else {
                    self.set_selected_index(Some(index));
                }
                self.state.focused = true;
                return EventResult::Consumed;
            }
        }

        // Mouse-wheel scroll. Most-common ListView frustration: long
        // file lists / dropdowns that simply truncate beyond the viewport.
        // Scroll by ~3 items per wheel tick, matching desktop convention.
        if let Event::MouseWheel(MouseWheelEvent { position, delta, .. }) = event {
            if self.state.bounds.contains(*position) {
                // Negative dy is scroll-down (content moves up, offset increases).
                let step = self.item_height * 3;
                let pixels = (-delta.y).saturating_mul(step);
                self.scroll_offset = self.scroll_offset.saturating_add(pixels);
                self.clamp_scroll();
                return EventResult::Consumed;
            }
        }

        // Keyboard navigation when focused. Standard list behavior:
        // Up/Down move selection, Home/End jump, Space/Enter on the
        // selected item is host-meaningful (no internal action — the
        // host wires it through `on_selection_change` already firing
        // on `set_selected_index`).
        if let Event::KeyPress(ev) = event {
            if !self.state.focused {
                return EventResult::Ignored;
            }
            if self.items.is_empty() {
                return EventResult::Ignored;
            }
            let cur = self.selected_index.unwrap_or(0);
            let last = self.items.len() - 1;
            match ev.key {
                Key::Down => {
                    let next = (cur + 1).min(last);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::Up => {
                    let next = cur.saturating_sub(1);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::Home => {
                    self.set_selected_index(Some(0));
                    return EventResult::Consumed;
                }
                Key::End => {
                    self.set_selected_index(Some(last));
                    return EventResult::Consumed;
                }
                Key::PageDown => {
                    // 10 items as a reasonable page size since item height
                    // is fixed at 24 and viewport size isn't known at this
                    // layer.
                    let next = (cur + 10).min(last);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                Key::PageUp => {
                    let next = cur.saturating_sub(10);
                    self.set_selected_index(Some(next));
                    return EventResult::Consumed;
                }
                _ => {}
            }
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

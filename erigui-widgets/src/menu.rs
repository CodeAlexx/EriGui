use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton, Point,
    Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

/// Approximate per-character pixel width used when sizing menu rows.
/// Bug m17: replaces `text.len()` (which counted UTF-8 bytes) with a
/// character count so multi-byte labels don't over-allocate.
fn char_count(s: &str) -> i32 {
    s.chars().count() as i32
}

#[derive(Clone)]
pub struct MenuItem {
    pub text: String,
    pub shortcut: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub submenu_items: Vec<MenuItem>,
    pub data: i32,
    pub on_click: Option<fn()>,
}

impl MenuItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shortcut: String::new(),
            enabled: true,
            is_separator: false,
            submenu_items: Vec::new(),
            data: 0,
            on_click: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self
    }

    pub fn with_submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu_items = items;
        self
    }

    pub fn with_on_click(mut self, handler: fn()) -> Self {
        self.on_click = Some(handler);
        self
    }

    pub fn separator() -> Self {
        Self {
            text: String::new(),
            shortcut: String::new(),
            enabled: true,
            is_separator: true,
            submenu_items: Vec::new(),
            data: 0,
            on_click: None,
        }
    }
}

pub struct MenuBar {
    state: WidgetState,
    menu_items: Vec<MenuItem>,
    item_widths: Vec<i32>,
    active_menu: i32,
    hover_item: i32,
    dropdown_visible: bool,
    dropdown_hover: i32,
    /// Bug m18: per-menu dropdown width so a wide submenu doesn't bloat
    /// every other dropdown. Indexed parallel to `menu_items`.
    dropdown_widths: Vec<i32>,
    dropdown_default_width: i32,
    dropdown_height: i32,
    padding: i32,
    /// Optional viewport supplied by the host so dropdowns can flip on
    /// edge overflow. Bug m13 + m18.
    viewport: Option<Rect>,
    /// (parent menu index, parent dropdown item index) of the currently
    /// expanded submenu, if any. Bug m12 — minimal submenu tracking so
    /// hosts can render the second-level dropdown.
    submenu_open: Option<(i32, i32)>,
}

impl MenuBar {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            menu_items: Vec::new(),
            item_widths: Vec::new(),
            active_menu: -1,
            hover_item: -1,
            dropdown_visible: false,
            dropdown_hover: -1,
            dropdown_widths: Vec::new(),
            dropdown_default_width: 200,
            dropdown_height: 28,
            padding: 8,
            viewport: None,
            submenu_open: None,
        }
    }

    /// Test/debug accessor: currently-open submenu, as
    /// (parent_menu_index, parent_item_index). Bug m12.
    #[doc(hidden)]
    pub fn submenu_open(&self) -> Option<(i32, i32)> {
        self.submenu_open
    }

    /// Bug m13: hosts can supply a viewport rectangle so dropdowns flip
    /// above the menu bar when they would otherwise overflow the bottom
    /// edge. Without a viewport we fall back to the menu bar's own bounds
    /// for clamping.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = Some(viewport);
    }

    pub fn add_menu(&mut self, text: impl Into<String>, items: Vec<MenuItem>) {
        let text = text.into();
        // Bug m17: count chars (not bytes) so multi-byte UTF-8 menu titles
        // don't over-allocate width and break hit-testing.
        let text_width = char_count(&text) * 8 + 3 * self.padding;
        self.item_widths.push(text_width);

        // Bug m18: compute *this* menu's dropdown width independently.
        // Previously the same `dropdown_width` was mutated per add_menu,
        // forcing every dropdown to the widest one's size.
        let mut menu_width = self.dropdown_default_width;
        for item in &items {
            if item.is_separator {
                continue;
            }
            let text_len = char_count(&item.text) * 8;
            let shortcut_len = char_count(&item.shortcut) * 7;
            let total = text_len + shortcut_len + 4 * self.padding;
            if total > menu_width {
                menu_width = total;
            }
        }
        self.dropdown_widths.push(menu_width);

        let mut menu = MenuItem::new(text);
        menu.submenu_items = items;
        self.menu_items.push(menu);
    }

    fn dropdown_width_for(&self, menu_index: i32) -> i32 {
        if menu_index < 0 || menu_index >= self.dropdown_widths.len() as i32 {
            self.dropdown_default_width
        } else {
            self.dropdown_widths[menu_index as usize]
        }
    }

    pub fn is_dropdown_visible(&self) -> bool {
        self.dropdown_visible
    }

    /// Test/debug accessor: index of currently active menu (-1 if none).
    #[doc(hidden)]
    pub fn active_menu(&self) -> i32 {
        self.active_menu
    }

    /// Test/debug accessor: currently-hovered dropdown item (-1 if none).
    #[doc(hidden)]
    pub fn dropdown_hover(&self) -> i32 {
        self.dropdown_hover
    }

    /// Test/debug accessor: per-menu dropdown rect.
    #[doc(hidden)]
    pub fn dropdown_rect_for(&self, menu_index: i32) -> Rect {
        self.get_dropdown_rect(menu_index)
    }

    /// Test/debug accessor: per-menu cached dropdown width.
    #[doc(hidden)]
    pub fn dropdown_width_at(&self, menu_index: i32) -> i32 {
        self.dropdown_width_for(menu_index)
    }

    /// Test/debug accessor: hit-test a point against the menu bar items.
    #[doc(hidden)]
    pub fn menu_index_at(&self, point: Point) -> i32 {
        self.menu_index_from_point(point)
    }

    pub fn draw_dropdown_only(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if self.dropdown_visible && self.active_menu >= 0 {
            // Draw the dropdown ONLY - no menu bar redraw
            self.draw_dropdown(context, theme, self.active_menu);
        }
    }

    pub fn hide_dropdown(&mut self) {
        self.dropdown_visible = false;
        self.active_menu = -1;
        self.dropdown_hover = -1;
        self.submenu_open = None;
    }

    fn close_dropdown_internal(&mut self) {
        self.dropdown_visible = false;
        self.active_menu = -1;
        self.dropdown_hover = -1;
        self.submenu_open = None;
    }

    pub fn draw_menu_bar_only(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background with surface color for better visibility
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Draw bottom border
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(self.state.bounds.x(), self.state.bounds.bottom() - 1),
            Point::new(self.state.bounds.right(), self.state.bounds.bottom() - 1),
            1,
        );

        // Draw menu items
        for (i, menu) in self.menu_items.iter().enumerate() {
            let menu_rect = self.get_menu_rect(i as i32);
            let is_active = i as i32 == self.active_menu;
            let is_hover = i as i32 == self.hover_item && !is_active && self.dropdown_hover < 0;

            // Draw item background
            if is_active {
                context.set_color(theme.colors.primary);
                context.fill_rect(menu_rect);
            } else if is_hover {
                context.set_color(theme.colors.surface_variant);
                context.fill_rect(menu_rect);
            }

            // Draw item text
            let text_color = if is_active {
                theme.colors.background // White text on active menu
            } else {
                theme.colors.text
            };
            context.set_color(text_color);

            // Center text vertically in the menu bar
            let text_y =
                menu_rect.y() + (menu_rect.height() + theme.typography.font_size_base) / 2 - 2;
            context.draw_text(
                &menu.text,
                Point::new(menu_rect.x() + self.padding, text_y),
                theme.typography.font_size_base,
            );
        }
    }

    fn get_menu_rect(&self, menu_index: i32) -> Rect {
        if menu_index < 0 || menu_index >= self.menu_items.len() as i32 {
            return Rect::new(0, 0, 0, 0);
        }

        // Start with a small left margin
        let mut x = self.state.bounds.x() + 4;
        for i in 0..menu_index as usize {
            x += self.item_widths[i];
        }

        Rect::new(
            x,
            self.state.bounds.y(),
            self.item_widths[menu_index as usize],
            self.state.bounds.height(),
        )
    }

    fn get_dropdown_rect(&self, menu_index: i32) -> Rect {
        let menu_rect = self.get_menu_rect(menu_index);
        let items = &self.menu_items[menu_index as usize].submenu_items;
        // Account for separators having a smaller height (matches the
        // draw path's row layout). Bug m12 prep + cm-style sizing.
        let dropdown_height: i32 = items
            .iter()
            .map(|i| {
                if i.is_separator {
                    self.dropdown_height / 2
                } else {
                    self.dropdown_height
                }
            })
            .sum();
        let width = self.dropdown_width_for(menu_index);
        let mut x = menu_rect.x();
        // Bug m18: per-menu width.
        if let Some(viewport_right) = self.viewport_right_clamp() {
            if x + width > viewport_right {
                x = viewport_right - width;
            }
        }
        if x < self.state.bounds.x() {
            x = self.state.bounds.x();
        }

        // Bug m13: dropdown bottom-overflow. If the dropdown would extend
        // beyond the viewport's bottom, flip it above the menu bar.
        // When the flipped position would itself be off the top, clamp
        // to 0 — the dropdown is taller than the viewport either way and
        // some content will be clipped, but the cursor-anchor edge stays
        // on-screen.
        let mut y = menu_rect.bottom();
        if let Some(viewport_bottom) = self.viewport_bottom_clamp() {
            if y + dropdown_height > viewport_bottom {
                let above = menu_rect.y() - dropdown_height;
                y = above.max(0);
            }
        }

        Rect::new(x, y, width, dropdown_height)
    }

    /// Clamp width by the menu bar's right edge as a fallback. Hosts can
    /// set a viewport via `set_viewport` for accurate clamping.
    fn viewport_right_clamp(&self) -> Option<i32> {
        Some(
            self.viewport
                .map(|v| v.right())
                .unwrap_or_else(|| self.state.bounds.right()),
        )
    }

    fn viewport_bottom_clamp(&self) -> Option<i32> {
        self.viewport.map(|v| v.bottom())
    }

    fn menu_index_from_point(&self, point: Point) -> i32 {
        if !self.state.bounds.contains(point) {
            return -1;
        }

        // Match the +4 left margin used by `get_menu_rect` so hit-testing
        // is aligned with the drawn item rectangles.
        let mut x = self.state.bounds.x() + 4;
        for i in 0..self.menu_items.len() {
            if point.x >= x && point.x < x + self.item_widths[i] {
                return i as i32;
            }
            x += self.item_widths[i];
        }
        -1
    }

    /// First focusable (non-separator, enabled) item index in the given
    /// dropdown, or None if there is none. Bug m11.
    fn first_focusable_in(&self, menu_index: i32) -> Option<i32> {
        if menu_index < 0 || menu_index >= self.menu_items.len() as i32 {
            return None;
        }
        let items = &self.menu_items[menu_index as usize].submenu_items;
        for (i, it) in items.iter().enumerate() {
            if !it.is_separator && it.enabled {
                return Some(i as i32);
            }
        }
        None
    }

    /// Cycle dropdown hover by one in the given direction, skipping
    /// separators and disabled items. Bug m11.
    fn step_dropdown_hover(&self, menu_index: i32, forward: bool) -> Option<i32> {
        if menu_index < 0 || menu_index >= self.menu_items.len() as i32 {
            return None;
        }
        let items = &self.menu_items[menu_index as usize].submenu_items;
        let n = items.len() as i32;
        if n == 0 {
            return None;
        }
        let start = if self.dropdown_hover >= 0 {
            self.dropdown_hover
        } else if forward {
            n - 1
        } else {
            0
        };
        let mut i = start;
        for _ in 0..n {
            i = if forward {
                (i + 1) % n
            } else if i == 0 {
                n - 1
            } else {
                i - 1
            };
            let it = &items[i as usize];
            if !it.is_separator && it.enabled {
                return Some(i);
            }
        }
        Some(self.dropdown_hover)
    }

    /// Match an Alt+key combo to a menu by its first character. Bug m11.
    fn find_mnemonic_match(&self, key: &Key) -> Option<i32> {
        // Map Key::Character / Key::A..Key::Z to a lowercase ASCII char.
        let target = match key {
            Key::Character(c) => Some(c.to_ascii_lowercase()),
            Key::A => Some('a'),
            Key::B => Some('b'),
            Key::C => Some('c'),
            Key::D => Some('d'),
            Key::E => Some('e'),
            Key::F => Some('f'),
            Key::G => Some('g'),
            Key::H => Some('h'),
            Key::I => Some('i'),
            Key::J => Some('j'),
            Key::K => Some('k'),
            Key::L => Some('l'),
            Key::M => Some('m'),
            Key::N => Some('n'),
            Key::O => Some('o'),
            Key::P => Some('p'),
            Key::Q => Some('q'),
            Key::R => Some('r'),
            Key::S => Some('s'),
            Key::T => Some('t'),
            Key::U => Some('u'),
            Key::V => Some('v'),
            Key::W => Some('w'),
            Key::X => Some('x'),
            Key::Y => Some('y'),
            Key::Z => Some('z'),
            _ => None,
        }?;
        for (i, m) in self.menu_items.iter().enumerate() {
            if let Some(first) = m.text.chars().next() {
                if first.to_ascii_lowercase() == target {
                    return Some(i as i32);
                }
            }
        }
        None
    }

    fn dropdown_item_from_point(&self, point: Point, menu_index: i32) -> i32 {
        if menu_index < 0 || menu_index >= self.menu_items.len() as i32 {
            return -1;
        }

        let dropdown_rect = self.get_dropdown_rect(menu_index);
        if !dropdown_rect.contains(point) {
            return -1;
        }

        let relative_y = point.y - dropdown_rect.y();
        if relative_y < 0 {
            return -1;
        }
        let items = &self.menu_items[menu_index as usize].submenu_items;
        let mut acc = 0i32;
        for (i, item) in items.iter().enumerate() {
            let row_h = if item.is_separator {
                self.dropdown_height / 2
            } else {
                self.dropdown_height
            };
            if relative_y >= acc && relative_y < acc + row_h {
                return i as i32;
            }
            acc += row_h;
        }
        -1
    }

    #[allow(dead_code)]
    fn is_over_dropdown(&self, point: Point) -> bool {
        if self.dropdown_visible && self.active_menu >= 0 {
            let dropdown_rect = self.get_dropdown_rect(self.active_menu);
            dropdown_rect.contains(point)
        } else {
            false
        }
    }

    fn draw_dropdown(&self, context: &mut dyn DrawContext, theme: &Theme, menu_index: i32) {
        let dropdown_rect = self.get_dropdown_rect(menu_index);

        // Draw dropdown background
        context.set_color(theme.colors.surface);
        context.fill_rect(dropdown_rect);

        // Draw dropdown border
        context.set_color(theme.colors.border);
        context.draw_rect(dropdown_rect);

        // Draw dropdown items
        let items = &self.menu_items[menu_index as usize].submenu_items;
        let mut row_offset = 0i32;
        for (i, item) in items.iter().enumerate() {
            let row_h = if item.is_separator {
                self.dropdown_height / 2
            } else {
                self.dropdown_height
            };
            let item_y = dropdown_rect.y() + row_offset;
            let item_rect = Rect::new(
                dropdown_rect.x(),
                item_y,
                dropdown_rect.width(),
                row_h,
            );
            let is_hover = i as i32 == self.dropdown_hover;

            // Draw item background
            if is_hover && item.enabled {
                // Use a bright color to make it obvious
                context.set_color(theme.colors.primary);
                context.fill_rect(item_rect);
            }

            if item.is_separator {
                // Draw separator line
                context.set_color(theme.colors.border);
                let y = item_y + row_h / 2;
                context.draw_line(
                    Point::new(dropdown_rect.x() + 5, y),
                    Point::new(dropdown_rect.right() - 5, y),
                    1,
                );
            } else {
                // Draw text
                let text_color = if !item.enabled {
                    theme.colors.text_disabled
                } else if is_hover {
                    theme.colors.background // White text on hover
                } else {
                    theme.colors.text
                };
                context.set_color(text_color);

                let text_y =
                    item_y + (row_h + theme.typography.font_size_base) / 2 - 2;
                context.draw_text(
                    &item.text,
                    Point::new(dropdown_rect.x() + self.padding, text_y),
                    theme.typography.font_size_base,
                );

                // Draw shortcut if present
                if !item.shortcut.is_empty() {
                    let shortcut_width = item.shortcut.len() as i32 * 6;
                    let shortcut_x = dropdown_rect.right() - self.padding - shortcut_width;
                    context.draw_text(
                        &item.shortcut,
                        Point::new(shortcut_x, text_y),
                        theme.typography.font_size_base,
                    );
                }
            }

            row_offset += row_h;
        }
    }
}

impl Widget for MenuBar {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        // Calculate total width needed for all menu items
        let total_width = self.item_widths.iter().sum::<i32>().max(100);
        Size::new(
            total_width,
            theme.typography.font_size_base + 2 * self.padding,
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        // Just call the menu bar only method - dropdown should be drawn separately
        self.draw_menu_bar_only(context, theme);
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseButton(mouse_event) => {
                let point = mouse_event.position;

                if mouse_event.button == MouseButton::Left && mouse_event.pressed {
                    // Check menu bar clicks
                    let menu_index = self.menu_index_from_point(point);
                    if menu_index >= 0 {
                        if self.active_menu == menu_index {
                            // Close dropdown
                            self.close_dropdown_internal();
                        } else {
                            // Open dropdown
                            self.active_menu = menu_index;
                            self.dropdown_visible = true;
                            self.dropdown_hover = self
                                .first_focusable_in(menu_index)
                                .unwrap_or(-1);
                            self.submenu_open = None;
                            // Bug m16: replace stdout println with gated log.
                            log::trace!("menu: opening dropdown {}", menu_index);
                        }
                        return EventResult::Consumed;
                    }

                    // Check dropdown clicks
                    if self.dropdown_visible && self.active_menu >= 0 {
                        let dropdown_rect = self.get_dropdown_rect(self.active_menu);
                        let dropdown_item = self.dropdown_item_from_point(point, self.active_menu);
                        if dropdown_item >= 0 {
                            let item = &self.menu_items[self.active_menu as usize].submenu_items
                                [dropdown_item as usize];
                            if item.is_separator {
                                // Bug m15: clicks on separators are
                                // swallowed but the dropdown stays open.
                                return EventResult::Consumed;
                            }
                            if !item.enabled {
                                // Bug m14: clicks on disabled items are
                                // swallowed but dropdown stays open.
                                return EventResult::Consumed;
                            }
                            if let Some(handler) = item.on_click {
                                handler();
                            }
                            self.close_dropdown_internal();
                            return EventResult::Consumed;
                        } else if !dropdown_rect.contains(point)
                            && !self.state.bounds.contains(point)
                        {
                            // Click outside both menu bar and dropdown -
                            // dismiss the dropdown and consume the event so
                            // the click doesn't accidentally re-open it.
                            self.close_dropdown_internal();
                            return EventResult::Consumed;
                        }
                    }
                }

                EventResult::Ignored
            }

            Event::KeyPress(KeyPressEvent { key, modifiers, .. }) => {
                // Bug m11: arrow / Enter / mnemonic / Esc keyboard nav.
                if matches!(key, Key::Escape) && self.dropdown_visible {
                    self.close_dropdown_internal();
                    return EventResult::Consumed;
                }

                if self.dropdown_visible && self.active_menu >= 0 {
                    match key {
                        Key::Down => {
                            self.dropdown_hover = self
                                .step_dropdown_hover(self.active_menu, true)
                                .unwrap_or(-1);
                            return EventResult::Consumed;
                        }
                        Key::Up => {
                            self.dropdown_hover = self
                                .step_dropdown_hover(self.active_menu, false)
                                .unwrap_or(-1);
                            return EventResult::Consumed;
                        }
                        Key::Left => {
                            // Cycle to the previous menu and open its dropdown.
                            let n = self.menu_items.len() as i32;
                            if n > 1 {
                                let prev = (self.active_menu - 1 + n) % n;
                                self.active_menu = prev;
                                self.dropdown_hover =
                                    self.first_focusable_in(prev).unwrap_or(-1);
                            }
                            return EventResult::Consumed;
                        }
                        Key::Right => {
                            let n = self.menu_items.len() as i32;
                            if n > 1 {
                                let next = (self.active_menu + 1) % n;
                                self.active_menu = next;
                                self.dropdown_hover =
                                    self.first_focusable_in(next).unwrap_or(-1);
                            }
                            return EventResult::Consumed;
                        }
                        Key::Enter => {
                            if self.dropdown_hover >= 0 {
                                let item = &self.menu_items[self.active_menu as usize]
                                    .submenu_items[self.dropdown_hover as usize];
                                if item.enabled && !item.is_separator {
                                    if let Some(handler) = item.on_click {
                                        handler();
                                    }
                                    self.close_dropdown_internal();
                                }
                            }
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }

                // Bug m11: mnemonic activation. Alt+letter opens the menu
                // whose first character matches.
                if modifiers.contains(erigui_core::Modifiers::ALT) {
                    if let Some(idx) = self.find_mnemonic_match(key) {
                        self.active_menu = idx;
                        self.dropdown_visible = true;
                        self.dropdown_hover = self.first_focusable_in(idx).unwrap_or(-1);
                        return EventResult::Consumed;
                    }
                }
                EventResult::Ignored
            }

            Event::MouseMove(mouse_event) => {
                let point = mouse_event.position;

                // Update hover states
                if self.dropdown_visible && self.active_menu >= 0 {
                    // When dropdown is visible, check dropdown hover first
                    let dropdown_rect = self.get_dropdown_rect(self.active_menu);

                    if dropdown_rect.contains(point) {
                        // Mouse is over dropdown - only update dropdown hover
                        self.dropdown_hover =
                            self.dropdown_item_from_point(point, self.active_menu);
                        self.hover_item = -1; // Clear menu bar hover
                        // Bug m12: open / close submenu on hover.
                        if self.dropdown_hover >= 0 {
                            let item = &self.menu_items[self.active_menu as usize]
                                .submenu_items[self.dropdown_hover as usize];
                            if !item.submenu_items.is_empty() {
                                self.submenu_open =
                                    Some((self.active_menu, self.dropdown_hover));
                            } else {
                                self.submenu_open = None;
                            }
                        } else {
                            self.submenu_open = None;
                        }
                        // Bug m16: replace stdout println with gated log.
                        log::trace!(
                            "menu: dropdown hover -> {}",
                            self.dropdown_hover
                        );
                    } else {
                        // Mouse is not over dropdown - update menu bar hover
                        self.dropdown_hover = -1;
                        self.hover_item = self.menu_index_from_point(point);
                    }
                } else {
                    // Normal menu bar hover when dropdown is closed
                    self.hover_item = self.menu_index_from_point(point);
                    self.dropdown_hover = -1;
                }

                if self.state.bounds.contains(point)
                    || (self.dropdown_visible && self.active_menu >= 0)
                {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
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
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

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

use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, Point, Rect, Size, Theme,
    Widget, WidgetId, WidgetState,
};
use std::any::Any;

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
    dropdown_width: i32,
    dropdown_height: i32,
    padding: i32,
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
            dropdown_width: 200,
            dropdown_height: 28,
            padding: 8,
        }
    }

    pub fn add_menu(&mut self, text: impl Into<String>, items: Vec<MenuItem>) {
        let text = text.into();
        // Better width approximation: 8 pixels per character + extra padding
        let text_width = text.len() as i32 * 8 + 3 * self.padding;
        self.item_widths.push(text_width);

        // Compute dropdown width based on submenu text + shortcuts
        let mut max_width = self.dropdown_width;
        for item in &items {
            if item.is_separator {
                continue;
            }
            let text_len = item.text.len() as i32 * 8;
            let shortcut_len = item.shortcut.len() as i32 * 7;
            let total = text_len + shortcut_len + 4 * self.padding;
            if total > max_width {
                max_width = total;
            }
        }
        self.dropdown_width = max_width;

        let mut menu = MenuItem::new(text);
        menu.submenu_items = items;
        self.menu_items.push(menu);
    }

    pub fn is_dropdown_visible(&self) -> bool {
        self.dropdown_visible
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
        let item_count = self.menu_items[menu_index as usize].submenu_items.len() as i32;
        let dropdown_height = item_count * self.dropdown_height;
        let mut x = menu_rect.x();
        if x + self.dropdown_width > self.state.bounds.right() {
            x = self.state.bounds.right() - self.dropdown_width;
        }
        Rect::new(x, menu_rect.bottom(), self.dropdown_width, dropdown_height)
    }

    fn menu_index_from_point(&self, point: Point) -> i32 {
        if !self.state.bounds.contains(point) {
            return -1;
        }

        let mut x = self.state.bounds.x();
        for i in 0..self.menu_items.len() {
            if point.x >= x && point.x < x + self.item_widths[i] {
                return i as i32;
            }
            x += self.item_widths[i];
        }
        -1
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
        let item_index = relative_y / self.dropdown_height;

        if item_index >= 0
            && item_index < self.menu_items[menu_index as usize].submenu_items.len() as i32
        {
            item_index
        } else {
            -1
        }
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
        for (i, item) in items.iter().enumerate() {
            let item_y = dropdown_rect.y() + i as i32 * self.dropdown_height;
            let item_rect = Rect::new(
                dropdown_rect.x(),
                item_y,
                dropdown_rect.width(),
                self.dropdown_height,
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
                let y = item_y + self.dropdown_height / 2;
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
                    item_y + (self.dropdown_height + theme.typography.font_size_base) / 2 - 2;
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
                            self.active_menu = -1;
                            self.dropdown_visible = false;
                        } else {
                            // Open dropdown
                            self.active_menu = menu_index;
                            self.dropdown_visible = true;
                            println!("Opening dropdown for menu {}", menu_index);
                        }
                        return EventResult::Consumed;
                    }

                    // Check dropdown clicks
                    if self.dropdown_visible && self.active_menu >= 0 {
                        let dropdown_item = self.dropdown_item_from_point(point, self.active_menu);
                        if dropdown_item >= 0 {
                            let item = &self.menu_items[self.active_menu as usize].submenu_items
                                [dropdown_item as usize];
                            if item.enabled && !item.is_separator {
                                if let Some(handler) = item.on_click {
                                    handler();
                                }
                                self.dropdown_visible = false;
                                self.active_menu = -1;
                                return EventResult::Consumed;
                            }
                        } else {
                            // Click outside dropdown - close it
                            self.dropdown_visible = false;
                            self.active_menu = -1;
                        }
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
                        println!("Mouse over dropdown, item hover: {}", self.dropdown_hover);
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
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

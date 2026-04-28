use crate::Icon;
use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent, Point, Rect,
    Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

/// Callback fired when the user navigates to a breadcrumb item.
/// Arguments: item id, item index.
type NavigateCallback = Box<dyn FnMut(&str, usize)>;

#[derive(Clone)]
pub struct BreadcrumbItem {
    pub text: String,
    pub id: String,
    pub enabled: bool,
}

impl BreadcrumbItem {
    pub fn new(text: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            id: id.into(),
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub struct Breadcrumb {
    state: WidgetState,
    items: Vec<BreadcrumbItem>,
    separator: String,
    show_home_icon: bool,
    home_item: Option<BreadcrumbItem>,

    // Visual state
    hovered_index: Option<usize>,
    pressed_index: Option<usize>,
    item_rects: Vec<Rect>,

    // Style
    spacing: i32,
    padding: i32,

    // Callbacks
    on_navigate: Option<NavigateCallback>,
}

impl Breadcrumb {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            separator: "/".to_string(),
            show_home_icon: true,
            home_item: Some(BreadcrumbItem::new("Home", "home")),
            hovered_index: None,
            pressed_index: None,
            item_rects: Vec::new(),
            spacing: 8,
            padding: 12,
            on_navigate: None,
        }
    }

    pub fn with_items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }

    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn with_home_icon(mut self, show: bool) -> Self {
        self.show_home_icon = show;
        self
    }

    pub fn with_home_item(mut self, item: BreadcrumbItem) -> Self {
        self.home_item = Some(item);
        self
    }

    pub fn with_on_navigate<F: FnMut(&str, usize) + 'static>(mut self, f: F) -> Self {
        self.on_navigate = Some(Box::new(f));
        self
    }

    pub fn push(&mut self, item: BreadcrumbItem) {
        self.items.push(item);
    }

    pub fn pop(&mut self) -> Option<BreadcrumbItem> {
        self.items.pop()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn set_path(&mut self, items: Vec<BreadcrumbItem>) {
        self.items = items;
    }

    pub fn get_path(&self) -> Vec<String> {
        let mut path = Vec::new();

        if let Some(home) = &self.home_item {
            path.push(home.text.clone());
        }

        for item in &self.items {
            path.push(item.text.clone());
        }

        path
    }

    pub fn navigate_to(&mut self, index: usize) {
        if self.home_item.is_some() && index == 0 {
            // Navigate to home
            self.items.clear();
            if let Some(callback) = &mut self.on_navigate {
                if let Some(home) = &self.home_item {
                    callback(&home.id, 0);
                }
            }
        } else {
            // Navigate to specific item
            let actual_index = if self.home_item.is_some() {
                index - 1
            } else {
                index
            };

            if actual_index < self.items.len() {
                let item_id = self.items[actual_index].id.clone();
                self.items.truncate(actual_index + 1);

                if let Some(callback) = &mut self.on_navigate {
                    callback(&item_id, index);
                }
            }
        }
    }

    // Internal drawing helper — flat positional params keep the call sites
    // (the per-item draw loop in `draw`) readable.
    #[allow(clippy::too_many_arguments)]
    fn draw_item(
        &self,
        context: &mut dyn DrawContext,
        theme: &Theme,
        text: &str,
        rect: Rect,
        is_hovered: bool,
        is_pressed: bool,
        enabled: bool,
    ) {
        let text_color = if !enabled {
            theme.colors.text_disabled
        } else if is_pressed {
            theme.colors.primary_active
        } else if is_hovered {
            theme.colors.primary
        } else {
            theme.colors.text
        };

        context.set_color(text_color);
        context.draw_text(
            text,
            Point::new(
                rect.x() + self.padding,
                rect.center().y + theme.typography.font_size_base / 2 - 2,
            ),
            theme.typography.font_size_base,
        );

        // Draw underline on hover
        if is_hovered && enabled {
            context.fill_rect(Rect::new(
                rect.x() + self.padding,
                rect.bottom() - 2,
                rect.width() - self.padding * 2,
                1,
            ));
        }
    }

    fn draw_separator(&self, context: &mut dyn DrawContext, theme: &Theme, position: Point) {
        context.set_color(theme.colors.text_secondary);
        context.draw_text(
            &self.separator,
            Point::new(
                position.x,
                position.y + theme.typography.font_size_base / 2 - 2,
            ),
            theme.typography.font_size_base,
        );
    }
}

impl Widget for Breadcrumb {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let mut width = 0;
        let height = theme.typography.font_size_base + self.padding * 2;

        // Measure home icon/text
        if let Some(home) = &self.home_item {
            if self.show_home_icon {
                width += 20 + self.padding * 2; // Icon size + padding
            } else {
                let text_size = theme.typography.font_size_base * home.text.len() as i32 / 2;
                width += text_size + self.padding * 2;
            }
        }

        // Measure items
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 || self.home_item.is_some() {
                // Add separator width
                let sep_width = theme.typography.font_size_base * self.separator.len() as i32 / 2;
                width += self.spacing + sep_width + self.spacing;
            }

            let text_size = theme.typography.font_size_base * item.text.len() as i32 / 2;
            width += text_size + self.padding * 2;
        }

        Size::new(width, height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        self.item_rects.clear();

        let mut x = rect.x();
        let height = rect.height();

        // Layout home item
        if let Some(home) = &self.home_item {
            let width = if self.show_home_icon {
                20 + self.padding * 2
            } else {
                let text_size = theme.typography.font_size_base * home.text.len() as i32 / 2;
                text_size + self.padding * 2
            };

            self.item_rects.push(Rect::new(x, rect.y(), width, height));
            x += width;
        }

        // Layout items
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 || self.home_item.is_some() {
                // Add separator spacing
                let sep_width = theme.typography.font_size_base * self.separator.len() as i32 / 2;
                x += self.spacing + sep_width + self.spacing;
            }

            let text_size = theme.typography.font_size_base * item.text.len() as i32 / 2;
            let width = text_size + self.padding * 2;

            self.item_rects.push(Rect::new(x, rect.y(), width, height));
            x += width;
        }
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let mut item_index = 0;

        // Draw home item
        if let Some(home) = &self.home_item {
            if item_index < self.item_rects.len() {
                let rect = self.item_rects[item_index];
                let is_hovered = self.hovered_index == Some(item_index);
                let is_pressed = self.pressed_index == Some(item_index);

                if self.show_home_icon {
                    // Draw home icon
                    let icon_rect =
                        Rect::new(rect.x() + self.padding, rect.center().y - 10, 20, 20);

                    let icon_color = if !home.enabled {
                        theme.colors.text_disabled
                    } else if is_pressed {
                        theme.colors.primary_active
                    } else if is_hovered {
                        theme.colors.primary
                    } else {
                        theme.colors.text
                    };

                    Icon::draw_home(context, icon_rect, icon_color);

                    // Draw underline on hover
                    if is_hovered && home.enabled {
                        context.set_color(icon_color);
                        context.fill_rect(Rect::new(
                            rect.x() + self.padding,
                            rect.bottom() - 2,
                            20,
                            1,
                        ));
                    }
                } else {
                    self.draw_item(
                        context,
                        theme,
                        &home.text,
                        rect,
                        is_hovered,
                        is_pressed,
                        home.enabled,
                    );
                }

                item_index += 1;
            }
        }

        // Draw items
        for (i, item) in self.items.iter().enumerate() {
            // Draw separator
            if i > 0 || self.home_item.is_some() {
                let sep_x = if item_index > 0 && item_index - 1 < self.item_rects.len() {
                    self.item_rects[item_index - 1].right() + self.spacing
                } else {
                    self.state.bounds.x()
                };

                self.draw_separator(
                    context,
                    theme,
                    Point::new(
                        sep_x,
                        self.state.bounds.center().y - theme.typography.font_size_base / 2 + 2,
                    ),
                );
            }

            // Draw item
            if item_index < self.item_rects.len() {
                let rect = self.item_rects[item_index];
                let is_hovered = self.hovered_index == Some(item_index);
                let is_pressed = self.pressed_index == Some(item_index);

                self.draw_item(
                    context,
                    theme,
                    &item.text,
                    rect,
                    is_hovered,
                    is_pressed,
                    item.enabled,
                );

                item_index += 1;
            }
        }
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
                ..
            }) => {
                if !self.state.bounds.contains(*position) {
                    return EventResult::Ignored;
                }

                // Check which item was clicked
                for (i, rect) in self.item_rects.iter().enumerate() {
                    if rect.contains(*position) {
                        let is_enabled = if i == 0 && self.home_item.is_some() {
                            self.home_item.as_ref().unwrap().enabled
                        } else {
                            let actual_index = if self.home_item.is_some() { i - 1 } else { i };
                            actual_index < self.items.len() && self.items[actual_index].enabled
                        };

                        if is_enabled {
                            if *pressed {
                                self.pressed_index = Some(i);
                            } else if self.pressed_index == Some(i) {
                                // Only navigate if not the last item
                                let is_last = if self.home_item.is_some() {
                                    i == self.items.len()
                                } else {
                                    i == self.items.len() - 1
                                };

                                if !is_last {
                                    self.navigate_to(i);
                                }
                                self.pressed_index = None;
                            }
                            return EventResult::Consumed;
                        }
                    }
                }

                if !pressed {
                    self.pressed_index = None;
                }
            }
            Event::MouseMove(move_event) => {
                if !self.state.bounds.contains(move_event.position) {
                    self.hovered_index = None;
                    return EventResult::Ignored;
                }

                // Check which item is hovered
                let mut found_hover = false;
                for (i, rect) in self.item_rects.iter().enumerate() {
                    if rect.contains(move_event.position) {
                        self.hovered_index = Some(i);
                        found_hover = true;
                        break;
                    }
                }

                if !found_hover {
                    self.hovered_index = None;
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
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

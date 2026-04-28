use crate::tooltip::TooltipState;
use crate::{ButtonIcon, MenuItem};
use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent, Point, Rect,
    Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

pub enum ToolbarItem {
    Button {
        id: WidgetId,
        text: String,
        icon: Option<ButtonIcon>,
        tooltip: Option<TooltipState>,
        enabled: bool,
        on_click: Option<Box<dyn FnMut()>>,
    },
    Separator,
    DropdownButton {
        id: WidgetId,
        text: String,
        icon: Option<ButtonIcon>,
        tooltip: Option<TooltipState>,
        menu_items: Vec<MenuItem>,
        enabled: bool,
    },
    ToggleButton {
        id: WidgetId,
        text: String,
        icon: Option<ButtonIcon>,
        tooltip: Option<TooltipState>,
        pressed: bool,
        group: Option<u32>,
        enabled: bool,
        on_toggle: Option<Box<dyn FnMut(bool)>>,
    },
    Custom {
        widget: Box<dyn Widget>,
    },
}

impl ToolbarItem {
    /// Attach a hover tooltip to this item. No-op for `Separator` and
    /// `Custom`, which don't carry a tooltip slot — composing toolbars
    /// with custom widgets should set tooltips on the inner widget.
    pub fn with_tooltip(mut self, text: impl Into<String>) -> Self {
        match &mut self {
            ToolbarItem::Button { tooltip, .. }
            | ToolbarItem::DropdownButton { tooltip, .. }
            | ToolbarItem::ToggleButton { tooltip, .. } => {
                *tooltip = Some(TooltipState::new(text));
            }
            ToolbarItem::Separator | ToolbarItem::Custom { .. } => {}
        }
        self
    }

    /// Variant of `with_tooltip` that lets the caller pre-configure
    /// position and delay on the `TooltipState`.
    pub fn with_tooltip_state(mut self, ts: TooltipState) -> Self {
        match &mut self {
            ToolbarItem::Button { tooltip, .. }
            | ToolbarItem::DropdownButton { tooltip, .. }
            | ToolbarItem::ToggleButton { tooltip, .. } => {
                *tooltip = Some(ts);
            }
            ToolbarItem::Separator | ToolbarItem::Custom { .. } => {}
        }
        self
    }
}

pub struct Toolbar {
    state: WidgetState,
    items: Vec<ToolbarItem>,
    orientation: Orientation,
    item_spacing: i32,
    item_padding: i32,
    show_text: bool,
    overflow_items: Vec<usize>, // Indices of items that don't fit
    overflow_button_visible: bool,

    // Interaction state
    hovered_item: Option<usize>,
    active_dropdown: Option<usize>,
}

impl Toolbar {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            orientation: Orientation::Horizontal,
            item_spacing: 2,
            item_padding: 4,
            show_text: true,
            overflow_items: Vec::new(),
            overflow_button_visible: false,
            hovered_item: None,
            active_dropdown: None,
        }
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_show_text(mut self, show_text: bool) -> Self {
        self.show_text = show_text;
        self
    }

    pub fn add_button(
        mut self,
        text: impl Into<String>,
        icon: Option<ButtonIcon>,
        on_click: impl FnMut() + 'static,
    ) -> Self {
        self.items.push(ToolbarItem::Button {
            id: WidgetId::default(),
            text: text.into(),
            icon,
            tooltip: None,
            enabled: true,
            on_click: Some(Box::new(on_click)),
        });
        self
    }

    pub fn add_separator(mut self) -> Self {
        self.items.push(ToolbarItem::Separator);
        self
    }

    pub fn add_dropdown_button(
        mut self,
        text: impl Into<String>,
        icon: Option<ButtonIcon>,
        menu_items: Vec<MenuItem>,
    ) -> Self {
        self.items.push(ToolbarItem::DropdownButton {
            id: WidgetId::default(),
            text: text.into(),
            icon,
            tooltip: None,
            menu_items,
            enabled: true,
        });
        self
    }

    pub fn add_toggle_button(
        mut self,
        text: impl Into<String>,
        icon: Option<ButtonIcon>,
        group: Option<u32>,
        on_toggle: impl FnMut(bool) + 'static,
    ) -> Self {
        self.items.push(ToolbarItem::ToggleButton {
            id: WidgetId::default(),
            text: text.into(),
            icon,
            tooltip: None,
            pressed: false,
            group,
            enabled: true,
            on_toggle: Some(Box::new(on_toggle)),
        });
        self
    }

    /// Attach a hover tooltip to the most-recently-added toolbar item.
    /// Call this immediately after `add_button` / `add_dropdown_button`
    /// / `add_toggle_button` in a builder chain. No-op if the last item
    /// is a `Separator` or `Custom`, or if the toolbar is empty.
    pub fn with_last_tooltip(mut self, text: impl Into<String>) -> Self {
        if let Some(item) = self.items.last_mut() {
            match item {
                ToolbarItem::Button { tooltip, .. }
                | ToolbarItem::DropdownButton { tooltip, .. }
                | ToolbarItem::ToggleButton { tooltip, .. } => {
                    *tooltip = Some(TooltipState::new(text));
                }
                ToolbarItem::Separator | ToolbarItem::Custom { .. } => {}
            }
        }
        self
    }

    /// Like `with_last_tooltip` but takes a pre-configured `TooltipState`
    /// — use when defaults aren't right (custom delay, position).
    pub fn with_last_tooltip_state(mut self, ts: TooltipState) -> Self {
        if let Some(item) = self.items.last_mut() {
            match item {
                ToolbarItem::Button { tooltip, .. }
                | ToolbarItem::DropdownButton { tooltip, .. }
                | ToolbarItem::ToggleButton { tooltip, .. } => {
                    *tooltip = Some(ts);
                }
                ToolbarItem::Separator | ToolbarItem::Custom { .. } => {}
            }
        }
        self
    }

    /// Read access to a single item's tooltip — used by tests to
    /// verify show/hide state. Returns `None` if the index is out of
    /// range or the item is a `Separator` / `Custom` (no tooltip slot).
    pub fn item_tooltip(&self, index: usize) -> Option<&TooltipState> {
        match self.items.get(index)? {
            ToolbarItem::Button { tooltip, .. }
            | ToolbarItem::DropdownButton { tooltip, .. }
            | ToolbarItem::ToggleButton { tooltip, .. } => tooltip.as_ref(),
            _ => None,
        }
    }

    /// Walk the visible (non-overflow) toolbar items and compute the
    /// draw-rect for each. Used by `draw`, `handle_event`, and the
    /// tooltip dispatch loop so all three agree on hit geometry. The
    /// draw rect matches what `draw` paints (chrome-aligned), so
    /// tooltips anchor cleanly to the visible button.
    fn item_draw_rects(&self, theme: &Theme) -> Vec<(usize, Rect)> {
        let mut out = Vec::with_capacity(self.items.len());
        let mut current_pos = match self.orientation {
            Orientation::Horizontal => self.state.bounds.x(),
            Orientation::Vertical => self.state.bounds.y(),
        };
        for (i, item) in self.items.iter().enumerate() {
            if self.overflow_items.contains(&i) {
                continue;
            }
            let item_size = self.calculate_item_size(item, theme);
            let item_rect = match self.orientation {
                Orientation::Horizontal => Rect::new(
                    current_pos,
                    self.state.bounds.y() + (self.state.bounds.height() - item_size.height) / 2,
                    item_size.width,
                    item_size.height,
                ),
                Orientation::Vertical => Rect::new(
                    self.state.bounds.x() + (self.state.bounds.width() - item_size.width) / 2,
                    current_pos,
                    item_size.width,
                    item_size.height,
                ),
            };
            out.push((i, item_rect));

            current_pos += match self.orientation {
                Orientation::Horizontal => item_size.width,
                Orientation::Vertical => item_size.height,
            };
            if i < self.items.len() - 1 && !self.overflow_items.contains(&(i + 1)) {
                current_pos += self.item_spacing;
            }
        }
        out
    }

    fn calculate_item_size(&self, item: &ToolbarItem, theme: &Theme) -> Size {
        match item {
            ToolbarItem::Button { text, icon, .. }
            | ToolbarItem::DropdownButton { text, icon, .. }
            | ToolbarItem::ToggleButton { text, icon, .. } => {
                // Reserve a larger icon box so PNG glyphs stay visible.
                let icon_size = if icon.is_some() { 28 } else { 0 };
                let text_width = if self.show_text && !text.is_empty() {
                    text.len() as i32 * theme.typography.font_size_base * 3 / 5
                } else {
                    0
                };
                let spacing = if icon.is_some() && self.show_text && !text.is_empty() {
                    theme.spacing.gap_small
                } else {
                    0
                };

                let width = icon_size + spacing + text_width + self.item_padding * 2;
                let height = icon_size.max(theme.typography.font_size_base) + self.item_padding * 2;

                // Add space for dropdown arrow
                let width = if matches!(item, ToolbarItem::DropdownButton { .. }) {
                    width + 12
                } else {
                    width
                };

                Size::new(width, height)
            }
            ToolbarItem::Separator => match self.orientation {
                Orientation::Horizontal => Size::new(3, 24),
                Orientation::Vertical => Size::new(24, 3),
            },
            ToolbarItem::Custom { widget } => widget.measure(&LayoutConstraints::default(), theme),
        }
    }

    // Internal drawing helper — flat positional params keep the toolbar
    // item-drawing loop readable.
    #[allow(clippy::too_many_arguments)]
    fn draw_button(
        &self,
        context: &mut dyn DrawContext,
        theme: &Theme,
        rect: Rect,
        text: &str,
        icon: &Option<ButtonIcon>,
        enabled: bool,
        pressed: bool,
        hovered: bool,
    ) {
        // Draw button background
        let bg_color = if !enabled {
            theme.colors.surface_variant
        } else if pressed {
            theme.colors.primary_active
        } else if hovered {
            theme.colors.primary_hover
        } else {
            theme.colors.surface
        };

        context.set_color(bg_color);
        context.fill_rect(rect);

        // Draw border on hover or press
        if enabled && (hovered || pressed) {
            context.set_color(theme.colors.primary);
            context.draw_rect(rect);
        }

        let mut x = rect.x() + self.item_padding;
        let center_y = rect.center().y;

        // Draw icon
        if let Some(icon) = icon {
            let icon_size = 24;
            let icon_rect = Rect::new(x, center_y - icon_size / 2, icon_size, icon_size);
            let icon_color = if !enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.text
            };

            match icon {
                ButtonIcon::Back => {
                    crate::icon::Icon::draw_back_arrow(context, icon_rect, icon_color)
                }
                ButtonIcon::Forward => {
                    crate::icon::Icon::draw_forward_arrow(context, icon_rect, icon_color)
                }
                ButtonIcon::Up => crate::icon::Icon::draw_up_arrow(context, icon_rect, icon_color),
                ButtonIcon::Home => crate::icon::Icon::draw_home(context, icon_rect, icon_color),
                ButtonIcon::Refresh => {
                    crate::icon::Icon::draw_refresh(context, icon_rect, icon_color)
                }
                ButtonIcon::Folder => {
                    crate::icon::Icon::draw_folder(context, icon_rect, icon_color)
                }
                ButtonIcon::FolderNew => {
                    crate::icon::Icon::draw_folder_new(context, icon_rect, icon_color)
                }
                ButtonIcon::Delete => {
                    crate::icon::Icon::draw_delete(context, icon_rect, icon_color)
                }
                ButtonIcon::File => crate::icon::Icon::draw_file(context, icon_rect, icon_color),
                ButtonIcon::Search => {
                    crate::icon::Icon::draw_search(context, icon_rect, icon_color)
                }
                ButtonIcon::Custom(draw_fn) => draw_fn(context, icon_rect, icon_color),
                ButtonIcon::Bitmap {
                    width,
                    height,
                    data,
                } => {
                    // Center the bitmap with a small inset and keep aspect.
                    let avail_w = icon_rect.width() - 6;
                    let avail_h = icon_rect.height() - 6;
                    let scale =
                        (avail_w as f32 / *width as f32).min(avail_h as f32 / *height as f32);
                    let draw_w = (*width as f32 * scale) as i32;
                    let draw_h = (*height as f32 * scale) as i32;
                    let draw_rect = Rect::new(
                        icon_rect.x() + (icon_rect.width() - draw_w) / 2,
                        icon_rect.y() + (icon_rect.height() - draw_h) / 2,
                        draw_w,
                        draw_h,
                    );
                    context.draw_image_rgba(draw_rect, *width, *height, data);
                }
            }

            x += icon_size + theme.spacing.gap_small;
        }

        // Draw text
        if self.show_text && !text.is_empty() {
            let text_color = if !enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.text
            };

            context.set_color(text_color);
            context.draw_text(
                text,
                Point::new(x, center_y + theme.typography.font_size_base / 2 - 2),
                theme.typography.font_size_base,
            );
        }
    }

    fn draw_dropdown_arrow(
        &self,
        context: &mut dyn DrawContext,
        theme: &Theme,
        rect: Rect,
        enabled: bool,
    ) {
        let arrow_x = rect.right() - 10;
        let arrow_y = rect.center().y;

        let color = if enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };

        context.set_color(color);
        context.fill_rect(Rect::new(arrow_x, arrow_y - 1, 5, 1));
        context.fill_rect(Rect::new(arrow_x + 1, arrow_y, 3, 1));
        context.fill_rect(Rect::new(arrow_x + 2, arrow_y + 1, 1, 1));
    }
}

impl Widget for Toolbar {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let mut total_size = Size::ZERO;

        for (i, item) in self.items.iter().enumerate() {
            let item_size = self.calculate_item_size(item, theme);

            match self.orientation {
                Orientation::Horizontal => {
                    total_size.width += item_size.width;
                    if i > 0 {
                        total_size.width += self.item_spacing;
                    }
                    total_size.height = total_size.height.max(item_size.height);
                }
                Orientation::Vertical => {
                    total_size.width = total_size.width.max(item_size.width);
                    total_size.height += item_size.height;
                    if i > 0 {
                        total_size.height += self.item_spacing;
                    }
                }
            }
        }

        // Constrain to max size
        if let Some(max_width) = constraints.max_width {
            total_size.width = total_size.width.min(max_width);
        }
        if let Some(max_height) = constraints.max_height {
            total_size.height = total_size.height.min(max_height);
        }

        total_size
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;

        // Calculate which items fit
        self.overflow_items.clear();
        let mut current_pos = match self.orientation {
            Orientation::Horizontal => rect.x(),
            Orientation::Vertical => rect.y(),
        };

        let overflow_button_size = 20; // Space for overflow button if needed
        let available_space = match self.orientation {
            Orientation::Horizontal => rect.width() - overflow_button_size,
            Orientation::Vertical => rect.height() - overflow_button_size,
        };

        for (i, item) in self.items.iter().enumerate() {
            let item_size = self.calculate_item_size(item, theme);
            let item_extent = match self.orientation {
                Orientation::Horizontal => item_size.width,
                Orientation::Vertical => item_size.height,
            };

            let new_pos = current_pos + item_extent + if i > 0 { self.item_spacing } else { 0 };
            let item_end = match self.orientation {
                Orientation::Horizontal => new_pos - rect.x(),
                Orientation::Vertical => new_pos - rect.y(),
            };

            if item_end > available_space {
                self.overflow_items.push(i);
            } else {
                current_pos = new_pos;
            }
        }

        self.overflow_button_visible = !self.overflow_items.is_empty();
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        let mut current_pos = match self.orientation {
            Orientation::Horizontal => self.state.bounds.x(),
            Orientation::Vertical => self.state.bounds.y(),
        };

        // Draw items
        for (i, item) in self.items.iter().enumerate() {
            if self.overflow_items.contains(&i) {
                continue;
            }

            let item_size = self.calculate_item_size(item, theme);
            let item_rect = match self.orientation {
                Orientation::Horizontal => Rect::new(
                    current_pos,
                    self.state.bounds.y() + (self.state.bounds.height() - item_size.height) / 2,
                    item_size.width,
                    item_size.height,
                ),
                Orientation::Vertical => Rect::new(
                    self.state.bounds.x() + (self.state.bounds.width() - item_size.width) / 2,
                    current_pos,
                    item_size.width,
                    item_size.height,
                ),
            };

            match item {
                ToolbarItem::Button {
                    text,
                    icon,
                    enabled,
                    ..
                } => {
                    let hovered = self.hovered_item == Some(i);
                    self.draw_button(
                        context, theme, item_rect, text, icon, *enabled, false, hovered,
                    );
                }
                ToolbarItem::DropdownButton {
                    text,
                    icon,
                    enabled,
                    ..
                } => {
                    let hovered = self.hovered_item == Some(i);
                    let active = self.active_dropdown == Some(i);
                    self.draw_button(
                        context, theme, item_rect, text, icon, *enabled, active, hovered,
                    );
                    self.draw_dropdown_arrow(context, theme, item_rect, *enabled);
                }
                ToolbarItem::ToggleButton {
                    text,
                    icon,
                    pressed,
                    enabled,
                    ..
                } => {
                    let hovered = self.hovered_item == Some(i);
                    self.draw_button(
                        context, theme, item_rect, text, icon, *enabled, *pressed, hovered,
                    );
                }
                ToolbarItem::Separator => {
                    context.set_color(theme.colors.border);
                    match self.orientation {
                        Orientation::Horizontal => {
                            let x = item_rect.center().x;
                            context.fill_rect(Rect::new(
                                x,
                                item_rect.y() + 4,
                                1,
                                item_rect.height() - 8,
                            ));
                        }
                        Orientation::Vertical => {
                            let y = item_rect.center().y;
                            context.fill_rect(Rect::new(
                                item_rect.x() + 4,
                                y,
                                item_rect.width() - 8,
                                1,
                            ));
                        }
                    }
                }
                ToolbarItem::Custom { widget } => {
                    widget.draw(context, theme);
                }
            }

            current_pos += match self.orientation {
                Orientation::Horizontal => item_size.width,
                Orientation::Vertical => item_size.height,
            };

            if i < self.items.len() - 1 && !self.overflow_items.contains(&(i + 1)) {
                current_pos += self.item_spacing;
            }
        }

        // Draw overflow button if needed
        if self.overflow_button_visible {
            let overflow_rect = match self.orientation {
                Orientation::Horizontal => Rect::new(
                    self.state.bounds.right() - 20,
                    self.state.bounds.y(),
                    20,
                    self.state.bounds.height(),
                ),
                Orientation::Vertical => Rect::new(
                    self.state.bounds.x(),
                    self.state.bounds.bottom() - 20,
                    self.state.bounds.width(),
                    20,
                ),
            };

            context.set_color(theme.colors.surface_variant);
            context.fill_rect(overflow_rect);

            // Draw >> or vv symbol
            context.set_color(theme.colors.text);
            let center = overflow_rect.center();
            match self.orientation {
                Orientation::Horizontal => {
                    context.fill_rect(Rect::new(center.x - 4, center.y - 1, 3, 2));
                    context.fill_rect(Rect::new(center.x - 1, center.y - 1, 3, 2));
                }
                Orientation::Vertical => {
                    context.fill_rect(Rect::new(center.x - 1, center.y - 4, 2, 3));
                    context.fill_rect(Rect::new(center.x - 1, center.y - 1, 2, 3));
                }
            }
        }

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);

        // Draw tooltips last so they overlay all toolbar chrome.
        // Each item's tooltip anchors to the same draw rect computed
        // by `item_draw_rects`, so the tooltip floats above the
        // button it belongs to.
        let rects = self.item_draw_rects(theme);
        for (i, item_rect) in rects {
            let tooltip = match &self.items[i] {
                ToolbarItem::Button { tooltip, .. }
                | ToolbarItem::DropdownButton { tooltip, .. }
                | ToolbarItem::ToggleButton { tooltip, .. } => tooltip.as_ref(),
                _ => None,
            };
            if let Some(t) = tooltip {
                t.draw(context, theme, item_rect);
            }
        }
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            // Hidden / disabled toolbar — clear any latched tooltip
            // state so it doesn't pop up next time we re-enable.
            for item in &mut self.items {
                let tooltip = match item {
                    ToolbarItem::Button { tooltip, .. }
                    | ToolbarItem::DropdownButton { tooltip, .. }
                    | ToolbarItem::ToggleButton { tooltip, .. } => tooltip.as_mut(),
                    _ => None,
                };
                if let Some(t) = tooltip {
                    t.hide();
                }
            }
            return EventResult::Ignored;
        }

        // Dispatch tooltip events to each visible item using its draw
        // rect. Done before the main match so a button press both
        // dismisses the tooltip and fires the click handler in the
        // same event. Per-item disabled state hides the tooltip
        // (matches Button's behaviour).
        let rects = self.item_draw_rects(theme);
        for (i, item_rect) in &rects {
            let item = &mut self.items[*i];
            let (tooltip, item_enabled) = match item {
                ToolbarItem::Button { tooltip, enabled, .. }
                | ToolbarItem::DropdownButton { tooltip, enabled, .. }
                | ToolbarItem::ToggleButton { tooltip, enabled, .. } => {
                    (tooltip.as_mut(), *enabled)
                }
                _ => (None, true),
            };
            if let Some(t) = tooltip {
                if !item_enabled {
                    t.hide();
                } else {
                    t.update_on_event(event, *item_rect);
                }
            }
        }

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                if !self.state.bounds.contains(*position) {
                    self.active_dropdown = None;
                    return EventResult::Ignored;
                }

                // Find which item was clicked
                let mut clicked_item: Option<usize> = None;
                let mut clicked_action = None;

                let mut current_pos = match self.orientation {
                    Orientation::Horizontal => self.state.bounds.x(),
                    Orientation::Vertical => self.state.bounds.y(),
                };

                for (i, item) in self.items.iter().enumerate() {
                    if self.overflow_items.contains(&i) {
                        continue;
                    }

                    let item_size = self.calculate_item_size(item, theme);
                    let item_rect = match self.orientation {
                        Orientation::Horizontal => Rect::new(
                            current_pos,
                            self.state.bounds.y(),
                            item_size.width,
                            self.state.bounds.height(),
                        ),
                        Orientation::Vertical => Rect::new(
                            self.state.bounds.x(),
                            current_pos,
                            self.state.bounds.width(),
                            item_size.height,
                        ),
                    };

                    if item_rect.contains(*position) {
                        clicked_item = Some(i);
                        match item {
                            ToolbarItem::Button { enabled, .. } => {
                                if *enabled {
                                    clicked_action = Some("button");
                                }
                            }
                            ToolbarItem::DropdownButton { enabled, .. } => {
                                if *enabled {
                                    clicked_action = Some("dropdown");
                                }
                            }
                            ToolbarItem::ToggleButton { enabled, .. } => {
                                if *enabled {
                                    clicked_action = Some("toggle");
                                }
                            }
                            _ => {}
                        }
                        break;
                    }

                    current_pos += match self.orientation {
                        Orientation::Horizontal => item_size.width,
                        Orientation::Vertical => item_size.height,
                    };

                    if i < self.items.len() - 1 {
                        current_pos += self.item_spacing;
                    }
                }

                // Handle the click action
                if let Some(index) = clicked_item {
                    match clicked_action {
                        Some("button") => {
                            if let ToolbarItem::Button {
                                on_click: Some(callback),
                                ..
                            } = &mut self.items[index]
                            {
                                callback();
                            }
                        }
                        Some("dropdown") => {
                            self.active_dropdown = Some(index);
                        }
                        Some("toggle") => {
                            // First toggle the button
                            let mut group_id = None;
                            let mut was_pressed = false;

                            if let ToolbarItem::ToggleButton { pressed, group, .. } =
                                &mut self.items[index]
                            {
                                *pressed = !*pressed;
                                was_pressed = *pressed;
                                group_id = *group;
                            }

                            // Handle toggle groups
                            if let Some(gid) = group_id.filter(|_| was_pressed) {
                                for (j, item) in self.items.iter_mut().enumerate() {
                                    if index != j {
                                        if let ToolbarItem::ToggleButton {
                                            pressed, group, ..
                                        } = item
                                        {
                                            if *group == Some(gid) {
                                                *pressed = false;
                                            }
                                        }
                                    }
                                }
                            }

                            // Call the callback
                            if let ToolbarItem::ToggleButton {
                                on_toggle: Some(callback),
                                ..
                            } = &mut self.items[index]
                            {
                                callback(was_pressed);
                            }
                        }
                        _ => {}
                    }
                    return EventResult::Consumed;
                }
            }
            Event::MouseMove(move_event) => {
                if !self.state.bounds.contains(move_event.position) {
                    self.hovered_item = None;
                    return EventResult::Ignored;
                }

                // Find which item is hovered
                let mut current_pos = match self.orientation {
                    Orientation::Horizontal => self.state.bounds.x(),
                    Orientation::Vertical => self.state.bounds.y(),
                };

                self.hovered_item = None;

                for (i, item) in self.items.iter().enumerate() {
                    if self.overflow_items.contains(&i) {
                        continue;
                    }

                    let item_size = self.calculate_item_size(item, theme);
                    let item_rect = match self.orientation {
                        Orientation::Horizontal => Rect::new(
                            current_pos,
                            self.state.bounds.y(),
                            item_size.width,
                            self.state.bounds.height(),
                        ),
                        Orientation::Vertical => Rect::new(
                            self.state.bounds.x(),
                            current_pos,
                            self.state.bounds.width(),
                            item_size.height,
                        ),
                    };

                    if item_rect.contains(move_event.position) {
                        self.hovered_item = Some(i);
                        return EventResult::Consumed;
                    }

                    current_pos += match self.orientation {
                        Orientation::Horizontal => item_size.width,
                        Orientation::Vertical => item_size.height,
                    };

                    if i < self.items.len() - 1 {
                        current_pos += self.item_spacing;
                    }
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

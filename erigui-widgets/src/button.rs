use crate::icon::Icon;
use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Toolbar,
}

pub struct Button {
    state: WidgetState,
    text: String,
    icon: Option<ButtonIcon>,
    on_click: Option<Box<dyn FnMut()>>,
    pressed: bool,
    hovered: bool,
    style: ButtonStyle,
}

pub enum ButtonIcon {
    Back,
    Forward,
    Up,
    Home,
    Refresh,
    Folder,
    FolderNew,
    Delete,
    File,
    Search,
    Custom(fn(&mut dyn DrawContext, Rect, Color)),
    Bitmap {
        width: i32,
        height: i32,
        data: Vec<u8>,
    },
}

impl Button {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            state: WidgetState::new(id),
            text: text.into(),
            icon: None,
            on_click: None,
            pressed: false,
            hovered: false,
            style: ButtonStyle::Primary,
        }
    }

    pub fn with_icon(mut self, icon: ButtonIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_on_click<F: FnMut() + 'static>(mut self, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.state.tooltip = Some(tooltip.into());
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let padding = theme.spacing.padding;
        let mut width = 0;
        let mut height = theme.typography.font_size_base;

        // Icon size
        if self.icon.is_some() {
            let icon_size = 24;
            width += icon_size;
            height = height.max(icon_size);
            if !self.text.is_empty() {
                width += theme.spacing.gap_small; // Space between icon and text
            }
        }

        // Text size
        if !self.text.is_empty() {
            width += self.text.len() as i32 * theme.typography.font_size_base / 2;
        }

        // If no text and no icon, default size
        if width == 0 {
            width = 32;
            height = 32;
        }

        Size::new(width + padding.horizontal(), height + padding.vertical())
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let bounds = self.state.bounds;

        // Determine colors based on state
        let (bg_color, text_color, border_color) = match self.style {
            ButtonStyle::Primary => {
                if !self.state.enabled {
                    (
                        theme.colors.surface_variant,
                        theme.colors.text_disabled,
                        theme.colors.border,
                    )
                } else if self.pressed {
                    (
                        theme.colors.primary_active,
                        theme.colors.surface,
                        theme.colors.primary_active,
                    )
                } else if self.hovered {
                    (
                        theme.colors.primary_hover,
                        theme.colors.surface,
                        theme.colors.primary_hover,
                    )
                } else if self.state.focused {
                    (
                        theme.colors.primary,
                        theme.colors.surface,
                        theme.colors.border_focus,
                    )
                } else {
                    (
                        theme.colors.primary,
                        theme.colors.surface,
                        theme.colors.primary,
                    )
                }
            }
            ButtonStyle::Toolbar => {
                if !self.state.enabled {
                    (
                        theme.colors.surface_variant,
                        theme.colors.text_disabled,
                        theme.colors.border,
                    )
                } else if self.pressed {
                    (
                        theme.colors.primary_active.with_alpha(120),
                        theme.colors.text,
                        theme.colors.primary_active,
                    )
                } else if self.hovered {
                    (
                        theme.colors.primary_hover.with_alpha(80),
                        theme.colors.text,
                        theme.colors.primary_hover,
                    )
                } else {
                    (
                        theme.colors.surface_variant,
                        theme.colors.text,
                        theme.colors.border,
                    )
                }
            }
        };

        // Draw background
        context.set_color(bg_color);
        context.fill_rect(bounds);

        // Draw border
        context.set_color(border_color);
        context.draw_rect(bounds);

        // Calculate content layout
        let _content_height =
            theme
                .typography
                .font_size_base
                .max(if self.icon.is_some() { 20 } else { 0 });
        let mut x = bounds.x() + theme.spacing.padding.left;
        let y_center = bounds.center().y;

        // Draw icon if present
        if let Some(ref icon) = self.icon {
            // Use a slightly larger icon box to keep bitmaps readable.
            let icon_size = 28;
            let icon_rect = Rect::new(x, y_center - icon_size / 2, icon_size, icon_size);

            match icon {
                ButtonIcon::Back => Icon::draw_back_arrow(context, icon_rect, text_color),
                ButtonIcon::Forward => Icon::draw_forward_arrow(context, icon_rect, text_color),
                ButtonIcon::Up => Icon::draw_up_arrow(context, icon_rect, text_color),
                ButtonIcon::Home => Icon::draw_home(context, icon_rect, text_color),
                ButtonIcon::Refresh => Icon::draw_refresh(context, icon_rect, text_color),
                ButtonIcon::Folder => Icon::draw_folder(context, icon_rect, text_color),
                ButtonIcon::FolderNew => Icon::draw_folder_new(context, icon_rect, text_color),
                ButtonIcon::Delete => Icon::draw_delete(context, icon_rect, text_color),
                ButtonIcon::File => Icon::draw_file(context, icon_rect, text_color),
                ButtonIcon::Search => Icon::draw_search(context, icon_rect, text_color),
                ButtonIcon::Custom(draw_fn) => draw_fn(context, icon_rect, text_color),
                ButtonIcon::Bitmap {
                    width,
                    height,
                    data,
                } => {
                    // Center the bitmap in the icon_rect without extra inset so strokes stay visible.
                    context.draw_image_rgba(icon_rect, *width, *height, data);
                }
            }

            if !self.text.is_empty() {
                x += icon_size + theme.spacing.gap_small;
            }
        }

        // Draw text if present
        if !self.text.is_empty() {
            context.set_color(text_color);
            let text_size = context.measure_text(&self.text, theme.typography.font_size_base);
            let text_pos = Point::new(x, bounds.center().y - text_size.height / 2);
            context.draw_text(&self.text, text_pos, theme.typography.font_size_base);
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(e) => {
                let was_hovered = self.hovered;
                self.hovered = self.state.bounds.contains(e.position);
                if was_hovered != self.hovered {
                    return EventResult::Consumed;
                }
            }
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    self.pressed = *pressed;
                    if !pressed && self.hovered {
                        if let Some(callback) = &mut self.on_click {
                            callback();
                        }
                    }
                    self.state.focused = true;
                    return EventResult::Consumed;
                }
            }
            // Keyboard activation — Space or Enter when focused fires the
            // on_click callback. Standard accessibility expectation.
            Event::KeyPress(ev) if self.state.focused => {
                use erigui_core::Key;
                if matches!(ev.key, Key::Space | Key::Enter) {
                    if let Some(callback) = &mut self.on_click {
                        callback();
                    }
                    return EventResult::Consumed;
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

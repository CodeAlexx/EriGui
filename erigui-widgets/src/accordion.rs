use crate::Icon;
use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

pub struct AccordionPanel {
    pub id: WidgetId,
    pub title: String,
    pub content: Box<dyn Widget>,
    pub expanded: bool,
    pub enabled: bool,
    pub icon: Option<String>,
}

impl AccordionPanel {
    pub fn new(title: impl Into<String>, content: Box<dyn Widget>) -> Self {
        Self {
            id: WidgetId::default(),
            title: title.into(),
            content,
            expanded: false,
            enabled: true,
            icon: None,
        }
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub struct Accordion {
    state: WidgetState,
    panels: Vec<AccordionPanel>,
    header_height: i32,
    spacing: i32,
    allow_multiple: bool,
    animate_expansion: bool,
    animation_speed: f32,

    // Visual state
    hovered_panel: Option<usize>,
    pressed_panel: Option<usize>,
    /// Panel that receives keyboard focus when the Accordion itself is focused.
    /// `None` means no panel is the keyboard cursor; Up/Down picks the first
    /// enabled panel. Distinct from mouse hover/press.
    focused_panel: Option<usize>,
    panel_animations: Vec<f32>, // 0.0 = collapsed, 1.0 = expanded

    // Callbacks
    on_panel_toggle: Option<Box<dyn FnMut(usize, bool)>>,
}

impl Accordion {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            panels: Vec::new(),
            header_height: 40,
            spacing: 1,
            allow_multiple: true,
            animate_expansion: true,
            animation_speed: 0.15,
            hovered_panel: None,
            pressed_panel: None,
            focused_panel: None,
            panel_animations: Vec::new(),
            on_panel_toggle: None,
        }
    }

    pub fn with_panels(mut self, panels: Vec<AccordionPanel>) -> Self {
        self.panel_animations = vec![0.0; panels.len()];
        for (i, panel) in panels.iter().enumerate() {
            if panel.expanded {
                self.panel_animations[i] = 1.0;
            }
        }
        self.panels = panels;
        self
    }

    pub fn with_allow_multiple(mut self, allow: bool) -> Self {
        self.allow_multiple = allow;
        self
    }

    pub fn with_header_height(mut self, height: i32) -> Self {
        self.header_height = height;
        self
    }

    pub fn with_animation(mut self, enabled: bool) -> Self {
        self.animate_expansion = enabled;
        self
    }

    pub fn with_animation_speed(mut self, speed: f32) -> Self {
        self.animation_speed = speed.clamp(0.05, 1.0);
        self
    }

    pub fn with_on_toggle<F: FnMut(usize, bool) + 'static>(mut self, f: F) -> Self {
        self.on_panel_toggle = Some(Box::new(f));
        self
    }

    pub fn add_panel(&mut self, panel: AccordionPanel) {
        self.panel_animations
            .push(if panel.expanded { 1.0 } else { 0.0 });
        self.panels.push(panel);
    }

    pub fn remove_panel(&mut self, index: usize) -> Option<AccordionPanel> {
        if index < self.panels.len() {
            self.panel_animations.remove(index);
            Some(self.panels.remove(index))
        } else {
            None
        }
    }

    pub fn toggle_panel(&mut self, index: usize) {
        if index >= self.panels.len() || !self.panels[index].enabled {
            return;
        }

        let new_state = !self.panels[index].expanded;

        if !self.allow_multiple && new_state {
            // Collapse all other panels
            for (i, panel) in self.panels.iter_mut().enumerate() {
                if i != index && panel.expanded {
                    panel.expanded = false;
                }
            }
        }

        self.panels[index].expanded = new_state;

        if let Some(callback) = &mut self.on_panel_toggle {
            callback(index, new_state);
        }
    }

    pub fn expand_panel(&mut self, index: usize) {
        if index < self.panels.len() && self.panels[index].enabled && !self.panels[index].expanded {
            self.toggle_panel(index);
        }
    }

    pub fn collapse_panel(&mut self, index: usize) {
        if index < self.panels.len() && self.panels[index].expanded {
            self.toggle_panel(index);
        }
    }

    pub fn expand_all(&mut self) {
        if self.allow_multiple {
            for panel in &mut self.panels {
                if panel.enabled {
                    panel.expanded = true;
                }
            }
        }
    }

    pub fn collapse_all(&mut self) {
        for panel in &mut self.panels {
            panel.expanded = false;
        }
    }

    /// Index of the currently keyboard-focused panel, or `None` if no panel
    /// is the cursor. Exposed for tests.
    pub fn focused_panel(&self) -> Option<usize> {
        self.focused_panel
    }

    /// Find the next enabled panel from `start`, going `forward` or backward.
    /// Wraps around. Skips disabled panels. Returns `None` if no enabled
    /// panel exists at all (or only `start` itself is enabled, leaving
    /// nowhere to go).
    fn next_enabled_panel(&self, start: usize, forward: bool) -> Option<usize> {
        let n = self.panels.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            return if self.panels[0].enabled { Some(0) } else { None };
        }
        for step in 1..=n {
            let i = if forward {
                (start + step) % n
            } else {
                // step <= n so (start + n - step) is in [start, start+n-1]
                (start + n - step) % n
            };
            if self.panels[i].enabled {
                return Some(i);
            }
        }
        None
    }

    /// Pick a starting panel for keyboard navigation when none is focused.
    /// Prefers the first enabled panel.
    fn first_enabled_panel(&self) -> Option<usize> {
        self.panels.iter().position(|p| p.enabled)
    }

    fn update_animations(&mut self) {
        for (i, panel) in self.panels.iter().enumerate() {
            let target = if panel.expanded { 1.0 } else { 0.0 };
            let current = self.panel_animations[i];

            if (target - current).abs() > 0.01 {
                let diff = target - current;
                self.panel_animations[i] = current + diff * self.animation_speed;
            } else {
                self.panel_animations[i] = target;
            }
        }
    }

    fn draw_header(&self, context: &mut dyn DrawContext, theme: &Theme, index: usize, rect: Rect) {
        let panel = &self.panels[index];
        let is_hovered = self.hovered_panel == Some(index);
        let is_pressed = self.pressed_panel == Some(index);

        // Draw header background
        let bg_color = if !panel.enabled {
            theme.colors.surface_variant
        } else if is_pressed {
            theme.colors.primary_active
        } else if is_hovered {
            theme.colors.primary_hover
        } else {
            theme.colors.surface
        };

        context.set_color(bg_color);
        context.fill_rect(rect);

        // Draw bottom border
        context.set_color(theme.colors.border);
        context.fill_rect(Rect::new(rect.x(), rect.bottom() - 1, rect.width(), 1));

        let padding = 12;
        let mut x = rect.x() + padding;

        // Draw expand/collapse icon
        let icon_size = 16;
        let icon_y = rect.y() + (rect.height() - icon_size) / 2;
        let icon_rect = Rect::new(x, icon_y, icon_size, icon_size);

        let icon_color = if panel.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };

        // Draw icon - chevron down when expanded, right when collapsed
        if self.panel_animations[index] > 0.5 {
            // Draw chevron pointing down
            let center = icon_rect.center();
            let size = icon_rect.width() as f32 * 0.5;

            let points = [Point::new(
                    (center.x as f32 - size * 0.5) as i32,
                    (center.y as f32 - size * 0.3) as i32,
                ),
                Point::new(center.x, (center.y as f32 + size * 0.3) as i32),
                Point::new(
                    (center.x as f32 + size * 0.5) as i32,
                    (center.y as f32 - size * 0.3) as i32,
                )];

            context.set_color(icon_color);
            context.set_line_width(2);
            context.draw_line(points[0], points[1], 2);
            context.draw_line(points[1], points[2], 2);
        } else {
            Icon::draw_chevron_right(context, icon_rect, icon_color);
        }

        x += icon_size + padding;

        // Draw custom icon if provided
        if let Some(icon_name) = &panel.icon {
            let custom_icon_rect = Rect::new(x, icon_y, icon_size, icon_size);

            // Draw icon based on name
            match icon_name.as_str() {
                "folder" => Icon::draw_folder(context, custom_icon_rect, icon_color),
                "file" => Icon::draw_file(context, custom_icon_rect, icon_color),
                "settings" => Icon::draw_gear(
                    context,
                    custom_icon_rect,
                    icon_color,
                    theme.colors.surface,
                ),
                "palette" => Icon::draw_palette(
                    context,
                    custom_icon_rect,
                    icon_color,
                    theme.colors.surface,
                ),
                _ => {}
            }

            x += icon_size + padding;
        }

        // Draw title
        let text_color = if panel.enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };

        context.set_color(text_color);
        context.draw_text(
            &panel.title,
            Point::new(x, rect.center().y + theme.typography.font_size_base / 2 - 2),
            theme.typography.font_size_base,
        );
    }
}

impl Widget for Accordion {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let width = constraints.max_width.unwrap_or(300);
        let mut height = 0;

        // Calculate total height including animations
        for (i, panel) in self.panels.iter().enumerate() {
            height += self.header_height;

            if self.panel_animations[i] > 0.0 {
                let content_size = panel.content.measure(
                    &LayoutConstraints {
                        min_width: Some(width),
                        max_width: Some(width),
                        min_height: None,
                        max_height: None,
                    },
                    theme,
                );

                height += (content_size.height as f32 * self.panel_animations[i]) as i32;
            }

            if i < self.panels.len() - 1 {
                height += self.spacing;
            }
        }

        Size::new(width, height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;

        let mut y = rect.y();
        let panels_count = self.panels.len();

        for i in 0..panels_count {
            // Layout header
            y += self.header_height;

            // Layout content if expanded
            if self.panel_animations[i] > 0.0 {
                let content_constraints = LayoutConstraints {
                    min_width: Some(rect.width()),
                    max_width: Some(rect.width()),
                    min_height: None,
                    max_height: None,
                };

                let content_size = self.panels[i].content.measure(&content_constraints, theme);
                let content_height = (content_size.height as f32 * self.panel_animations[i]) as i32;

                if content_height > 0 {
                    self.panels[i].content.layout(
                        Rect::new(rect.x(), y, rect.width(), content_size.height),
                        theme,
                    );
                }

                y += content_height;
            }

            if i < panels_count - 1 {
                y += self.spacing;
            }
        }
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let mut y = self.state.bounds.y();

        for (i, panel) in self.panels.iter().enumerate() {
            // Draw header
            let header_rect = Rect::new(
                self.state.bounds.x(),
                y,
                self.state.bounds.width(),
                self.header_height,
            );

            self.draw_header(context, theme, i, header_rect);
            y += self.header_height;

            // Draw content if visible
            if self.panel_animations[i] > 0.0 {
                let content_height = panel.content.bounds().height();
                let visible_height = (content_height as f32 * self.panel_animations[i]) as i32;

                if visible_height > 0 {
                    // Clip content area
                    context.push_clip_rect(Rect::new(
                        self.state.bounds.x(),
                        y,
                        self.state.bounds.width(),
                        visible_height,
                    ));

                    // Draw content background
                    context.set_color(theme.colors.background);
                    context.fill_rect(Rect::new(
                        self.state.bounds.x(),
                        y,
                        self.state.bounds.width(),
                        visible_height,
                    ));

                    // Draw content
                    panel.content.draw(context, theme);

                    context.pop_clip_rect();

                    y += visible_height;
                }
            }

            if i < self.panels.len() - 1 {
                y += self.spacing;
            }
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Update animations
        if self.animate_expansion {
            self.update_animations();
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

                // Find which header was clicked
                let mut y = self.state.bounds.y();

                for (i, panel) in self.panels.iter().enumerate() {
                    let header_rect = Rect::new(
                        self.state.bounds.x(),
                        y,
                        self.state.bounds.width(),
                        self.header_height,
                    );

                    if header_rect.contains(*position) && panel.enabled {
                        if *pressed {
                            self.pressed_panel = Some(i);
                        } else if self.pressed_panel == Some(i) {
                            self.toggle_panel(i);
                            self.pressed_panel = None;
                        }
                        return EventResult::Consumed;
                    }

                    y += self.header_height;

                    if self.panel_animations[i] > 0.0 {
                        let content_height = panel.content.bounds().height();
                        let visible_height =
                            (content_height as f32 * self.panel_animations[i]) as i32;
                        y += visible_height;
                    }

                    if i < self.panels.len() - 1 {
                        y += self.spacing;
                    }
                }

                if !pressed {
                    self.pressed_panel = None;
                }
            }
            Event::MouseMove(move_event) => {
                if !self.state.bounds.contains(move_event.position) {
                    self.hovered_panel = None;
                    return EventResult::Ignored;
                }

                // Find which header is hovered
                let mut y = self.state.bounds.y();
                let mut found_hover = false;

                for (i, panel) in self.panels.iter().enumerate() {
                    let header_rect = Rect::new(
                        self.state.bounds.x(),
                        y,
                        self.state.bounds.width(),
                        self.header_height,
                    );

                    if header_rect.contains(move_event.position) {
                        self.hovered_panel = Some(i);
                        found_hover = true;
                        break;
                    }

                    y += self.header_height;

                    if self.panel_animations[i] > 0.0 {
                        let content_height = panel.content.bounds().height();
                        let visible_height =
                            (content_height as f32 * self.panel_animations[i]) as i32;
                        y += visible_height;
                    }

                    if i < self.panels.len() - 1 {
                        y += self.spacing;
                    }
                }

                if !found_hover {
                    self.hovered_panel = None;
                }

                return EventResult::Consumed;
            }
            Event::KeyPress(KeyPressEvent { key, modifiers, .. })
                if self.state.focused && modifiers.is_empty() =>
            {
                // Only consume keyboard nav when the Accordion itself owns
                // focus. If a child widget inside an expanded panel has
                // focus, the Accordion is unfocused and the event falls
                // through to the forwarding arm below.
                match key {
                    Key::Down => {
                        let next = match self.focused_panel {
                            Some(start) => self.next_enabled_panel(start, true),
                            None => self.first_enabled_panel(),
                        };
                        if let Some(idx) = next {
                            self.focused_panel = Some(idx);
                            return EventResult::Consumed;
                        }
                        return EventResult::Ignored;
                    }
                    Key::Up => {
                        let next = match self.focused_panel {
                            Some(start) => self.next_enabled_panel(start, false),
                            None => self.first_enabled_panel(),
                        };
                        if let Some(idx) = next {
                            self.focused_panel = Some(idx);
                            return EventResult::Consumed;
                        }
                        return EventResult::Ignored;
                    }
                    Key::Enter | Key::Space => {
                        if let Some(idx) = self.focused_panel {
                            if idx < self.panels.len() && self.panels[idx].enabled {
                                self.toggle_panel(idx);
                                return EventResult::Consumed;
                            }
                        }
                        return EventResult::Ignored;
                    }
                    _ => {
                        // Other keys: do not consume, do not forward to
                        // content (the Accordion has focus, not the
                        // content). Host can decide what to do.
                        return EventResult::Ignored;
                    }
                }
            }
            _ => {
                // Pass events to expanded content panels
                let panels_count = self.panels.len();
                for i in 0..panels_count {
                    if self.panels[i].expanded && self.panel_animations[i] > 0.0 {
                        let result = self.panels[i].content.handle_event(event, _theme);
                        if result == EventResult::Consumed {
                            return result;
                        }
                    }
                }
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

use crate::Icon;
use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Callback fired when a notification action button is clicked. Argument: notification id.
type ActionClickedCallback = Box<dyn FnMut(&str)>;
/// Callback fired when a notification is closed. Argument: notification id.
type NotificationClosedCallback = Box<dyn FnMut(&str)>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub struct Notification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub icon: Option<String>,
    pub duration: Duration,
    pub can_close: bool,
    pub action_text: Option<String>,
    pub created_at: Instant,
    pub progress: Option<f32>,
}

impl Notification {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            message: message.into(),
            notification_type: NotificationType::Info,
            icon: None,
            duration: Duration::from_secs(5),
            can_close: true,
            action_text: None,
            created_at: Instant::now(),
            progress: None,
        }
    }

    pub fn with_type(mut self, notification_type: NotificationType) -> Self {
        self.notification_type = notification_type;
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_can_close(mut self, can_close: bool) -> Self {
        self.can_close = can_close;
        self
    }

    pub fn with_action(mut self, action_text: impl Into<String>) -> Self {
        self.action_text = Some(action_text.into());
        self
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Update the progress value at runtime. Mirrors `with_progress`
    /// but operates on an existing notification, e.g. so a long-running
    /// task can advance its progress bar without rebuilding the
    /// notification each tick. Clamps to [0.0, 1.0].
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = Some(progress.clamp(0.0, 1.0));
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn remaining_ratio(&self) -> f32 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            0.0
        } else {
            1.0 - (elapsed.as_secs_f32() / self.duration.as_secs_f32())
        }
    }
}

struct NotificationItem {
    notification: Notification,
    bounds: Rect,
    close_button_rect: Option<Rect>,
    action_button_rect: Option<Rect>,
    hover_close: bool,
    hover_action: bool,
    animation_offset: i32,
}

pub struct NotificationManager {
    state: WidgetState,
    notifications: VecDeque<NotificationItem>,
    position: NotificationPosition,
    max_visible: usize,
    spacing: i32,
    margin: i32,
    notification_width: i32,
    notification_height: i32,
    animation_speed: f32,
    on_action_clicked: Option<ActionClickedCallback>,
    on_notification_closed: Option<NotificationClosedCallback>,
}

impl NotificationManager {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            notifications: VecDeque::new(),
            position: NotificationPosition::TopRight,
            max_visible: 5,
            spacing: 10,
            margin: 20,
            notification_width: 350,
            notification_height: 80,
            animation_speed: 500.0,
            on_action_clicked: None,
            on_notification_closed: None,
        }
    }

    pub fn with_position(mut self, position: NotificationPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible;
        self
    }

    pub fn with_size(mut self, width: i32, height: i32) -> Self {
        self.notification_width = width;
        self.notification_height = height;
        self
    }

    pub fn with_on_action_clicked<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_action_clicked = Some(Box::new(f));
        self
    }

    pub fn with_on_notification_closed<F: FnMut(&str) + 'static>(mut self, f: F) -> Self {
        self.on_notification_closed = Some(Box::new(f));
        self
    }

    pub fn show(&mut self, notification: Notification) {
        let item = NotificationItem {
            notification,
            bounds: Rect::default(),
            close_button_rect: None,
            action_button_rect: None,
            hover_close: false,
            hover_action: false,
            animation_offset: self.notification_width,
        };

        self.notifications.push_back(item);

        // Remove oldest if exceeding max visible
        while self.notifications.len() > self.max_visible {
            if let Some(removed) = self.notifications.pop_front() {
                if let Some(callback) = &mut self.on_notification_closed {
                    callback(&removed.notification.id);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn remove(&mut self, id: &str) {
        self.notifications.retain(|item| {
            if item.notification.id == id {
                if let Some(callback) = &mut self.on_notification_closed {
                    callback(id);
                }
                false
            } else {
                true
            }
        });
    }

    fn update_layout(&mut self) {
        let viewport = self.state.bounds;
        let mut y_offset = 0;

        for (index, item) in self.notifications.iter_mut().enumerate() {
            if index >= self.max_visible {
                break;
            }

            // Calculate notification position
            let (x, y) = match self.position {
                NotificationPosition::TopLeft => {
                    (self.margin + item.animation_offset, self.margin + y_offset)
                }
                NotificationPosition::TopCenter => (
                    (viewport.width() - self.notification_width) / 2 + item.animation_offset,
                    self.margin + y_offset,
                ),
                NotificationPosition::TopRight => (
                    viewport.width()
                        - self.notification_width
                        - self.margin
                        - item.animation_offset,
                    self.margin + y_offset,
                ),
                NotificationPosition::BottomLeft => (
                    self.margin + item.animation_offset,
                    viewport.height()
                        - self.margin
                        - (self.notification_height + self.spacing) * (index + 1) as i32
                        + self.spacing,
                ),
                NotificationPosition::BottomCenter => (
                    (viewport.width() - self.notification_width) / 2 + item.animation_offset,
                    viewport.height()
                        - self.margin
                        - (self.notification_height + self.spacing) * (index + 1) as i32
                        + self.spacing,
                ),
                NotificationPosition::BottomRight => (
                    viewport.width()
                        - self.notification_width
                        - self.margin
                        - item.animation_offset,
                    viewport.height()
                        - self.margin
                        - (self.notification_height + self.spacing) * (index + 1) as i32
                        + self.spacing,
                ),
            };

            item.bounds = Rect::new(x, y, self.notification_width, self.notification_height);

            // Calculate button rects
            if item.notification.can_close {
                let close_size = 20;
                item.close_button_rect = Some(Rect::new(
                    item.bounds.right() - close_size - 8,
                    item.bounds.y() + 8,
                    close_size,
                    close_size,
                ));
            }

            if item.notification.action_text.is_some() {
                let action_width = 80;
                let action_height = 24;
                item.action_button_rect = Some(Rect::new(
                    item.bounds.right() - action_width - 8,
                    item.bounds.bottom() - action_height - 8,
                    action_width,
                    action_height,
                ));
            }

            y_offset += self.notification_height + self.spacing;
        }
    }

    pub fn update_animations(&mut self, delta_time: f32) {
        // Update slide-in animations
        for item in &mut self.notifications {
            if item.animation_offset > 0 {
                item.animation_offset = (item.animation_offset as f32
                    - self.animation_speed * delta_time)
                    .max(0.0) as i32;
            }
        }

        // Remove expired notifications
        let mut to_remove = Vec::new();
        for (index, item) in self.notifications.iter().enumerate() {
            if item.notification.is_expired() {
                to_remove.push(index);
            }
        }

        // Remove in reverse order to maintain indices
        for index in to_remove.iter().rev() {
            if let Some(removed) = self.notifications.remove(*index) {
                if let Some(callback) = &mut self.on_notification_closed {
                    callback(&removed.notification.id);
                }
            }
        }
    }

    fn get_type_color(&self, notification_type: NotificationType, theme: &Theme) -> Color {
        match notification_type {
            NotificationType::Info => theme.colors.primary,
            NotificationType::Success => theme.colors.success,
            NotificationType::Warning => theme.colors.warning,
            NotificationType::Error => theme.colors.error,
        }
    }

    fn get_type_icon(&self, notification_type: NotificationType) -> &str {
        match notification_type {
            NotificationType::Info => "info",
            NotificationType::Success => "check",
            NotificationType::Warning => "warning",
            NotificationType::Error => "error",
        }
    }
}

impl Widget for NotificationManager {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(
            constraints.max_width.unwrap_or(0),
            constraints.max_height.unwrap_or(0),
        )
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
        self.update_layout();
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        for (index, item) in self.notifications.iter().enumerate() {
            if index >= self.max_visible {
                break;
            }

            // Draw shadow
            let shadow_offset = 2;
            let shadow_rect = Rect::new(
                item.bounds.x() + shadow_offset,
                item.bounds.y() + shadow_offset,
                item.bounds.width(),
                item.bounds.height(),
            );
            context.set_color(Color::rgba(0, 0, 0, 30));
            context.fill_rect(shadow_rect);

            // Draw background
            context.set_color(theme.colors.surface);
            context.fill_rect(item.bounds);

            // Draw border
            context.set_color(theme.colors.border);
            context.draw_rect(item.bounds);

            // Draw type indicator
            let type_color = self.get_type_color(item.notification.notification_type, theme);
            let indicator_rect =
                Rect::new(item.bounds.x(), item.bounds.y(), 4, item.bounds.height());
            context.set_color(type_color);
            context.fill_rect(indicator_rect);

            // Draw icon
            let icon_x = item.bounds.x() + 12;
            let icon_y = item.bounds.y() + 12;
            let icon_size = 24;

            let icon_name = item
                .notification
                .icon
                .as_deref()
                .unwrap_or_else(|| self.get_type_icon(item.notification.notification_type));

            context.set_color(type_color);
            match icon_name {
                "info" => Icon::draw_info(context, Point::new(icon_x, icon_y), icon_size),
                "check" => Icon::draw_check(context, Point::new(icon_x, icon_y), icon_size),
                "warning" => Icon::draw_warning(context, Point::new(icon_x, icon_y), icon_size),
                "error" => Icon::draw_error(context, Point::new(icon_x, icon_y), icon_size),
                _ => {}
            }

            // Draw title
            let text_x = icon_x + icon_size + 8;
            let title_y = item.bounds.y() + 16;
            context.set_color(theme.colors.text);
            context.draw_text(
                &item.notification.title,
                Point::new(text_x, title_y),
                theme.typography.font_size_base,
            );

            // Draw message
            let message_y = title_y + theme.typography.font_size_base + 4;
            context.set_color(theme.colors.text_secondary);
            context.draw_text(
                &item.notification.message,
                Point::new(text_x, message_y),
                theme.typography.font_size_small,
            );

            // Draw progress bar if present
            if let Some(progress) = item.notification.progress {
                let progress_y = item.bounds.bottom() - 4;
                let progress_rect = Rect::new(item.bounds.x(), progress_y, item.bounds.width(), 4);

                // Background
                context.set_color(theme.colors.surface_variant);
                context.fill_rect(progress_rect);

                // Progress
                let progress_width = (item.bounds.width() as f32 * progress) as i32;
                context.set_color(type_color);
                context.fill_rect(Rect::new(
                    progress_rect.x(),
                    progress_rect.y(),
                    progress_width,
                    progress_rect.height(),
                ));
            }

            // Draw duration indicator
            let remaining_ratio = item.notification.remaining_ratio();
            if remaining_ratio > 0.0 && remaining_ratio < 1.0 {
                let duration_rect = Rect::new(
                    item.bounds.x(),
                    item.bounds.bottom() - 2,
                    (item.bounds.width() as f32 * remaining_ratio) as i32,
                    2,
                );
                context.set_color(type_color.with_alpha(128));
                context.fill_rect(duration_rect);
            }

            // Draw close button
            if let Some(close_rect) = item.close_button_rect {
                let close_color = if item.hover_close {
                    theme.colors.error
                } else {
                    theme.colors.text_secondary
                };

                context.set_color(close_color);
                Icon::draw_close(context, close_rect.center(), 12);
            }

            // Draw action button
            if let (Some(action_rect), Some(action_text)) =
                (&item.action_button_rect, &item.notification.action_text)
            {
                let button_color = if item.hover_action {
                    color_darker(&type_color, 0.1)
                } else {
                    type_color
                };

                context.set_color(button_color);
                context.fill_rect(*action_rect);

                context.set_color(theme.colors.background);
                context.draw_text(
                    action_text,
                    Point::new(action_rect.center().x - 20, action_rect.center().y + 4),
                    theme.typography.font_size_small,
                );
            }
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        match event {
            Event::MouseMove(mouse_event) => {
                let mut needs_redraw = false;

                for item in &mut self.notifications {
                    // Check close button hover
                    if let Some(close_rect) = item.close_button_rect {
                        let was_hover = item.hover_close;
                        item.hover_close = close_rect.contains(mouse_event.position);
                        if was_hover != item.hover_close {
                            needs_redraw = true;
                        }
                    }

                    // Check action button hover
                    if let Some(action_rect) = item.action_button_rect {
                        let was_hover = item.hover_action;
                        item.hover_action = action_rect.contains(mouse_event.position);
                        if was_hover != item.hover_action {
                            needs_redraw = true;
                        }
                    }
                }

                if needs_redraw {
                    return EventResult::Consumed;
                }
            }

            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if !pressed {
                    // Check for close button clicks
                    let mut clicked_close = None;
                    for (index, item) in self.notifications.iter().enumerate() {
                        if let Some(close_rect) = item.close_button_rect {
                            if close_rect.contains(*position) {
                                clicked_close = Some(index);
                                break;
                            }
                        }
                    }

                    if let Some(index) = clicked_close {
                        if let Some(removed) = self.notifications.remove(index) {
                            if let Some(callback) = &mut self.on_notification_closed {
                                callback(&removed.notification.id);
                            }
                        }
                        return EventResult::Consumed;
                    }

                    // Check for action button clicks
                    for item in &self.notifications {
                        if let Some(action_rect) = item.action_button_rect {
                            if action_rect.contains(*position) {
                                if let Some(callback) = &mut self.on_action_clicked {
                                    callback(&item.notification.id);
                                }
                                return EventResult::Consumed;
                            }
                        }
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
        self.update_layout();
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

// Helper function for Color
fn color_darker(color: &Color, amount: f32) -> Color {
    Color::rgba(
        (color.r as f32 * (1.0 - amount)).max(0.0) as u8,
        (color.g as f32 * (1.0 - amount)).max(0.0) as u8,
        (color.b as f32 * (1.0 - amount)).max(0.0) as u8,
        color.a,
    )
}

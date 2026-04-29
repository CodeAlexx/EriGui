use erigui_core::*;
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::*;
use std::time::Duration;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct NotificationDemo {
    notification_manager: NotificationManager,
    info_button: Button,
    success_button: Button,
    warning_button: Button,
    error_button: Button,
    progress_button: Button,
    action_button: Button,
    clear_button: Button,
    position_combo: ComboBox,
    notification_counter: i32,
}

impl NotificationDemo {
    fn new() -> Self {
        let notification_manager = NotificationManager::new(WidgetId::default())
            .with_position(NotificationPosition::TopRight)
            .with_max_visible(5)
            .with_on_action_clicked(|id| {
                println!("Action clicked for notification: {}", id);
            })
            .with_on_notification_closed(|id| {
                println!("Notification closed: {}", id);
            });

        let info_button = Button::new(WidgetId::default(), "Show Info");
        let success_button = Button::new(WidgetId::default(), "Show Success");
        let warning_button = Button::new(WidgetId::default(), "Show Warning");
        let error_button = Button::new(WidgetId::default(), "Show Error");
        let progress_button = Button::new(WidgetId::default(), "Show Progress");
        let action_button = Button::new(WidgetId::default(), "Show with Action");
        let clear_button = Button::new(WidgetId::default(), "Clear All");

        let position_combo = ComboBox::new(WidgetId::default())
            .with_items(vec![
                "Top Left".to_string(),
                "Top Center".to_string(),
                "Top Right".to_string(),
                "Bottom Left".to_string(),
                "Bottom Center".to_string(),
                "Bottom Right".to_string(),
            ])
            .with_selected(2); // Top Right

        Self {
            notification_manager,
            info_button,
            success_button,
            warning_button,
            error_button,
            progress_button,
            action_button,
            clear_button,
            position_combo,
            notification_counter: 0,
        }
    }

    fn show_notification(
        &mut self,
        notification_type: NotificationType,
        title: &str,
        message: &str,
    ) {
        self.notification_counter += 1;
        let notification = Notification::new(
            format!("notification-{}", self.notification_counter),
            title,
            message,
        )
        .with_type(notification_type)
        .with_duration(Duration::from_secs(5));

        self.notification_manager.show(notification);
    }

    fn show_progress_notification(&mut self) {
        self.notification_counter += 1;
        let notification = Notification::new(
            format!("notification-{}", self.notification_counter),
            "Download in progress",
            "Downloading file.zip...",
        )
        .with_type(NotificationType::Info)
        .with_progress(0.65)
        .with_duration(Duration::from_secs(8))
        .with_can_close(false);

        self.notification_manager.show(notification);
    }

    fn show_action_notification(&mut self) {
        self.notification_counter += 1;
        let notification = Notification::new(
            format!("notification-{}", self.notification_counter),
            "New message received",
            "You have a new message from John Doe",
        )
        .with_type(NotificationType::Info)
        .with_action("View")
        .with_duration(Duration::from_secs(10));

        self.notification_manager.show(notification);
    }

    fn update_position(&mut self) {
        let position = match self.position_combo.selected_index() {
            Some(0) => NotificationPosition::TopLeft,
            Some(1) => NotificationPosition::TopCenter,
            Some(2) => NotificationPosition::TopRight,
            Some(3) => NotificationPosition::BottomLeft,
            Some(4) => NotificationPosition::BottomCenter,
            Some(5) => NotificationPosition::BottomRight,
            _ => NotificationPosition::TopRight,
        };

        self.notification_manager = NotificationManager::new(self.notification_manager.id())
            .with_position(position)
            .with_max_visible(5)
            .with_on_action_clicked(|id| {
                println!("Action clicked for notification: {}", id);
            })
            .with_on_notification_closed(|id| {
                println!("Notification closed: {}", id);
            });
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer = Renderer::new(&event_loop, 1024, 768, "Notification Demo")
        .expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = NotificationDemo::new();
    let mut last_frame_time = std::time::Instant::now();
    let mut translator = EventTranslator::new(renderer.viewport_size());

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        translator.set_window_size(renderer.viewport_size());
                    }
                    _ => {}
                }

                // Convert window event to EriGui event
                if let Some(gui_event) = translator.translate(&event) {
                    // Handle button events
                    if demo.info_button.handle_event(&gui_event, &theme) == EventResult::Consumed {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_notification(
                                    NotificationType::Info,
                                    "Information",
                                    "This is an informational notification",
                                );
                            }
                        }
                    }

                    if demo.success_button.handle_event(&gui_event, &theme) == EventResult::Consumed
                    {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_notification(
                                    NotificationType::Success,
                                    "Success!",
                                    "Operation completed successfully",
                                );
                            }
                        }
                    }

                    if demo.warning_button.handle_event(&gui_event, &theme) == EventResult::Consumed
                    {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_notification(
                                    NotificationType::Warning,
                                    "Warning",
                                    "Please check your settings",
                                );
                            }
                        }
                    }

                    if demo.error_button.handle_event(&gui_event, &theme) == EventResult::Consumed {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_notification(
                                    NotificationType::Error,
                                    "Error",
                                    "Failed to save the file",
                                );
                            }
                        }
                    }

                    if demo.progress_button.handle_event(&gui_event, &theme)
                        == EventResult::Consumed
                    {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_progress_notification();
                            }
                        }
                    }

                    if demo.action_button.handle_event(&gui_event, &theme) == EventResult::Consumed
                    {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.show_action_notification();
                            }
                        }
                    }

                    if demo.clear_button.handle_event(&gui_event, &theme) == EventResult::Consumed {
                        if let Event::MouseButton(e) = &gui_event {
                            if !e.pressed {
                                demo.notification_manager.clear();
                            }
                        }
                    }

                    if demo.position_combo.handle_event(&gui_event, &theme) == EventResult::Consumed
                    {
                        demo.update_position();
                    }

                    // Handle notification events
                    demo.notification_manager.handle_event(&gui_event, &theme);

                    renderer.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Update animations
                let now = std::time::Instant::now();
                let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                last_frame_time = now;

                // Update notification animations
                demo.notification_manager.update_animations(delta_time);

                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let button_width = 150;
                let button_height = 36;
                let spacing = 10;

                // Create a control panel
                let panel_width = button_width + padding * 2;
                let panel_height = window_size.height - padding * 2;
                let panel_rect = Rect::new(padding, padding, panel_width, panel_height);

                // Layout buttons
                let mut y = panel_rect.y() + padding;

                // Title
                let mut title_label = Label::new(WidgetId::default(), "Notification Demo");
                title_label.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, 30),
                    &theme,
                );
                y += 40;

                // Position combo box
                demo.position_combo.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing * 2;

                // Notification buttons
                demo.info_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing;

                demo.success_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing;

                demo.warning_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing;

                demo.error_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing * 2;

                demo.progress_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing;

                demo.action_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );
                y += button_height + spacing * 2;

                demo.clear_button.layout(
                    Rect::new(panel_rect.x() + padding, y, button_width, button_height),
                    &theme,
                );

                // Layout notification manager to cover entire window
                demo.notification_manager.layout(
                    Rect::new(0, 0, window_size.width, window_size.height),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                // Draw control panel background
                renderer.set_color(theme.colors.surface);
                renderer.fill_rect(panel_rect);
                renderer.set_color(theme.colors.border);
                renderer.draw_rect(panel_rect);

                // Draw title
                title_label.draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw controls
                demo.position_combo
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.info_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.success_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.warning_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.error_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.progress_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.action_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.clear_button
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw notifications (should be drawn last to appear on top)
                demo.notification_manager
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

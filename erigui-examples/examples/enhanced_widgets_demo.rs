use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct EnhancedWidgetsDemo {
    // Drag & Drop demo
    drag_source_list: ListView,
    drop_target_list: ListView,
    drag_label: Label,

    // Keyboard navigation demo
    nav_buttons: Vec<Button>,
    nav_text_input: TextInput,
    nav_combo: ComboBox,
    nav_checkbox: Checkbox,

    // Accessibility status
    accessibility_label: Label,
    announcements_text: TextArea,
}

impl EnhancedWidgetsDemo {
    fn new() -> Self {
        // Setup drag & drop lists
        let mut drag_source_list = ListView::new(WidgetId::default()).with_items(vec![
            ListItem {
                id: "item1".into(),
                text: "Item 1".into(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item2".into(),
                text: "Item 2".into(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item3".into(),
                text: "Item 3".into(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item4".into(),
                text: "Item 4".into(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item5".into(),
                text: "Item 5".into(),
                icon: None,
                selected: false,
            },
        ]);

        let drop_target_list = ListView::new(WidgetId::default()).with_items(vec![ListItem {
            id: "placeholder".into(),
            text: "Drop items here".into(),
            icon: None,
            selected: false,
        }]);

        // Setup keyboard navigation widgets
        let nav_buttons = vec![
            Button::new(WidgetId::default(), "Button 1"),
            Button::new(WidgetId::default(), "Button 2"),
            Button::new(WidgetId::default(), "Button 3"),
        ];

        let nav_text_input = TextInput::new(WidgetId::default()).with_placeholder("Type here...");

        let nav_combo = ComboBox::new(WidgetId::default()).with_items(vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ]);

        let nav_checkbox = Checkbox::new(WidgetId::default(), "Enable feature");

        // Setup accessibility widgets
        let accessibility_label = Label::new(WidgetId::default(), "Accessibility Status: Active");
        let mut announcements_text = TextArea::new(WidgetId::default());
        announcements_text.set_text("Screen reader announcements will appear here...");

        // Register widgets for drag & drop
        {
            let mut drag_manager = drag_drop_manager();
            drag_manager.register_drop_target(drop_target_list.id(), vec!["list_item".to_string()]);
        }

        // Register widgets for keyboard navigation
        {
            let mut nav_manager = keyboard_nav_manager();
            for (i, button) in nav_buttons.iter().enumerate() {
                nav_manager.register_widget(FocusableWidget {
                    id: button.id(),
                    bounds: Rect::default(),
                    tab_index: Some(i as i32),
                    focusable: true,
                    group_id: None,
                });
            }

            nav_manager.register_widget(FocusableWidget {
                id: nav_text_input.id(),
                bounds: Rect::default(),
                tab_index: Some(3),
                focusable: true,
                group_id: None,
            });

            nav_manager.register_widget(FocusableWidget {
                id: nav_combo.id(),
                bounds: Rect::default(),
                tab_index: Some(4),
                focusable: true,
                group_id: None,
            });

            nav_manager.register_widget(FocusableWidget {
                id: nav_checkbox.id(),
                bounds: Rect::default(),
                tab_index: Some(5),
                focusable: true,
                group_id: None,
            });
        }

        // Register widgets for accessibility
        {
            let mut a11y_manager = accessibility_manager();
            for (i, button) in nav_buttons.iter().enumerate() {
                a11y_manager.register_node(AccessibilityNode {
                    id: button.id(),
                    role: AccessibilityRole::Button,
                    label: format!("Button {}", i + 1),
                    description: Some(format!("This is button number {}", i + 1)),
                    bounds: Rect::default(),
                    state: AccessibilityState::default(),
                    children: vec![],
                    parent: None,
                    actions: vec![AccessibilityAction::Click, AccessibilityAction::Focus],
                });
            }
        }

        Self {
            drag_source_list,
            drop_target_list,
            drag_label: Label::new(
                WidgetId::default(),
                "Drag items from left list to right list",
            ),
            nav_buttons,
            nav_text_input,
            nav_combo,
            nav_checkbox,
            accessibility_label,
            announcements_text,
        }
    }

    fn update_accessibility_announcements(&mut self) {
        let mut a11y_manager = accessibility_manager();
        let announcements = a11y_manager.get_announcements();

        if !announcements.is_empty() {
            let current_text = self.announcements_text.get_text();
            let new_text = if current_text.is_empty() {
                announcements.join("\n")
            } else {
                format!("{}\n{}", current_text, announcements.join("\n"))
            };
            self.announcements_text.set_text(&new_text);

            // Keep only last 10 lines
            let lines: Vec<_> = new_text.lines().collect();
            if lines.len() > 10 {
                let trimmed = lines[lines.len() - 10..].join("\n");
                self.announcements_text.set_text(&trimmed);
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer = Renderer::new(&event_loop, 1200, 800, "Enhanced Widgets Demo")
        .expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = EnhancedWidgetsDemo::new();

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
                    }
                    _ => {}
                }

                // Convert window event to EriGui event
                if let Some(gui_event) =
                    erigui_rendering::window::convert_window_event(event, renderer.viewport_size())
                {
                    // Handle keyboard navigation
                    if let Event::KeyPress(key_event) = &gui_event {
                        let mut nav_manager = keyboard_nav_manager();
                        if let Some(next_widget) = nav_manager.handle_key(key_event.key.clone()) {
                            // Update accessibility focus
                            let mut a11y_manager = accessibility_manager();
                            a11y_manager.set_focus(Some(next_widget));

                            // Update widget focus states
                            for button in &mut demo.nav_buttons {
                                button.set_focused(button.id() == next_widget);
                            }
                            demo.nav_text_input
                                .set_focused(demo.nav_text_input.id() == next_widget);
                            demo.nav_combo
                                .set_focused(demo.nav_combo.id() == next_widget);
                            demo.nav_checkbox
                                .set_focused(demo.nav_checkbox.id() == next_widget);
                        }
                    }

                    // Handle drag & drop
                    let mut drag_manager = drag_drop_manager();

                    // Handle events for all widgets
                    let mut needs_redraw = false;

                    // Drag source list
                    if demo.drag_source_list.handle_event(&gui_event, &theme)
                        == EventResult::Consumed
                    {
                        if let Event::MouseButton(mouse_event) = &gui_event {
                            if mouse_event.button == MouseButton::Left && mouse_event.pressed {
                                if let Some(selected) = demo.drag_source_list.selected_item() {
                                    let drag_copy = ListItem {
                                        id: selected.id.clone(),
                                        text: selected.text.clone(),
                                        icon: selected.icon.clone(),
                                        selected: false,
                                    };
                                    drag_manager.start_drag(
                                        demo.drag_source_list.id(),
                                        "list_item".to_string(),
                                        Box::new(drag_copy),
                                        Point::new(10, 10),
                                    );
                                }
                            }
                        }
                        needs_redraw = true;
                    }

                    // Drop target list
                    if demo.drop_target_list.handle_event(&gui_event, &theme)
                        == EventResult::Consumed
                    {
                        needs_redraw = true;
                    }

                    // Handle drop
                    if let Event::MouseButton(mouse_event) = &gui_event {
                        if !mouse_event.pressed && drag_manager.is_dragging() {
                            if drag_manager.check_drop_target(
                                demo.drop_target_list.id(),
                                demo.drop_target_list.bounds(),
                            ) {
                                if let Some(drag_data) = drag_manager.get_drag_data() {
                                    if let Some(item) = drag_data.data.downcast_ref::<ListItem>() {
                                        demo.drop_target_list.add_item(ListItem {
                                            id: item.id.clone(),
                                            text: item.text.clone(),
                                            icon: item.icon.clone(),
                                            selected: false,
                                        });

                                        let mut a11y_manager = accessibility_manager();
                                        a11y_manager
                                            .announce("Item dropped into target list".to_string());
                                    }
                                }
                                drag_manager.complete_drop();
                            } else {
                                drag_manager.cancel_drag();
                            }
                            needs_redraw = true;
                        }
                    }

                    // Update drag position
                    if let Event::MouseMove(mouse_event) = &gui_event {
                        if drag_manager.is_dragging() {
                            drag_manager.update_drag_position(mouse_event.position);
                            needs_redraw = true;
                        }
                    }

                    // Navigation widgets
                    for (i, button) in demo.nav_buttons.iter_mut().enumerate() {
                        if button.handle_event(&gui_event, &theme) == EventResult::Consumed {
                            if let Event::MouseButton(e) = &gui_event {
                                if !e.pressed {
                                    let mut a11y_manager = accessibility_manager();
                                    a11y_manager
                                        .perform_action(button.id(), AccessibilityAction::Click);
                                }
                            }
                            needs_redraw = true;
                        }
                    }

                    if demo.nav_text_input.handle_event(&gui_event, &theme) == EventResult::Consumed
                    {
                        needs_redraw = true;
                    }

                    if demo.nav_combo.handle_event(&gui_event, &theme) == EventResult::Consumed {
                        needs_redraw = true;
                    }

                    if demo.nav_checkbox.handle_event(&gui_event, &theme) == EventResult::Consumed {
                        let mut a11y_manager = accessibility_manager();
                        a11y_manager.update_node_state(
                            demo.nav_checkbox.id(),
                            AccessibilityState {
                                checked: Some(demo.nav_checkbox.is_checked()),
                                ..Default::default()
                            },
                        );
                        needs_redraw = true;
                    }

                    if demo.announcements_text.handle_event(&gui_event, &theme)
                        == EventResult::Consumed
                    {
                        needs_redraw = true;
                    }

                    if needs_redraw {
                        demo.update_accessibility_announcements();
                        renderer.window().request_redraw();
                    }
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let section_height = (window_size.height - padding * 4) / 3;

                // Drag & Drop section
                let drag_section_y = padding;
                let list_width = (window_size.width - padding * 3) / 2;
                let list_height = section_height - 40;

                demo.drag_label.layout(
                    Rect::new(padding, drag_section_y, window_size.width - padding * 2, 30),
                    &theme,
                );

                demo.drag_source_list.layout(
                    Rect::new(padding, drag_section_y + 40, list_width, list_height),
                    &theme,
                );

                demo.drop_target_list.layout(
                    Rect::new(
                        padding * 2 + list_width,
                        drag_section_y + 40,
                        list_width,
                        list_height,
                    ),
                    &theme,
                );

                // Keyboard Navigation section
                let nav_section_y = drag_section_y + section_height + padding;
                let nav_label = Label::new(
                    WidgetId::default(),
                    "Keyboard Navigation (Use Tab/Arrow keys)",
                );
                let mut nav_label_mut = nav_label;
                nav_label_mut.layout(
                    Rect::new(padding, nav_section_y, window_size.width - padding * 2, 30),
                    &theme,
                );

                let button_width = 120;
                let button_height = 36;
                let widget_spacing = 10;

                let mut x = padding;
                let y = nav_section_y + 40;

                // Update keyboard nav bounds
                let mut nav_manager = keyboard_nav_manager();

                for button in &mut demo.nav_buttons {
                    button.layout(Rect::new(x, y, button_width, button_height), &theme);
                    nav_manager.update_widget_bounds(button.id(), button.bounds());
                    x += button_width + widget_spacing;
                }

                demo.nav_text_input
                    .layout(Rect::new(x, y, 200, button_height), &theme);
                nav_manager
                    .update_widget_bounds(demo.nav_text_input.id(), demo.nav_text_input.bounds());
                x += 200 + widget_spacing;

                demo.nav_combo
                    .layout(Rect::new(x, y, 150, button_height), &theme);
                nav_manager.update_widget_bounds(demo.nav_combo.id(), demo.nav_combo.bounds());

                demo.nav_checkbox
                    .layout(Rect::new(padding, y + button_height + 10, 200, 30), &theme);
                nav_manager
                    .update_widget_bounds(demo.nav_checkbox.id(), demo.nav_checkbox.bounds());

                // Accessibility section
                let a11y_section_y = nav_section_y + section_height + padding;

                demo.accessibility_label.layout(
                    Rect::new(padding, a11y_section_y, window_size.width - padding * 2, 30),
                    &theme,
                );

                demo.announcements_text.layout(
                    Rect::new(
                        padding,
                        a11y_section_y + 40,
                        window_size.width - padding * 2,
                        section_height - 50,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                // Draw section backgrounds
                renderer.set_color(theme.colors.surface);
                renderer.fill_rect(Rect::new(
                    padding - 10,
                    drag_section_y - 10,
                    window_size.width - padding * 2 + 20,
                    section_height,
                ));

                renderer.fill_rect(Rect::new(
                    padding - 10,
                    nav_section_y - 10,
                    window_size.width - padding * 2 + 20,
                    section_height,
                ));

                renderer.fill_rect(Rect::new(
                    padding - 10,
                    a11y_section_y - 10,
                    window_size.width - padding * 2 + 20,
                    section_height,
                ));

                // Draw widgets
                demo.drag_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.drag_source_list
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.drop_target_list
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                nav_label_mut.draw(&mut renderer as &mut dyn DrawContext, &theme);
                for button in &demo.nav_buttons {
                    button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                }
                demo.nav_text_input
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.nav_combo
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.nav_checkbox
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                demo.accessibility_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.announcements_text
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw drag preview
                let mut drag_manager = drag_drop_manager();
                if drag_manager.is_dragging() {
                    let pos = drag_manager.get_drag_visual_position();
                    renderer.set_color(theme.colors.primary.with_alpha(128));
                    renderer.fill_rect(Rect::new(pos.x, pos.y, 150, 30));

                    if let Some(drag_data) = drag_manager.get_drag_data() {
                        renderer.set_color(theme.colors.text);
                        renderer.draw_text("Dragging item", Point::new(pos.x + 10, pos.y + 20), 14);
                    }
                }

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

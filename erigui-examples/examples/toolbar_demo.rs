use erigui_core::*;
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct ToolbarDemo {
    // Main toolbar
    main_toolbar: Toolbar,

    // Vertical toolbar
    vertical_toolbar: Toolbar,

    // Labels
    status_label: Label,
    content_label: Label,

    // State
    view_mode: String,
    tool_mode: String,
}

impl ToolbarDemo {
    fn new() -> Self {
        // Create main horizontal toolbar
        let main_toolbar = Toolbar::new(WidgetId::default())
            .with_orientation(Orientation::Horizontal)
            .add_button("New", Some(ButtonIcon::File), || println!("New clicked"))
            .add_button("Open", Some(ButtonIcon::Folder), || {
                println!("Open clicked")
            })
            .add_button("Save", None, || println!("Save clicked"))
            .add_separator()
            .add_button("Cut", None, || println!("Cut clicked"))
            .add_button("Copy", None, || println!("Copy clicked"))
            .add_button("Paste", None, || println!("Paste clicked"))
            .add_separator()
            .add_dropdown_button(
                "View",
                None,
                vec![
                    MenuItem::new("Grid View").with_on_click(|| println!("Grid View")),
                    MenuItem::new("List View").with_on_click(|| println!("List View")),
                    MenuItem::new("Details View").with_on_click(|| println!("Details View")),
                ],
            )
            .add_separator()
            .add_toggle_button("Bold", None, Some(1), |pressed| {
                println!("Bold: {}", pressed);
            })
            .add_toggle_button("Italic", None, Some(1), |pressed| {
                println!("Italic: {}", pressed);
            })
            .add_toggle_button("Underline", None, Some(1), |pressed| {
                println!("Underline: {}", pressed);
            })
            .add_separator()
            .add_button("Help", None, || println!("Help clicked"));

        // Create vertical toolbar
        let vertical_toolbar = Toolbar::new(WidgetId::default())
            .with_orientation(Orientation::Vertical)
            .with_show_text(false) // Icons only
            .add_toggle_button("Select", Some(ButtonIcon::Search), Some(2), |pressed| {
                println!("Select tool: {}", pressed);
            })
            .add_toggle_button("Draw", None, Some(2), |pressed| {
                println!("Draw tool: {}", pressed);
            })
            .add_toggle_button("Move", None, Some(2), |pressed| {
                println!("Move tool: {}", pressed);
            })
            .add_separator()
            .add_button("Zoom In", None, || println!("Zoom In"))
            .add_button("Zoom Out", None, || println!("Zoom Out"))
            .add_button("Reset", Some(ButtonIcon::Home), || println!("Reset View"));

        Self {
            main_toolbar,
            vertical_toolbar,
            status_label: Label::new(WidgetId::default(), "Ready"),
            content_label: Label::new(
                WidgetId::default(),
                "Toolbar Demo - Showing horizontal and vertical toolbars with various button types",
            ),
            view_mode: "Grid".to_string(),
            tool_mode: "Select".to_string(),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer =
        Renderer::new(&event_loop, 800, 600, "Toolbar Demo").expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = ToolbarDemo::new();
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
                    let _ = demo.main_toolbar.handle_event(&gui_event, &theme);
                    let _ = demo.vertical_toolbar.handle_event(&gui_event, &theme);

                    renderer.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();

                // Main toolbar at top
                let main_toolbar_constraints = LayoutConstraints {
                    min_width: Some(window_size.width),
                    max_width: Some(window_size.width),
                    min_height: None,
                    max_height: None,
                };
                let main_toolbar_size =
                    demo.main_toolbar.measure(&main_toolbar_constraints, &theme);
                demo.main_toolbar.layout(
                    Rect::new(0, 0, window_size.width, main_toolbar_size.height),
                    &theme,
                );

                // Vertical toolbar on left
                let vertical_toolbar_constraints = LayoutConstraints {
                    min_width: None,
                    max_width: None,
                    min_height: Some(window_size.height - main_toolbar_size.height - 30),
                    max_height: Some(window_size.height - main_toolbar_size.height - 30),
                };
                let vertical_toolbar_size = demo
                    .vertical_toolbar
                    .measure(&vertical_toolbar_constraints, &theme);
                demo.vertical_toolbar.layout(
                    Rect::new(
                        0,
                        main_toolbar_size.height,
                        vertical_toolbar_size.width,
                        window_size.height - main_toolbar_size.height - 30,
                    ),
                    &theme,
                );

                // Content area
                let content_x = vertical_toolbar_size.width + 20;
                let content_y = main_toolbar_size.height + 20;
                let content_label_size = demo
                    .content_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.content_label.layout(
                    Rect::new(
                        content_x,
                        content_y,
                        content_label_size.width,
                        content_label_size.height,
                    ),
                    &theme,
                );

                // Status bar at bottom
                let status_label_size = demo
                    .status_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.status_label.layout(
                    Rect::new(
                        10,
                        window_size.height - status_label_size.height - 10,
                        status_label_size.width,
                        status_label_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.main_toolbar
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.vertical_toolbar
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.content_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.status_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw status bar background
                renderer.set_color(theme.colors.surface_variant);
                renderer.fill_rect(Rect::new(0, window_size.height - 30, window_size.width, 30));

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

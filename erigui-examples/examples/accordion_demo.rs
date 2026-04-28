use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct AccordionDemo {
    basic_accordion: Accordion,
    single_accordion: Accordion,
    info_label: Label,
}

impl AccordionDemo {
    fn new() -> Self {
        // Create basic accordion with multiple panels allowed
        let mut basic_accordion = Accordion::new(WidgetId::default())
            .with_allow_multiple(true)
            .with_animation(true)
            .with_on_toggle(|index, expanded| {
                println!(
                    "Panel {} {}",
                    index,
                    if expanded { "expanded" } else { "collapsed" }
                );
            });

        // Add panels with different content
        basic_accordion.add_panel(AccordionPanel::new(
            "General Settings",
            Box::new(Label::new(WidgetId::default(), "Application Settings:\n\n✓ Enable notifications\n   Auto-save documents\n✓ Show tips on startup"))
        ).with_expanded(true).with_icon("settings"));

        basic_accordion.add_panel(AccordionPanel::new(
            "Appearance",
            Box::new(Label::new(WidgetId::default(), "Theme Options:\n\n◯ Light Theme\n◉ Dark Theme\n◯ Auto (System)\n\nFont Size: 14"))
        ).with_icon("palette"));

        basic_accordion.add_panel(AccordionPanel::new(
            "Advanced Options",
            Box::new(Label::new(WidgetId::default(), "Performance Settings:\n\n✓ Hardware acceleration\n✓ Smooth scrolling\n\nCache Size: 100 MB"))
        ));

        // Create single expansion accordion
        let mut single_accordion = Accordion::new(WidgetId::default())
            .with_allow_multiple(false)
            .with_header_height(36)
            .with_animation_speed(0.2);

        single_accordion.add_panel(
            AccordionPanel::new(
                "Project Structure",
                Box::new(Label::new(
                    WidgetId::default(),
                    "src/\n  ├── main.rs\n  ├── lib.rs\n  └── modules/",
                )),
            )
            .with_expanded(true)
            .with_icon("folder"),
        );

        single_accordion.add_panel(AccordionPanel::new(
            "Dependencies",
            Box::new(Label::new(WidgetId::default(), "erigui-core = \"0.1.0\"\nerigui-widgets = \"0.1.0\"\nwinit = \"0.29\"\nwgpu = \"0.19\""))
        ).with_icon("folder"));

        single_accordion.add_panel(
            AccordionPanel::new(
                "Build Configuration",
                Box::new(Label::new(
                    WidgetId::default(),
                    "Target: x86_64-unknown-linux-gnu\nProfile: release\nOptimization: 3",
                )),
            )
            .with_icon("file"),
        );

        Self {
            basic_accordion,
            single_accordion,
            info_label: Label::new(
                WidgetId::default(),
                "Accordion Demo - Click headers to expand/collapse",
            ),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer =
        Renderer::new(&event_loop, 800, 700, "Accordion Demo").expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = AccordionDemo::new();

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
                    let basic_result = demo.basic_accordion.handle_event(&gui_event, &theme);
                    let single_result = demo.single_accordion.handle_event(&gui_event, &theme);

                    if basic_result == EventResult::Consumed
                        || single_result == EventResult::Consumed
                    {
                        renderer.window().request_redraw();
                    }
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;

                // Info label at top
                let info_size = demo
                    .info_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(
                    Rect::new(
                        (window_size.width - info_size.width) / 2,
                        padding,
                        info_size.width,
                        info_size.height,
                    ),
                    &theme,
                );

                let accordion_width = (window_size.width - padding * 3) / 2;
                let accordion_y = padding + info_size.height + 20;

                // Basic accordion on left
                let basic_constraints = LayoutConstraints {
                    min_width: Some(accordion_width),
                    max_width: Some(accordion_width),
                    min_height: None,
                    max_height: Some(window_size.height - accordion_y - padding),
                };

                let basic_size = demo.basic_accordion.measure(&basic_constraints, &theme);
                demo.basic_accordion.layout(
                    Rect::new(padding, accordion_y, accordion_width, basic_size.height),
                    &theme,
                );

                // Single accordion on right
                let single_constraints = LayoutConstraints {
                    min_width: Some(accordion_width),
                    max_width: Some(accordion_width),
                    min_height: None,
                    max_height: Some(window_size.height - accordion_y - padding),
                };

                let single_size = demo.single_accordion.measure(&single_constraints, &theme);
                demo.single_accordion.layout(
                    Rect::new(
                        padding * 2 + accordion_width,
                        accordion_y,
                        accordion_width,
                        single_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.info_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.basic_accordion
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.single_accordion
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw labels
                let mut label_renderer = &mut renderer as &mut dyn DrawContext;
                label_renderer.set_color(theme.colors.text_secondary);
                label_renderer.draw_text(
                    "Multiple Expansion",
                    Point::new(padding, accordion_y - 20),
                    theme.typography.font_size_small,
                );
                label_renderer.draw_text(
                    "Single Expansion",
                    Point::new(padding * 2 + accordion_width, accordion_y - 20),
                    theme.typography.font_size_small,
                );

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

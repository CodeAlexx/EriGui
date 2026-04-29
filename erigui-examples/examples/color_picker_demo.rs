use erigui_core::*;
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct ColorPickerDemo {
    // Different style pickers
    compact_picker: ColorPicker,
    square_picker: ColorPicker,
    no_alpha_picker: ColorPicker,

    // Labels to show selected colors
    compact_label: Label,
    square_label: Label,
    no_alpha_label: Label,

    // Preview containers
    preview_container: Container,
}

impl ColorPickerDemo {
    fn new() -> Self {
        let compact_picker = ColorPicker::new(WidgetId::default())
            .with_style(ColorPickerStyle::Compact)
            .with_color(Color::from_hex(0xFF5733))
            .with_on_change(|color| println!("Compact picker: {:?}", color));

        let square_picker = ColorPicker::new(WidgetId::default())
            .with_style(ColorPickerStyle::Square)
            .with_color(Color::from_hex(0x33FF57))
            .with_on_change(|color| println!("Square picker: {:?}", color));

        let no_alpha_picker = ColorPicker::new(WidgetId::default())
            .with_style(ColorPickerStyle::Compact)
            .with_alpha(false)
            .with_color(Color::from_hex(0x5733FF))
            .with_on_change(|color| println!("No alpha picker: {:?}", color));

        Self {
            compact_picker,
            square_picker,
            no_alpha_picker,
            compact_label: Label::new(WidgetId::default(), "Compact Color Picker (with alpha)"),
            square_label: Label::new(WidgetId::default(), "Square Color Picker"),
            no_alpha_label: Label::new(WidgetId::default(), "Compact Color Picker (no alpha)"),
            preview_container: Container::new(WidgetId::default()),
        }
    }

    fn update_preview(&mut self) {
        // Update label texts with color values
        let compact_color = self.compact_picker.get_color();
        self.compact_label.set_text(format!(
            "Compact: #{:02X}{:02X}{:02X}{:02X}",
            compact_color.r, compact_color.g, compact_color.b, compact_color.a
        ));

        let square_color = self.square_picker.get_color();
        self.square_label.set_text(format!(
            "Square: #{:02X}{:02X}{:02X}{:02X}",
            square_color.r, square_color.g, square_color.b, square_color.a
        ));

        let no_alpha_color = self.no_alpha_picker.get_color();
        self.no_alpha_label.set_text(format!(
            "No Alpha: #{:02X}{:02X}{:02X}",
            no_alpha_color.r, no_alpha_color.g, no_alpha_color.b
        ));
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer = Renderer::new(&event_loop, 800, 600, "Color Picker Demo")
        .expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = ColorPickerDemo::new();
    demo.update_preview();
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
                    let _ = demo.compact_picker.handle_event(&gui_event, &theme);
                    let _ = demo.square_picker.handle_event(&gui_event, &theme);
                    let _ = demo.no_alpha_picker.handle_event(&gui_event, &theme);

                    demo.update_preview();
                    renderer.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let section_spacing = 60;

                let mut y = padding;

                // Compact picker section
                let compact_label_size = demo
                    .compact_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.compact_label.layout(
                    Rect::new(
                        padding,
                        y,
                        compact_label_size.width,
                        compact_label_size.height,
                    ),
                    &theme,
                );
                y += compact_label_size.height + 10;

                let compact_size = demo
                    .compact_picker
                    .measure(&LayoutConstraints::default(), &theme);
                demo.compact_picker.layout(
                    Rect::new(padding, y, compact_size.width, compact_size.height),
                    &theme,
                );

                // Color preview box for compact picker
                let preview_size = 60;
                let compact_color = demo.compact_picker.get_color();
                let compact_preview_rect = Rect::new(
                    padding + compact_size.width + 20,
                    y - 10,
                    preview_size,
                    preview_size,
                );

                y += compact_size.height + section_spacing;

                // No alpha picker section
                let no_alpha_label_size = demo
                    .no_alpha_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.no_alpha_label.layout(
                    Rect::new(
                        padding,
                        y,
                        no_alpha_label_size.width,
                        no_alpha_label_size.height,
                    ),
                    &theme,
                );
                y += no_alpha_label_size.height + 10;

                let no_alpha_size = demo
                    .no_alpha_picker
                    .measure(&LayoutConstraints::default(), &theme);
                demo.no_alpha_picker.layout(
                    Rect::new(padding, y, no_alpha_size.width, no_alpha_size.height),
                    &theme,
                );

                // Color preview box for no alpha picker
                let no_alpha_color = demo.no_alpha_picker.get_color();
                let no_alpha_preview_rect = Rect::new(
                    padding + no_alpha_size.width + 20,
                    y - 10,
                    preview_size,
                    preview_size,
                );

                y += no_alpha_size.height + section_spacing;

                // Square picker section (always visible)
                let square_label_size = demo
                    .square_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.square_label.layout(
                    Rect::new(
                        padding + 300,
                        padding,
                        square_label_size.width,
                        square_label_size.height,
                    ),
                    &theme,
                );

                let square_size = demo
                    .square_picker
                    .measure(&LayoutConstraints::default(), &theme);
                demo.square_picker.layout(
                    Rect::new(
                        padding + 300,
                        padding + square_label_size.height + 10,
                        square_size.width,
                        square_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                // Draw labels and pickers
                demo.compact_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.compact_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                demo.no_alpha_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.no_alpha_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                demo.square_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.square_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw preview boxes
                // Compact picker preview
                renderer.set_color(Color::from_hex(0xCCCCCC));
                renderer.fill_rect(compact_preview_rect);
                renderer.set_color(Color::from_hex(0xFFFFFF));
                renderer.fill_rect(Rect::new(
                    compact_preview_rect.x() + preview_size / 2,
                    compact_preview_rect.y(),
                    preview_size / 2,
                    preview_size / 2,
                ));
                renderer.fill_rect(Rect::new(
                    compact_preview_rect.x(),
                    compact_preview_rect.y() + preview_size / 2,
                    preview_size / 2,
                    preview_size / 2,
                ));
                renderer.set_color(compact_color);
                renderer.fill_rect(compact_preview_rect);
                renderer.set_color(theme.colors.border);
                renderer.draw_rect(compact_preview_rect);

                // No alpha picker preview
                renderer.set_color(no_alpha_color);
                renderer.fill_rect(no_alpha_preview_rect);
                renderer.set_color(theme.colors.border);
                renderer.draw_rect(no_alpha_preview_rect);

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

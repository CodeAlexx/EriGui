use erigui_core::*;
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct TextAreaDemo {
    text_area: TextArea,
    info_label: Label,
    line_numbers_checkbox: Checkbox,
    word_wrap_checkbox: Checkbox,
}

impl TextAreaDemo {
    fn new() -> Self {
        let sample_text = r#"Welcome to the TextArea Demo!

This is a multi-line text editor with the following features:
- Line numbers (toggle with checkbox)
- Selection support (click and drag or shift+arrows)
- Basic editing operations
- Undo/Redo (Ctrl+Z/Ctrl+Y)
- Select All (Ctrl+A)
- Scrolling with mouse wheel
- Tab support

Try editing this text to see it in action!

fn hello_world() {
    println!("Hello, TextArea!");
}

The text area supports multiple lines and basic text editing.
You can navigate with arrow keys, Home/End, and Ctrl+Home/End.
Mouse selection and keyboard selection are both supported."#;

        let text_area = TextArea::new(WidgetId::default())
            .with_text(sample_text)
            .with_line_numbers(true)
            .with_on_change(|text| {
                println!(
                    "Text changed, {} lines, {} chars",
                    text.lines().count(),
                    text.len()
                );
            });

        Self {
            text_area,
            info_label: Label::new(
                WidgetId::default(),
                "TextArea Demo - Multi-line text editor",
            ),
            line_numbers_checkbox: Checkbox::new(WidgetId::default(), "Show line numbers")
                .with_checked(true),
            word_wrap_checkbox: Checkbox::new(WidgetId::default(), "Word wrap").with_checked(false),
        }
    }

    fn update_settings(&mut self) {
        // Update line numbers visibility
        if self.line_numbers_checkbox.is_checked() != self.text_area.line_numbers {
            self.text_area.line_numbers = self.line_numbers_checkbox.is_checked();
        }

        // Update word wrap
        if self.word_wrap_checkbox.is_checked() != self.text_area.word_wrap {
            self.text_area.word_wrap = self.word_wrap_checkbox.is_checked();
        }

        // Update info label
        let text = self.text_area.get_text();
        let lines = text.lines().count();
        let chars = text.len();
        let selected = self
            .text_area
            .get_selected_text()
            .map(|s| s.len())
            .unwrap_or(0);

        let info = if selected > 0 {
            format!(
                "Lines: {} | Characters: {} | Selected: {}",
                lines, chars, selected
            )
        } else {
            format!("Lines: {} | Characters: {}", lines, chars)
        };

        self.info_label.set_text(info);
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer =
        Renderer::new(&event_loop, 900, 700, "TextArea Demo").expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = TextAreaDemo::new();
    let mut translator = EventTranslator::new(renderer.viewport_size());

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                let mut handled = false;
                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                        handled = true;
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        translator.set_window_size(renderer.viewport_size());
                        handled = true;
                    }
                    // (winit 0.29) WindowEvent::ReceivedCharacter arm removed; EventTranslator now derives TextInput from KeyEvent.text
                    _ => {}
                }

                // Convert window event to EriGui event
                if handled {
                    renderer.window().request_redraw();
                } else if let Some(gui_event) = translator.translate(&event) {
                    let _ = demo.text_area.handle_event(&gui_event, &theme);
                    let _ = demo.line_numbers_checkbox.handle_event(&gui_event, &theme);
                    let _ = demo.word_wrap_checkbox.handle_event(&gui_event, &theme);

                    demo.update_settings();
                    renderer.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;

                // Controls at top
                let checkbox_size = demo
                    .line_numbers_checkbox
                    .measure(&LayoutConstraints::default(), &theme);
                demo.line_numbers_checkbox.layout(
                    Rect::new(padding, padding, checkbox_size.width, checkbox_size.height),
                    &theme,
                );

                let wrap_checkbox_size = demo
                    .word_wrap_checkbox
                    .measure(&LayoutConstraints::default(), &theme);
                demo.word_wrap_checkbox.layout(
                    Rect::new(
                        padding + checkbox_size.width + 20,
                        padding,
                        wrap_checkbox_size.width,
                        wrap_checkbox_size.height,
                    ),
                    &theme,
                );

                // Text area in center
                let text_area_y = padding + checkbox_size.height + 10;
                let text_area_height = window_size.height - text_area_y - 50 - padding;

                demo.text_area.layout(
                    Rect::new(
                        padding,
                        text_area_y,
                        window_size.width - padding * 2,
                        text_area_height,
                    ),
                    &theme,
                );

                // Info label at bottom
                let info_size = demo
                    .info_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(
                    Rect::new(
                        padding,
                        window_size.height - padding - info_size.height,
                        info_size.width,
                        info_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.line_numbers_checkbox
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.word_wrap_checkbox
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.text_area
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.info_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

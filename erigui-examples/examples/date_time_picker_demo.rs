use chrono::{Local, NaiveDateTime};
use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct DateTimePickerDemo {
    date_picker: DateTimePicker,
    time_picker: DateTimePicker,
    datetime_picker: DateTimePicker,
    custom_picker: DateTimePicker,
    info_label: Label,
    result_label: Label,
}

impl DateTimePickerDemo {
    fn new() -> Self {
        let now = Local::now().naive_local();

        // Date only picker
        let date_picker = DateTimePicker::new(WidgetId::default())
            .with_mode(DateTimePickerMode::Date)
            .with_date_format(DateFormat::YMD)
            .with_value(now)
            .with_on_change(|datetime| {
                println!("Date selected: {}", datetime.date());
            });

        // Time only picker
        let time_picker = DateTimePicker::new(WidgetId::default())
            .with_mode(DateTimePickerMode::Time)
            .with_time_format(TimeFormat::H24)
            .with_value(now)
            .with_on_change(|datetime| {
                println!("Time selected: {}", datetime.time());
            });

        // DateTime picker
        let datetime_picker = DateTimePicker::new(WidgetId::default())
            .with_mode(DateTimePickerMode::DateTime)
            .with_date_format(DateFormat::DMY)
            .with_time_format(TimeFormat::H12)
            .with_value(now)
            .with_on_change(|datetime| {
                println!("DateTime selected: {}", datetime);
            });

        // Custom format picker
        let custom_picker = DateTimePicker::new(WidgetId::default())
            .with_mode(DateTimePickerMode::DateTime)
            .with_date_format(DateFormat::MDY)
            .with_time_format(TimeFormat::H12)
            .with_value(now)
            .with_on_change(|datetime| {
                println!("Custom format selected: {}", datetime);
            });

        Self {
            date_picker,
            time_picker,
            datetime_picker,
            custom_picker,
            info_label: Label::new(
                WidgetId::default(),
                "DateTimePicker Demo - Click calendar button to select",
            ),
            result_label: Label::new(WidgetId::default(), "Selected values will appear here"),
        }
    }

    fn update_result_label(&mut self) {
        let date_val = self.date_picker.get_value().date();
        let time_val = self.time_picker.get_value().time();
        let datetime_val = self.datetime_picker.get_value();
        let custom_val = self.custom_picker.get_value();

        let result = format!(
            "Date: {} | Time: {} | DateTime: {} | Custom: {}",
            date_val.format("%Y-%m-%d"),
            time_val.format("%H:%M"),
            datetime_val.format("%d/%m/%Y %I:%M %p"),
            custom_val.format("%m/%d/%Y %I:%M %p")
        );

        self.result_label.set_text(result);
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let mut renderer = Renderer::new(&event_loop, 900, 700, "DateTimePicker Demo")
        .expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = DateTimePickerDemo::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
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
                    let date_result = demo.date_picker.handle_event(&gui_event, &theme);
                    let time_result = demo.time_picker.handle_event(&gui_event, &theme);
                    let datetime_result = demo.datetime_picker.handle_event(&gui_event, &theme);
                    let custom_result = demo.custom_picker.handle_event(&gui_event, &theme);

                    if date_result == EventResult::Consumed
                        || time_result == EventResult::Consumed
                        || datetime_result == EventResult::Consumed
                        || custom_result == EventResult::Consumed
                    {
                        demo.update_result_label();
                        renderer.window().request_redraw();
                    }
                }
            }

            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let picker_width = 250;
                let picker_height = 40;
                let spacing = 60;

                let mut y = padding;

                // Info label
                let info_size = demo
                    .info_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(
                    Rect::new(
                        (window_size.width - info_size.width) / 2,
                        y,
                        info_size.width,
                        info_size.height,
                    ),
                    &theme,
                );
                y += info_size.height + spacing;

                // Date picker
                demo.date_picker
                    .layout(Rect::new(padding, y, picker_width, picker_height), &theme);

                // Time picker
                demo.time_picker.layout(
                    Rect::new(padding + picker_width + 40, y, picker_width, picker_height),
                    &theme,
                );
                y += picker_height + spacing;

                // DateTime picker
                demo.datetime_picker
                    .layout(Rect::new(padding, y, picker_width, picker_height), &theme);

                // Custom format picker
                demo.custom_picker.layout(
                    Rect::new(padding + picker_width + 40, y, picker_width, picker_height),
                    &theme,
                );
                y += picker_height + 350; // Extra space for popups

                // Result label
                let result_size = demo
                    .result_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.result_label.layout(
                    Rect::new(
                        (window_size.width - result_size.width) / 2,
                        y,
                        result_size.width,
                        result_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.info_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                // Draw labels
                let mut label_renderer = &mut renderer as &mut dyn DrawContext;
                label_renderer.set_color(theme.colors.text_secondary);

                label_renderer.draw_text(
                    "Date Only (YYYY-MM-DD):",
                    Point::new(padding, demo.date_picker.bounds().y() - 20),
                    theme.typography.font_size_small,
                );

                label_renderer.draw_text(
                    "Time Only (24-hour):",
                    Point::new(
                        padding + picker_width + 40,
                        demo.time_picker.bounds().y() - 20,
                    ),
                    theme.typography.font_size_small,
                );

                label_renderer.draw_text(
                    "DateTime (DD/MM/YYYY 12-hour):",
                    Point::new(padding, demo.datetime_picker.bounds().y() - 20),
                    theme.typography.font_size_small,
                );

                label_renderer.draw_text(
                    "Custom Format (MM/DD/YYYY 12-hour):",
                    Point::new(
                        padding + picker_width + 40,
                        demo.custom_picker.bounds().y() - 20,
                    ),
                    theme.typography.font_size_small,
                );

                // Draw pickers
                demo.date_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.time_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.datetime_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.custom_picker
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.result_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                renderer.end_frame();
            }

            _ => {}
        }
    });
}

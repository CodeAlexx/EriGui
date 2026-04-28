use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use std::cell::RefCell;
use std::rc::Rc;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct RadioButtonDemo {
    // Radio buttons for size selection
    size_small: RadioButton,
    size_medium: RadioButton,
    size_large: RadioButton,

    // Radio buttons for color selection
    color_red: RadioButton,
    color_green: RadioButton,
    color_blue: RadioButton,

    // Radio buttons with different label positions
    pos_left: RadioButton,
    pos_right: RadioButton,
    pos_top: RadioButton,
    pos_bottom: RadioButton,

    // Labels to show selection
    size_label: Label,
    color_label: Label,
    position_label: Label,

    selected_size: String,
    selected_color: String,
    selected_position: String,
}

impl RadioButtonDemo {
    fn new() -> Self {
        // One host-owned RadioGroup per logical mutual-exclusion set.
        // Replaces the deleted `RadioGroupManager` global singleton.
        let size_group = Rc::new(RefCell::new(RadioGroup::new()));
        let color_group = Rc::new(RefCell::new(RadioGroup::new()));
        let position_group = Rc::new(RefCell::new(RadioGroup::new()));

        // Size selection group
        let size_small = RadioButton::new(WidgetId::default(), "Small")
            .with_group(size_group.clone())
            .with_on_change(|_| println!("Size: Small selected"));

        let size_medium = RadioButton::new(WidgetId::default(), "Medium")
            .with_checked(true)
            .with_group(size_group.clone())
            .with_on_change(|_| println!("Size: Medium selected"));

        let size_large = RadioButton::new(WidgetId::default(), "Large")
            .with_group(size_group)
            .with_on_change(|_| println!("Size: Large selected"));

        // Color selection group
        let color_red = RadioButton::new(WidgetId::default(), "Red")
            .with_checked(true)
            .with_group(color_group.clone())
            .with_on_change(|_| println!("Color: Red selected"));

        let color_green = RadioButton::new(WidgetId::default(), "Green")
            .with_group(color_group.clone())
            .with_on_change(|_| println!("Color: Green selected"));

        let color_blue = RadioButton::new(WidgetId::default(), "Blue")
            .with_group(color_group)
            .with_on_change(|_| println!("Color: Blue selected"));

        // Label position demonstration
        let pos_left = RadioButton::new(WidgetId::default(), "Label Left")
            .with_label_position(LabelPosition::Left)
            .with_checked(true)
            .with_group(position_group.clone());

        let pos_right = RadioButton::new(WidgetId::default(), "Label Right")
            .with_label_position(LabelPosition::Right)
            .with_group(position_group.clone());

        let pos_top = RadioButton::new(WidgetId::default(), "Label Top")
            .with_label_position(LabelPosition::Top)
            .with_group(position_group.clone());

        let pos_bottom = RadioButton::new(WidgetId::default(), "Label Bottom")
            .with_label_position(LabelPosition::Bottom)
            .with_group(position_group);

        Self {
            size_small,
            size_medium,
            size_large,
            color_red,
            color_green,
            color_blue,
            pos_left,
            pos_right,
            pos_top,
            pos_bottom,
            size_label: Label::new(WidgetId::default(), "Selected size: Medium"),
            color_label: Label::new(WidgetId::default(), "Selected color: Red"),
            position_label: Label::new(WidgetId::default(), "Label Position Demo"),
            selected_size: "Medium".to_string(),
            selected_color: "Red".to_string(),
            selected_position: "Left".to_string(),
        }
    }

    fn update_labels(&mut self) {
        // Update size selection
        if self.size_small.is_checked() {
            self.selected_size = "Small".to_string();
        } else if self.size_medium.is_checked() {
            self.selected_size = "Medium".to_string();
        } else if self.size_large.is_checked() {
            self.selected_size = "Large".to_string();
        }

        // Update color selection
        if self.color_red.is_checked() {
            self.selected_color = "Red".to_string();
        } else if self.color_green.is_checked() {
            self.selected_color = "Green".to_string();
        } else if self.color_blue.is_checked() {
            self.selected_color = "Blue".to_string();
        }

        // Update position selection
        if self.pos_left.is_checked() {
            self.selected_position = "Left".to_string();
        } else if self.pos_right.is_checked() {
            self.selected_position = "Right".to_string();
        } else if self.pos_top.is_checked() {
            self.selected_position = "Top".to_string();
        } else if self.pos_bottom.is_checked() {
            self.selected_position = "Bottom".to_string();
        }

        self.size_label
            .set_text(format!("Selected size: {}", self.selected_size));
        self.color_label
            .set_text(format!("Selected color: {}", self.selected_color));
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer = Renderer::new(&event_loop, 800, 600, "Radio Button Demo")
        .expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = RadioButtonDemo::new();

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
                    // Send Update event for animations
                    let _ = demo.size_small.handle_event(&Event::Update, &theme);
                    let _ = demo.size_medium.handle_event(&Event::Update, &theme);
                    let _ = demo.size_large.handle_event(&Event::Update, &theme);
                    let _ = demo.color_red.handle_event(&Event::Update, &theme);
                    let _ = demo.color_green.handle_event(&Event::Update, &theme);
                    let _ = demo.color_blue.handle_event(&Event::Update, &theme);
                    let _ = demo.pos_left.handle_event(&Event::Update, &theme);
                    let _ = demo.pos_right.handle_event(&Event::Update, &theme);
                    let _ = demo.pos_top.handle_event(&Event::Update, &theme);
                    let _ = demo.pos_bottom.handle_event(&Event::Update, &theme);

                    // Handle the actual event
                    let _ = demo.size_small.handle_event(&gui_event, &theme);
                    let _ = demo.size_medium.handle_event(&gui_event, &theme);
                    let _ = demo.size_large.handle_event(&gui_event, &theme);
                    let _ = demo.color_red.handle_event(&gui_event, &theme);
                    let _ = demo.color_green.handle_event(&gui_event, &theme);
                    let _ = demo.color_blue.handle_event(&gui_event, &theme);
                    let _ = demo.pos_left.handle_event(&gui_event, &theme);
                    let _ = demo.pos_right.handle_event(&gui_event, &theme);
                    let _ = demo.pos_top.handle_event(&gui_event, &theme);
                    let _ = demo.pos_bottom.handle_event(&gui_event, &theme);

                    demo.update_labels();
                    renderer.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let section_spacing = 40;
                let item_spacing = 30;

                // Size selection section
                let mut y = padding;
                let size_label_size = demo
                    .size_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.size_label.layout(
                    Rect::new(padding, y, size_label_size.width, size_label_size.height),
                    &theme,
                );
                y += size_label_size.height + 10;

                let size_small_size = demo
                    .size_small
                    .measure(&LayoutConstraints::default(), &theme);
                demo.size_small.layout(
                    Rect::new(padding, y, size_small_size.width, size_small_size.height),
                    &theme,
                );
                y += item_spacing;

                let size_medium_size = demo
                    .size_medium
                    .measure(&LayoutConstraints::default(), &theme);
                demo.size_medium.layout(
                    Rect::new(padding, y, size_medium_size.width, size_medium_size.height),
                    &theme,
                );
                y += item_spacing;

                let size_large_size = demo
                    .size_large
                    .measure(&LayoutConstraints::default(), &theme);
                demo.size_large.layout(
                    Rect::new(padding, y, size_large_size.width, size_large_size.height),
                    &theme,
                );
                y += section_spacing;

                // Color selection section
                let color_label_size = demo
                    .color_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.color_label.layout(
                    Rect::new(padding, y, color_label_size.width, color_label_size.height),
                    &theme,
                );
                y += color_label_size.height + 10;

                let color_red_size = demo
                    .color_red
                    .measure(&LayoutConstraints::default(), &theme);
                demo.color_red.layout(
                    Rect::new(padding, y, color_red_size.width, color_red_size.height),
                    &theme,
                );
                y += item_spacing;

                let color_green_size = demo
                    .color_green
                    .measure(&LayoutConstraints::default(), &theme);
                demo.color_green.layout(
                    Rect::new(padding, y, color_green_size.width, color_green_size.height),
                    &theme,
                );
                y += item_spacing;

                let color_blue_size = demo
                    .color_blue
                    .measure(&LayoutConstraints::default(), &theme);
                demo.color_blue.layout(
                    Rect::new(padding, y, color_blue_size.width, color_blue_size.height),
                    &theme,
                );
                y += section_spacing;

                // Label position demonstration
                let position_label_size = demo
                    .position_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.position_label.layout(
                    Rect::new(
                        padding,
                        y,
                        position_label_size.width,
                        position_label_size.height,
                    ),
                    &theme,
                );
                y += position_label_size.height + 20;

                // Position radio buttons in a grid
                let pos_left_size = demo.pos_left.measure(&LayoutConstraints::default(), &theme);
                demo.pos_left.layout(
                    Rect::new(padding, y, pos_left_size.width + 20, pos_left_size.height),
                    &theme,
                );

                let pos_right_size = demo
                    .pos_right
                    .measure(&LayoutConstraints::default(), &theme);
                demo.pos_right.layout(
                    Rect::new(
                        padding + 200,
                        y,
                        pos_right_size.width,
                        pos_right_size.height,
                    ),
                    &theme,
                );

                y += 50;

                let pos_top_size = demo.pos_top.measure(&LayoutConstraints::default(), &theme);
                demo.pos_top.layout(
                    Rect::new(padding, y, pos_top_size.width, pos_top_size.height + 20),
                    &theme,
                );

                let pos_bottom_size = demo
                    .pos_bottom
                    .measure(&LayoutConstraints::default(), &theme);
                demo.pos_bottom.layout(
                    Rect::new(
                        padding + 200,
                        y,
                        pos_bottom_size.width,
                        pos_bottom_size.height + 20,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.size_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.size_small
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.size_medium
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.size_large
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                demo.color_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.color_red
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.color_green
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.color_blue
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                demo.position_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.pos_left
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.pos_right
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.pos_top
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.pos_bottom
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                renderer.end_frame();
            }

            _ => {}
        }
    }).unwrap();
}

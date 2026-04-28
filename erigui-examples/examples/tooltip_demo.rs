use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

// Per-widget tooltips replace the old global TooltipManager singleton.
// Each Button owns its TooltipState directly via `with_tooltip` /
// `with_tooltip_state`. Hover-to-show is driven from the button's own
// event stream — no central manager polling, no per-frame Mutex.
//
// Checkbox and Slider don't yet have built-in tooltip support; once
// they adopt TooltipState the same way Button did, they can be
// demonstrated here. For now, only buttons carry tooltips.
struct TooltipDemo {
    button_auto: Button,
    button_above: Button,
    button_below: Button,
    button_left: Button,
    button_right: Button,

    checkbox: Checkbox,
    slider: Slider,

    title_label: Label,
    hover_label: Label,
}

impl TooltipDemo {
    fn new() -> Self {
        let button_auto = Button::new(WidgetId::default(), "Auto Position")
            .with_tooltip_state(
                TooltipState::new("This tooltip automatically finds the best position")
                    .with_position(TooltipPosition::Auto),
            )
            .with_on_click(|| println!("Auto button clicked"));

        let button_above = Button::new(WidgetId::default(), "Above")
            .with_tooltip_state(
                TooltipState::new("This tooltip appears above the button")
                    .with_position(TooltipPosition::Above),
            )
            .with_on_click(|| println!("Above button clicked"));

        let button_below = Button::new(WidgetId::default(), "Below")
            .with_tooltip_state(
                TooltipState::new("This tooltip appears below the button")
                    .with_position(TooltipPosition::Below),
            )
            .with_on_click(|| println!("Below button clicked"));

        let button_left = Button::new(WidgetId::default(), "Left")
            .with_tooltip_state(
                TooltipState::new("This tooltip appears to the left")
                    .with_position(TooltipPosition::Left),
            )
            .with_on_click(|| println!("Left button clicked"));

        let button_right = Button::new(WidgetId::default(), "Right")
            .with_tooltip_state(
                TooltipState::new("This tooltip appears to the right")
                    .with_position(TooltipPosition::Right),
            )
            .with_on_click(|| println!("Right button clicked"));

        Self {
            button_auto,
            button_above,
            button_below,
            button_left,
            button_right,
            checkbox: Checkbox::new(WidgetId::default(), "Enable feature"),
            slider: Slider::new(WidgetId::default(), 0.0, 100.0, 50.0),
            title_label: Label::new(
                WidgetId::default(),
                "Tooltip Demo - Hover over buttons to see tooltips",
            ),
            hover_label: Label::new(
                WidgetId::default(),
                "Hover state: hover any button for ~500ms",
            ),
        }
    }

    fn check_hover_label(&mut self, mouse_pos: Point) {
        let hovered = if self.button_auto.bounds().contains(mouse_pos) {
            Some("Auto")
        } else if self.button_above.bounds().contains(mouse_pos) {
            Some("Above")
        } else if self.button_below.bounds().contains(mouse_pos) {
            Some("Below")
        } else if self.button_left.bounds().contains(mouse_pos) {
            Some("Left")
        } else if self.button_right.bounds().contains(mouse_pos) {
            Some("Right")
        } else {
            None
        };

        match hovered {
            Some(name) => self.hover_label.set_text(format!("Hovering: {}", name)),
            None => self.hover_label.set_text("Hover state: Nothing"),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut renderer =
        Renderer::new(&event_loop, 800, 600, "Tooltip Demo").expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = TooltipDemo::new();

    event_loop
        .run(move |event, elwt| {
            // Poll so the hover-to-show timers fire even when the
            // cursor is sitting still — without continuous redraws,
            // the 500 ms delay tooltip would never elapse.
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                WinitEvent::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    elwt.exit();
                }
                WinitEvent::WindowEvent {
                    event: WindowEvent::Resized(physical_size),
                    ..
                } => {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                WinitEvent::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    // Layout
                    let window_size = renderer.viewport_size();
                    let padding = 20;
                    let button_spacing = 10;
                    let section_spacing = 40;

                    let title_size = demo
                        .title_label
                        .measure(&LayoutConstraints::default(), &theme);
                    demo.title_label.layout(
                        Rect::new(padding, padding, title_size.width, title_size.height),
                        &theme,
                    );

                    let button_size = Size::new(120, 32);

                    // Center button (auto position)
                    demo.button_auto.layout(
                        Rect::new(
                            window_size.width / 2 - button_size.width / 2,
                            window_size.height / 2 - button_size.height / 2,
                            button_size.width,
                            button_size.height,
                        ),
                        &theme,
                    );

                    // Top button (tooltip below)
                    demo.button_below.layout(
                        Rect::new(
                            window_size.width / 2 - button_size.width / 2,
                            padding + title_size.height + section_spacing,
                            button_size.width,
                            button_size.height,
                        ),
                        &theme,
                    );

                    // Bottom button (tooltip above)
                    demo.button_above.layout(
                        Rect::new(
                            window_size.width / 2 - button_size.width / 2,
                            window_size.height - padding - button_size.height,
                            button_size.width,
                            button_size.height,
                        ),
                        &theme,
                    );

                    // Left edge button (tooltip right)
                    demo.button_right.layout(
                        Rect::new(
                            padding,
                            window_size.height / 2 - button_size.height / 2,
                            button_size.width,
                            button_size.height,
                        ),
                        &theme,
                    );

                    // Right edge button (tooltip left)
                    demo.button_left.layout(
                        Rect::new(
                            window_size.width - padding - button_size.width,
                            window_size.height / 2 - button_size.height / 2,
                            button_size.width,
                            button_size.height,
                        ),
                        &theme,
                    );

                    let mut y = padding
                        + title_size.height
                        + section_spacing
                        + button_size.height
                        + section_spacing;

                    let checkbox_size =
                        demo.checkbox.measure(&LayoutConstraints::default(), &theme);
                    demo.checkbox.layout(
                        Rect::new(padding, y, checkbox_size.width, checkbox_size.height),
                        &theme,
                    );

                    y += checkbox_size.height + button_spacing;

                    let slider_size = Size::new(200, 24);
                    demo.slider.layout(
                        Rect::new(padding, y, slider_size.width, slider_size.height),
                        &theme,
                    );

                    let hover_size = demo
                        .hover_label
                        .measure(&LayoutConstraints::default(), &theme);
                    demo.hover_label.layout(
                        Rect::new(
                            padding,
                            window_size.height - padding - hover_size.height - 40,
                            hover_size.width,
                            hover_size.height,
                        ),
                        &theme,
                    );

                    // Draw
                    renderer.begin_frame(theme.colors.background);

                    demo.title_label
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    // Each button now draws its own tooltip after its
                    // chrome — no central tooltip_manager().draw() pass
                    // at the end of the frame.
                    demo.button_auto
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.button_above
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.button_below
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.button_left
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.button_right
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.checkbox
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.slider
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);
                    demo.hover_label
                        .draw(&mut renderer as &mut dyn DrawContext, &theme);

                    renderer.end_frame();
                }
                WinitEvent::WindowEvent { event, .. } => {
                    // All other window events: convert to GUI event and
                    // dispatch. Each button drives its own tooltip from
                    // handle_event.
                    if let Some(gui_event) = erigui_rendering::window::convert_window_event(
                        event,
                        renderer.viewport_size(),
                    ) {
                        if let Event::MouseMove(move_event) = &gui_event {
                            demo.check_hover_label(move_event.position);
                        }

                        let _ = demo.button_auto.handle_event(&gui_event, &theme);
                        let _ = demo.button_above.handle_event(&gui_event, &theme);
                        let _ = demo.button_below.handle_event(&gui_event, &theme);
                        let _ = demo.button_left.handle_event(&gui_event, &theme);
                        let _ = demo.button_right.handle_event(&gui_event, &theme);
                        let _ = demo.checkbox.handle_event(&gui_event, &theme);
                        let _ = demo.slider.handle_event(&gui_event, &theme);

                        renderer.window().request_redraw();
                    }
                }
                WinitEvent::AboutToWait => {
                    // Continuous redraws so hover-to-show timers tick
                    // forward even when the cursor is stationary.
                    renderer.window().request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}

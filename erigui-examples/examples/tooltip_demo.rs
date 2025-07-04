use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct TooltipDemo {
    // Buttons with different tooltip positions
    button_auto: Button,
    button_above: Button,
    button_below: Button,
    button_left: Button,
    button_right: Button,
    
    // Other widgets with tooltips
    checkbox: Checkbox,
    slider: Slider,
    
    // Labels
    title_label: Label,
    hover_label: Label,
    
    // Track which widget is hovered
    hovered_widget: Option<WidgetId>,
}

impl TooltipDemo {
    fn new() -> Self {
        // Register tooltips with the manager
        let button_auto = Button::new(WidgetId::default(), "Auto Position")
            .with_tooltip("This tooltip automatically finds the best position")
            .with_on_click(|| println!("Auto button clicked"));
        tooltip_manager().register_tooltip(
            button_auto.id(),
            Tooltip::new(button_auto.id(), "This tooltip automatically finds the best position")
                .with_position(TooltipPosition::Auto)
        );
        
        let button_above = Button::new(WidgetId::default(), "Above")
            .with_tooltip("This tooltip appears above the button")
            .with_on_click(|| println!("Above button clicked"));
        tooltip_manager().register_tooltip(
            button_above.id(),
            Tooltip::new(button_above.id(), "This tooltip appears above the button")
                .with_position(TooltipPosition::Above)
        );
        
        let button_below = Button::new(WidgetId::default(), "Below")
            .with_tooltip("This tooltip appears below the button")
            .with_on_click(|| println!("Below button clicked"));
        tooltip_manager().register_tooltip(
            button_below.id(),
            Tooltip::new(button_below.id(), "This tooltip appears below the button")
                .with_position(TooltipPosition::Below)
        );
        
        let button_left = Button::new(WidgetId::default(), "Left")
            .with_tooltip("This tooltip appears to the left")
            .with_on_click(|| println!("Left button clicked"));
        tooltip_manager().register_tooltip(
            button_left.id(),
            Tooltip::new(button_left.id(), "This tooltip appears to the left")
                .with_position(TooltipPosition::Left)
        );
        
        let button_right = Button::new(WidgetId::default(), "Right")
            .with_tooltip("This tooltip appears to the right")
            .with_on_click(|| println!("Right button clicked"));
        tooltip_manager().register_tooltip(
            button_right.id(),
            Tooltip::new(button_right.id(), "This tooltip appears to the right")
                .with_position(TooltipPosition::Right)
        );
        
        let checkbox = Checkbox::new(WidgetId::default(), "Enable feature");
        tooltip_manager().register_tooltip(
            checkbox.id(),
            Tooltip::new(checkbox.id(), "Check this box to enable the special feature")
                .with_delay(300)
        );
        
        let slider = Slider::new(WidgetId::default())
            .with_range(0.0, 100.0)
            .with_value(50.0);
        tooltip_manager().register_tooltip(
            slider.id(),
            Tooltip::new(slider.id(), "Drag to adjust the value")
                .with_delay(800)
        );
        
        Self {
            button_auto,
            button_above,
            button_below,
            button_left,
            button_right,
            checkbox,
            slider,
            title_label: Label::new(WidgetId::default(), "Tooltip Demo - Hover over widgets to see tooltips"),
            hover_label: Label::new(WidgetId::default(), "Hover state: Nothing"),
            hovered_widget: None,
        }
    }
    
    fn check_hover(&mut self, mouse_pos: Point) -> Option<(WidgetId, Rect)> {
        // Check each widget for hover
        if self.button_auto.bounds().contains(mouse_pos) {
            return Some((self.button_auto.id(), self.button_auto.bounds()));
        }
        if self.button_above.bounds().contains(mouse_pos) {
            return Some((self.button_above.id(), self.button_above.bounds()));
        }
        if self.button_below.bounds().contains(mouse_pos) {
            return Some((self.button_below.id(), self.button_below.bounds()));
        }
        if self.button_left.bounds().contains(mouse_pos) {
            return Some((self.button_left.id(), self.button_left.bounds()));
        }
        if self.button_right.bounds().contains(mouse_pos) {
            return Some((self.button_right.id(), self.button_right.bounds()));
        }
        if self.checkbox.bounds().contains(mouse_pos) {
            return Some((self.checkbox.id(), self.checkbox.bounds()));
        }
        if self.slider.bounds().contains(mouse_pos) {
            return Some((self.slider.id(), self.slider.bounds()));
        }
        None
    }
}

fn main() {
    let event_loop = EventLoop::new();
    
    let mut renderer = Renderer::new(&event_loop, 800, 600, "Tooltip Demo")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    
    let mut demo = TooltipDemo::new();
    let mut mouse_pos = Point::ZERO;
    
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
                if let Some(gui_event) = erigui_rendering::window::convert_window_event(event, renderer.viewport_size()) {
                    // Update mouse position
                    if let Event::MouseMove(move_event) = &gui_event {
                        mouse_pos = move_event.position;
                        
                        // Check hover and update tooltip manager
                        let hover_info = demo.check_hover(mouse_pos);
                        let hovered_widget = hover_info.as_ref().map(|(id, _)| *id);
                        let widget_bounds = hover_info.as_ref().map(|(_, bounds)| *bounds);
                        
                        tooltip_manager().update(mouse_pos, hovered_widget, widget_bounds);
                        
                        // Update hover label
                        if let Some((widget_id, _)) = hover_info {
                            demo.hover_label.set_text(format!("Hovering over widget: {:?}", widget_id));
                        } else {
                            demo.hover_label.set_text("Hover state: Nothing");
                        }
                    }
                    
                    // Handle events
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
            
            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let button_spacing = 10;
                let section_spacing = 40;
                
                // Title
                let title_size = demo.title_label.measure(&LayoutConstraints::default(), &theme);
                demo.title_label.layout(Rect::new(
                    padding,
                    padding,
                    title_size.width,
                    title_size.height
                ), &theme);
                
                // Position buttons in different areas to test tooltip positioning
                let button_size = Size::new(120, 32);
                
                // Center button (auto position)
                demo.button_auto.layout(Rect::new(
                    window_size.width / 2 - button_size.width / 2,
                    window_size.height / 2 - button_size.height / 2,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                // Top button (tooltip below)
                demo.button_below.layout(Rect::new(
                    window_size.width / 2 - button_size.width / 2,
                    padding + title_size.height + section_spacing,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                // Bottom button (tooltip above)
                demo.button_above.layout(Rect::new(
                    window_size.width / 2 - button_size.width / 2,
                    window_size.height - padding - button_size.height,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                // Left button (tooltip right)
                demo.button_right.layout(Rect::new(
                    padding,
                    window_size.height / 2 - button_size.height / 2,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                // Right button (tooltip left)
                demo.button_left.layout(Rect::new(
                    window_size.width - padding - button_size.width,
                    window_size.height / 2 - button_size.height / 2,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                // Other widgets
                let mut y = padding + title_size.height + section_spacing + button_size.height + section_spacing;
                
                let checkbox_size = demo.checkbox.measure(&LayoutConstraints::default(), &theme);
                demo.checkbox.layout(Rect::new(
                    padding,
                    y,
                    checkbox_size.width,
                    checkbox_size.height
                ), &theme);
                
                y += checkbox_size.height + button_spacing;
                
                let slider_size = Size::new(200, 24);
                demo.slider.layout(Rect::new(
                    padding,
                    y,
                    slider_size.width,
                    slider_size.height
                ), &theme);
                
                // Hover label at bottom
                let hover_size = demo.hover_label.measure(&LayoutConstraints::default(), &theme);
                demo.hover_label.layout(Rect::new(
                    padding,
                    window_size.height - padding - hover_size.height - 40,
                    hover_size.width,
                    hover_size.height
                ), &theme);
                
                // Draw
                renderer.begin_frame(theme.colors.background);
                
                // Draw widgets
                demo.title_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.button_auto.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.button_above.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.button_below.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.button_left.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.button_right.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.checkbox.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.slider.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.hover_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                // Draw tooltips on top
                tooltip_manager().draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}
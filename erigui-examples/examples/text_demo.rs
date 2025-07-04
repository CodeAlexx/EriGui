use erigui_core::{Color, Rect, Size, Point, Margins, LayoutConfig, LayoutMode, Widget, DrawContext, Theme};
use erigui_rendering::{Renderer, convert_window_event};
use erigui_widgets::{
    Button, Container, Label, TextInput,
    WidgetManager, TextAlign, WidgetId
};

struct App {
    widget_manager: WidgetManager,
    root_container: WidgetId,
}

impl App {
    fn new() -> Self {
        let mut widget_manager = WidgetManager::new();
        
        // Create root container
        let root_id = widget_manager.add_widget(Box::new(
            Container::new(WidgetId::default()).with_layout(LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 20,
                padding: Margins::all(20),
                ..Default::default()
            }),
        ));
        
        // Title - Large centered text
        let title_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Text Rendering Demo")
                .with_align(TextAlign::Center)
        ));
        
        // Subtitle with different color
        let subtitle_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Testing various text features in EriGui")
                .with_align(TextAlign::Center)
                .with_color(Color::rgb(100, 150, 200))
        ));
        
        // Left aligned text
        let left_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Left aligned text (default)")
                .with_align(TextAlign::Left)
        ));
        
        // Center aligned text
        let center_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Center aligned text")
                .with_align(TextAlign::Center)
                .with_color(Color::rgb(255, 128, 0))
        ));
        
        // Right aligned text
        let right_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Right aligned text")
                .with_align(TextAlign::Right)
                .with_color(Color::rgb(0, 200, 100))
        ));
        
        // Long text to test wrapping
        let long_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), 
                "This is a very long text that might need to be wrapped or clipped depending on the container width. Let's see how it renders!")
        ));
        
        // Special characters
        let special_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Special chars: @#$%^&*()_+-=[]{}|;':\",./<>?")
                .with_color(Color::rgb(200, 100, 200))
        ));
        
        // Numbers and mixed content
        let numbers_text_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Numbers: 0123456789 Mix: abc123XYZ!@#")
        ));
        
        // Text input section
        let input_container_id = widget_manager.add_widget(Box::new(
            Container::new(WidgetId::default()).with_layout(LayoutConfig {
                mode: LayoutMode::Horizontal,
                spacing: 10,
                ..Default::default()
            }),
        ));
        
        let input_label_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Type here:")
        ));
        
        let text_input_id = widget_manager.add_widget(Box::new(
            TextInput::new(WidgetId::default())
                .with_placeholder("Enter some text to see it rendered...")
        ));
        
        // Button with text
        let button_container_id = widget_manager.add_widget(Box::new(
            Container::new(WidgetId::default()).with_layout(LayoutConfig {
                mode: LayoutMode::Horizontal,
                spacing: 10,
                ..Default::default()
            }),
        ));
        
        let button1_id = widget_manager.add_widget(Box::new(
            Button::new(WidgetId::default(), "Short Text")
        ));
        
        let button2_id = widget_manager.add_widget(Box::new(
            Button::new(WidgetId::default(), "Medium Length Button Text")
        ));
        
        let button3_id = widget_manager.add_widget(Box::new(
            Button::new(WidgetId::default(), "Very Long Button Text That Might Overflow")
        ));
        
        // Disabled text
        let disabled_label_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "This text is disabled")
        ));
        widget_manager.get_mut(disabled_label_id).unwrap().set_enabled(false);
        
        // Build the UI hierarchy
        let root = widget_manager.get_typed_mut::<Container>(root_id).unwrap();
        root.add_child(title_id);
        root.add_child(subtitle_id);
        root.add_child(left_text_id);
        root.add_child(center_text_id);
        root.add_child(right_text_id);
        root.add_child(long_text_id);
        root.add_child(special_text_id);
        root.add_child(numbers_text_id);
        root.add_child(input_container_id);
        root.add_child(button_container_id);
        root.add_child(disabled_label_id);
        
        // Add children to input container
        let input_container = widget_manager.get_typed_mut::<Container>(input_container_id).unwrap();
        input_container.add_child(input_label_id);
        input_container.add_flex_child(text_input_id, 1.0);
        
        // Add buttons to button container
        let button_container = widget_manager.get_typed_mut::<Container>(button_container_id).unwrap();
        button_container.add_child(button1_id);
        button_container.add_child(button2_id);
        button_container.add_child(button3_id);
        
        Self {
            widget_manager,
            root_container: root_id,
        }
    }
    
    fn layout_widgets(&mut self, size: Size) {
        if let Some(root) = self.widget_manager.get_mut(self.root_container) {
            let theme = Theme::light();
            root.layout(Rect::from_origin_size(Point::ZERO, size), &theme);
        }
        
        let theme = Theme::light();
        self.layout_children(self.root_container, &theme);
    }
    
    fn layout_children(&mut self, parent_id: WidgetId, theme: &Theme) {
        if let Some(container) = self.widget_manager.get_typed::<Container>(parent_id) {
            let layout_info = container.get_layout_info();
            let children = container.children().to_vec();
            let bounds = container.bounds();
            
            self.perform_container_layout(parent_id, &layout_info, &children, bounds, theme);
        }
        
        let children: Vec<WidgetId> = if let Some(parent) = self.widget_manager.get(parent_id) {
            parent.children().to_vec()
        } else {
            return;
        };
        
        for &child_id in &children {
            self.layout_children(child_id, theme);
        }
    }
    
    fn perform_container_layout(&mut self, _container_id: WidgetId, layout: &LayoutConfig, children: &[WidgetId], bounds: Rect, theme: &Theme) {
        let content_rect = bounds.inset(layout.padding.left);
        let spacing = layout.spacing;
        
        match layout.mode {
            LayoutMode::Vertical => {
                let mut y = content_rect.y();
                let mut remaining_height = content_rect.height();
                let mut flex_items = Vec::new();
                let mut fixed_height = 0;
                
                // First pass: measure fixed items
                for &child_id in children {
                    if let Some(child) = self.widget_manager.get(child_id) {
                        if let Some(container) = self.widget_manager.get_typed::<Container>(child_id) {
                            let flex_children = container.get_layout_info();
                            if flex_children.mode != LayoutMode::None {
                                flex_items.push(child_id);
                                continue;
                            }
                        }
                        
                        let child_height = 50; // Default height for labels/buttons
                        fixed_height += child_height + spacing;
                    }
                }
                
                remaining_height = (remaining_height - fixed_height).max(0);
                let flex_height = if flex_items.is_empty() { 0 } else {
                    remaining_height / flex_items.len() as i32
                };
                
                // Second pass: layout all children
                for &child_id in children {
                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        let height = if flex_items.contains(&child_id) {
                            flex_height
                        } else {
                            50 // Default height
                        };
                        
                        child.layout(Rect::new(content_rect.x(), y, content_rect.width(), height), theme);
                        y += height + spacing;
                    }
                }
            }
            
            LayoutMode::Horizontal => {
                let mut x = content_rect.x();
                let total_flex: f32 = 1.0; // Simplified
                let child_count = children.len() as i32;
                let total_spacing = spacing * (child_count - 1).max(0);
                let available_width = content_rect.width() - total_spacing;
                
                for (i, &child_id) in children.iter().enumerate() {
                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        let width = if i == children.len() - 1 && total_flex > 0.0 {
                            // Last flex item takes remaining space
                            content_rect.right() - x
                        } else {
                            available_width / child_count
                        };
                        
                        child.layout(Rect::new(x, content_rect.y(), width, content_rect.height()), theme);
                        x += width + spacing;
                    }
                }
            }
            
            _ => {}
        }
    }
    
    fn draw_widget(&self, widget_id: WidgetId, renderer: &mut Renderer) {
        let theme = Theme::light();
        
        if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.draw(renderer, &theme);
            
            for &child_id in widget.children() {
                self.draw_widget(child_id, renderer);
            }
        }
    }
    
    fn handle_event(&mut self, event: &erigui_core::Event) {
        let root_container = self.root_container;
        let theme = Theme::light();
        self.route_event(root_container, event, &theme);
    }
    
    fn route_event(&mut self, widget_id: WidgetId, event: &erigui_core::Event, theme: &Theme) -> erigui_core::EventResult {
        let children: Vec<WidgetId> = if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.children().to_vec()
        } else {
            return erigui_core::EventResult::Ignored;
        };
        
        for &child_id in children.iter().rev() {
            if self.route_event(child_id, event, theme).is_consumed() {
                return erigui_core::EventResult::Consumed;
            }
        }
        
        if let Some(widget) = self.widget_manager.get_mut(widget_id) {
            widget.handle_event(event, theme)
        } else {
            erigui_core::EventResult::Ignored
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let event_loop = winit::event_loop::EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 800, 600, "EriGui - Text Rendering Demo")?;
    let mut app = App::new();
    
    app.layout_widgets(Size::new(800, 600));
    
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        
        match event {
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        app.layout_widgets(Size::new(physical_size.width as i32, physical_size.height as i32));
                    }
                    _ => {}
                }
                
                if let Some(gui_event) = convert_window_event(event, renderer.viewport_size()) {
                    app.handle_event(&gui_event);
                }
            }
            Event::MainEventsCleared => {
                renderer.begin_frame(Color::rgb(30, 30, 30));
                
                // Draw some additional text directly to test raw text rendering
                renderer.set_color(Color::rgb(128, 128, 128));
                renderer.draw_text("Raw text at (600, 50)", Point::new(600, 50), 14);
                renderer.draw_text("Size 12", Point::new(600, 80), 12);
                renderer.draw_text("Size 16", Point::new(600, 100), 16);
                renderer.draw_text("Size 20", Point::new(600, 130), 20);
                renderer.draw_text("Size 24", Point::new(600, 165), 24);
                
                // Draw all widgets
                app.draw_widget(app.root_container, &mut renderer);
                
                renderer.end_frame();
            }
            _ => {}
        }
    })
}
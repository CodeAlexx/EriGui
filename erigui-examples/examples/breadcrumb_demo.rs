use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct BreadcrumbDemo {
    file_breadcrumb: Breadcrumb,
    nav_breadcrumb: Breadcrumb,
    info_label: Label,
    add_button: Button,
    clear_button: Button,
    path_label: Label,
}

impl BreadcrumbDemo {
    fn new() -> Self {
        // File system style breadcrumb
        let file_breadcrumb = Breadcrumb::new(WidgetId::default())
            .with_separator(" > ")
            .with_items(vec![
                BreadcrumbItem::new("Documents", "documents"),
                BreadcrumbItem::new("Projects", "projects"),
                BreadcrumbItem::new("EriGui", "erigui"),
                BreadcrumbItem::new("src", "src"),
            ])
            .with_on_navigate(|id, index| {
                println!("Navigate to: {} (index: {})", id, index);
            });
        
        // Navigation style breadcrumb (without home icon)
        let nav_breadcrumb = Breadcrumb::new(WidgetId::default())
            .with_separator(" / ")
            .with_show_home_icon(false)
            .with_home_item(BreadcrumbItem::new("Dashboard", "dashboard"))
            .with_items(vec![
                BreadcrumbItem::new("Settings", "settings"),
                BreadcrumbItem::new("Account", "account"),
                BreadcrumbItem::new("Security", "security"),
            ])
            .with_on_navigate(|id, index| {
                println!("Navigate to: {} (index: {})", id, index);
            });
        
        Self {
            file_breadcrumb,
            nav_breadcrumb,
            info_label: Label::new(WidgetId::default(), "Breadcrumb Demo - Click any item to navigate"),
            add_button: Button::new(WidgetId::default(), "Add Item"),
            clear_button: Button::new(WidgetId::default(), "Clear Path"),
            path_label: Label::new(WidgetId::default(), "Current path: Home > Documents > Projects > EriGui > src"),
        }
    }
    
    fn update_path_label(&mut self) {
        let path = self.file_breadcrumb.get_path();
        let path_str = if path.is_empty() {
            "Home".to_string()
        } else {
            path.join(" > ")
        };
        self.path_label.set_text(format!("Current path: {}", path_str));
    }
}

fn main() {
    let event_loop = EventLoop::new();
    
    let mut renderer = Renderer::new(&event_loop, 800, 600, "Breadcrumb Demo")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    
    let mut demo = BreadcrumbDemo::new();
    let mut item_counter = 1;
    
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
                    let file_result = demo.file_breadcrumb.handle_event(&gui_event, &theme);
                    let nav_result = demo.nav_breadcrumb.handle_event(&gui_event, &theme);
                    let add_result = demo.add_button.handle_event(&gui_event, &theme);
                    let clear_result = demo.clear_button.handle_event(&gui_event, &theme);
                    
                    // Handle button clicks
                    if let Event::MouseButton(mouse_event) = &gui_event {
                        if !mouse_event.pressed && mouse_event.button == MouseButton::Left {
                            if demo.add_button.bounds().contains(mouse_event.position) {
                                demo.file_breadcrumb.push(BreadcrumbItem::new(
                                    format!("Item{}", item_counter),
                                    format!("item{}", item_counter)
                                ));
                                item_counter += 1;
                                demo.update_path_label();
                            } else if demo.clear_button.bounds().contains(mouse_event.position) {
                                demo.file_breadcrumb.clear();
                                demo.update_path_label();
                            }
                        }
                    }
                    
                    if file_result == EventResult::Consumed || 
                       nav_result == EventResult::Consumed ||
                       add_result == EventResult::Consumed ||
                       clear_result == EventResult::Consumed {
                        // Update path label when breadcrumb changes
                        demo.update_path_label();
                        renderer.window().request_redraw();
                    }
                }
            }
            
            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let spacing = 20;
                
                let mut y = padding;
                
                // Info label
                let info_size = demo.info_label.measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(Rect::new(
                    (window_size.width - info_size.width) / 2,
                    y,
                    info_size.width,
                    info_size.height
                ), &theme);
                y += info_size.height + spacing * 2;
                
                // File system breadcrumb
                let file_size = demo.file_breadcrumb.measure(&LayoutConstraints::default(), &theme);
                demo.file_breadcrumb.layout(Rect::new(
                    padding,
                    y,
                    file_size.width.min(window_size.width - padding * 2),
                    file_size.height
                ), &theme);
                y += file_size.height + spacing;
                
                // Navigation breadcrumb
                let nav_size = demo.nav_breadcrumb.measure(&LayoutConstraints::default(), &theme);
                demo.nav_breadcrumb.layout(Rect::new(
                    padding,
                    y,
                    nav_size.width.min(window_size.width - padding * 2),
                    nav_size.height
                ), &theme);
                y += nav_size.height + spacing * 2;
                
                // Buttons
                let button_size = demo.add_button.measure(&LayoutConstraints::default(), &theme);
                demo.add_button.layout(Rect::new(
                    padding,
                    y,
                    button_size.width,
                    button_size.height
                ), &theme);
                
                demo.clear_button.layout(Rect::new(
                    padding + button_size.width + 10,
                    y,
                    button_size.width,
                    button_size.height
                ), &theme);
                y += button_size.height + spacing;
                
                // Path label
                let path_size = demo.path_label.measure(&LayoutConstraints::default(), &theme);
                demo.path_label.layout(Rect::new(
                    padding,
                    y,
                    path_size.width,
                    path_size.height
                ), &theme);
                
                // Draw
                renderer.begin_frame(theme.colors.background);
                
                demo.info_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                // Draw labels
                let mut label_renderer = &mut renderer as &mut dyn DrawContext;
                label_renderer.set_color(theme.colors.text_secondary);
                label_renderer.draw_text(
                    "File System Style (with home icon):",
                    Point::new(padding, demo.file_breadcrumb.bounds().y() - 20),
                    theme.typography.font_size_small
                );
                
                demo.file_breadcrumb.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                label_renderer.set_color(theme.colors.text_secondary);
                label_renderer.draw_text(
                    "Navigation Style (text home):",
                    Point::new(padding, demo.nav_breadcrumb.bounds().y() - 20),
                    theme.typography.font_size_small
                );
                
                demo.nav_breadcrumb.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.add_button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.clear_button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.path_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}
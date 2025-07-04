use erigui_core::{Color, Size, Theme};
use erigui_rendering::Renderer;
use erigui_widgets::{create_file_manager, WidgetManager};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let mut renderer = Renderer::new(1024, 768, "EriGui - File Manager")?;
    let mut widget_manager = WidgetManager::new();
    
    // Create file manager
    let file_manager_id = create_file_manager(&mut widget_manager);
    
    // Get theme
    let theme = Theme::light();
    
    // Layout file manager
    if let Some(fm) = widget_manager.get_mut(file_manager_id) {
        fm.layout(
            erigui_core::Rect::new(0, 0, 1024, 768),
            &theme,
        );
    }
    
    while !renderer.should_close() {
        renderer.begin_frame(Color::rgb(30, 30, 30));
        
        // Draw file manager
        if let Some(fm) = widget_manager.get(file_manager_id) {
            fm.draw(&mut renderer, &theme);
            
            // Draw all child widgets recursively
            fn draw_children(
                widget_manager: &WidgetManager,
                parent_id: erigui_core::WidgetId,
                renderer: &mut Renderer,
                theme: &Theme,
            ) {
                if let Some(parent) = widget_manager.get(parent_id) {
                    for &child_id in parent.children() {
                        if let Some(child) = widget_manager.get(child_id) {
                            child.draw(renderer, theme);
                            draw_children(widget_manager, child_id, renderer, theme);
                        }
                    }
                }
            }
            
            draw_children(&widget_manager, file_manager_id, &mut renderer, &theme);
        }
        
        renderer.end_frame();
        
        // Handle events
        for window_event in renderer.poll_events() {
            if let Some(event) = erigui_rendering::convert_window_event(
                window_event,
                Size::new(1024, 768),
            ) {
                // Route events to widgets
                fn route_event(
                    widget_manager: &mut WidgetManager,
                    widget_id: erigui_core::WidgetId,
                    event: &erigui_core::Event,
                    theme: &Theme,
                ) -> erigui_core::EventResult {
                    // First try children
                    let children: Vec<erigui_core::WidgetId> =
                        if let Some(widget) = widget_manager.get(widget_id) {
                            widget.children().to_vec()
                        } else {
                            return erigui_core::EventResult::Ignored;
                        };
                    
                    for &child_id in children.iter().rev() {
                        if route_event(widget_manager, child_id, event, theme).is_consumed() {
                            return erigui_core::EventResult::Consumed;
                        }
                    }
                    
                    // Then try the widget itself
                    if let Some(widget) = widget_manager.get_mut(widget_id) {
                        widget.handle_event(event, theme)
                    } else {
                        erigui_core::EventResult::Ignored
                    }
                }
                
                route_event(&mut widget_manager, file_manager_id, &event, &theme);
            }
        }
    }
    
    Ok(())
}
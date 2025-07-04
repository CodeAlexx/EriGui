use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct DockPanelDemo {
    dock_panel: DockPanel,
    info_label: Label,
}

impl DockPanelDemo {
    fn new() -> Self {
        let mut dock_panel = DockPanel::new(WidgetId::default())
            .with_on_panel_close(|panel_id| {
                println!("Panel closed: {}", panel_id);
            })
            .with_on_layout_change(|| {
                println!("Dock layout changed");
            });
        
        // Add some demo panels
        
        // Properties panel
        let properties_content = Box::new(Container::new(WidgetId::default()));
        let properties_panel = DockablePanel::new("properties", "Properties", properties_content)
            .with_icon("settings")
            .with_can_close(true);
        dock_panel.add_panel(properties_panel, DockPosition::Right);
        
        // Explorer panel
        let explorer_content = Box::new(Container::new(WidgetId::default()));
        let explorer_panel = DockablePanel::new("explorer", "Explorer", explorer_content)
            .with_icon("folder")
            .with_can_close(true);
        dock_panel.add_panel(explorer_panel, DockPosition::Left);
        
        // Output panel
        let output_content = Box::new(Container::new(WidgetId::default()));
        let output_panel = DockablePanel::new("output", "Output", output_content)
            .with_icon("terminal")
            .with_can_close(true);
        dock_panel.add_panel(output_panel, DockPosition::Bottom);
        
        // Main editor panels
        let editor1_content = Box::new(Label::new(WidgetId::default(), "Editor 1 Content\n\nThis is the main editor area.\nYou can drag panels around to rearrange the layout."));
        let editor1_panel = DockablePanel::new("editor1", "main.rs", editor1_content)
            .with_icon("file")
            .with_can_close(true);
        dock_panel.add_panel(editor1_panel, DockPosition::Center);
        
        let editor2_content = Box::new(Label::new(WidgetId::default(), "Editor 2 Content\n\nThis is another editor tab.\nTabs can be reordered by dragging."));
        let editor2_panel = DockablePanel::new("editor2", "lib.rs", editor2_content)
            .with_icon("file")
            .with_can_close(true);
        dock_panel.add_panel(editor2_panel, DockPosition::Center);
        
        // Floating panel
        let floating_content = Box::new(Label::new(WidgetId::default(), "Floating Window\n\nThis window can be moved around\nby dragging the title bar."));
        let floating_panel = DockablePanel::new("floating", "Floating Panel", floating_content)
            .with_icon("window")
            .with_can_close(true);
        dock_panel.add_panel(floating_panel, DockPosition::Floating);
        
        Self {
            dock_panel,
            info_label: Label::new(WidgetId::default(), "DockPanel Demo - Drag panels to rearrange, drag splitters to resize"),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    
    let mut renderer = Renderer::new(&event_loop, 1024, 768, "DockPanel Demo")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    
    let mut demo = DockPanelDemo::new();
    
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
                    let dock_result = demo.dock_panel.handle_event(&gui_event, &theme);
                    
                    if dock_result == EventResult::Consumed {
                        renderer.window().request_redraw();
                    }
                }
            }
            
            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                
                // Info label
                let info_size = demo.info_label.measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(Rect::new(
                    (window_size.width - info_size.width) / 2,
                    padding / 2,
                    info_size.width,
                    info_size.height
                ), &theme);
                
                // Dock panel fills remaining space
                demo.dock_panel.layout(Rect::new(
                    0,
                    padding,
                    window_size.width,
                    window_size.height - padding
                ), &theme);
                
                // Draw
                renderer.begin_frame(theme.colors.background);
                
                demo.info_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.dock_panel.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}
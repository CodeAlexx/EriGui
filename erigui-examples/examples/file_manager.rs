use erigui_core::{Color, Theme};
use erigui_rendering::Renderer;
use erigui_widgets::{create_file_manager, WidgetManager};
use winit::event_loop::EventLoop;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::ControlFlow,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 1024, 768, "EriGui - File Manager")?;
    let mut widget_manager = WidgetManager::new();

    // Create file manager
    let file_manager_id = create_file_manager(&mut widget_manager);

    // Get theme
    let theme = Theme::light();

    // Layout file manager
    if let Some(fm) = widget_manager.get_mut(file_manager_id) {
        fm.layout(erigui_core::Rect::new(0, 0, 1024, 768), &theme);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width, size.height);
                        if let Some(fm) = widget_manager.get_mut(file_manager_id) {
                            fm.layout(
                                erigui_core::Rect::new(0, 0, size.width as i32, size.height as i32),
                                &theme,
                            );
                        }
                    }
                    _ => {}
                }

                if let Some(evt) =
                    erigui_rendering::convert_window_event(event, renderer.viewport_size())
                {
                    route_event(&mut widget_manager, file_manager_id, &evt, &theme);
                }
            }
            WinitEvent::MainEventsCleared => {
                renderer.begin_frame(Color::rgb(30, 30, 30));

                if let Some(fm) = widget_manager.get(file_manager_id) {
                    fm.draw(&mut renderer, &theme);

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
            }
            _ => {}
        }
    });
}

fn route_event(
    widget_manager: &mut WidgetManager,
    widget_id: erigui_core::WidgetId,
    event: &erigui_core::Event,
    theme: &Theme,
) -> erigui_core::EventResult {
    // First try children
    let children: Vec<erigui_core::WidgetId> = if let Some(widget) = widget_manager.get(widget_id) {
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

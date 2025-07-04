use erigui_core::{Color, DrawContext, Point, Rect};
use erigui_rendering::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 800, 600, "EriGui - Hello World")?;
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                renderer.begin_frame(Color::rgb(30, 30, 30));
                
                // Draw a white rectangle
                renderer.set_color(Color::WHITE);
                renderer.fill_rect(Rect::new(50, 50, 200, 100));
                
                // Draw a red outline
                renderer.set_color(Color::RED);
                renderer.draw_rect(Rect::new(50, 50, 200, 100));
                
                // Draw some text
                renderer.set_color(Color::rgb(100, 200, 255));
                renderer.draw_text("Hello, EriGui!", Point::new(100, 100), 24);
                
                // Draw a circle
                renderer.set_color(Color::GREEN);
                renderer.fill_circle(Point::new(400, 300), 50, 32);
                
                renderer.end_frame();
            }
            _ => {}
        }
    })
}
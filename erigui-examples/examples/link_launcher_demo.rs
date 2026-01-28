use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::{
    dpi::PhysicalPosition,
    event::{Event as WinitEvent, MouseButton as WinitMouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

static LAUNCH_CMD: AtomicBool = AtomicBool::new(false);

fn trigger_launch() {
    LAUNCH_CMD.store(true, Ordering::SeqCst);
}

fn open_crates_page() -> std::io::Result<()> {
    let target = "https://crates.io";
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(target)
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(target).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(target).spawn().map(|_| ())
    }
}

struct LauncherDemo {
    button: Button,
    status: String,
    button_bounds: Rect,
}

impl LauncherDemo {
    fn new() -> Self {
        let button = Button::new(WidgetId::default(), "Launch crates.io")
            .with_style(ButtonStyle::Primary)
            .with_on_click(trigger_launch);
        Self {
            button,
            status: "Click the button to open crates.io in your browser".to_string(),
            button_bounds: Rect::new(0, 0, 0, 0),
        }
    }

    fn layout(&mut self, viewport: Size) {
        let button_width = 260;
        let button_height = 50;
        let x = (viewport.width - button_width) / 2;
        let y = (viewport.height - button_height) / 2;
        self.button_bounds = Rect::new(x, y, button_width, button_height);
        self.button.set_bounds(self.button_bounds);
    }

    fn draw(&mut self, context: &mut dyn DrawContext, theme: &Theme, viewport: Size) {
        self.layout(viewport);

        let bounds = Rect::new(0, 0, viewport.width, viewport.height);
        context.set_color(theme.colors.background);
        context.fill_rect(bounds);

        self.button.draw(context, theme);

        context.set_color(theme.colors.text);
        let text_x = ((viewport.width / 2) - 260).max(20);
        context.draw_text(
            &self.status,
            Point::new(text_x, self.button_bounds.bottom() + 30),
            theme.typography.font_size_base,
        );
    }

    fn handle_mouse_move(&mut self, pos: Point, theme: &Theme) {
        let event = Event::MouseMove(MouseMoveEvent {
            position: pos,
            delta: Point::ZERO,
            modifiers: Modifiers::empty(),
        });
        let _ = self.button.handle_event(&event, theme);
    }

    fn handle_mouse_button(
        &mut self,
        pos: Point,
        button: MouseButton,
        pressed: bool,
        theme: &Theme,
    ) {
        let event = Event::MouseButton(MouseButtonEvent {
            button,
            position: pos,
            pressed,
            modifiers: Modifiers::empty(),
        });
        let _ = self.button.handle_event(&event, theme);
    }

    fn process_launch(&mut self) {
        match open_crates_page() {
            Ok(_) => self.status = "Opening crates.io ...".to_string(),
            Err(err) => self.status = format!("Failed to open crates.io: {}", err),
        }
    }
}

fn map_mouse_button(btn: WinitMouseButton) -> MouseButton {
    match btn {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Other(1) => MouseButton::Extra1,
        WinitMouseButton::Other(2) => MouseButton::Extra2,
        _ => MouseButton::Left,
    }
}

fn main() -> erigui_core::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 640, 360, "Link Launcher Demo")?;
    let mut last_cursor = PhysicalPosition::new(0.0, 0.0);
    let theme = Theme::dark();
    let mut demo = LauncherDemo::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if LAUNCH_CMD.swap(false, Ordering::SeqCst) {
            demo.process_launch();
            renderer.window().request_redraw();
        }

        match event {
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(new_size) => {
                    renderer.resize(new_size.width, new_size.height);
                    renderer.window().request_redraw();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    last_cursor = position;
                    let pos = Point::new(position.x as i32, position.y as i32);
                    demo.handle_mouse_move(pos, &theme);
                    renderer.window().request_redraw();
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let pos = Point::new(last_cursor.x as i32, last_cursor.y as i32);
                    let mapped = map_mouse_button(button);
                    let pressed = state == winit::event::ElementState::Pressed;
                    demo.handle_mouse_button(pos, mapped, pressed, &theme);
                    renderer.window().request_redraw();
                }
                _ => {}
            },
            WinitEvent::RedrawRequested(_) => {
                let size = renderer.viewport_size();
                renderer.begin_frame(theme.colors.background);
                demo.draw(&mut renderer, &theme, size);
                renderer.end_frame();
            }
            _ => {}
        }
    });
}

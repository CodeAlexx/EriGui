use erigui_core::{Event, MouseButton, MouseButtonEvent, MouseMoveEvent, Point, Modifiers, Key, KeyPressEvent, KeyReleaseEvent, ResizeEvent, Size};
use winit::event::{WindowEvent, ElementState, MouseButton as WinitMouseButton};

pub fn convert_window_event(event: WindowEvent, window_size: Size) -> Option<Event> {
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            Some(Event::MouseMove(MouseMoveEvent {
                position: Point::new(position.x as i32, position.y as i32),
                delta: Point::ZERO, // TODO: Track previous position
                modifiers: Modifiers::empty(), // TODO: Track modifiers
            }))
        }
        
        WindowEvent::MouseInput { state, button, .. } => {
            let button = match button {
                WinitMouseButton::Left => MouseButton::Left,
                WinitMouseButton::Right => MouseButton::Right,
                WinitMouseButton::Middle => MouseButton::Middle,
                WinitMouseButton::Other(4) => MouseButton::Extra1,
                WinitMouseButton::Other(5) => MouseButton::Extra2,
                _ => return None,
            };
            
            Some(Event::MouseButton(MouseButtonEvent {
                button,
                position: Point::ZERO, // TODO: Track cursor position
                pressed: matches!(state, ElementState::Pressed),
                modifiers: Modifiers::empty(), // TODO: Track modifiers
            }))
        }
        
        WindowEvent::KeyboardInput { device_id: _, input, is_synthetic: _ } => {
            let key = Key::Unknown(0); // TODO: Convert key properly
            
            match input.state {
                ElementState::Pressed => Some(Event::KeyPress(KeyPressEvent {
                    key,
                    modifiers: Modifiers::empty(),
                    repeat: false, // winit 0.28 doesn't provide repeat info directly
                })),
                ElementState::Released => Some(Event::KeyRelease(KeyReleaseEvent {
                    key,
                    modifiers: Modifiers::empty(),
                })),
            }
        }
        
        
        WindowEvent::Resized(physical_size) => {
            let new_size = Size::new(physical_size.width as i32, physical_size.height as i32);
            Some(Event::Resize(ResizeEvent {
                old_size: window_size,
                new_size,
            }))
        }
        
        _ => None,
    }
}


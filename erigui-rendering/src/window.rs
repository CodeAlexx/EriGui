use erigui_core::{
    DragData, DragDropEvent, Event, Key, KeyPressEvent, KeyReleaseEvent, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, ResizeEvent, Size, TextInputEvent,
};
use winit::event::{
    ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};

/// Simple helper that tracks cursor/modifier state and converts winit events to EriGui events.
#[derive(Debug, Clone)]
pub struct EventTranslator {
    last_cursor: Point,
    last_mods: Modifiers,
    window_size: Size,
    active_drag: Option<DragData>,
}

impl EventTranslator {
    pub fn new(window_size: Size) -> Self {
        Self {
            last_cursor: Point::ZERO,
            last_mods: Modifiers::empty(),
            window_size,
            active_drag: None,
        }
    }

    pub fn set_window_size(&mut self, size: Size) {
        self.window_size = size;
    }

    pub fn translate(&mut self, event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Point::new(position.x as i32, position.y as i32);
                let delta = Point::new(pos.x - self.last_cursor.x, pos.y - self.last_cursor.y);
                self.last_cursor = pos;
                if let Some(data) = self.active_drag.clone() {
                    Some(Event::DragDrop(DragDropEvent::Over {
                        position: pos,
                        data,
                    }))
                } else {
                    Some(Event::MouseMove(MouseMoveEvent {
                        position: pos,
                        delta,
                        modifiers: self.last_mods,
                    }))
                }
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
                    position: self.last_cursor,
                    pressed: matches!(state, ElementState::Pressed),
                    modifiers: self.last_mods,
                }))
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => ((x * 40.0) as i32, (y * 40.0) as i32),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as i32, pos.y as i32),
                };
                Some(Event::MouseWheel(MouseWheelEvent {
                    delta: Point::new(dx, dy),
                    position: self.last_cursor,
                    modifiers: self.last_mods,
                }))
            }
            WindowEvent::KeyboardInput { input, .. } => {
                let key = input
                    .virtual_keycode
                    .map(map_key)
                    .unwrap_or(Key::Unknown(0));
                match input.state {
                    ElementState::Pressed => Some(Event::KeyPress(KeyPressEvent {
                        key,
                        modifiers: self.last_mods,
                        repeat: false,
                    })),
                    ElementState::Released => Some(Event::KeyRelease(KeyReleaseEvent {
                        key,
                        modifiers: self.last_mods,
                    })),
                }
            }
            WindowEvent::ReceivedCharacter(ch) => Some(Event::TextInput(TextInputEvent {
                text: ch.to_string(),
            })),
            WindowEvent::Resized(physical_size) => {
                let new_size = Size::new(physical_size.width as i32, physical_size.height as i32);
                let old = self.window_size;
                self.window_size = new_size;
                Some(Event::Resize(ResizeEvent {
                    old_size: old,
                    new_size,
                }))
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.last_mods = map_mods(*mods);
                None
            }
            WindowEvent::HoveredFile(path) => {
                let data = DragData::Files(vec![path.to_string_lossy().into_owned()]);
                self.active_drag = Some(data.clone());
                Some(Event::DragDrop(DragDropEvent::Enter {
                    position: self.last_cursor,
                    data,
                }))
            }
            WindowEvent::HoveredFileCancelled => {
                if self.active_drag.take().is_some() {
                    Some(Event::DragDrop(DragDropEvent::Leave))
                } else {
                    None
                }
            }
            WindowEvent::DroppedFile(path) => {
                let data = DragData::Files(vec![path.to_string_lossy().into_owned()]);
                self.active_drag = None;
                Some(Event::DragDrop(DragDropEvent::Drop {
                    position: self.last_cursor,
                    data,
                }))
            }
            _ => None,
        }
    }
}

fn map_mods(mods: winit::event::ModifiersState) -> Modifiers {
    let mut out = Modifiers::empty();
    if mods.shift() {
        out |= Modifiers::SHIFT;
    }
    if mods.ctrl() {
        out |= Modifiers::CTRL;
    }
    if mods.alt() {
        out |= Modifiers::ALT;
    }
    if mods.logo() {
        out |= Modifiers::SUPER;
    }
    out
}

fn map_key(key: VirtualKeyCode) -> Key {
    use VirtualKeyCode as V;
    match key {
        V::A => Key::A,
        V::B => Key::B,
        V::C => Key::C,
        V::D => Key::D,
        V::E => Key::E,
        V::F => Key::F,
        V::G => Key::G,
        V::H => Key::H,
        V::I => Key::I,
        V::J => Key::J,
        V::K => Key::K,
        V::L => Key::L,
        V::M => Key::M,
        V::N => Key::N,
        V::O => Key::O,
        V::P => Key::P,
        V::Q => Key::Q,
        V::R => Key::R,
        V::S => Key::S,
        V::T => Key::T,
        V::U => Key::U,
        V::V => Key::V,
        V::W => Key::W,
        V::X => Key::X,
        V::Y => Key::Y,
        V::Z => Key::Z,
        V::Key0 => Key::Num0,
        V::Key1 => Key::Num1,
        V::Key2 => Key::Num2,
        V::Key3 => Key::Num3,
        V::Key4 => Key::Num4,
        V::Key5 => Key::Num5,
        V::Key6 => Key::Num6,
        V::Key7 => Key::Num7,
        V::Key8 => Key::Num8,
        V::Key9 => Key::Num9,
        V::Space => Key::Space,
        V::Return => Key::Enter,
        V::Tab => Key::Tab,
        V::Escape => Key::Escape,
        V::Back => Key::Backspace,
        V::Delete => Key::Delete,
        V::Home => Key::Home,
        V::End => Key::End,
        V::PageUp => Key::PageUp,
        V::PageDown => Key::PageDown,
        V::Up => Key::Up,
        V::Down => Key::Down,
        V::Left => Key::Left,
        V::Right => Key::Right,
        V::LShift => Key::LeftShift,
        V::RShift => Key::RightShift,
        V::LControl => Key::LeftCtrl,
        V::RControl => Key::RightCtrl,
        V::LAlt => Key::LeftAlt,
        V::RAlt => Key::RightAlt,
        V::LWin | V::RWin => Key::LeftSuper,
        _ => Key::Unknown(key as u32),
    }
}

/// Legacy helper kept for compatibility; for richer handling prefer `EventTranslator`.
pub fn convert_window_event(event: WindowEvent, window_size: Size) -> Option<Event> {
    let mut tx = EventTranslator::new(window_size);
    tx.translate(&event)
}

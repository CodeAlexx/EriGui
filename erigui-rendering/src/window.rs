use erigui_core::{
    DragData, DragDropEvent, Event, Key, KeyPressEvent, KeyReleaseEvent, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, ResizeEvent, Size, TextInputEvent,
};
use winit::event::{
    ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent,
};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};

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
                    // winit 0.29 introduced Back/Forward variants explicitly.
                    WinitMouseButton::Back => MouseButton::Extra1,
                    WinitMouseButton::Forward => MouseButton::Extra2,
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
            WindowEvent::KeyboardInput { event, .. } => {
                // winit 0.29: KeyboardInput now carries a KeyEvent. Use physical_key for
                // mapping (layout-independent) and event.text for character input
                // (replaces the removed WindowEvent::ReceivedCharacter).
                let key = match event.physical_key {
                    PhysicalKey::Code(code) => map_key(code),
                    PhysicalKey::Unidentified(_) => Key::Unknown(0),
                };
                match event.state {
                    ElementState::Pressed => {
                        // Space is an activation key for buttons, checkboxes,
                        // accordions, dialog buttons, etc. — they need it as
                        // KeyPress(Key::Space). Pre-fix the translator
                        // emitted TextInput(" ") because event.text is the
                        // printable " ", and KeyPress was suppressed.
                        // Widgets that want a literal ' ' inserted (TextInput,
                        // TextArea) handle Key::Space explicitly.
                        let space_pressed = matches!(key, Key::Space);
                        if !space_pressed {
                            if let Some(text) = event.text.as_ref() {
                                // Some "text" payloads are control characters (e.g. \x08 backspace,
                                // \r enter, \x1b esc). Filter those out so widgets only see real
                                // typed characters.
                                let filtered: String = text
                                    .chars()
                                    .filter(|c| !c.is_control())
                                    .collect();
                                if !filtered.is_empty() {
                                    return Some(Event::TextInput(TextInputEvent { text: filtered }));
                                }
                            }
                        }
                        Some(Event::KeyPress(KeyPressEvent {
                            key,
                            modifiers: self.last_mods,
                            repeat: event.repeat,
                        }))
                    }
                    ElementState::Released => Some(Event::KeyRelease(KeyReleaseEvent {
                        key,
                        modifiers: self.last_mods,
                    })),
                }
            }
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
                // winit 0.29 wraps state in a Modifiers struct. Pull out the bitflag state.
                self.last_mods = map_mods(mods.state());
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

fn map_mods(mods: ModifiersState) -> Modifiers {
    let mut out = Modifiers::empty();
    if mods.shift_key() {
        out |= Modifiers::SHIFT;
    }
    if mods.control_key() {
        out |= Modifiers::CTRL;
    }
    if mods.alt_key() {
        out |= Modifiers::ALT;
    }
    if mods.super_key() {
        out |= Modifiers::SUPER;
    }
    out
}

fn map_key(key: KeyCode) -> Key {
    use KeyCode as V;
    match key {
        V::KeyA => Key::A,
        V::KeyB => Key::B,
        V::KeyC => Key::C,
        V::KeyD => Key::D,
        V::KeyE => Key::E,
        V::KeyF => Key::F,
        V::KeyG => Key::G,
        V::KeyH => Key::H,
        V::KeyI => Key::I,
        V::KeyJ => Key::J,
        V::KeyK => Key::K,
        V::KeyL => Key::L,
        V::KeyM => Key::M,
        V::KeyN => Key::N,
        V::KeyO => Key::O,
        V::KeyP => Key::P,
        V::KeyQ => Key::Q,
        V::KeyR => Key::R,
        V::KeyS => Key::S,
        V::KeyT => Key::T,
        V::KeyU => Key::U,
        V::KeyV => Key::V,
        V::KeyW => Key::W,
        V::KeyX => Key::X,
        V::KeyY => Key::Y,
        V::KeyZ => Key::Z,
        V::Digit0 => Key::Num0,
        V::Digit1 => Key::Num1,
        V::Digit2 => Key::Num2,
        V::Digit3 => Key::Num3,
        V::Digit4 => Key::Num4,
        V::Digit5 => Key::Num5,
        V::Digit6 => Key::Num6,
        V::Digit7 => Key::Num7,
        V::Digit8 => Key::Num8,
        V::Digit9 => Key::Num9,
        V::Space => Key::Space,
        V::Enter => Key::Enter,
        V::Tab => Key::Tab,
        V::Escape => Key::Escape,
        V::Backspace => Key::Backspace,
        V::Delete => Key::Delete,
        V::Home => Key::Home,
        V::End => Key::End,
        V::PageUp => Key::PageUp,
        V::PageDown => Key::PageDown,
        V::ArrowUp => Key::Up,
        V::ArrowDown => Key::Down,
        V::ArrowLeft => Key::Left,
        V::ArrowRight => Key::Right,
        V::ShiftLeft => Key::LeftShift,
        V::ShiftRight => Key::RightShift,
        V::ControlLeft => Key::LeftCtrl,
        V::ControlRight => Key::RightCtrl,
        V::AltLeft => Key::LeftAlt,
        V::AltRight => Key::RightAlt,
        V::SuperLeft | V::SuperRight => Key::LeftSuper,
        // We can't expose the discriminant in a stable u32; use the Debug repr length as
        // a tiny fingerprint so unmapped keys still get unique-ish IDs for debugging.
        _ => Key::Unknown(0),
    }
}

/// Broken-by-design helper. Constructs a fresh `EventTranslator` on
/// every call, which means `last_cursor` is always `Point::ZERO` and
/// every `WindowEvent::MouseInput` translates to a click at `(0, 0)`
/// because winit's `MouseInput` doesn't carry the cursor position;
/// the translator only knows it via the most recent `CursorMoved`,
/// which a fresh translator hasn't seen.
///
/// Use a persistent `EventTranslator` instead — keep one for the
/// lifetime of the window, mirror `Resize` events into
/// `set_window_size`, and call `translate(&event)` per event. See
/// `examples/kitchen_sink.rs` for the canonical pattern (commit
/// a59a7f3).
#[deprecated(
    since = "0.0.4-alpha",
    note = "Constructs a fresh EventTranslator per call so MouseInput clicks always report (0, 0). \
            Hold a persistent EventTranslator instead and call translate(&event)."
)]
pub fn convert_window_event(event: WindowEvent, window_size: Size) -> Option<Event> {
    let mut tx = EventTranslator::new(window_size);
    tx.translate(&event)
}

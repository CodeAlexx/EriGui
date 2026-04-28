use crate::{Point, Size};
use bitflags::bitflags;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    MouseMove(MouseMoveEvent),
    MouseButton(MouseButtonEvent),
    MouseWheel(MouseWheelEvent),
    KeyPress(KeyPressEvent),
    KeyRelease(KeyReleaseEvent),
    TextInput(TextInputEvent),
    Focus(FocusEvent),
    Resize(ResizeEvent),
    DragDrop(DragDropEvent),
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMoveEvent {
    pub position: Point,
    pub delta: Point,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub position: Point,
    pub pressed: bool,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseWheelEvent {
    pub delta: Point,
    pub position: Point,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyPressEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyReleaseEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputEvent {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusEvent {
    pub gained: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeEvent {
    pub old_size: Size,
    pub new_size: Size,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragDropEvent {
    Enter { position: Point, data: DragData },
    Over { position: Point, data: DragData },
    Leave,
    Drop { position: Point, data: DragData },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragData {
    Text(String),
    Files(Vec<String>),
    Custom(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Extra1,
    Extra2,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const CTRL  = 0b0010;
        const ALT   = 0b0100;
        const SUPER = 0b1000;
    }
}

impl Default for Modifiers {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,
    Tab,
    Enter,
    Escape,
    Space,

    // Modifiers
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftSuper,
    RightSuper,

    // Punctuation
    Comma,
    Period,
    Slash,
    Backslash,
    Semicolon,
    Quote,
    LeftBracket,
    RightBracket,
    Minus,
    Equals,
    Grave,

    // Numpad
    NumpadNum0,
    NumpadNum1,
    NumpadNum2,
    NumpadNum3,
    NumpadNum4,
    NumpadNum5,
    NumpadNum6,
    NumpadNum7,
    NumpadNum8,
    NumpadNum9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,

    // Media
    PlayPause,
    Stop,
    NextTrack,
    PreviousTrack,
    VolumeUp,
    VolumeDown,
    Mute,

    // Other
    PrintScreen,
    Pause,
    CapsLock,
    NumLock,
    ScrollLock,
    Menu,

    // Character input
    Character(char),

    // Unknown
    Unknown(u32),
}

impl Key {
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Key::LeftShift
                | Key::RightShift
                | Key::LeftCtrl
                | Key::RightCtrl
                | Key::LeftAlt
                | Key::RightAlt
                | Key::LeftSuper
                | Key::RightSuper
        )
    }

    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            Key::Up
                | Key::Down
                | Key::Left
                | Key::Right
                | Key::Home
                | Key::End
                | Key::PageUp
                | Key::PageDown
        )
    }

    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            Key::Backspace
                | Key::Delete
                | Key::Insert
                | Key::Tab
                | Key::Enter
                | Key::Escape
                | Key::Space
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Modifiers (bitflags) -----

    #[test]
    fn modifiers_default_is_empty() {
        assert_eq!(Modifiers::default(), Modifiers::empty());
        assert!(Modifiers::default().is_empty());
    }

    #[test]
    fn modifiers_or_combines_bits() {
        let combined = Modifiers::SHIFT | Modifiers::CTRL;
        assert!(combined.contains(Modifiers::SHIFT));
        assert!(combined.contains(Modifiers::CTRL));
        assert!(!combined.contains(Modifiers::ALT));
        assert!(!combined.contains(Modifiers::SUPER));
    }

    #[test]
    fn modifiers_and_intersects() {
        let a = Modifiers::SHIFT | Modifiers::CTRL;
        let b = Modifiers::CTRL | Modifiers::ALT;
        assert_eq!(a & b, Modifiers::CTRL);
    }

    #[test]
    fn modifiers_remove_clears_bit() {
        let mut m = Modifiers::SHIFT | Modifiers::CTRL;
        m.remove(Modifiers::SHIFT);
        assert!(!m.contains(Modifiers::SHIFT));
        assert!(m.contains(Modifiers::CTRL));
    }

    #[test]
    fn modifiers_all_four_bits_distinct() {
        let all = Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT | Modifiers::SUPER;
        assert_eq!(all.bits(), 0b1111);
    }

    // ----- MouseButtonEvent / MouseMoveEvent fields -----

    #[test]
    fn mouse_button_event_pressed_field() {
        let evt = MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 20),
            pressed: true,
            modifiers: Modifiers::empty(),
        };
        assert!(evt.pressed);
        assert_eq!(evt.button, MouseButton::Left);
        assert_eq!(evt.position, Point::new(10, 20));
    }

    #[test]
    fn mouse_button_event_release_has_pressed_false() {
        let evt = MouseButtonEvent {
            button: MouseButton::Right,
            position: Point::ZERO,
            pressed: false,
            modifiers: Modifiers::CTRL,
        };
        assert!(!evt.pressed);
        assert_eq!(evt.button, MouseButton::Right);
        assert!(evt.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn mouse_move_event_carries_delta() {
        let evt = MouseMoveEvent {
            position: Point::new(50, 60),
            delta: Point::new(5, -3),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(evt.delta, Point::new(5, -3));
        assert_eq!(evt.position, Point::new(50, 60));
    }

    #[test]
    fn mouse_button_variants_distinct() {
        // Hash + Eq behavior for use as map keys.
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MouseButton::Left);
        set.insert(MouseButton::Right);
        set.insert(MouseButton::Middle);
        set.insert(MouseButton::Extra1);
        set.insert(MouseButton::Extra2);
        assert_eq!(set.len(), 5);
    }

    // ----- KeyPressEvent / KeyReleaseEvent -----

    #[test]
    fn key_press_event_repeat_flag() {
        let evt = KeyPressEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            repeat: true,
        };
        assert!(evt.repeat);
        assert_eq!(evt.key, Key::A);
    }

    #[test]
    fn key_press_event_modifiers_stored() {
        let evt = KeyPressEvent {
            key: Key::S,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
            repeat: false,
        };
        assert!(evt.modifiers.contains(Modifiers::CTRL));
        assert!(evt.modifiers.contains(Modifiers::SHIFT));
        assert!(!evt.repeat);
    }

    #[test]
    fn key_release_has_no_repeat_field() {
        // Ensure construction works as documented; KeyReleaseEvent intentionally omits repeat.
        let _ = KeyReleaseEvent {
            key: Key::Escape,
            modifiers: Modifiers::empty(),
        };
    }

    // ----- Key classification helpers -----

    #[test]
    fn key_is_modifier_true_for_modifier_keys() {
        for k in [
            Key::LeftShift,
            Key::RightShift,
            Key::LeftCtrl,
            Key::RightCtrl,
            Key::LeftAlt,
            Key::RightAlt,
            Key::LeftSuper,
            Key::RightSuper,
        ] {
            assert!(k.is_modifier(), "{:?} should be a modifier", k);
        }
    }

    #[test]
    fn key_is_modifier_false_for_letters() {
        assert!(!Key::A.is_modifier());
        assert!(!Key::Z.is_modifier());
        assert!(!Key::F1.is_modifier());
        assert!(!Key::Space.is_modifier());
    }

    #[test]
    fn key_is_navigation_true_for_arrows_and_paging() {
        for k in [
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::Home,
            Key::End,
            Key::PageUp,
            Key::PageDown,
        ] {
            assert!(k.is_navigation(), "{:?} should be navigation", k);
        }
    }

    #[test]
    fn key_is_navigation_false_for_others() {
        assert!(!Key::A.is_navigation());
        assert!(!Key::Enter.is_navigation());
        assert!(!Key::LeftShift.is_navigation());
    }

    #[test]
    fn key_is_editing_true_for_editing_keys() {
        for k in [
            Key::Backspace,
            Key::Delete,
            Key::Insert,
            Key::Tab,
            Key::Enter,
            Key::Escape,
            Key::Space,
        ] {
            assert!(k.is_editing(), "{:?} should be editing", k);
        }
    }

    #[test]
    fn key_is_editing_false_for_letters_and_navigation() {
        assert!(!Key::A.is_editing());
        assert!(!Key::Up.is_editing());
        assert!(!Key::F1.is_editing());
    }

    #[test]
    fn key_classification_categories_are_disjoint() {
        // A key should be at most one of: modifier, navigation, editing.
        let samples = [
            Key::LeftShift,
            Key::Up,
            Key::Backspace,
            Key::A,
            Key::F5,
            Key::Character('q'),
        ];
        for k in samples {
            let count =
                k.is_modifier() as u8 + k.is_navigation() as u8 + k.is_editing() as u8;
            assert!(count <= 1, "{:?} matched multiple categories", k);
        }
    }

    #[test]
    fn key_character_variant_carries_char() {
        let k = Key::Character('Ω');
        match k {
            Key::Character(c) => assert_eq!(c, 'Ω'),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn key_unknown_carries_scancode() {
        let k = Key::Unknown(0xDEAD);
        match k {
            Key::Unknown(code) => assert_eq!(code, 0xDEAD),
            _ => panic!("wrong variant"),
        }
    }

    // ----- DragDropEvent / DragData -----

    #[test]
    fn drag_data_text_round_trip() {
        let d = DragData::Text("hello".to_string());
        if let DragData::Text(s) = d {
            assert_eq!(s, "hello");
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn drag_data_files_round_trip() {
        let d = DragData::Files(vec!["/tmp/a".to_string(), "/tmp/b".to_string()]);
        if let DragData::Files(files) = d {
            assert_eq!(files.len(), 2);
            assert_eq!(files[0], "/tmp/a");
        } else {
            panic!("expected Files");
        }
    }

    #[test]
    fn drag_drop_event_leave_has_no_payload() {
        let evt = DragDropEvent::Leave;
        assert_eq!(evt, DragDropEvent::Leave);
    }

    #[test]
    fn drag_drop_event_enter_carries_position_and_data() {
        let evt = DragDropEvent::Enter {
            position: Point::new(7, 8),
            data: DragData::Text("x".into()),
        };
        match evt {
            DragDropEvent::Enter { position, data } => {
                assert_eq!(position, Point::new(7, 8));
                assert_eq!(data, DragData::Text("x".into()));
            }
            _ => panic!("expected Enter"),
        }
    }

    // ----- ResizeEvent / FocusEvent -----

    #[test]
    fn resize_event_old_and_new_size() {
        let evt = ResizeEvent {
            old_size: Size::new(800, 600),
            new_size: Size::new(1024, 768),
        };
        assert_eq!(evt.old_size, Size::new(800, 600));
        assert_eq!(evt.new_size, Size::new(1024, 768));
    }

    #[test]
    fn focus_event_gained_flag() {
        let gained = FocusEvent { gained: true };
        let lost = FocusEvent { gained: false };
        assert!(gained.gained);
        assert!(!lost.gained);
    }

    // ----- Event enum -----

    #[test]
    fn event_update_equals_self() {
        assert_eq!(Event::Update, Event::Update);
    }

    #[test]
    fn event_text_input_carries_string() {
        let evt = Event::TextInput(TextInputEvent {
            text: "hello".to_string(),
        });
        if let Event::TextInput(ti) = evt {
            assert_eq!(ti.text, "hello");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn event_clone_preserves_data() {
        let evt = Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(1, 2),
            pressed: true,
            modifiers: Modifiers::SHIFT,
        });
        let clone = evt.clone();
        assert_eq!(evt, clone);
    }
}

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
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8,
    F9, F10, F11, F12, F13, F14, F15,
    F16, F17, F18, F19, F20, F21, F22,
    F23, F24,
    
    // Navigation
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,
    
    // Editing
    Backspace, Delete, Insert,
    Tab, Enter, Escape, Space,
    
    // Modifiers
    LeftShift, RightShift,
    LeftCtrl, RightCtrl,
    LeftAlt, RightAlt,
    LeftSuper, RightSuper,
    
    // Punctuation
    Comma, Period, Slash, Backslash,
    Semicolon, Quote, LeftBracket, RightBracket,
    Minus, Equals, Grave,
    
    // Numpad
    NumpadNum0, NumpadNum1, NumpadNum2, NumpadNum3, NumpadNum4,
    NumpadNum5, NumpadNum6, NumpadNum7, NumpadNum8, NumpadNum9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide,
    NumpadDecimal, NumpadEnter,
    
    // Media
    PlayPause, Stop, NextTrack, PreviousTrack,
    VolumeUp, VolumeDown, Mute,
    
    // Other
    PrintScreen, Pause, CapsLock, NumLock, ScrollLock,
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
            Key::LeftShift | Key::RightShift |
            Key::LeftCtrl | Key::RightCtrl |
            Key::LeftAlt | Key::RightAlt |
            Key::LeftSuper | Key::RightSuper
        )
    }
    
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            Key::Up | Key::Down | Key::Left | Key::Right |
            Key::Home | Key::End | Key::PageUp | Key::PageDown
        )
    }
    
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            Key::Backspace | Key::Delete | Key::Insert |
            Key::Tab | Key::Enter | Key::Escape | Key::Space
        )
    }
}
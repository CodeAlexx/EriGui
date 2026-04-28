//! Simple in-process clipboard for text widgets.
//!
//! This is *not* a system clipboard — it's a process-local `RwLock<String>`.
//! Cross-process integration (X11/Wayland selections, win32, macOS) is a
//! separate piece of work that requires per-platform infrastructure and is
//! tracked as a follow-up. The widget tests run against this in-process
//! clipboard, so cut/copy/paste behavior is verifiable without touching
//! a real OS clipboard.

use std::sync::RwLock;

static CLIPBOARD: RwLock<String> = RwLock::new(String::new());

/// Write text to the in-process clipboard, replacing any prior contents.
pub fn set_clipboard(text: &str) {
    if let Ok(mut guard) = CLIPBOARD.write() {
        guard.clear();
        guard.push_str(text);
    }
}

/// Read the current in-process clipboard contents.
pub fn get_clipboard() -> String {
    CLIPBOARD
        .read()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// Clear the in-process clipboard. Mainly used by tests that want a clean
/// slate.
pub fn clear_clipboard() {
    if let Ok(mut guard) = CLIPBOARD.write() {
        guard.clear();
    }
}

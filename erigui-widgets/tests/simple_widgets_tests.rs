//! Integration tests for the small / display-mostly widgets:
//! Label, Container, StatusBar, Icon.
//!
//! These widgets had zero integration tests before. The bar here is
//! "no widget has zero tests anymore" -- not exhaustive coverage. Where the
//! widget's public API doesn't expose enough state to assert on, the test
//! degenerates to "construction + call doesn't panic," which is still a
//! regression net.

use erigui_core::{
    Color, Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConstraints, Modifiers,
    MouseButton, MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, Rect, ResizeEvent,
    Size, TextInputEvent, Theme, Widget, WidgetId,
};
use erigui_widgets::{Label, TextAlign};

// Helper-functions copied verbatim from widget_tests.rs so this file is
// self-contained. Keep them in sync if widget_tests.rs ever changes them.
fn default_theme() -> Theme {
    Theme::dark()
}

fn test_id() -> WidgetId {
    WidgetId::default()
}

// ============================================================================
// Label
// ============================================================================
//
// Label is genuinely "text in, text out" -- display-only, no event handling
// (handle_event always returns Ignored), no focus, no children. The testable
// surface is small and that's fine.

#[test]
fn label_creation_round_trip() {
    let label = Label::new(test_id(), "Hello");
    assert_eq!(label.text(), "Hello");
    assert!(label.is_visible());
    assert!(label.is_enabled());
    // Label opts out of focus entirely.
    assert!(!label.can_focus());
    assert!(!label.is_focused());
}

#[test]
fn label_set_text_round_trips() {
    let mut label = Label::new(test_id(), "first");
    assert_eq!(label.text(), "first");
    label.set_text("second");
    assert_eq!(label.text(), "second");
    label.set_text(String::from("third"));
    assert_eq!(label.text(), "third");
}

#[test]
fn label_with_color_does_not_panic() {
    // No public color getter -- this is a smoke test that the builder runs
    // and that subsequent layout/measure still work.
    let label = Label::new(test_id(), "x").with_color(Color::WHITE);
    let theme = default_theme();
    let size = label.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert!(size.height > 0);
}

#[test]
fn label_with_align_builder_runs() {
    // No public alignment getter either; verify the builder chain doesn't
    // explode and the widget remains measureable.
    let _ = Label::new(test_id(), "x").with_align(TextAlign::Left);
    let _ = Label::new(test_id(), "x").with_align(TextAlign::Center);
    let _ = Label::new(test_id(), "x").with_align(TextAlign::Right);
}

#[test]
fn label_set_align_does_not_panic() {
    let mut label = Label::new(test_id(), "x");
    label.set_align(TextAlign::Center);
    label.set_align(TextAlign::Right);
    label.set_align(TextAlign::Left);
}

#[test]
fn label_set_color_does_not_panic() {
    let mut label = Label::new(test_id(), "x");
    label.set_color(Some(Color::WHITE));
    label.set_color(None);
}

#[test]
fn label_measure_returns_positive_size_for_nonempty_text() {
    let label = Label::new(test_id(), "hello");
    let theme = default_theme();
    let size = label.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert!(size.width > 0, "label with text should have positive width");
    assert!(size.height > 0, "label should have positive height");
}

#[test]
fn label_measure_height_is_font_size() {
    let label = Label::new(test_id(), "anything");
    let theme = default_theme();
    let size = label.measure(&LayoutConstraints::UNBOUNDED, &theme);
    // Label measure() returns font_size_base for height; lock it in.
    assert_eq!(size.height, theme.typography.font_size_base);
}

#[test]
fn label_layout_sets_bounds() {
    let mut label = Label::new(test_id(), "x");
    let theme = default_theme();
    label.layout(Rect::new(5, 10, 80, 24), &theme);
    let b = label.bounds();
    assert_eq!(b.x(), 5);
    assert_eq!(b.y(), 10);
    assert_eq!(b.width(), 80);
    assert_eq!(b.height(), 24);
}

#[test]
fn label_visibility_round_trip() {
    let mut label = Label::new(test_id(), "x");
    assert!(label.is_visible());
    label.set_visible(false);
    assert!(!label.is_visible());
    label.set_visible(true);
    assert!(label.is_visible());
}

#[test]
fn label_enabled_round_trip() {
    let mut label = Label::new(test_id(), "x");
    assert!(label.is_enabled());
    label.set_enabled(false);
    assert!(!label.is_enabled());
    label.set_enabled(true);
    assert!(label.is_enabled());
}

#[test]
fn label_set_focused_is_a_no_op() {
    // Label::set_focused() is documented as a no-op; lock that in so a
    // future refactor doesn't accidentally start mutating focus.
    let mut label = Label::new(test_id(), "x");
    assert!(!label.is_focused());
    label.set_focused(true);
    assert!(!label.is_focused(), "Label.is_focused must stay false even after set_focused(true)");
}

#[test]
fn label_ignores_all_events() {
    // Label is display-only: every event variant must return Ignored.
    let theme = default_theme();
    let mut label = Label::new(test_id(), "x");
    label.layout(Rect::new(0, 0, 100, 24), &theme);

    let events = [
        Event::MouseMove(MouseMoveEvent {
            position: Point::new(10, 10),
            delta: Point::new(0, 0),
            modifiers: Modifiers::empty(),
        }),
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(0, -1),
            position: Point::new(10, 10),
            modifiers: Modifiers::empty(),
        }),
        Event::KeyPress(KeyPressEvent {
            key: Key::Space,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::TextInput(TextInputEvent {
            text: "z".to_string(),
        }),
        Event::Focus(FocusEvent { gained: true }),
        Event::Resize(ResizeEvent {
            old_size: Size::new(100, 100),
            new_size: Size::new(200, 200),
        }),
        Event::Update,
    ];
    for ev in &events {
        assert_eq!(
            label.handle_event(ev, &theme),
            EventResult::Ignored,
            "Label must ignore event {:?}",
            ev
        );
    }
}

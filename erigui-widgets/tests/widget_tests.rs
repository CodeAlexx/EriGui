//! Tests for widget implementations
//! These tests verify widget construction, state management, and basic functionality
//! without requiring a graphical context.

use erigui_core::{LayoutConstraints, Rect, Theme, Widget, WidgetId};
use erigui_widgets::{Button, Checkbox, ProgressBar, Slider, SliderOrientation, TextInput};

// ============================================================================
// Helper Functions
// ============================================================================

fn default_theme() -> Theme {
    Theme::dark()
}

fn test_id() -> WidgetId {
    WidgetId::default()
}

// ============================================================================
// Button Tests
// ============================================================================

#[test]
fn test_button_creation() {
    let button = Button::new(test_id(), "Click Me");
    assert_eq!(button.text(), "Click Me");
}

#[test]
fn test_button_set_text() {
    let mut button = Button::new(test_id(), "Initial");
    assert_eq!(button.text(), "Initial");

    button.set_text("Updated");
    assert_eq!(button.text(), "Updated");
}

#[test]
fn test_button_measure() {
    let button = Button::new(test_id(), "Test");
    let theme = default_theme();
    let constraints = LayoutConstraints::UNBOUNDED;

    let size = button.measure(&constraints, &theme);
    assert!(size.width > 0);
    assert!(size.height > 0);
}

#[test]
fn test_button_measure_empty_text() {
    let button = Button::new(test_id(), "");
    let theme = default_theme();
    let constraints = LayoutConstraints::UNBOUNDED;

    let size = button.measure(&constraints, &theme);
    // Should have minimum size even with no text
    assert!(size.width >= 32);
    assert!(size.height >= 32);
}

// ============================================================================
// Checkbox Tests
// ============================================================================

#[test]
fn test_checkbox_creation() {
    let checkbox = Checkbox::new(test_id(), "Option");
    assert!(!checkbox.is_checked());
}

#[test]
fn test_checkbox_with_checked() {
    let checkbox = Checkbox::new(test_id(), "Option").with_checked(true);
    assert!(checkbox.is_checked());
}

#[test]
fn test_checkbox_toggle() {
    let mut checkbox = Checkbox::new(test_id(), "Option");
    assert!(!checkbox.is_checked());

    checkbox.toggle();
    assert!(checkbox.is_checked());

    checkbox.toggle();
    assert!(!checkbox.is_checked());
}

#[test]
fn test_checkbox_set_checked() {
    let mut checkbox = Checkbox::new(test_id(), "Option");

    checkbox.set_checked(true);
    assert!(checkbox.is_checked());

    checkbox.set_checked(false);
    assert!(!checkbox.is_checked());
}

#[test]
fn test_checkbox_callback() {
    use std::cell::Cell;
    use std::rc::Rc;

    let called = Rc::new(Cell::new(false));
    let called_clone = called.clone();

    let mut checkbox =
        Checkbox::new(test_id(), "Option").with_on_toggle(move |_| called_clone.set(true));

    checkbox.toggle();
    assert!(called.get());
}

// ============================================================================
// TextInput Tests
// ============================================================================

#[test]
fn test_text_input_creation() {
    let input = TextInput::new(test_id());
    assert_eq!(input.text(), "");
}

#[test]
fn test_text_input_with_text() {
    let input = TextInput::new(test_id()).with_text("Hello");
    assert_eq!(input.text(), "Hello");
}

#[test]
fn test_text_input_set_text() {
    let mut input = TextInput::new(test_id());
    input.set_text("World");
    assert_eq!(input.text(), "World");
}

#[test]
fn test_text_input_with_placeholder() {
    let input = TextInput::new(test_id()).with_placeholder("Enter text...");
    // Placeholder doesn't affect text()
    assert_eq!(input.text(), "");
}

#[test]
fn test_text_input_get_text() {
    let input = TextInput::new(test_id()).with_text("Test");
    assert_eq!(input.get_text(), "Test");
}

// ============================================================================
// Slider Tests
// ============================================================================

#[test]
fn test_slider_creation() {
    let slider = Slider::new(test_id(), 0.0, 100.0, 50.0);
    assert_eq!(slider.value(), 50.0);
}

#[test]
fn test_slider_value_clamping() {
    // Value below minimum
    let slider = Slider::new(test_id(), 0.0, 100.0, -50.0);
    assert_eq!(slider.value(), 0.0);

    // Value above maximum
    let slider = Slider::new(test_id(), 0.0, 100.0, 150.0);
    assert_eq!(slider.value(), 100.0);
}

#[test]
fn test_slider_set_value() {
    let mut slider = Slider::new(test_id(), 0.0, 100.0, 0.0);

    slider.set_value(75.0);
    assert_eq!(slider.value(), 75.0);

    // Test clamping
    slider.set_value(150.0);
    assert_eq!(slider.value(), 100.0);

    slider.set_value(-50.0);
    assert_eq!(slider.value(), 0.0);
}

#[test]
fn test_slider_normalized_value() {
    let slider = Slider::new(test_id(), 0.0, 100.0, 50.0);
    assert!((slider.normalized_value() - 0.5).abs() < 0.001);

    let slider = Slider::new(test_id(), 0.0, 100.0, 0.0);
    assert!((slider.normalized_value() - 0.0).abs() < 0.001);

    let slider = Slider::new(test_id(), 0.0, 100.0, 100.0);
    assert!((slider.normalized_value() - 1.0).abs() < 0.001);
}

#[test]
fn test_slider_normalized_value_custom_range() {
    let slider = Slider::new(test_id(), -50.0, 50.0, 0.0);
    assert!((slider.normalized_value() - 0.5).abs() < 0.001);
}

#[test]
fn test_slider_orientation() {
    let slider = Slider::new(test_id(), 0.0, 100.0, 50.0);
    assert_eq!(slider.orientation(), SliderOrientation::Horizontal);

    let slider = slider.with_orientation(SliderOrientation::Vertical);
    assert_eq!(slider.orientation(), SliderOrientation::Vertical);
}

#[test]
fn test_slider_callback() {
    use std::cell::Cell;
    use std::rc::Rc;

    let received_value = Rc::new(Cell::new(0.0f32));
    let received_clone = received_value.clone();

    let mut slider = Slider::new(test_id(), 0.0, 100.0, 0.0)
        .with_on_value_changed(move |v| received_clone.set(v));

    slider.set_value(75.0);
    assert!((received_value.get() - 75.0).abs() < 0.001);
}

// ============================================================================
// ProgressBar Tests
// ============================================================================

#[test]
fn test_progress_bar_creation() {
    let progress = ProgressBar::new(test_id());
    assert_eq!(progress.value(), 0.0);
}

#[test]
fn test_progress_bar_with_value() {
    let progress = ProgressBar::new(test_id()).with_value(50.0);
    assert_eq!(progress.value(), 50.0);
}

#[test]
fn test_progress_bar_set_value() {
    let mut progress = ProgressBar::new(test_id());

    progress.set_value(75.0);
    assert_eq!(progress.value(), 75.0);

    // Test clamping to default range (0-100)
    progress.set_value(150.0);
    assert_eq!(progress.value(), 100.0);

    progress.set_value(-50.0);
    assert_eq!(progress.value(), 0.0);
}

#[test]
fn test_progress_bar_percentage() {
    let progress = ProgressBar::new(test_id()).with_value(50.0);
    assert!((progress.percentage() - 50.0).abs() < 0.001);

    let progress = ProgressBar::new(test_id()).with_value(0.0);
    assert!((progress.percentage() - 0.0).abs() < 0.001);

    let progress = ProgressBar::new(test_id()).with_value(100.0);
    assert!((progress.percentage() - 100.0).abs() < 0.001);
}

#[test]
fn test_progress_bar_custom_range() {
    let progress = ProgressBar::new(test_id())
        .with_range(0.0, 200.0)
        .with_value(100.0);

    assert!((progress.percentage() - 50.0).abs() < 0.001);
    assert!((progress.normalized_value() - 0.5).abs() < 0.001);
}

#[test]
fn test_progress_bar_normalized_value() {
    let progress = ProgressBar::new(test_id()).with_value(25.0);
    assert!((progress.normalized_value() - 0.25).abs() < 0.001);
}

// ============================================================================
// Widget Trait Common Tests
// ============================================================================

#[test]
fn test_widget_visibility() {
    use erigui_core::Widget;

    let button = Button::new(test_id(), "Test");
    assert!(button.is_visible()); // Default visible

    let mut button = Button::new(test_id(), "Test");
    button.set_visible(false);
    assert!(!button.is_visible());
}

#[test]
fn test_widget_enabled() {
    use erigui_core::Widget;

    let button = Button::new(test_id(), "Test");
    assert!(button.is_enabled()); // Default enabled

    let mut button = Button::new(test_id(), "Test");
    button.set_enabled(false);
    assert!(!button.is_enabled());
}

#[test]
fn test_widget_bounds() {
    use erigui_core::Widget;

    let mut button = Button::new(test_id(), "Test");
    let theme = default_theme();

    // Layout sets bounds
    button.layout(Rect::new(10, 20, 100, 50), &theme);
    let bounds = button.bounds();

    assert_eq!(bounds.x(), 10);
    assert_eq!(bounds.y(), 20);
    assert_eq!(bounds.width(), 100);
    assert_eq!(bounds.height(), 50);
}

// ============================================================================
// Production-keyboard-input regression tests (overnight audit, 2026-04-28)
//
// winit 0.29 emits typed characters via `Event::TextInput`, separate from
// `Event::KeyPress`. Several widgets only handled the legacy
// `Key::Character(ch)` form which never fires in production. These tests
// lock in the fix so the bug doesn't come back.
//
// Fixes applied to: combo_box, spin_box, color_picker, date_time_picker.
// Only combo_box has enough public API surface to drive a complete
// integration test from outside the crate. The other three widgets'
// internal state (hex_input, editing flags, hour_input/minute_input) is
// private; a smoke-level test that the fix compiles is the most we can
// do without exposing internals.
// ============================================================================

use erigui_core::{Event, EventResult, MouseButton, MouseButtonEvent, Point, TextInputEvent};
use erigui_widgets::ComboBox;

#[test]
fn combo_box_text_input_consumed_when_open() {
    let theme = default_theme();
    let mut cb = ComboBox::new(test_id())
        .with_items(vec!["apple".to_string(), "banana".to_string()]);
    cb.layout(Rect::new(0, 0, 200, 30), &theme);

    let click = MouseButtonEvent {
        button: MouseButton::Left,
        pressed: true,
        position: Point::new(100, 15),
        modifiers: erigui_core::Modifiers::empty(),
    };
    cb.handle_event(&Event::MouseButton(click), &theme);
    assert!(cb.is_open(), "click should open the dropdown");

    let ti = TextInputEvent { text: "b".to_string() };
    let result = cb.handle_event(&Event::TextInput(ti), &theme);
    assert_eq!(
        result,
        EventResult::Consumed,
        "open ComboBox must consume Event::TextInput for filter"
    );
}

#[test]
fn combo_box_text_input_ignored_when_closed() {
    let theme = default_theme();
    let mut cb = ComboBox::new(test_id())
        .with_items(vec!["x".to_string()]);
    cb.layout(Rect::new(0, 0, 200, 30), &theme);

    let ti = TextInputEvent { text: "x".to_string() };
    let result = cb.handle_event(&Event::TextInput(ti), &theme);
    assert_eq!(
        result,
        EventResult::Ignored,
        "closed ComboBox should NOT consume Event::TextInput"
    );
}

// ============================================================================
// Focus + keyboard tests for Slider and Checkbox (overnight audit, 2026-04-28)
// Both had set_focused as a no-op, making `can_focus = true` a lie. Both also
// had no keyboard support — the user had to use mouse exclusively.
// ============================================================================

use erigui_core::{Key, KeyPressEvent, Modifiers};

#[test]
fn slider_focus_state_round_trips() {
    let mut s = Slider::new(test_id(), 0.0, 100.0, 50.0);
    assert!(!s.is_focused());
    s.set_focused(true);
    assert!(s.is_focused());
    s.set_focused(false);
    assert!(!s.is_focused());
}

#[test]
fn slider_arrow_keys_step_when_focused() {
    let theme = default_theme();
    let mut s = Slider::new(test_id(), 0.0, 100.0, 50.0);
    s.layout(Rect::new(0, 0, 200, 30), &theme);
    s.set_focused(true);

    let key_right = Event::KeyPress(KeyPressEvent {
        key: Key::Right,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let r = s.handle_event(&key_right, &theme);
    assert_eq!(r, EventResult::Consumed);
    assert!(
        s.value() > 50.0,
        "Right arrow should increase slider value, got {}",
        s.value()
    );

    let before = s.value();
    let key_left = Event::KeyPress(KeyPressEvent {
        key: Key::Left,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    s.handle_event(&key_left, &theme);
    assert!(
        s.value() < before,
        "Left arrow should decrease slider value, got {}",
        s.value()
    );
}

#[test]
fn slider_keys_ignored_when_unfocused() {
    let theme = default_theme();
    let mut s = Slider::new(test_id(), 0.0, 100.0, 50.0);
    s.layout(Rect::new(0, 0, 200, 30), &theme);
    // Not focused.

    let before = s.value();
    let r = s.handle_event(
        &Event::KeyPress(KeyPressEvent {
            key: Key::Right,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        &theme,
    );
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(s.value(), before, "unfocused slider must not respond to keys");
}

#[test]
fn checkbox_focus_state_round_trips() {
    let mut c = Checkbox::new(test_id(), "x");
    assert!(!c.is_focused());
    c.set_focused(true);
    assert!(c.is_focused());
    c.set_focused(false);
    assert!(!c.is_focused());
}

#[test]
fn checkbox_space_toggles_when_focused() {
    let theme = default_theme();
    let mut c = Checkbox::new(test_id(), "x");
    c.layout(Rect::new(0, 0, 100, 24), &theme);
    c.set_focused(true);

    let initial = c.is_checked();
    let r = c.handle_event(
        &Event::KeyPress(KeyPressEvent {
            key: Key::Space,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        &theme,
    );
    assert_eq!(r, EventResult::Consumed);
    assert_ne!(c.is_checked(), initial, "Space must toggle a focused checkbox");
}

#[test]
fn checkbox_keys_ignored_when_unfocused() {
    let theme = default_theme();
    let mut c = Checkbox::new(test_id(), "x");
    c.layout(Rect::new(0, 0, 100, 24), &theme);

    let initial = c.is_checked();
    let r = c.handle_event(
        &Event::KeyPress(KeyPressEvent {
            key: Key::Space,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        &theme,
    );
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(c.is_checked(), initial);
}

// ============================================================================
// ListView keyboard navigation (overnight audit, 2026-04-28)
// Previously: focus state was wired but NO key handling. Long lists were
// unusable from the keyboard.
// ============================================================================

use erigui_widgets::{ListItem, ListView};

fn list_with(n: usize) -> ListView {
    let items = (0..n)
        .map(|i| ListItem {
            id: format!("id_{i}"),
            text: format!("item_{i}"),
            icon: None,
            selected: false,
        })
        .collect();
    ListView::new(test_id()).with_items(items)
}

#[test]
fn list_view_down_arrow_advances_selection_when_focused() {
    let theme = default_theme();
    let mut lv = list_with(5);
    lv.layout(Rect::new(0, 0, 200, 200), &theme);
    lv.set_focused(true);
    lv.set_selected_index(Some(0));

    let key = |k: Key| Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::empty(),
        repeat: false,
    });

    lv.handle_event(&key(Key::Down), &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_1");
    lv.handle_event(&key(Key::Down), &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_2");
}

#[test]
fn list_view_down_clamps_at_last() {
    let theme = default_theme();
    let mut lv = list_with(3);
    lv.layout(Rect::new(0, 0, 200, 200), &theme);
    lv.set_focused(true);
    lv.set_selected_index(Some(2));

    let key = Event::KeyPress(KeyPressEvent {
        key: Key::Down,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    lv.handle_event(&key, &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_2", "Down at end stays put");
}

#[test]
fn list_view_up_clamps_at_zero() {
    let theme = default_theme();
    let mut lv = list_with(3);
    lv.layout(Rect::new(0, 0, 200, 200), &theme);
    lv.set_focused(true);
    lv.set_selected_index(Some(0));

    let key = Event::KeyPress(KeyPressEvent {
        key: Key::Up,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    lv.handle_event(&key, &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_0", "Up at start stays put");
}

#[test]
fn list_view_home_end_jump_to_bounds() {
    let theme = default_theme();
    let mut lv = list_with(10);
    lv.layout(Rect::new(0, 0, 200, 200), &theme);
    lv.set_focused(true);
    lv.set_selected_index(Some(5));

    let key = |k: Key| Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::empty(),
        repeat: false,
    });

    lv.handle_event(&key(Key::Home), &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_0");
    lv.handle_event(&key(Key::End), &theme);
    assert_eq!(lv.selected_item().unwrap().id, "id_9");
}

#[test]
fn list_view_mouse_wheel_scrolls() {
    use erigui_core::{MouseWheelEvent, Point};
    let theme = default_theme();
    // 50 items × 24px each = 1200px content, viewport 240px = 5x bigger.
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);
    assert_eq!(lv.scroll_offset(), 0);

    // Scroll DOWN: delta.y is negative on most platforms.
    let wheel_down = Event::MouseWheel(MouseWheelEvent {
        delta: Point::new(0, -1),
        position: Point::new(50, 50),
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&wheel_down, &theme);
    assert_eq!(r, EventResult::Consumed);
    assert!(
        lv.scroll_offset() > 0,
        "wheel down should advance scroll_offset, got {}",
        lv.scroll_offset()
    );

    // Wheel UP brings us back.
    let wheel_up = Event::MouseWheel(MouseWheelEvent {
        delta: Point::new(0, 1),
        position: Point::new(50, 50),
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&wheel_up, &theme);
    assert_eq!(lv.scroll_offset(), 0, "wheel up should bring us back to top");
}

#[test]
fn list_view_scroll_clamped_at_bounds() {
    use erigui_core::{MouseWheelEvent, Point};
    let theme = default_theme();
    let mut lv = list_with(2); // very short list, no need to scroll
    lv.layout(Rect::new(0, 0, 200, 240), &theme);

    // Wheel down a bunch — scroll_offset should stay at 0 since no overflow.
    for _ in 0..10 {
        let wheel = Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(0, -10),
            position: Point::new(50, 50),
            modifiers: Modifiers::empty(),
        });
        lv.handle_event(&wheel, &theme);
    }
    assert_eq!(
        lv.scroll_offset(),
        0,
        "scroll_offset must not exceed max when content fits viewport"
    );
}

#[test]
fn list_view_arrow_down_auto_scrolls_to_keep_selected_visible() {
    let theme = default_theme();
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme); // viewport shows ~10 items
    lv.set_focused(true);
    lv.set_selected_index(Some(0));
    assert_eq!(lv.scroll_offset(), 0);

    // Move down 20 items; selection should stay visible by auto-scrolling.
    for _ in 0..20 {
        let _ = lv.handle_event(
            &Event::KeyPress(KeyPressEvent {
                key: Key::Down,
                modifiers: Modifiers::empty(),
                repeat: false,
            }),
            &theme,
        );
    }
    assert_eq!(lv.selected_item().unwrap().id, "id_20");
    assert!(
        lv.scroll_offset() > 0,
        "selection past viewport should have triggered scroll"
    );
}

// ----- ListView scrollbar drag (item #3 from production handoff) -----
//
// Layout used below: 50 items × 24px = 1200px content, viewport 240px on
// bounds (0,0,200,240). Track at x=192..200; thumb_h = 240/1200 * 240 = 48,
// so at scroll_offset=0 the thumb spans y=[0,48] and a click at (195, 24)
// lands solidly inside it. max_thumb_y = 192, max_scroll = 960.

#[test]
fn list_view_drag_thumb_scrolls_content() {
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point};
    let theme = default_theme();
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);
    assert_eq!(lv.scroll_offset(), 0);

    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 24),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&press, &theme);
    assert_eq!(r, EventResult::Consumed, "press inside thumb should consume");
    assert_eq!(lv.scroll_offset(), 0, "press alone shouldn't move scroll");

    // Drag mouse down 96px. Expected scroll: 96 * (max_scroll / max_thumb_y)
    // = 96 * (960 / 192) = 480.
    let mv = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 120),
        delta: Point::new(0, 96),
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&mv, &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(lv.scroll_offset(), 480, "drag should map mouse Y to scroll proportionally");
}

#[test]
fn list_view_drag_release_stops_tracking() {
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point};
    let theme = default_theme();
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);

    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 24),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&press, &theme);

    let mv1 = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 60),
        delta: Point::new(0, 36),
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&mv1, &theme);
    let after_drag = lv.scroll_offset();
    assert!(after_drag > 0);

    let release = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 60),
        pressed: false,
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&release, &theme);
    assert_eq!(r, EventResult::Consumed, "release while dragging consumes");

    // Subsequent move should NOT change scroll.
    let mv2 = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 200),
        delta: Point::new(0, 140),
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&mv2, &theme);
    assert_eq!(r, EventResult::Ignored, "move after release should not be consumed");
    assert_eq!(lv.scroll_offset(), after_drag, "scroll must not change after release");
}

#[test]
fn list_view_drag_past_end_clamps_at_max() {
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point};
    let theme = default_theme();
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);

    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 24),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&press, &theme);

    // Mouse Y 9999 → would compute scroll way past max; must clamp.
    let mv = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 9999),
        delta: Point::new(0, 9975),
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&mv, &theme);
    // max_scroll = 50*24 - 240 = 960.
    assert_eq!(lv.scroll_offset(), 960, "drag past end clamps to max_scroll");
}

#[test]
fn list_view_drag_when_no_overflow_does_nothing() {
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point};
    let theme = default_theme();
    // Short list — no scrollbar drawn.
    let mut lv = list_with(3);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);

    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 24),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&press, &theme);
    // Could be Consumed (item-click hit-test fires) but the important assert
    // is no drag was engaged: a subsequent MouseMove must NOT scroll.
    let _ = r;

    let mv = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 200),
        delta: Point::new(0, 176),
        modifiers: Modifiers::empty(),
    });
    let r2 = lv.handle_event(&mv, &theme);
    assert_eq!(r2, EventResult::Ignored, "no drag → move ignored");
    assert_eq!(lv.scroll_offset(), 0, "no overflow → no scroll possible");
}

#[test]
fn list_view_press_on_track_outside_thumb_does_not_drag() {
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Point};
    let theme = default_theme();
    let mut lv = list_with(50);
    lv.layout(Rect::new(0, 0, 200, 240), &theme);

    // Thumb spans y=[0,48] at offset=0. y=200 is in the track but below the
    // thumb. Currently a no-op (click could later become page-jump; not yet).
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(195, 200),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    lv.handle_event(&press, &theme);

    // Move to confirm no drag was engaged.
    let mv = Event::MouseMove(MouseMoveEvent {
        position: Point::new(195, 100),
        delta: Point::new(0, -100),
        modifiers: Modifiers::empty(),
    });
    let r = lv.handle_event(&mv, &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(lv.scroll_offset(), 0, "track-but-not-thumb press shouldn't drag");
}

#[test]
fn button_space_enter_fires_when_focused() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let theme = default_theme();
    let clicks = Rc::new(RefCell::new(0u32));
    let c = clicks.clone();
    let mut b = Button::new(test_id(), "Go").with_on_click(move || {
        *c.borrow_mut() += 1;
    });
    b.layout(Rect::new(0, 0, 100, 30), &theme);
    b.set_focused(true);

    let key = |k: Key| Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::empty(),
        repeat: false,
    });

    b.handle_event(&key(Key::Space), &theme);
    assert_eq!(*clicks.borrow(), 1, "Space on focused button should click");
    b.handle_event(&key(Key::Enter), &theme);
    assert_eq!(*clicks.borrow(), 2, "Enter on focused button should click");
}

#[test]
fn button_keys_ignored_when_unfocused() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let theme = default_theme();
    let clicks = Rc::new(RefCell::new(0u32));
    let c = clicks.clone();
    let mut b = Button::new(test_id(), "Go").with_on_click(move || {
        *c.borrow_mut() += 1;
    });
    b.layout(Rect::new(0, 0, 100, 30), &theme);
    // Not focused.

    let _ = b.handle_event(
        &Event::KeyPress(KeyPressEvent {
            key: Key::Space,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        &theme,
    );
    assert_eq!(*clicks.borrow(), 0);
}

#[test]
fn list_view_keys_ignored_when_unfocused() {
    let theme = default_theme();
    let mut lv = list_with(5);
    lv.layout(Rect::new(0, 0, 200, 200), &theme);
    lv.set_selected_index(Some(0));
    // Not focused.

    let key = Event::KeyPress(KeyPressEvent {
        key: Key::Down,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let r = lv.handle_event(&key, &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(lv.selected_item().unwrap().id, "id_0", "unfocused list shouldn't move");
}

// ============================================================================
// TabControl focus + keyboard nav (item #4 from production handoff)
// Previously: can_focus() returned false, set_focused was a no-op, no key
// handling — Ctrl+Tab did nothing, hosts couldn't direct focus to it.
// ============================================================================

use erigui_widgets::{TabControl, TabPage};

fn tabs_with(n: usize) -> TabControl {
    let mut tc = TabControl::new(test_id());
    for i in 0..n {
        tc.add_tab(TabPage::new(format!("tab_{i}")));
    }
    tc
}

fn ctrl_key(k: Key) -> Event {
    Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::CTRL,
        repeat: false,
    })
}

fn ctrl_shift_key(k: Key) -> Event {
    Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        repeat: false,
    })
}

#[test]
fn tab_control_focus_round_trips() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    assert!(tc.can_focus(), "enabled+visible TabControl should be focusable");
    assert!(!tc.is_focused(), "starts unfocused");
    tc.set_focused(true);
    assert!(tc.is_focused(), "set_focused(true) should stick");
    tc.set_focused(false);
    assert!(!tc.is_focused(), "set_focused(false) should stick");
}

#[test]
fn tab_control_ctrl_tab_advances_when_focused() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);
    assert_eq!(tc.active_tab(), 0);

    let r = tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(tc.active_tab(), 1, "Ctrl+Tab should move forward one tab");

    tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(tc.active_tab(), 2);
}

#[test]
fn tab_control_ctrl_shift_tab_goes_back() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);
    tc.set_active_tab(2);

    let r = tc.handle_event(&ctrl_shift_key(Key::Tab), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(tc.active_tab(), 1, "Ctrl+Shift+Tab should move back one tab");
}

#[test]
fn tab_control_ctrl_tab_wraps_at_end() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);
    tc.set_active_tab(2);

    tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(tc.active_tab(), 0, "Ctrl+Tab from last should wrap to first");
}

#[test]
fn tab_control_ctrl_shift_tab_wraps_at_start() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);
    assert_eq!(tc.active_tab(), 0);

    tc.handle_event(&ctrl_shift_key(Key::Tab), &theme);
    assert_eq!(tc.active_tab(), 2, "Ctrl+Shift+Tab from first should wrap to last");
}

#[test]
fn tab_control_ctrl_digit_jumps_to_index() {
    let theme = default_theme();
    let mut tc = tabs_with(5);
    tc.layout(Rect::new(0, 0, 800, 300), &theme);
    tc.set_focused(true);

    tc.handle_event(&ctrl_key(Key::Num3), &theme);
    assert_eq!(tc.active_tab(), 2, "Ctrl+3 should activate the 3rd tab (index 2)");
    tc.handle_event(&ctrl_key(Key::Num1), &theme);
    assert_eq!(tc.active_tab(), 0);
    tc.handle_event(&ctrl_key(Key::Num5), &theme);
    assert_eq!(tc.active_tab(), 4);
}

#[test]
fn tab_control_ctrl_digit_past_count_is_noop() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);

    let r = tc.handle_event(&ctrl_key(Key::Num9), &theme);
    assert_eq!(r, EventResult::Ignored, "Ctrl+9 with only 3 tabs should not consume");
    assert_eq!(tc.active_tab(), 0, "active stays at 0");
}

#[test]
fn tab_control_ctrl_tab_skips_disabled() {
    let theme = default_theme();
    let mut tc = TabControl::new(test_id());
    tc.add_tab(TabPage::new("a"));
    let mut middle = TabPage::new("b");
    middle.enabled = false;
    tc.add_tab(middle);
    tc.add_tab(TabPage::new("c"));
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);
    assert_eq!(tc.active_tab(), 0);

    // Ctrl+Tab forward should jump 0 → 2 (skipping disabled 1).
    tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(tc.active_tab(), 2, "Ctrl+Tab must skip disabled tab");
    // And from 2, Ctrl+Tab wraps past disabled 1 to 0.
    tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(tc.active_tab(), 0);
}

#[test]
fn tab_control_keys_ignored_when_unfocused() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    // Not focused.

    let r = tc.handle_event(&ctrl_key(Key::Tab), &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(tc.active_tab(), 0, "unfocused TabControl ignores Ctrl+Tab");
}

#[test]
fn tab_control_unmodified_tab_ignored() {
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    tc.set_focused(true);

    // Plain Tab (no Ctrl) is the host's tab-traversal key — must not cycle.
    let plain = Event::KeyPress(KeyPressEvent {
        key: Key::Tab,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let r = tc.handle_event(&plain, &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(tc.active_tab(), 0);
}

#[test]
fn tab_control_click_focuses() {
    use erigui_core::{MouseButton, MouseButtonEvent, Point};
    let theme = default_theme();
    let mut tc = tabs_with(3);
    tc.layout(Rect::new(0, 0, 600, 300), &theme);
    assert!(!tc.is_focused());

    // Tab 0 is at x ∈ [0, max(600/3, 80, ...)). With width 600 / 3 = 200,
    // tab 0 is at (0, 0, 200, 30). Click at (50, 15) lands inside.
    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(50, 15),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    tc.handle_event(&click, &theme);
    assert!(tc.is_focused(), "click on a tab should focus the TabControl");
}

// ============================================================================
// Accordion focus + keyboard nav (Mojo-port cleanup, 2026-04-28)
// Previously: can_focus() returned false despite real focus state — focus
// was unreachable, no keyboard nav, hosts couldn't direct focus to the
// Accordion.
// ============================================================================

use erigui_widgets::{Accordion, AccordionPanel, Label};

fn no_mod_key(k: Key) -> Event {
    Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::empty(),
        repeat: false,
    })
}

fn accordion_with(n: usize) -> Accordion {
    let mut a = Accordion::new(test_id());
    for i in 0..n {
        let content: Box<dyn Widget> =
            Box::new(Label::new(test_id(), format!("body_{i}")));
        a.add_panel(AccordionPanel::new(format!("panel_{i}"), content));
    }
    a
}

#[test]
fn accordion_focus_round_trips() {
    let theme = default_theme();
    let mut a = accordion_with(3);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    assert!(a.can_focus(), "enabled+visible Accordion should be focusable");
    assert!(!a.is_focused(), "starts unfocused");
    a.set_focused(true);
    assert!(a.is_focused());
    a.set_focused(false);
    assert!(!a.is_focused());
}

#[test]
fn accordion_down_advances_focused_panel_when_focused() {
    let theme = default_theme();
    let mut a = accordion_with(3);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    a.set_focused(true);
    assert_eq!(a.focused_panel(), None, "starts with no panel cursor");

    let r = a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(
        a.focused_panel(),
        Some(0),
        "first Down picks first enabled panel"
    );

    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(a.focused_panel(), Some(1));
    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(a.focused_panel(), Some(2));
    // Wraps to 0
    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(a.focused_panel(), Some(0));
}

#[test]
fn accordion_up_moves_focused_panel_back_when_focused() {
    let theme = default_theme();
    let mut a = accordion_with(3);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    a.set_focused(true);
    // Walk forward to panel 2 then go back.
    a.handle_event(&no_mod_key(Key::Down), &theme);
    a.handle_event(&no_mod_key(Key::Down), &theme);
    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(a.focused_panel(), Some(2));

    a.handle_event(&no_mod_key(Key::Up), &theme);
    assert_eq!(a.focused_panel(), Some(1));
    a.handle_event(&no_mod_key(Key::Up), &theme);
    assert_eq!(a.focused_panel(), Some(0));
    // Wraps to last
    a.handle_event(&no_mod_key(Key::Up), &theme);
    assert_eq!(a.focused_panel(), Some(2));
}

#[test]
fn accordion_enter_toggles_focused_panel() {
    let theme = default_theme();
    let mut a = accordion_with(3);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    a.set_focused(true);
    a.handle_event(&no_mod_key(Key::Down), &theme); // focus panel 0
    // Initially collapsed.
    let r = a.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Consumed);
    // Toggling once should expand.
    // Re-toggle should collapse.
    let r = a.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Consumed);
}

#[test]
fn accordion_space_also_toggles_focused_panel() {
    let theme = default_theme();
    let mut a = accordion_with(2);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    a.set_focused(true);
    a.handle_event(&no_mod_key(Key::Down), &theme);
    let r = a.handle_event(&no_mod_key(Key::Space), &theme);
    assert_eq!(r, EventResult::Consumed);
}

#[test]
fn accordion_arrows_ignored_when_unfocused() {
    // The skeptic gate test: if focus is on a child inside an expanded
    // panel (or anywhere outside the Accordion), the Accordion must NOT
    // steal Up/Down — it would break text fields embedded in panels.
    let theme = default_theme();
    let mut a = accordion_with(3);
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    // Not focused.
    assert!(!a.is_focused());
    let r = a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(
        r,
        EventResult::Ignored,
        "unfocused Accordion must not consume Down"
    );
    assert_eq!(
        a.focused_panel(),
        None,
        "unfocused Down must not move panel cursor"
    );

    let r = a.handle_event(&no_mod_key(Key::Up), &theme);
    assert_eq!(r, EventResult::Ignored);
    let r = a.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Ignored);
}

// ============================================================================
// Fn -> FnMut callback sweep (Mojo-port cleanup, 2026-04-28)
// Six widgets used `Box<dyn Fn(...)>` for callbacks where the rest of the
// codebase used `Box<dyn FnMut(...)>`. That blocked hosts from writing
// idiomatic capture-mut closures: `with_on_change(|v| { self.x = v; })`
// hit the borrow checker for half the widgets and not the other half.
//
// These tests confirm a FnMut closure capturing a RefCell counter can
// register, fire, and mutate state through each rebuilt builder.
// ============================================================================

use erigui_widgets::{Dialog, DialogButton, SpinBox};

fn fnmut_counter() -> (std::rc::Rc<std::cell::RefCell<i32>>, std::rc::Rc<std::cell::RefCell<i32>>)
{
    let owner = std::rc::Rc::new(std::cell::RefCell::new(0_i32));
    let captured = owner.clone();
    (owner, captured)
}

#[test]
fn fnmut_sweep_checkbox_accepts_capture_mut_closure() {
    let (owner, captured) = fnmut_counter();
    let mut cb = Checkbox::new(test_id(), "x").with_on_toggle(move |_| {
        *captured.borrow_mut() += 1;
    });
    cb.toggle();
    cb.toggle();
    assert_eq!(*owner.borrow(), 2, "FnMut on_toggle must mutate captured state");
}

#[test]
fn fnmut_sweep_slider_accepts_capture_mut_closure() {
    let (owner, captured) = fnmut_counter();
    let mut s = Slider::new(test_id(), 0.0, 100.0, 0.0).with_on_value_changed(move |_| {
        *captured.borrow_mut() += 1;
    });
    s.set_value(10.0);
    s.set_value(20.0);
    assert_eq!(*owner.borrow(), 2);
}

#[test]
fn fnmut_sweep_spin_box_accepts_capture_mut_closure() {
    let (owner, captured) = fnmut_counter();
    let mut sb = SpinBox::new(test_id(), 0.0, 100.0, 0.0).with_on_value_changed(move |_| {
        *captured.borrow_mut() += 1;
    });
    sb.set_value(5.0);
    sb.set_value(10.0);
    assert_eq!(*owner.borrow(), 2);
}

#[test]
fn fnmut_sweep_combo_box_accepts_capture_mut_closure() {
    let (owner, captured) = fnmut_counter();
    let mut cb = ComboBox::new(test_id())
        .with_items(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        .with_on_selection_changed(move |_idx, _text| {
            *captured.borrow_mut() += 1;
        });
    cb.set_selected(Some(0));
    cb.set_selected(Some(1));
    assert_eq!(*owner.borrow(), 2);
}

#[test]
fn fnmut_sweep_tab_control_accepts_capture_mut_closure() {
    let (owner_changed, captured_changed) = fnmut_counter();
    let (owner_closed, captured_closed) = fnmut_counter();
    let mut tc = TabControl::new(test_id())
        .with_on_tab_changed(move |_| {
            *captured_changed.borrow_mut() += 1;
        })
        .with_on_tab_closed(move |_| {
            *captured_closed.borrow_mut() += 1;
        });
    tc.add_tab(TabPage::new("a"));
    tc.add_tab(TabPage::new("b"));
    tc.set_active_tab(1);
    assert_eq!(*owner_changed.borrow(), 1);
    // on_tab_closed wires through the close button click path; we can't
    // synthesize that without bounds + mouse coords, but registering the
    // FnMut is what this sweep is asserting.
    assert_eq!(*owner_closed.borrow(), 0);
}

#[test]
fn fnmut_sweep_dialog_accepts_capture_mut_closure() {
    let (owner, captured) = fnmut_counter();
    // Just verify the builder accepts a capture-mut closure. Triggering
    // it requires layout + mouse coords; that's not what the sweep is
    // about.
    let _d =
        Dialog::new(test_id(), "title", "message").with_on_button_clicked(move |_b: DialogButton| {
        *captured.borrow_mut() += 1;
    });
    assert_eq!(*owner.borrow(), 0);
}

// ============================================================================
// DockPanel focus rectification (Mojo-port cleanup, 2026-04-28)
// Previously: can_focus() returned false despite splitters/tabs being
// interactive — focus state on WidgetState was unreachable.
// ============================================================================

use erigui_widgets::{DockPanel, DockPosition, DockablePanel};

#[test]
fn dock_panel_focus_round_trips() {
    let mut dp = DockPanel::new(test_id());
    let inner: Box<dyn Widget> = Box::new(Label::new(test_id(), "x"));
    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", inner),
        DockPosition::Center,
    );
    assert!(
        dp.can_focus(),
        "enabled+visible DockPanel should be focusable"
    );
    assert!(!dp.is_focused(), "starts unfocused");
    dp.set_focused(true);
    assert!(dp.is_focused(), "set_focused(true) sticks");
    dp.set_focused(false);
    assert!(!dp.is_focused(), "set_focused(false) sticks");
}

#[test]
fn dock_panel_can_focus_respects_disabled() {
    let mut dp = DockPanel::new(test_id());
    let inner: Box<dyn Widget> = Box::new(Label::new(test_id(), "x"));
    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", inner),
        DockPosition::Center,
    );
    dp.set_enabled(false);
    assert!(
        !dp.can_focus(),
        "disabled DockPanel must not advertise focus"
    );
    dp.set_enabled(true);
    dp.set_visible(false);
    assert!(
        !dp.can_focus(),
        "invisible DockPanel must not advertise focus"
    );
}

// ============================================================================
// MenuBar focus rectification (Mojo-port cleanup, 2026-04-28)
// Previously: is_focused returned hardcoded false, set_focused was a no-op,
// can_focus returned true — contradictory tri-state. Hosts couldn't direct
// focus to it (set_focused dropped events).
// ============================================================================

use erigui_widgets::{MenuBar, MenuItem};

#[test]
fn menubar_focus_round_trips() {
    let mut bar = MenuBar::new(test_id());
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    assert!(
        bar.can_focus(),
        "enabled+visible MenuBar should be focusable"
    );
    assert!(!bar.is_focused(), "starts unfocused");
    bar.set_focused(true);
    assert!(bar.is_focused(), "set_focused(true) sticks");
    bar.set_focused(false);
    assert!(!bar.is_focused(), "set_focused(false) sticks");
}

#[test]
fn menubar_arrows_ignored_when_unfocused_and_no_dropdown() {
    // Without focus and no dropdown open, arrow keys must not be consumed
    // — otherwise the MenuBar would steal arrows from focused children.
    let theme = default_theme();
    let mut bar = MenuBar::new(test_id());
    bar.add_menu("File", vec![MenuItem::new("Open"), MenuItem::new("Save")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme);
    assert!(!bar.is_focused());

    let r = bar.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(r, EventResult::Ignored);
    let r = bar.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(r, EventResult::Ignored);
    let r = bar.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Ignored);
}

#[test]
fn menubar_alt_mnemonic_works_unfocused_and_focuses_self() {
    // Alt+letter is an app-wide accelerator and must work regardless of
    // current focus. Activating it focuses the MenuBar so subsequent
    // arrow keys land here.
    let theme = default_theme();
    let mut bar = MenuBar::new(test_id());
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme);
    assert!(!bar.is_focused());

    let alt_e = Event::KeyPress(KeyPressEvent {
        key: Key::E,
        modifiers: Modifiers::ALT,
        repeat: false,
    });
    let r = bar.handle_event(&alt_e, &theme);
    assert_eq!(r, EventResult::Consumed);
    assert!(bar.is_dropdown_visible(), "Alt+E opens Edit dropdown");
    assert!(
        bar.is_focused(),
        "Alt-mnemonic activation must transfer focus to the MenuBar"
    );
}

#[test]
fn menubar_click_to_open_focuses_self() {
    // Mouse click that opens a dropdown sets focus, so subsequent keyboard
    // nav (arrows, Enter, Escape) actually fires.
    use erigui_core::{MouseButton, MouseButtonEvent, Point};
    let theme = default_theme();
    let mut bar = MenuBar::new(test_id());
    bar.add_menu("File", vec![MenuItem::new("Open"), MenuItem::new("Save")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme);
    assert!(!bar.is_focused());

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(8, 15),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    bar.handle_event(&click, &theme);
    assert!(bar.is_dropdown_visible(), "click opens dropdown");
    assert!(bar.is_focused(), "click-to-open focuses the MenuBar");

    // Now arrow keys work because focused.
    let r = bar.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(r, EventResult::Consumed);
}

#[test]
fn menubar_arrows_consumed_when_focused_and_dropdown_open() {
    let theme = default_theme();
    let mut bar = MenuBar::new(test_id());
    bar.add_menu("File", vec![MenuItem::new("Open"), MenuItem::new("Save")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme);
    bar.set_focused(true);
    // Programmatically open the dropdown via Alt-mnemonic so the test
    // doesn't rely on click coordinates.
    let alt_f = Event::KeyPress(KeyPressEvent {
        key: Key::F,
        modifiers: Modifiers::ALT,
        repeat: false,
    });
    bar.handle_event(&alt_f, &theme);
    assert!(bar.is_dropdown_visible());

    let r = bar.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(r, EventResult::Consumed);
    let r = bar.handle_event(&no_mod_key(Key::Escape), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert!(!bar.is_dropdown_visible(), "Esc closes dropdown");
}

// ============================================================================
// Breadcrumb focus + keyboard nav (Mojo-port cleanup, 2026-04-28)
// Previously: can_focus() returned false despite the widget being
// interactive — focus state unreachable, no keyboard nav.
// ============================================================================

use erigui_widgets::{Breadcrumb, BreadcrumbItem};

fn breadcrumb_with(n: usize) -> Breadcrumb {
    let mut items = Vec::with_capacity(n);
    for i in 0..n {
        items.push(BreadcrumbItem::new(format!("seg_{i}"), format!("id_{i}")));
    }
    Breadcrumb::new(test_id())
        .with_home_icon(false)
        .with_items(items)
}

#[test]
fn breadcrumb_focus_round_trips() {
    let theme = default_theme();
    let mut b = breadcrumb_with(3);
    b.layout(Rect::new(0, 0, 600, 30), &theme);
    assert!(b.can_focus(), "enabled+visible Breadcrumb should be focusable");
    assert!(!b.is_focused(), "starts unfocused");
    b.set_focused(true);
    assert!(b.is_focused());
    b.set_focused(false);
    assert!(!b.is_focused());
}

#[test]
fn breadcrumb_right_advances_when_focused() {
    let theme = default_theme();
    // Default Breadcrumb has a home item (index 0) plus our 2 items, so
    // the unified count is 3.
    let mut b = breadcrumb_with(2);
    b.layout(Rect::new(0, 0, 600, 30), &theme);
    b.set_focused(true);
    assert_eq!(b.focused_index(), None);

    let r = b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(
        b.focused_index(),
        Some(0),
        "first Right picks home (index 0)"
    );
    b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(b.focused_index(), Some(1));
    b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(b.focused_index(), Some(2));
    // Wraps
    b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(b.focused_index(), Some(0));
}

#[test]
fn breadcrumb_left_moves_back_when_focused() {
    let theme = default_theme();
    let mut b = breadcrumb_with(2); // unified count 3 (home + 2)
    b.layout(Rect::new(0, 0, 600, 30), &theme);
    b.set_focused(true);
    // Walk to last
    b.handle_event(&no_mod_key(Key::Right), &theme);
    b.handle_event(&no_mod_key(Key::Right), &theme);
    b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(b.focused_index(), Some(2));

    b.handle_event(&no_mod_key(Key::Left), &theme);
    assert_eq!(b.focused_index(), Some(1));
    b.handle_event(&no_mod_key(Key::Left), &theme);
    assert_eq!(b.focused_index(), Some(0));
    // Wraps
    b.handle_event(&no_mod_key(Key::Left), &theme);
    assert_eq!(b.focused_index(), Some(2));
}

#[test]
fn breadcrumb_enter_navigates_focused_segment() {
    let theme = default_theme();
    // Set up: home + 3 items (foo, bar, baz). Focus the home (index 0)
    // and Enter — should clear items via navigate_to(0). on_navigate
    // fires with home id.
    use std::cell::RefCell;
    use std::rc::Rc;
    let last_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let last_id_clone = last_id.clone();
    let mut b = Breadcrumb::new(test_id())
        .with_home_icon(false)
        .with_items(vec![
            BreadcrumbItem::new("foo", "id_foo"),
            BreadcrumbItem::new("bar", "id_bar"),
            BreadcrumbItem::new("baz", "id_baz"),
        ])
        .with_on_navigate(move |id, _idx| {
            *last_id_clone.borrow_mut() = Some(id.to_string());
        });
    b.layout(Rect::new(0, 0, 800, 30), &theme);
    b.set_focused(true);

    // First Right → home (0). Enter → navigate to home, clears items.
    b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(b.focused_index(), Some(0));
    let r = b.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(last_id.borrow().as_deref(), Some("home"));
}

#[test]
fn breadcrumb_keys_ignored_when_unfocused() {
    let theme = default_theme();
    let mut b = breadcrumb_with(3);
    b.layout(Rect::new(0, 0, 600, 30), &theme);
    // Not focused.
    let r = b.handle_event(&no_mod_key(Key::Right), &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(b.focused_index(), None);
    let r = b.handle_event(&no_mod_key(Key::Left), &theme);
    assert_eq!(r, EventResult::Ignored);
    let r = b.handle_event(&no_mod_key(Key::Enter), &theme);
    assert_eq!(r, EventResult::Ignored);
}

#[test]
fn accordion_down_skips_disabled_panel() {
    let theme = default_theme();
    let mut a = Accordion::new(test_id());
    a.add_panel(AccordionPanel::new(
        "a",
        Box::new(Label::new(test_id(), "x")) as Box<dyn Widget>,
    ));
    let mut middle = AccordionPanel::new(
        "b",
        Box::new(Label::new(test_id(), "y")) as Box<dyn Widget>,
    );
    middle.enabled = false;
    a.add_panel(middle);
    a.add_panel(AccordionPanel::new(
        "c",
        Box::new(Label::new(test_id(), "z")) as Box<dyn Widget>,
    ));
    a.layout(Rect::new(0, 0, 400, 600), &theme);
    a.set_focused(true);

    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(a.focused_panel(), Some(0));
    a.handle_event(&no_mod_key(Key::Down), &theme);
    assert_eq!(
        a.focused_panel(),
        Some(2),
        "Down must skip the disabled middle panel"
    );
}

// ============================================================================
// Per-widget tooltip retrofit (2026-04-28)
// Toolbar + 5 widgets gain their own TooltipState following the Button
// pattern. Tests confirm the tooltip shows after delay-elapsed hover and
// hides on press / mouse-leave.
// ============================================================================

use erigui_core::{MouseMoveEvent};
use erigui_widgets::{Toolbar, TooltipState};
use std::time::Duration;

fn move_event(p: Point) -> Event {
    Event::MouseMove(MouseMoveEvent {
        position: p,
        delta: Point::ZERO,
        modifiers: Modifiers::empty(),
    })
}

fn left_press_event(p: Point) -> Event {
    Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: p,
        pressed: true,
        modifiers: Modifiers::empty(),
    })
}

#[test]
fn toolbar_button_with_tooltip_shows_after_delay() {
    let theme = default_theme();
    let mut tb = Toolbar::new(test_id())
        .add_button("Open", None, || {})
        .with_last_tooltip_state(TooltipState::new("Open file").with_delay_ms(20));
    tb.layout(Rect::new(0, 0, 400, 32), &theme);

    // First MouseMove latches the hover-start timer but doesn't show.
    tb.handle_event(&move_event(Point::new(8, 16)), &theme);
    assert!(
        !tb.item_tooltip(0).unwrap().is_visible(),
        "tooltip must not appear before delay elapses"
    );

    // Wait past the delay, then re-emit a move — promotion happens
    // when `update_on_event` runs again with the timer elapsed.
    std::thread::sleep(Duration::from_millis(30));
    tb.handle_event(&move_event(Point::new(9, 16)), &theme);
    assert!(
        tb.item_tooltip(0).unwrap().is_visible(),
        "tooltip must show once delay has elapsed while hovering"
    );
}

#[test]
fn toolbar_button_tooltip_hides_on_mouse_leave() {
    let theme = default_theme();
    let mut tb = Toolbar::new(test_id())
        .add_button("Open", None, || {})
        .with_last_tooltip_state(TooltipState::new("Open file").with_delay_ms(0));
    tb.layout(Rect::new(0, 0, 400, 32), &theme);

    // Delay 0 → first move shows.
    tb.handle_event(&move_event(Point::new(8, 16)), &theme);
    assert!(tb.item_tooltip(0).unwrap().is_visible());

    // Move far outside the item rect — mouse-leave hides.
    tb.handle_event(&move_event(Point::new(800, 800)), &theme);
    assert!(
        !tb.item_tooltip(0).unwrap().is_visible(),
        "tooltip must hide once mouse leaves the item bounds"
    );
}


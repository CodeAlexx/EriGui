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

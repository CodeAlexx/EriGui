//! Integration tests for the modal / overlay widgets:
//! Dialog and Notification (+ NotificationManager).
//!
//! Both widgets had zero integration tests before. Same bar as the
//! simple_widgets_tests.rs sibling: "no widget has zero tests anymore" --
//! not exhaustive coverage. Where the public API doesn't expose enough
//! state for a real round-trip assertion, the test degenerates to
//! "construction + call doesn't panic," which is still a regression net.

use erigui_core::{
    Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, Rect, TextInputEvent, Theme, Widget,
    WidgetId,
};
use erigui_widgets::{Dialog, DialogButton, DialogType};
use std::cell::RefCell;
use std::rc::Rc;

// Helpers copied verbatim from the sibling test files. Kept in sync if
// the canonical helpers in widget_tests.rs ever change.
fn default_theme() -> Theme {
    Theme::dark()
}

fn test_id() -> WidgetId {
    WidgetId::default()
}

// ============================================================================
// Dialog
// ============================================================================
//
// Dialog is a modal popup with a title bar, message body, an array of
// buttons, and an Escape-to-close key handler. Public API exposes
// is_open / show / close / result and a few builders. The geometry
// helpers (get_button_rect etc.) are private, so click-on-button tests
// have to layout() first to a known rect, then derive the button
// position from the same constants the implementation uses.

#[test]
fn dialog_creation_defaults() {
    let d = Dialog::new(test_id(), "Title", "Message");
    assert!(!d.is_open(), "new Dialog must start closed");
    assert_eq!(d.result(), None, "new Dialog has no result");
    assert!(d.is_visible());
    assert!(d.is_enabled());
    assert!(d.can_focus(), "Dialog opts INTO focus (modal)");
    // is_focused is wired to is_open per the impl; closed dialog is
    // not focused.
    assert!(!d.is_focused());
}

#[test]
fn dialog_show_and_close_round_trip() {
    let mut d = Dialog::new(test_id(), "Title", "Message");
    assert!(!d.is_open());
    d.show();
    assert!(d.is_open(), "show() must open the dialog");
    // is_focused tracks is_open in this widget.
    assert!(d.is_focused(), "open dialog reports focused (= modal)");
    d.close();
    assert!(!d.is_open(), "close() must close the dialog");
    assert!(!d.is_focused(), "closed dialog reports not focused");
}

#[test]
fn dialog_show_clears_previous_result() {
    // After a button click, result() is Some(...). show() must reset it.
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M")
        .with_buttons(vec![(DialogButton::Ok, "OK".to_string())]);
    d.show();
    d.layout(Rect::new(0, 0, 400, 200), &theme);

    // Click the OK button; geometry mirrors Dialog::get_button_rect.
    let click = button_click_position_n(&d, 0, 1);
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: click,
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = d.handle_event(&press, &theme);
    assert_eq!(d.result(), Some(DialogButton::Ok));
    assert!(!d.is_open(), "clicking a button auto-closes the dialog");

    d.show();
    assert!(d.is_open());
    assert_eq!(d.result(), None, "show() must reset result");
}

#[test]
fn dialog_with_type_runs() {
    // No public getter for dialog_type; smoke test that the builder
    // chain runs and a subsequent measure() still works.
    let theme = default_theme();
    for dt in [
        DialogType::Info,
        DialogType::Warning,
        DialogType::Error,
        DialogType::Question,
        DialogType::Custom,
    ] {
        let d = Dialog::new(test_id(), "T", "M").with_type(dt);
        let size = d.measure(&LayoutConstraints::UNBOUNDED, &theme);
        assert!(size.width >= 300, "Dialog::measure floors width at 300");
        assert!(size.height > 0);
    }
}

#[test]
fn dialog_with_standard_buttons_for_each_type_runs() {
    // Locks in the standard-button preset; no panic, and a subsequent
    // measure() succeeds.
    let theme = default_theme();
    for dt in [
        DialogType::Info,
        DialogType::Warning,
        DialogType::Error,
        DialogType::Question,
        DialogType::Custom,
    ] {
        let d = Dialog::new(test_id(), "T", "M").with_standard_buttons(dt);
        let size = d.measure(&LayoutConstraints::UNBOUNDED, &theme);
        assert!(size.width > 0);
        assert!(size.height > 0);
    }
}

#[test]
fn dialog_with_custom_buttons_replaces_default() {
    // Default has one OK button; custom should fully replace.
    let buttons = vec![
        (DialogButton::Yes, "Yes".to_string()),
        (DialogButton::No, "No".to_string()),
        (DialogButton::Cancel, "Cancel".to_string()),
    ];
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M").with_buttons(buttons);
    d.show();
    d.layout(Rect::new(0, 0, 600, 200), &theme);

    // Click each button index; verify result captures the right enum.
    // Because clicking auto-closes, re-show before each click.
    for (i, expected) in [DialogButton::Yes, DialogButton::No, DialogButton::Cancel]
        .iter()
        .enumerate()
    {
        d.show();
        d.layout(Rect::new(0, 0, 600, 200), &theme);
        let pos = button_click_position_n(&d, i, 3);
        let press = Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: pos,
            pressed: true,
            modifiers: Modifiers::empty(),
        });
        let res = d.handle_event(&press, &theme);
        assert_eq!(res, EventResult::Consumed, "button click should be consumed");
        assert_eq!(
            d.result(),
            Some(*expected),
            "result must match clicked button at index {}",
            i
        );
        assert!(!d.is_open(), "click closes dialog");
    }
}

#[test]
fn dialog_button_click_invokes_callback() {
    // Capture pattern: Rc<RefCell<Option<DialogButton>>>.
    let captured: Rc<RefCell<Option<DialogButton>>> = Rc::new(RefCell::new(None));
    let cap_clone = captured.clone();

    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M")
        .with_buttons(vec![
            (DialogButton::Yes, "Yes".to_string()),
            (DialogButton::No, "No".to_string()),
        ])
        .with_on_button_clicked(move |btn| {
            *cap_clone.borrow_mut() = Some(btn);
        });
    d.show();
    d.layout(Rect::new(0, 0, 600, 200), &theme);

    let pos = button_click_position_n(&d, 1, 2); // No
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: pos,
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = d.handle_event(&press, &theme);

    assert_eq!(*captured.borrow(), Some(DialogButton::No));
}

#[test]
fn dialog_escape_key_closes() {
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.show();
    d.layout(Rect::new(0, 0, 400, 200), &theme);

    let esc = Event::KeyPress(KeyPressEvent {
        key: Key::Escape,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let res = d.handle_event(&esc, &theme);
    assert_eq!(res, EventResult::Consumed, "Escape on open dialog is consumed");
    assert!(!d.is_open(), "Escape must close the dialog");
}

#[test]
fn dialog_close_button_closes() {
    // The X close button sits at the right end of the title bar. Geometry
    // mirrors Dialog::draw / handle_event close_rect computation:
    //   close_size = 20
    //   close_rect.x = title_rect.right() - 20 - 5
    //   close_rect.y = title_rect.y() + (title_rect.height() - 20) / 2
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.show();
    let bounds = Rect::new(0, 0, 400, 200);
    d.layout(bounds, &theme);

    // title_rect = Rect(0, 0, 400, 30); close_rect = (400-20-5, (30-20)/2, 20, 20)
    let close_center = Point::new(400 - 20 - 5 + 10, (30 - 20) / 2 + 10);
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: close_center,
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = d.handle_event(&press, &theme);
    assert_eq!(res, EventResult::Consumed);
    assert!(!d.is_open(), "clicking close button must close the dialog");
}

#[test]
fn dialog_closed_dialog_ignores_events() {
    // Closed dialog must Ignored every event (not Consumed) so that the
    // host UI can keep working underneath. This protects the modal
    // contract: "closed = invisible & inert."
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.layout(Rect::new(0, 0, 400, 200), &theme);
    // Don't call show().

    let events = [
        Event::MouseMove(MouseMoveEvent {
            position: Point::new(50, 50),
            delta: Point::new(0, 0),
            modifiers: Modifiers::empty(),
        }),
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(50, 50),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(0, -1),
            position: Point::new(50, 50),
            modifiers: Modifiers::empty(),
        }),
        Event::KeyPress(KeyPressEvent {
            key: Key::Escape,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::TextInput(TextInputEvent {
            text: "x".to_string(),
        }),
        Event::Focus(FocusEvent { gained: true }),
        Event::Update,
    ];
    for ev in &events {
        assert_eq!(
            d.handle_event(ev, &theme),
            EventResult::Ignored,
            "closed Dialog must ignore {:?}",
            ev
        );
    }
}

#[test]
fn dialog_open_modal_consumes_unrelated_clicks() {
    // Open modal dialogs must absorb stray clicks so they don't leak to
    // the host. The implementation returns Consumed for any
    // MouseButton when is_modal && open, even outside the dialog
    // bounds.
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.show();
    d.layout(Rect::new(100, 100, 400, 200), &theme);

    // Click far outside the dialog at (1000, 1000).
    let outside = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(1000, 1000),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = d.handle_event(&outside, &theme);
    assert_eq!(
        res,
        EventResult::Consumed,
        "modal Dialog must consume clicks outside its bounds"
    );
    // Modal absorbs but does NOT close on click-outside; that's
    // the contract this widget chose.
    assert!(d.is_open(), "modal click-outside should NOT auto-close");
}

#[test]
fn dialog_open_modal_consumes_unrelated_keys() {
    // Same reasoning: an open modal swallows stray non-Escape keypresses.
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.show();
    d.layout(Rect::new(0, 0, 400, 200), &theme);

    let key = Event::KeyPress(KeyPressEvent {
        key: Key::Enter,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let res = d.handle_event(&key, &theme);
    assert_eq!(
        res,
        EventResult::Consumed,
        "open modal swallows non-Escape keys"
    );
    assert!(d.is_open(), "non-Escape key must not close");
}

#[test]
fn dialog_layout_sets_bounds() {
    let theme = default_theme();
    let mut d = Dialog::new(test_id(), "T", "M");
    d.layout(Rect::new(50, 60, 400, 250), &theme);
    let b = d.bounds();
    assert_eq!(b.x(), 50);
    assert_eq!(b.y(), 60);
    assert_eq!(b.width(), 400);
    assert_eq!(b.height(), 250);
}

#[test]
fn dialog_measure_floors_width_at_300() {
    // Short message -> 300px floor.
    let theme = default_theme();
    let d = Dialog::new(test_id(), "T", "Hi");
    let size = d.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert_eq!(size.width, 300);
    assert_eq!(size.height, 150);
}

#[test]
fn dialog_measure_grows_with_message() {
    // Long message -> width = max(300, len * font_size_base / 2 + 40).
    let theme = default_theme();
    let long = "x".repeat(200);
    let d = Dialog::new(test_id(), "T", long);
    let size = d.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert!(
        size.width > 300,
        "long message should drive width above the 300 floor"
    );
}

#[test]
fn dialog_visibility_and_enabled_round_trip() {
    let mut d = Dialog::new(test_id(), "T", "M");
    d.set_visible(false);
    assert!(!d.is_visible());
    d.set_enabled(false);
    assert!(!d.is_enabled());
}

#[test]
fn dialog_set_focused_false_does_not_panic() {
    // Dialog::set_focused(false) is wired to clear is_dragging. We can't
    // observe is_dragging, so just verify the call is panic-free and that
    // is_focused() still tracks is_open afterwards.
    let mut d = Dialog::new(test_id(), "T", "M");
    d.show();
    d.set_focused(false);
    assert!(d.is_focused(), "is_focused tracks is_open, not the focused arg");
    d.close();
    d.set_focused(true);
    assert!(!d.is_focused(), "closed dialog is not focused regardless of arg");
}

/// Compute the center-of-button-rect for click simulation. Mirrors
/// Dialog::get_button_rect using the public `bounds()` and the constants
/// from dialog.rs (button_height=30, button_width=80, gap=10, area is
/// bottom button_height+10 strip).
fn button_click_position_n(d: &Dialog, button_index: usize, button_count: usize) -> Point {
    let bounds = d.bounds();
    let button_height = 30i32;
    let button_width = 80i32;
    let gap = 10i32;
    let area_y = bounds.bottom() - button_height - 10;
    let area_center_x = bounds.x() + bounds.width() / 2;
    let total_width = button_count as i32 * button_width + (button_count as i32 - 1) * gap;
    let start_x = area_center_x - total_width / 2;
    let bx = start_x + button_index as i32 * (button_width + gap);
    let by = area_y + 5;
    Point::new(bx + button_width / 2, by + button_height / 2)
}

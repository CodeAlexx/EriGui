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
use erigui_widgets::{
    Dialog, DialogButton, DialogType, Notification, NotificationManager, NotificationPosition,
    NotificationType,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

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

// ============================================================================
// Notification (single)
// ============================================================================
//
// Notification is a plain data struct with public fields plus builders.
// All state is observable, so tests can assert real round-trips.

#[test]
fn notification_creation_defaults() {
    let n = Notification::new("id-1", "Title", "Body");
    assert_eq!(n.id, "id-1");
    assert_eq!(n.title, "Title");
    assert_eq!(n.message, "Body");
    assert_eq!(n.notification_type, NotificationType::Info);
    assert_eq!(n.icon, None);
    assert_eq!(n.duration, Duration::from_secs(5));
    assert!(n.can_close, "default notifications are closable");
    assert_eq!(n.action_text, None);
    assert_eq!(n.progress, None);
}

#[test]
fn notification_with_each_type_round_trips() {
    for nt in [
        NotificationType::Info,
        NotificationType::Success,
        NotificationType::Warning,
        NotificationType::Error,
    ] {
        let n = Notification::new("x", "T", "M").with_type(nt);
        assert_eq!(n.notification_type, nt);
    }
}

#[test]
fn notification_with_icon_round_trips() {
    let n = Notification::new("x", "T", "M").with_icon("warning");
    assert_eq!(n.icon.as_deref(), Some("warning"));
}

#[test]
fn notification_with_duration_round_trips() {
    // No real sleeping -- just verify the field round-trips. Auto-dismiss
    // exercise is in NotificationManager tests via update_animations.
    let n =
        Notification::new("x", "T", "M").with_duration(Duration::from_millis(2500));
    assert_eq!(n.duration, Duration::from_millis(2500));
}

#[test]
fn notification_with_can_close_round_trips() {
    let n = Notification::new("x", "T", "M").with_can_close(false);
    assert!(!n.can_close);
    let n = Notification::new("x", "T", "M").with_can_close(true);
    assert!(n.can_close);
}

#[test]
fn notification_with_action_round_trips() {
    let n = Notification::new("x", "T", "M").with_action("Undo");
    assert_eq!(n.action_text.as_deref(), Some("Undo"));
}

#[test]
fn notification_with_progress_clamps() {
    let n = Notification::new("x", "T", "M").with_progress(0.5);
    assert_eq!(n.progress, Some(0.5));
    let n = Notification::new("x", "T", "M").with_progress(-1.0);
    assert_eq!(n.progress, Some(0.0), "progress floor is 0.0");
    let n = Notification::new("x", "T", "M").with_progress(2.5);
    assert_eq!(n.progress, Some(1.0), "progress ceiling is 1.0");
}

#[test]
fn notification_remaining_ratio_is_in_range() {
    // Just-created notification with 5s default duration has
    // remaining_ratio in (0, 1]. We don't sleep -- we only verify the
    // bounds, which is all we can portably observe.
    let n = Notification::new("x", "T", "M");
    let r = n.remaining_ratio();
    assert!(r > 0.0 && r <= 1.0, "remaining_ratio out of range: {}", r);
    // is_expired must be false on a fresh long-duration notification.
    assert!(!n.is_expired(), "fresh notification with 5s duration is not expired");
}

#[test]
fn notification_zero_duration_is_immediately_expired() {
    // Edge: duration = 0 makes elapsed >= duration on the very next
    // observation. remaining_ratio() then returns 0.0 by the early-return
    // branch.
    let n = Notification::new("x", "T", "M").with_duration(Duration::from_secs(0));
    // Tiny sleep is undesirable in unit tests; the elapsed >= 0 condition
    // is satisfied because Instant::elapsed is monotone-non-decreasing
    // and the duration is exactly 0, so the >= check holds.
    assert!(n.is_expired(), "duration=0 should report expired");
    assert_eq!(n.remaining_ratio(), 0.0);
}

// ============================================================================
// NotificationManager
// ============================================================================
//
// Manager owns a queue of notifications, with positioning, max-visible
// cap, animation, and click handling for close + action buttons.
// Public API exposes show / clear / remove and builders, but the queue
// itself is private. Tests focus on what's observable: callbacks fired,
// widget-trait surface, layout doesn't panic, click behavior on synthetic
// events.

#[test]
fn manager_creation_defaults() {
    let m = NotificationManager::new(test_id());
    assert!(m.is_visible());
    assert!(m.is_enabled());
    assert!(!m.can_focus(), "NotificationManager opts out of focus");
    assert!(!m.is_focused());
}

#[test]
fn manager_with_position_runs() {
    // No public position getter; smoke test that the builder runs and
    // a subsequent layout/measure still works for each variant.
    let theme = default_theme();
    for pos in [
        NotificationPosition::TopLeft,
        NotificationPosition::TopCenter,
        NotificationPosition::TopRight,
        NotificationPosition::BottomLeft,
        NotificationPosition::BottomCenter,
        NotificationPosition::BottomRight,
    ] {
        let mut m = NotificationManager::new(test_id()).with_position(pos);
        m.layout(Rect::new(0, 0, 800, 600), &theme);
        let _ = m.measure(&LayoutConstraints::bounded(800, 600), &theme);
    }
}

#[test]
fn manager_with_max_visible_runs() {
    let m = NotificationManager::new(test_id()).with_max_visible(3);
    // No getter; smoke test the builder + show() up to & past the cap.
    let mut m = m;
    let theme = default_theme();
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    for i in 0..6 {
        m.show(Notification::new(format!("id-{}", i), "T", "M"));
    }
    // At most 3 should remain queued; we can't assert the count
    // directly, but the close-callback test below proves the eviction.
}

#[test]
fn manager_with_size_runs() {
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id()).with_size(400, 100);
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));
    let _ = m.measure(&LayoutConstraints::bounded(800, 600), &theme);
}

#[test]
fn manager_show_does_not_panic() {
    // Smoke test that show() works through all 6 positions. update_layout
    // has 6 different geometry branches -- exercise them all.
    let theme = default_theme();
    for pos in [
        NotificationPosition::TopLeft,
        NotificationPosition::TopCenter,
        NotificationPosition::TopRight,
        NotificationPosition::BottomLeft,
        NotificationPosition::BottomCenter,
        NotificationPosition::BottomRight,
    ] {
        let mut m = NotificationManager::new(test_id()).with_position(pos);
        m.layout(Rect::new(0, 0, 800, 600), &theme);
        m.show(Notification::new("a", "Hello", "World"));
        m.show(Notification::new("b", "Hi", "There").with_can_close(false));
        m.show(Notification::new("c", "X", "Y").with_action("Undo"));
        m.layout(Rect::new(0, 0, 800, 600), &theme);
    }
}

#[test]
fn manager_max_visible_evicts_oldest_with_callback() {
    // When the queue exceeds max_visible, show() evicts the oldest and
    // fires on_notification_closed for it. That's the only externally
    // visible signal that eviction happened.
    let evicted: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = evicted.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_max_visible(2)
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(Notification::new("a", "T", "M"));
    m.show(Notification::new("b", "T", "M"));
    assert!(evicted.borrow().is_empty(), "no eviction at exactly max_visible");
    m.show(Notification::new("c", "T", "M"));
    assert_eq!(evicted.borrow().as_slice(), &["a".to_string()]);
    m.show(Notification::new("d", "T", "M"));
    assert_eq!(evicted.borrow().as_slice(), &["a".to_string(), "b".to_string()]);
}

#[test]
fn manager_remove_by_id_fires_close_callback() {
    let closed: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = closed.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(Notification::new("a", "T", "M"));
    m.show(Notification::new("b", "T", "M"));

    m.remove("a");
    assert_eq!(closed.borrow().as_slice(), &["a".to_string()]);

    // Removing an absent id is a silent no-op.
    m.remove("does-not-exist");
    assert_eq!(
        closed.borrow().as_slice(),
        &["a".to_string()],
        "remove of absent id must not fire callback"
    );
}

#[test]
fn manager_clear_does_not_panic_and_does_not_fire_close_callback() {
    // clear() truncates the VecDeque without going through the per-item
    // removal path, so the on_notification_closed callback is NOT
    // fired. Locking that in here keeps a future refactor honest --
    // if someone changes clear() to fire callbacks, they'll have to
    // intentionally update this test.
    let closed: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = closed.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(Notification::new("a", "T", "M"));
    m.show(Notification::new("b", "T", "M"));
    m.clear();
    assert!(
        closed.borrow().is_empty(),
        "clear() does NOT fire on_notification_closed (current contract)"
    );
    // Calling clear twice is idempotent.
    m.clear();
}

#[test]
fn manager_close_button_click_removes_and_fires_callback() {
    let closed: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = closed.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_position(NotificationPosition::TopLeft)
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(Notification::new("a", "T", "M")); // closable=true by default
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    // Compute close-button center for TopLeft layout. Geometry mirrors
    // NotificationManager::update_layout (close_size=20, inset=8 from
    // right, 8 from top of bounds; bounds.x = margin (20), bounds.y =
    // margin (20) for first item).
    //
    // First item bounds (TopLeft, animation_offset starts at width=350,
    // so the slide-in pushes it offscreen. update_animations is needed
    // to settle it. We call update_animations with a large delta to
    // zero the offset.
    m.update_animations(10.0);
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    // Now item bounds = Rect(20, 20, 350, 80); close_rect = Rect(20+350-20-8, 20+8, 20, 20)
    //                = Rect(342, 28, 20, 20); center = (352, 38).
    let close_press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(352, 38),
        pressed: false, // The handler reacts on release, not press.
        modifiers: Modifiers::empty(),
    });
    let res = m.handle_event(&close_press, &theme);
    assert_eq!(res, EventResult::Consumed, "close-button click should be consumed");
    assert_eq!(closed.borrow().as_slice(), &["a".to_string()]);
}

#[test]
fn manager_action_button_click_fires_action_callback() {
    let actions: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = actions.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_position(NotificationPosition::TopLeft)
        .with_on_action_clicked(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(
        Notification::new("a", "T", "M")
            .with_action("Undo")
            .with_can_close(false),
    );
    m.update_animations(10.0); // settle slide-in
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    // action_rect: width=80, height=24, inset 8 from right and bottom.
    // Item bounds = Rect(20, 20, 350, 80). action_rect = Rect(20+350-80-8, 20+80-24-8, 80, 24)
    //                                                  = Rect(282, 68, 80, 24); center = (322, 80).
    let action_release = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(322, 80),
        pressed: false,
        modifiers: Modifiers::empty(),
    });
    let res = m.handle_event(&action_release, &theme);
    assert_eq!(res, EventResult::Consumed, "action click should be consumed");
    assert_eq!(actions.borrow().as_slice(), &["a".to_string()]);
}

#[test]
fn manager_mouse_move_over_close_button_consumes_when_hover_changes() {
    // The hover state is private but observable indirectly: when hover
    // toggles, MouseMove returns Consumed (forces redraw). When hover
    // doesn't change, it returns Ignored.
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id()).with_position(NotificationPosition::TopLeft);
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));
    m.update_animations(10.0);
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    let move_onto = Event::MouseMove(MouseMoveEvent {
        position: Point::new(352, 38), // center of close button
        delta: Point::new(0, 0),
        modifiers: Modifiers::empty(),
    });
    let r1 = m.handle_event(&move_onto, &theme);
    assert_eq!(r1, EventResult::Consumed, "hover-on transition fires Consumed");

    // Same position again -> hover unchanged -> Ignored.
    let r2 = m.handle_event(&move_onto, &theme);
    assert_eq!(r2, EventResult::Ignored, "hover stable means Ignored");

    // Move away -> hover-off transition -> Consumed.
    let move_off = Event::MouseMove(MouseMoveEvent {
        position: Point::new(0, 0),
        delta: Point::new(0, 0),
        modifiers: Modifiers::empty(),
    });
    let r3 = m.handle_event(&move_off, &theme);
    assert_eq!(r3, EventResult::Consumed, "hover-off transition fires Consumed");
}

#[test]
fn manager_disabled_or_invisible_ignores_events() {
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(0, 0),
        pressed: false,
        modifiers: Modifiers::empty(),
    });

    m.set_visible(false);
    assert_eq!(m.handle_event(&click, &theme), EventResult::Ignored);
    m.set_visible(true);

    m.set_enabled(false);
    assert_eq!(m.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn manager_update_animations_removes_expired_with_callback() {
    // Set duration=0 so notifications are immediately expired and the
    // expiry-removal branch in update_animations fires. This avoids
    // any std::thread::sleep dependency.
    let closed: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = closed.clone();

    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    m.show(
        Notification::new("expire-me", "T", "M").with_duration(Duration::from_secs(0)),
    );
    m.show(
        Notification::new("also-expire", "T", "M").with_duration(Duration::from_secs(0)),
    );
    m.update_animations(0.016);

    let v = closed.borrow();
    assert!(v.contains(&"expire-me".to_string()));
    assert!(v.contains(&"also-expire".to_string()));
}

#[test]
fn manager_update_animations_advances_slide_in() {
    // animation_offset starts at notification_width and decreases
    // toward 0 with each update_animations call. We can't read the
    // offset directly, but we can show that after enough updates,
    // the close-button click at the *settled* position succeeds --
    // i.e. the geometry stops drifting.
    //
    // This is a smoke test: just verify update_animations doesn't
    // panic with various delta values, including 0 and large.
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));

    m.update_animations(0.0);
    m.update_animations(0.001);
    m.update_animations(1.0);
    m.update_animations(60.0); // many seconds of "delta" -- offset should hit 0
}

#[test]
fn manager_layout_sets_bounds() {
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(10, 20, 800, 600), &theme);
    let b = m.bounds();
    assert_eq!(b.x(), 10);
    assert_eq!(b.y(), 20);
    assert_eq!(b.width(), 800);
    assert_eq!(b.height(), 600);
}

#[test]
fn manager_set_bounds_triggers_layout_update() {
    // Smoke: set_bounds calls update_layout; verify it doesn't panic
    // when notifications are queued.
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M").with_action("Undo"));
    m.set_bounds(Rect::new(0, 0, 1024, 768));
    assert_eq!(m.bounds().width(), 1024);
}

#[test]
fn manager_set_focused_is_a_no_op() {
    // NotificationManager hardcodes is_focused -> false (passive widget,
    // see AUDIT_MOJO_PORT_2026-04-28.md). Lock it in.
    let mut m = NotificationManager::new(test_id());
    assert!(!m.is_focused());
    m.set_focused(true);
    assert!(!m.is_focused(), "passive notification manager must not gain focus");
}

#[test]
fn manager_ignores_unrelated_events() {
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));

    let events = [
        Event::KeyPress(KeyPressEvent {
            key: Key::Enter,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::TextInput(TextInputEvent {
            text: "x".to_string(),
        }),
        Event::Focus(FocusEvent { gained: true }),
        Event::Update,
        Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(0, -1),
            position: Point::new(50, 50),
            modifiers: Modifiers::empty(),
        }),
    ];
    for ev in &events {
        assert_eq!(
            m.handle_event(ev, &theme),
            EventResult::Ignored,
            "manager must ignore unrelated event {:?}",
            ev
        );
    }
}

#[test]
fn manager_press_event_does_not_remove_notifications() {
    // The handler only reacts to release (`pressed=false`). A press
    // event must NOT remove the notification, even if it falls inside
    // the close-button rect.
    let closed: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = closed.clone();
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id())
        .with_position(NotificationPosition::TopLeft)
        .with_on_notification_closed(move |id| cap.borrow_mut().push(id.to_string()));
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));
    m.update_animations(10.0);
    m.layout(Rect::new(0, 0, 800, 600), &theme);

    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(352, 38),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = m.handle_event(&press, &theme);
    assert!(
        closed.borrow().is_empty(),
        "press-only event must not close notification (release-triggered)"
    );
}

#[test]
fn manager_visibility_and_enabled_round_trip() {
    let mut m = NotificationManager::new(test_id());
    assert!(m.is_visible());
    m.set_visible(false);
    assert!(!m.is_visible());
    m.set_visible(true);
    assert!(m.is_visible());

    assert!(m.is_enabled());
    m.set_enabled(false);
    assert!(!m.is_enabled());
}

#[test]
fn manager_show_after_clear_works() {
    // Regression: clear() then show() must produce a working
    // notification. Earlier hand-rolled queues were known to corrupt
    // state.
    let theme = default_theme();
    let mut m = NotificationManager::new(test_id());
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    m.show(Notification::new("a", "T", "M"));
    m.clear();
    m.show(Notification::new("b", "T", "M"));
    m.layout(Rect::new(0, 0, 800, 600), &theme);
    // No panic, and a follow-up event handler runs.
    let _ = m.handle_event(
        &Event::MouseMove(MouseMoveEvent {
            position: Point::new(0, 0),
            delta: Point::new(0, 0),
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );
}


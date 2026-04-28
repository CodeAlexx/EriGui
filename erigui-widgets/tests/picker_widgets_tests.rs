//! Integration tests for the picker widgets:
//! ColorPicker and FileDialog.
//!
//! Both widgets had zero integration tests before. Same bar as the
//! sibling wave-3 files (simple_widgets_tests.rs / modal_widgets_tests.rs
//! / tree_dock_tests.rs): "no widget has zero tests anymore" -- not
//! exhaustive coverage. Where the public API doesn't expose enough state
//! for a real round-trip, the test degenerates to "construction + call
//! doesn't panic," which is still a regression net.
//!
//! This commit covers ColorPicker. The FileDialog + FileFilter tests
//! land in the next commit.
//!
//! Notes about the widget under test:
//!
//! ColorPicker is mostly self-contained. The exposed surface is:
//!   new(id), with_color, with_style, with_alpha, with_on_change,
//!   get_color, set_color, plus the Widget trait. HSV state is private
//!   and only observable indirectly through the callback or get_color.

use erigui_core::{
    Color, Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConstraints, Modifiers,
    MouseButton, MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, Rect, TextInputEvent,
    Theme, Widget, WidgetId,
};
use erigui_widgets::{ColorPicker, ColorPickerStyle};
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
// ColorPicker
// ============================================================================
//
// Default constructor sets color = Color::from_hex(0xFF0000) (red),
// hsv = (0.0, 1.0, 1.0), style = Square, show_alpha = true. Many of
// these fields are private; we observe via get_color() and the
// on_change callback.

#[test]
fn color_picker_creation_defaults() {
    let cp = ColorPicker::new(test_id());
    let c = cp.get_color();
    // 0xFF0000 = pure red.
    assert_eq!(c.r, 0xFF);
    assert_eq!(c.g, 0x00);
    assert_eq!(c.b, 0x00);
    assert_eq!(c.a, 0xFF, "default alpha is 255 (Color::rgb path)");
    assert!(cp.is_visible());
    assert!(cp.is_enabled());
    assert!(!cp.is_focused());
    assert!(cp.can_focus(), "default ColorPicker (enabled+visible) is focusable");
}

#[test]
fn color_picker_with_color_round_trips_rgb() {
    // Round-trip a few canonical colors. The get_color path returns the
    // stored color; HSV is derived inside but not exposed.
    for (r, g, b) in [
        (0u8, 0u8, 0u8),
        (255, 255, 255),
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (123, 45, 67),
    ] {
        let cp = ColorPicker::new(test_id()).with_color(Color::rgb(r, g, b));
        let c = cp.get_color();
        assert_eq!((c.r, c.g, c.b), (r, g, b));
        assert_eq!(c.a, 255, "with_color preserves the input alpha (255 default)");
    }
}

#[test]
fn color_picker_with_color_preserves_alpha() {
    // Color::rgba(.., a) round-trips intact through with_color even
    // though hsv only encodes hue/sat/value.
    let cp = ColorPicker::new(test_id()).with_color(Color::rgba(10, 20, 30, 128));
    let c = cp.get_color();
    assert_eq!((c.r, c.g, c.b, c.a), (10, 20, 30, 128));
}

#[test]
fn color_picker_set_color_round_trips() {
    let mut cp = ColorPicker::new(test_id());
    cp.set_color(Color::rgb(50, 100, 150));
    let c = cp.get_color();
    assert_eq!((c.r, c.g, c.b), (50, 100, 150));
}

#[test]
fn color_picker_set_color_fires_callback_each_call() {
    // The implementation calls the callback unconditionally on every
    // set_color, including when set to the SAME color. Lock that in --
    // if a future refactor adds equality short-circuit, this test will
    // catch the contract change.
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();
    let mut cp = ColorPicker::new(test_id())
        .with_on_change(move |c| cap.borrow_mut().push(c));

    cp.set_color(Color::rgb(10, 20, 30));
    cp.set_color(Color::rgb(10, 20, 30)); // same color
    cp.set_color(Color::rgb(40, 50, 60));

    let v = captured.borrow();
    assert_eq!(v.len(), 3, "set_color fires callback every time, even on same color");
    assert_eq!((v[0].r, v[0].g, v[0].b), (10, 20, 30));
    assert_eq!((v[1].r, v[1].g, v[1].b), (10, 20, 30));
    assert_eq!((v[2].r, v[2].g, v[2].b), (40, 50, 60));
}

#[test]
fn color_picker_with_color_does_not_fire_callback() {
    // with_color is a builder run BEFORE on_change is installed, but
    // even when chained as `new().with_on_change(..).with_color(..)`,
    // the implementation of with_color never invokes the callback --
    // only set_color does. Lock that in.
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();
    let _cp = ColorPicker::new(test_id())
        .with_on_change(move |c| cap.borrow_mut().push(c))
        .with_color(Color::rgb(99, 99, 99));
    assert!(
        captured.borrow().is_empty(),
        "with_color (builder) must NOT fire on_change"
    );
}

#[test]
fn color_picker_with_style_runs_for_each_style() {
    // No public style getter; smoke test that the builder runs and
    // a subsequent measure() succeeds for every variant.
    let theme = default_theme();
    for style in [
        ColorPickerStyle::Compact,
        ColorPickerStyle::Wheel,
        ColorPickerStyle::Square,
        ColorPickerStyle::Sliders,
    ] {
        let cp = ColorPicker::new(test_id()).with_style(style);
        let s = cp.measure(&LayoutConstraints::UNBOUNDED, &theme);
        assert!(s.width > 0);
        assert!(s.height > 0);
    }
}

#[test]
fn color_picker_measure_compact_size() {
    // Compact: Size::new(preview_size + 12, preview_size) where
    // preview_size = 32 by default.
    let theme = default_theme();
    let cp = ColorPicker::new(test_id()).with_style(ColorPickerStyle::Compact);
    let s = cp.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert_eq!(s.width, 32 + 12);
    assert_eq!(s.height, 32);
}

#[test]
fn color_picker_measure_non_compact_size() {
    // For Square / Wheel / Sliders styles, measure() returns
    // Size::new(250, 300).
    let theme = default_theme();
    for style in [
        ColorPickerStyle::Square,
        ColorPickerStyle::Wheel,
        ColorPickerStyle::Sliders,
    ] {
        let cp = ColorPicker::new(test_id()).with_style(style);
        let s = cp.measure(&LayoutConstraints::UNBOUNDED, &theme);
        assert_eq!(s.width, 250);
        assert_eq!(s.height, 300);
    }
}

#[test]
fn color_picker_with_alpha_builder_runs() {
    // No public show_alpha getter; smoke test that the builder runs in
    // both states and follow-up operations work.
    let theme = default_theme();
    for show in [true, false] {
        let cp = ColorPicker::new(test_id()).with_alpha(show);
        let _ = cp.measure(&LayoutConstraints::UNBOUNDED, &theme);
    }
}

#[test]
fn color_picker_layout_sets_bounds() {
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id());
    cp.layout(Rect::new(10, 20, 250, 300), &theme);
    let b = cp.bounds();
    assert_eq!(b.x(), 10);
    assert_eq!(b.y(), 20);
    assert_eq!(b.width(), 250);
    assert_eq!(b.height(), 300);
}

#[test]
fn color_picker_set_bounds_round_trips() {
    let mut cp = ColorPicker::new(test_id());
    cp.set_bounds(Rect::new(5, 5, 100, 100));
    assert_eq!(cp.bounds().x(), 5);
    assert_eq!(cp.bounds().width(), 100);
}

#[test]
fn color_picker_visibility_and_enabled_round_trip() {
    let mut cp = ColorPicker::new(test_id());
    cp.set_visible(false);
    assert!(!cp.is_visible());
    cp.set_visible(true);
    assert!(cp.is_visible());

    cp.set_enabled(false);
    assert!(!cp.is_enabled());
    cp.set_enabled(true);
    assert!(cp.is_enabled());
}

#[test]
fn color_picker_focus_round_trips() {
    let mut cp = ColorPicker::new(test_id());
    assert!(!cp.is_focused());
    cp.set_focused(true);
    assert!(cp.is_focused());
    cp.set_focused(false);
    assert!(!cp.is_focused());
}

#[test]
fn color_picker_can_focus_respects_disabled_and_invisible() {
    let mut cp = ColorPicker::new(test_id());
    assert!(cp.can_focus());
    cp.set_enabled(false);
    assert!(!cp.can_focus(), "disabled ColorPicker must not advertise focus");
    cp.set_enabled(true);
    cp.set_visible(false);
    assert!(!cp.can_focus(), "invisible ColorPicker must not advertise focus");
}

#[test]
fn color_picker_disabled_or_invisible_ignores_events() {
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id()).with_style(ColorPickerStyle::Compact);
    cp.layout(Rect::new(0, 0, 100, 32), &theme);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    cp.set_visible(false);
    assert_eq!(cp.handle_event(&click, &theme), EventResult::Ignored);
    cp.set_visible(true);
    cp.set_enabled(false);
    assert_eq!(cp.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn color_picker_compact_click_toggles_popup_consumed() {
    // In Compact style, clicking inside the preview rect must return
    // Consumed (and toggle popup_visible). popup_visible is private;
    // observe via subsequent click-on-popup that DOES update color.
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id()).with_style(ColorPickerStyle::Compact);
    cp.layout(Rect::new(0, 0, 44, 32), &theme);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = cp.handle_event(&click, &theme);
    assert_eq!(res, EventResult::Consumed, "compact preview click toggles popup");
}

#[test]
fn color_picker_compact_popup_sv_click_updates_color_and_fires_callback() {
    // Open the popup, then click in the SV square. The popup is laid
    // out at: x = bounds.x(), y = bounds.bottom() + 4, w = 250, h = 300.
    // padding = 10, sv_size = 200; SV rect is at
    //   (bounds.x()+10, bounds.bottom()+4+10, 200, 200).
    // Clicking the bottom-left of the SV square sets s=0, v=1
    // (the y formula is `1 - (py / sv_size)`), which corresponds to a
    // pure white-ish point, but with the current hue=0 (red) that maps
    // to red->white. Just assert that some callback fired and the
    // color changed away from the default red.
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();

    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_on_change(move |c| cap.borrow_mut().push(c));
    cp.layout(Rect::new(0, 0, 44, 32), &theme);

    // Open popup.
    let toggle = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = cp.handle_event(&toggle, &theme);

    // popup origin = (0, 32+4) = (0, 36); SV rect = (10, 46, 200, 200).
    // Click at the top-right corner of the SV square: (10+200-1, 46),
    // which is s ~= 1, v ~= 1 -> pure red (no color change visually
    // from the default). To force a CHANGE, click the bottom-right
    // corner (s=1, v=0) -> black-ish.
    let sv_click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10 + 199, 46 + 199),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = cp.handle_event(&sv_click, &theme);
    assert_eq!(res, EventResult::Consumed, "SV click inside popup is consumed");

    let v = captured.borrow();
    assert!(!v.is_empty(), "SV click must fire on_change callback");
    // After clicking bottom-right of SV with hue=0, value=0 -> color
    // should be black (or near-black).
    let last = v.last().unwrap();
    assert!(
        last.r < 16 && last.g < 16 && last.b < 16,
        "bottom-right of SV (v=0) should produce near-black, got rgb({},{},{})",
        last.r, last.g, last.b
    );
    // Color stored on the widget matches the last callback invocation.
    let stored = cp.get_color();
    assert_eq!(stored.r, last.r);
    assert_eq!(stored.g, last.g);
    assert_eq!(stored.b, last.b);
}

#[test]
fn color_picker_compact_popup_hue_click_changes_hue() {
    // Hue rect is below the SV square: y = popup.y + padding + sv_size + padding.
    // For our layout that's 36 + 10 + 200 + 10 = 256, height = 20.
    // Clicking the right edge sets hue ~= 360 (= 0 wrapped) -> back to red.
    // Clicking the LEFT edge keeps hue=0. Click the middle (x=10+100=110)
    // -> hue ~= 180 -> cyan. With default sat=1, val=1, expect r=0,
    // and substantial g+b.
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();

    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_on_change(move |c| cap.borrow_mut().push(c));
    cp.layout(Rect::new(0, 0, 44, 32), &theme);
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );

    let hue_click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10 + 100, 256 + 10), // hue ~ 180
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = cp.handle_event(&hue_click, &theme);
    assert_eq!(res, EventResult::Consumed);

    let last = *captured.borrow().last().expect("hue click fires callback");
    assert!(last.r < 16, "cyan-ish: red channel near 0, got {}", last.r);
    assert!(last.g > 200 || last.b > 200, "cyan-ish: green or blue near 255");
}

#[test]
fn color_picker_compact_popup_alpha_click_updates_alpha() {
    // Alpha rect is below hue: y = popup.y + padding + sv_size + padding
    //                              + hue_height + padding
    //                            = 36 + 10 + 200 + 10 + 20 + 10 = 286.
    // height = 20. Click at the LEFT edge -> alpha = 0.
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_alpha(true);
    cp.layout(Rect::new(0, 0, 44, 32), &theme);
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );

    // Click the LEFT edge of the alpha bar -> alpha = 0.
    let alpha_click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 286 + 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = cp.handle_event(&alpha_click, &theme);
    assert_eq!(res, EventResult::Consumed, "alpha-bar click is consumed");
    assert_eq!(cp.get_color().a, 0, "left-edge alpha click sets a = 0");
}

#[test]
fn color_picker_compact_click_outside_popup_closes_it() {
    // After opening, a click well outside the popup rect should be
    // Consumed (and close the popup). Verify by then clicking the SV
    // area location: the popup is gone, so the event should NOT
    // produce a color change.
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();

    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_on_change(move |c| cap.borrow_mut().push(c));
    cp.layout(Rect::new(0, 0, 44, 32), &theme);
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );

    // Click far outside the popup (popup spans x=[0,250], y=[36,336]).
    let outside = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(1000, 1000),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = cp.handle_event(&outside, &theme);
    assert_eq!(res, EventResult::Consumed, "click-outside closes popup (Consumed)");

    let n_before = captured.borrow().len();
    // Now click in what WAS the SV area; popup is closed, so this
    // should be Ignored and not add to callbacks.
    let sv_again = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(110, 146),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = cp.handle_event(&sv_again, &theme);
    let n_after = captured.borrow().len();
    assert_eq!(
        n_before, n_after,
        "after popup close, SV-area click must not invoke callback"
    );
}

#[test]
fn color_picker_mouse_release_clears_drag_state() {
    // Press inside SV opens drag; release elsewhere clears it. Without
    // a getter, observe indirectly: a follow-up MouseMove far from any
    // slider must NOT update the color (drag flags are cleared).
    let captured: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = captured.clone();

    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_on_change(move |c| cap.borrow_mut().push(c));
    cp.layout(Rect::new(0, 0, 44, 32), &theme);

    // Open popup.
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );

    // Press in SV.
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(110, 146),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );
    // Release outside the popup; the impl clears all dragging flags
    // when pressed == false (regardless of position).
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(1000, 1000),
            pressed: false,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );

    let n_after_release = captured.borrow().len();

    // A MouseMove now: even if popup is open, dragging flags are
    // false, so nothing updates and the call returns Ignored. The
    // callback count must not advance.
    let mv = Event::MouseMove(MouseMoveEvent {
        position: Point::new(110, 200),
        delta: Point::new(0, 0),
        modifiers: Modifiers::empty(),
    });
    let _ = cp.handle_event(&mv, &theme);
    assert_eq!(
        captured.borrow().len(),
        n_after_release,
        "MouseMove without active drag must not fire on_change"
    );
}

#[test]
fn color_picker_hex_input_keypress_path() {
    // The legacy headless KeyPress(Character) path is wired in
    // handle_event for hex_focused. Without a public way to set
    // hex_focused to true, this test exercises the path via the popup
    // hex-rect click first. Hex rect: y = popup.y + padding + sv_size
    //   + padding + hue_height + padding + alpha_height + padding
    //                = 36 + 10 + 200 + 10 + 20 + 10 + 20 + 10
    //                = 316; (x, w, h) = (10, 80, 24).
    // Click (50, 320) to focus hex.
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id())
        .with_style(ColorPickerStyle::Compact)
        .with_alpha(true);
    cp.layout(Rect::new(0, 0, 44, 32), &theme);

    // Open popup.
    let _ = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );
    // Focus hex.
    let res = cp.handle_event(
        &Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(50, 320),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        &theme,
    );
    assert_eq!(res, EventResult::Consumed, "hex-rect click is consumed");

    // The default hex_input is "FF0000" with cursor at 6 (end), and
    // the input is full (len == 6). Sending more characters should
    // be no-ops because the impl gates on `hex_input.len() < 6`.
    // Backspace removes the last char and re-parses.
    let bs = Event::KeyPress(KeyPressEvent {
        key: Key::Backspace,
        modifiers: Modifiers::empty(),
        repeat: false,
    });
    let res = cp.handle_event(&bs, &theme);
    assert_eq!(res, EventResult::Consumed);
    // After removing one digit, "FF000" parses as 0x00FF000, which
    // Color::from_hex interprets as r=0x0F, g=0xF0, b=0x00. Verify
    // that the color did update via the hex re-parse path.
    let after_bs = cp.get_color();
    assert!(
        after_bs.r != 0xFF || after_bs.g != 0x00 || after_bs.b != 0x00,
        "Backspace inside hex input should change the parsed color"
    );
}

#[test]
fn color_picker_text_input_when_unfocused_does_nothing() {
    // Without hex focus, TextInput is Ignored and color is unchanged.
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id());
    cp.layout(Rect::new(0, 0, 250, 300), &theme);
    let before = cp.get_color();

    let ti = Event::TextInput(TextInputEvent {
        text: "AABBCC".to_string(),
    });
    assert_eq!(cp.handle_event(&ti, &theme), EventResult::Ignored);
    let after = cp.get_color();
    assert_eq!((before.r, before.g, before.b), (after.r, after.g, after.b));
}

#[test]
fn color_picker_unrelated_events_are_ignored() {
    // The impl only responds to MouseButton, MouseMove, KeyPress (when
    // hex_focused), and TextInput (when hex_focused). Everything else
    // should fall through to Ignored.
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id());
    cp.layout(Rect::new(0, 0, 250, 300), &theme);

    let events = [
        Event::Update,
        Event::Focus(FocusEvent { gained: true }),
        Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(0, -1),
            position: Point::new(10, 10),
            modifiers: Modifiers::empty(),
        }),
        Event::KeyPress(KeyPressEvent {
            key: Key::Enter,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
    ];
    for ev in &events {
        assert_eq!(
            cp.handle_event(ev, &theme),
            EventResult::Ignored,
            "ColorPicker should ignore {:?} when popup closed and hex unfocused",
            ev
        );
    }
}

#[test]
fn color_picker_layout_event_smoke_does_not_panic() {
    // Defensive: drive a closed picker through a barrage of events
    // with various positions. No assertions about result -- the bar
    // is "no panic."
    let theme = default_theme();
    let mut cp = ColorPicker::new(test_id());
    cp.layout(Rect::new(0, 0, 250, 300), &theme);

    let positions = [
        Point::new(-100, -100),
        Point::new(0, 0),
        Point::new(125, 150),
        Point::new(10000, 10000),
    ];
    for p in &positions {
        for pressed in [true, false] {
            let _ = cp.handle_event(
                &Event::MouseButton(MouseButtonEvent {
                    button: MouseButton::Left,
                    position: *p,
                    pressed,
                    modifiers: Modifiers::empty(),
                }),
                &theme,
            );
            let _ = cp.handle_event(
                &Event::MouseMove(MouseMoveEvent {
                    position: *p,
                    delta: Point::new(0, 0),
                    modifiers: Modifiers::empty(),
                }),
                &theme,
            );
        }
    }
}

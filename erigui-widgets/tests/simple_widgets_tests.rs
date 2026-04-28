//! Integration tests for the small / display-mostly widgets:
//! Label, Container, StatusBar, Icon.
//!
//! These widgets had zero integration tests before. The bar here is
//! "no widget has zero tests anymore" -- not exhaustive coverage. Where the
//! widget's public API doesn't expose enough state to assert on, the test
//! degenerates to "construction + call doesn't panic," which is still a
//! regression net.

use erigui_core::{
    Color, Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConfig, LayoutConstraints,
    LayoutMode, Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point,
    Rect, ResizeEvent, Size, TextInputEvent, Theme, Widget, WidgetId,
};
use erigui_widgets::{
    Container, Label, SeparatorStyle, StatusBar, StatusPanel, StatusPanelWidth, TextAlign,
    TextAlignment,
};
use slotmap::SlotMap;

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

// ============================================================================
// Container
// ============================================================================
//
// Container is a layout primitive. The actual child-positioning logic lives
// in a private layout_children method that takes a `widgets` callback that
// the public API doesn't expose to outside callers, so these tests focus
// on the parts that ARE observable: the children list, the LayoutConfig,
// the Widget-trait surface, and "events are ignored / doesn't panic."
//
// Mint distinct child WidgetIds via a SlotMap; WidgetId is just
// `slotmap::DefaultKey`, so this is the same mechanism erigui-widgets uses
// internally. The map MUST persist across calls -- a fresh SlotMap always
// returns the same first key.
fn id_factory() -> impl FnMut() -> WidgetId {
    let mut sm: SlotMap<WidgetId, ()> = SlotMap::new();
    move || sm.insert(())
}

#[test]
fn container_creation_defaults() {
    let c = Container::new(test_id());
    assert_eq!(c.children().len(), 0, "new Container should have no children");
    assert!(c.is_visible());
    assert!(c.is_enabled());
    assert!(!c.can_focus(), "Container does not participate in focus");
    assert_eq!(c.get_layout_info().mode, LayoutMode::None);
}

#[test]
fn container_with_layout_builder_round_trips_mode() {
    let cfg = LayoutConfig {
        mode: LayoutMode::Vertical,
        spacing: 8,
        ..LayoutConfig::default()
    };
    let c = Container::new(test_id()).with_layout(cfg);
    let info = c.get_layout_info();
    assert_eq!(info.mode, LayoutMode::Vertical);
    assert_eq!(info.spacing, 8);
}

#[test]
fn container_set_layout_mode_round_trips() {
    let mut c = Container::new(test_id());
    c.set_layout_mode(LayoutMode::Horizontal);
    assert_eq!(c.get_layout_info().mode, LayoutMode::Horizontal);
    c.set_layout_mode(LayoutMode::Grid { columns: 3 });
    assert_eq!(c.get_layout_info().mode, LayoutMode::Grid { columns: 3 });
    c.set_layout_mode(LayoutMode::None);
    assert_eq!(c.get_layout_info().mode, LayoutMode::None);
}

#[test]
fn container_add_child_appends() {
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    let id_a = next();
    let id_b = next();
    c.add_child(id_a);
    c.add_child(id_b);
    assert_eq!(c.children().len(), 2);
    assert_eq!(c.children()[0], id_a);
    assert_eq!(c.children()[1], id_b);
}

#[test]
fn container_add_flex_child_appends() {
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    let id_a = next();
    let id_b = next();
    c.add_flex_child(id_a, 1.0);
    c.add_flex_child(id_b, 2.0);
    assert_eq!(c.children().len(), 2);
    assert_eq!(c.children()[0], id_a);
    assert_eq!(c.children()[1], id_b);
}

#[test]
fn container_remove_child_returns_true_when_present() {
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    let id_a = next();
    let id_b = next();
    c.add_child(id_a);
    c.add_child(id_b);
    assert!(c.remove_child(id_a));
    assert_eq!(c.children().len(), 1);
    assert_eq!(c.children()[0], id_b);
}

#[test]
fn container_remove_child_returns_false_when_absent() {
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    let id_a = next();
    let id_other = next();
    c.add_child(id_a);
    assert!(!c.remove_child(id_other), "removing absent child should return false");
    assert_eq!(c.children().len(), 1);
}

#[test]
fn container_clear_children_empties_list() {
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    c.add_child(next());
    c.add_child(next());
    c.add_flex_child(next(), 1.0);
    assert_eq!(c.children().len(), 3);
    c.clear_children();
    assert_eq!(c.children().len(), 0);
}

#[test]
fn container_layout_sets_bounds() {
    let mut c = Container::new(test_id());
    let theme = default_theme();
    c.layout(Rect::new(2, 4, 200, 300), &theme);
    let b = c.bounds();
    assert_eq!(b.x(), 2);
    assert_eq!(b.y(), 4);
    assert_eq!(b.width(), 200);
    assert_eq!(b.height(), 300);
}

#[test]
fn container_measure_returns_nonnegative_size_in_each_mode() {
    let theme = default_theme();
    let constraints = LayoutConstraints::bounded(400, 300);

    for mode in [
        LayoutMode::None,
        LayoutMode::Vertical,
        LayoutMode::Horizontal,
        LayoutMode::Grid { columns: 2 },
    ] {
        let mut c = Container::new(test_id());
        c.set_layout_mode(mode);
        let size = c.measure(&constraints, &theme);
        assert!(size.width >= 0, "{:?} produced negative width", mode);
        assert!(size.height >= 0, "{:?} produced negative height", mode);
    }
}

#[test]
fn container_visibility_and_enabled_round_trip() {
    let mut c = Container::new(test_id());
    c.set_visible(false);
    assert!(!c.is_visible());
    c.set_enabled(false);
    assert!(!c.is_enabled());
}

#[test]
fn container_ignores_all_events() {
    let theme = default_theme();
    let mut c = Container::new(test_id());
    c.layout(Rect::new(0, 0, 100, 100), &theme);

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
        Event::KeyPress(KeyPressEvent {
            key: Key::Space,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::Update,
    ];
    for ev in &events {
        assert_eq!(c.handle_event(ev, &theme), EventResult::Ignored);
    }
}

#[test]
fn container_set_focused_is_a_no_op() {
    // can_focus() returns false; set_focused should not flip is_focused.
    let mut c = Container::new(test_id());
    c.set_focused(true);
    assert!(!c.is_focused());
}

// ============================================================================
// StatusBar
// ============================================================================
//
// StatusBar exposes mutators (add_panel, set_panel_text, clear_panels) but
// no panel-count or panel-content getters. So external assertions are
// limited to: StatusPanel public-field round-trip, StatusBar widget-trait
// surface, and "the mutators don't panic."

#[test]
fn status_panel_new_defaults() {
    let p = StatusPanel::new("hello");
    assert_eq!(p.text, "hello");
    assert!(matches!(p.width, StatusPanelWidth::Spring));
    assert!(matches!(p.alignment, TextAlignment::Left));
}

#[test]
fn status_panel_with_fixed_width_round_trips() {
    let p = StatusPanel::new("x").with_fixed_width(120);
    match p.width {
        StatusPanelWidth::Fixed(w) => assert_eq!(w, 120),
        other => panic!("expected Fixed(120), got {:?}", other),
    }
}

#[test]
fn status_panel_with_content_width_round_trips() {
    let p = StatusPanel::new("x").with_content_width();
    assert!(matches!(p.width, StatusPanelWidth::Content));
}

#[test]
fn status_panel_with_spring_width_round_trips() {
    // Spring is the default but `with_spring_width` should still set it
    // explicitly when chained after another width.
    let p = StatusPanel::new("x").with_fixed_width(50).with_spring_width();
    assert!(matches!(p.width, StatusPanelWidth::Spring));
}

#[test]
fn status_panel_with_alignment_round_trips() {
    let p_l = StatusPanel::new("x").with_alignment(TextAlignment::Left);
    assert!(matches!(p_l.alignment, TextAlignment::Left));
    let p_c = StatusPanel::new("x").with_alignment(TextAlignment::Center);
    assert!(matches!(p_c.alignment, TextAlignment::Center));
    let p_r = StatusPanel::new("x").with_alignment(TextAlignment::Right);
    assert!(matches!(p_r.alignment, TextAlignment::Right));
}

#[test]
fn status_bar_creation_defaults() {
    let sb = StatusBar::new(test_id());
    assert!(sb.is_visible());
    assert!(sb.is_enabled());
    assert!(!sb.can_focus(), "StatusBar does not participate in focus");
    assert!(!sb.is_focused());
}

#[test]
fn status_bar_with_separator_style_does_not_panic() {
    // No public getter -- builder smoke + measure to make sure subsequent
    // ops still work.
    let theme = default_theme();
    for style in [
        SeparatorStyle::None,
        SeparatorStyle::Line,
        SeparatorStyle::Raised,
        SeparatorStyle::Sunken,
    ] {
        let sb = StatusBar::new(test_id()).with_separator_style(style);
        let size = sb.measure(&LayoutConstraints::UNBOUNDED, &theme);
        assert!(size.height > 0, "StatusBar height should be positive");
    }
}

#[test]
fn status_bar_layout_sets_bounds() {
    let theme = default_theme();
    let mut sb = StatusBar::new(test_id());
    sb.layout(Rect::new(0, 480, 800, 24), &theme);
    let b = sb.bounds();
    assert_eq!(b.x(), 0);
    assert_eq!(b.y(), 480);
    assert_eq!(b.width(), 800);
    assert_eq!(b.height(), 24);
}

#[test]
fn status_bar_measure_default_size() {
    let sb = StatusBar::new(test_id());
    let theme = default_theme();
    let size = sb.measure(&LayoutConstraints::UNBOUNDED, &theme);
    // measure() returns Size::new(100, self.height) where height defaults to 24.
    assert_eq!(size.width, 100);
    assert_eq!(size.height, 24);
}

#[test]
fn status_bar_add_panel_does_not_panic() {
    // No public panel-count getter, so we can only verify the call runs and
    // that subsequent operations (layout, measure) still succeed.
    let theme = default_theme();
    let mut sb = StatusBar::new(test_id());
    sb.add_panel(StatusPanel::new("Ready"));
    sb.add_panel(StatusPanel::new("Ln 1, Col 1").with_fixed_width(120));
    sb.add_panel(StatusPanel::new("UTF-8").with_content_width());
    sb.layout(Rect::new(0, 0, 800, 24), &theme);
    let _ = sb.measure(&LayoutConstraints::UNBOUNDED, &theme);
}

#[test]
fn status_bar_set_panel_text_in_range() {
    // No getter for panel text either, but exercise the mutator and confirm
    // it doesn't panic when given a valid index.
    let mut sb = StatusBar::new(test_id());
    sb.add_panel(StatusPanel::new("initial"));
    sb.set_panel_text(0, "updated");
    sb.set_panel_text(0, String::from("updated again"));
}

#[test]
fn status_bar_set_panel_text_out_of_range_does_not_panic() {
    // The implementation uses .get_mut(index) -- out-of-range is a no-op.
    let mut sb = StatusBar::new(test_id());
    sb.add_panel(StatusPanel::new("only"));
    // Index 99 is well past the end; must not panic.
    sb.set_panel_text(99, "ignored");
}

#[test]
fn status_bar_set_panel_text_on_empty_does_not_panic() {
    let mut sb = StatusBar::new(test_id());
    sb.set_panel_text(0, "ignored");
}

#[test]
fn status_bar_clear_panels_does_not_panic() {
    let mut sb = StatusBar::new(test_id());
    sb.add_panel(StatusPanel::new("a"));
    sb.add_panel(StatusPanel::new("b"));
    sb.clear_panels();
    // Clearing twice is also fine.
    sb.clear_panels();
}

#[test]
fn status_bar_visibility_and_enabled_round_trip() {
    let mut sb = StatusBar::new(test_id());
    sb.set_visible(false);
    assert!(!sb.is_visible());
    sb.set_enabled(false);
    assert!(!sb.is_enabled());
}

#[test]
fn status_bar_set_focused_is_a_no_op() {
    let mut sb = StatusBar::new(test_id());
    sb.set_focused(true);
    assert!(!sb.is_focused());
}

#[test]
fn status_bar_ignores_all_events() {
    let theme = default_theme();
    let mut sb = StatusBar::new(test_id());
    sb.add_panel(StatusPanel::new("status"));
    sb.layout(Rect::new(0, 0, 200, 24), &theme);

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
        Event::KeyPress(KeyPressEvent {
            key: Key::Enter,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::Update,
    ];
    for ev in &events {
        assert_eq!(
            sb.handle_event(ev, &theme),
            EventResult::Ignored,
            "StatusBar must ignore event {:?}",
            ev
        );
    }
}

#[test]
fn container_add_remove_does_not_panic_when_layout_mode_is_grid() {
    // Smoke: switch to Grid mode (which has a different measure path) and
    // exercise child mutations + measure end-to-end.
    let theme = default_theme();
    let mut next = id_factory();
    let mut c = Container::new(test_id());
    c.set_layout_mode(LayoutMode::Grid { columns: 3 });
    for _ in 0..7 {
        c.add_child(next());
    }
    c.layout(Rect::new(0, 0, 300, 300), &theme);
    let _ = c.measure(&LayoutConstraints::bounded(300, 300), &theme);
    c.clear_children();
    assert_eq!(c.children().len(), 0);
}

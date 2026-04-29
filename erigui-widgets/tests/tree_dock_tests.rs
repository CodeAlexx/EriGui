//! Integration tests for TreeView and DockPanel.
//!
//! Both widgets had thin coverage before this file: TreeView had zero
//! integration tests, and DockPanel only had two focus-discipline tests
//! that landed in wave-2 commit `2242794` (rectifying the can_focus()
//! contract after the Mojo port). Same bar as the wave-3 sibling files:
//! "no widget has zero tests anymore" -- not exhaustive coverage. Where
//! the public API doesn't expose enough state for a real round-trip
//! assertion, the test degenerates to "construction + call doesn't
//! panic," which is still a regression net.
//!
//! TreeView is a hierarchical list with an explicit
//! `root_nodes: Vec<TreeNode>` field that's `pub`, so most state is
//! observable directly. DockPanel hides its `DockNode` tree entirely,
//! so coverage there is mostly behavioral (callbacks fire, layout-event
//! flow doesn't panic).

use erigui_core::{
    Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, Rect, TextInputEvent, Theme, Widget,
    WidgetId,
};
use erigui_widgets::{
    DockPanel, DockPosition, DockablePanel, Label, TreeNode, TreeView,
};
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
// TreeNode (the data struct)
// ============================================================================
//
// TreeNode is plain data: pub id, text, icon, children, expanded, selected.
// All fields round-trip directly. find_node_mut() walks the tree.

#[test]
fn tree_node_new_defaults() {
    let n = TreeNode::new("a".to_string(), "Alpha".to_string());
    assert_eq!(n.id, "a");
    assert_eq!(n.text, "Alpha");
    assert_eq!(n.icon, None);
    assert_eq!(n.children.len(), 0);
    assert!(!n.expanded);
    assert!(!n.selected);
}

#[test]
fn tree_node_add_child_appends() {
    let mut root = TreeNode::new("r".to_string(), "Root".to_string());
    root.add_child(TreeNode::new("a".to_string(), "Alpha".to_string()));
    root.add_child(TreeNode::new("b".to_string(), "Beta".to_string()));
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].id, "a");
    assert_eq!(root.children[1].id, "b");
}

#[test]
fn tree_node_find_node_mut_finds_self() {
    let mut root = TreeNode::new("r".to_string(), "Root".to_string());
    let found = root.find_node_mut("r");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "r");
}

#[test]
fn tree_node_find_node_mut_walks_descendants() {
    // Build:   root -> a -> a1
    //               -> b
    let mut a = TreeNode::new("a".to_string(), "A".to_string());
    a.add_child(TreeNode::new("a1".to_string(), "A1".to_string()));
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.add_child(a);
    root.add_child(TreeNode::new("b".to_string(), "B".to_string()));

    assert_eq!(root.find_node_mut("a").map(|n| n.id.clone()), Some("a".to_string()));
    assert_eq!(root.find_node_mut("a1").map(|n| n.id.clone()), Some("a1".to_string()));
    assert_eq!(root.find_node_mut("b").map(|n| n.id.clone()), Some("b".to_string()));
    assert!(root.find_node_mut("missing").is_none());
}

#[test]
fn tree_node_mutate_via_find() {
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.add_child(TreeNode::new("a".to_string(), "A".to_string()));
    if let Some(node) = root.find_node_mut("a") {
        node.expanded = true;
        node.selected = true;
    }
    assert!(root.children[0].expanded);
    assert!(root.children[0].selected);
}

// ============================================================================
// TreeView construction & state
// ============================================================================

#[test]
fn tree_view_creation_defaults() {
    let tv = TreeView::new(test_id());
    assert_eq!(tv.root_nodes.len(), 0);
    assert!(tv.is_visible());
    assert!(tv.is_enabled());
    assert!(!tv.is_focused());
    // can_focus = enabled && visible per the impl.
    assert!(tv.can_focus(), "default tree (enabled+visible) should be focusable");
    assert_eq!(tv.selected_id(), None);
}

#[test]
fn tree_view_with_root_nodes_round_trips() {
    let nodes = vec![
        TreeNode::new("a".to_string(), "A".to_string()),
        TreeNode::new("b".to_string(), "B".to_string()),
    ];
    let tv = TreeView::new(test_id()).with_root_nodes(nodes);
    assert_eq!(tv.root_nodes.len(), 2);
    assert_eq!(tv.root_nodes[0].id, "a");
    assert_eq!(tv.root_nodes[1].id, "b");
}

#[test]
fn tree_view_add_root_node_and_clear() {
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(TreeNode::new("x".to_string(), "X".to_string()));
    tv.add_root_node(TreeNode::new("y".to_string(), "Y".to_string()));
    assert_eq!(tv.root_nodes.len(), 2);
    tv.clear_nodes();
    assert_eq!(tv.root_nodes.len(), 0);
    // Idempotent.
    tv.clear_nodes();
    assert_eq!(tv.root_nodes.len(), 0);
}

#[test]
fn tree_view_find_node_mut_walks_tree() {
    let mut a = TreeNode::new("a".to_string(), "A".to_string());
    a.add_child(TreeNode::new("a1".to_string(), "A1".to_string()));
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(a);
    tv.add_root_node(TreeNode::new("b".to_string(), "B".to_string()));

    assert!(tv.find_node_mut("a").is_some());
    assert!(tv.find_node_mut("a1").is_some());
    assert!(tv.find_node_mut("b").is_some());
    assert!(tv.find_node_mut("does-not-exist").is_none());
}

#[test]
fn tree_view_selected_id_finds_selected_in_collapsed_root() {
    // selected_id walks visible nodes (root always visible, descendants
    // only when parent.expanded). A selected root must be reported even
    // if its parent is unexpanded.
    let mut tv = TreeView::new(test_id());
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.selected = true;
    tv.add_root_node(root);
    assert_eq!(tv.selected_id(), Some("r".to_string()));
}

#[test]
fn tree_view_selected_id_skips_descendants_of_collapsed_parent() {
    // Per the impl, selected_id only descends into a node when expanded.
    // A selected child of a collapsed parent is invisible -- and per this
    // contract, NOT reported.
    let mut child = TreeNode::new("c".to_string(), "C".to_string());
    child.selected = true;
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.expanded = false;
    root.add_child(child);

    let mut tv = TreeView::new(test_id());
    tv.add_root_node(root);
    assert_eq!(
        tv.selected_id(),
        None,
        "selected_id must not surface a selected child under a collapsed parent"
    );
}

#[test]
fn tree_view_selected_id_finds_descendant_when_expanded() {
    let mut child = TreeNode::new("c".to_string(), "C".to_string());
    child.selected = true;
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.expanded = true;
    root.add_child(child);

    let mut tv = TreeView::new(test_id());
    tv.add_root_node(root);
    assert_eq!(tv.selected_id(), Some("c".to_string()));
}

// ============================================================================
// TreeView Widget-trait surface
// ============================================================================

#[test]
fn tree_view_layout_sets_bounds() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.layout(Rect::new(8, 16, 250, 400), &theme);
    let b = tv.bounds();
    assert_eq!(b.x(), 8);
    assert_eq!(b.y(), 16);
    assert_eq!(b.width(), 250);
    assert_eq!(b.height(), 400);
}

#[test]
fn tree_view_measure_floors_height_at_100() {
    // measure() returns max(visible_count * item_height, 100). An empty
    // tree should still produce a non-empty rect.
    let theme = default_theme();
    let tv = TreeView::new(test_id());
    let size = tv.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert!(size.height >= 100, "empty tree's height floor is 100");
    // Default unbounded width path returns 300.
    assert_eq!(size.width, 300);
}

#[test]
fn tree_view_measure_grows_with_visible_nodes() {
    // count_visible_nodes counts root nodes + (recursively) children of
    // expanded nodes only. Build 5 collapsed roots; visible count = 5;
    // 5 * 24 = 120 > 100 floor.
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    for i in 0..5 {
        tv.add_root_node(TreeNode::new(format!("r{}", i), format!("R{}", i)));
    }
    let size = tv.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert_eq!(size.height, 5 * 24, "5 visible nodes at 24px each");
}

#[test]
fn tree_view_measure_does_not_count_collapsed_descendants() {
    // Build root -> 3 children, root is COLLAPSED. Visible = 1.
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    for i in 0..3 {
        root.add_child(TreeNode::new(format!("c{}", i), format!("C{}", i)));
    }
    root.expanded = false;

    let mut tv = TreeView::new(test_id());
    tv.add_root_node(root);
    let theme = default_theme();
    let size = tv.measure(&LayoutConstraints::UNBOUNDED, &theme);
    // 1 visible node -> 24 < 100 floor; size.height clamps to 100.
    assert_eq!(size.height, 100);
}

#[test]
fn tree_view_measure_counts_expanded_descendants() {
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    for i in 0..3 {
        root.add_child(TreeNode::new(format!("c{}", i), format!("C{}", i)));
    }
    root.expanded = true;

    let mut tv = TreeView::new(test_id());
    tv.add_root_node(root);
    let theme = default_theme();
    let size = tv.measure(&LayoutConstraints::UNBOUNDED, &theme);
    // 1 root + 3 children = 4 * 24 = 96 < 100; clamps to 100.
    assert_eq!(size.height, 100);

    // Add one more so we cross the floor.
    let mut tv2 = TreeView::new(test_id());
    let mut root2 = TreeNode::new("r".to_string(), "R".to_string());
    for i in 0..5 {
        root2.add_child(TreeNode::new(format!("c{}", i), format!("C{}", i)));
    }
    root2.expanded = true;
    tv2.add_root_node(root2);
    let size2 = tv2.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert_eq!(size2.height, 6 * 24, "1 root + 5 expanded children");
}

#[test]
fn tree_view_visibility_and_enabled_round_trip() {
    let mut tv = TreeView::new(test_id());
    tv.set_visible(false);
    assert!(!tv.is_visible());
    tv.set_visible(true);
    assert!(tv.is_visible());

    tv.set_enabled(false);
    assert!(!tv.is_enabled());
    tv.set_enabled(true);
    assert!(tv.is_enabled());
}

#[test]
fn tree_view_focus_round_trips() {
    // can_focus() = enabled && visible; set_focused round-trips on
    // WidgetState; is_focused reflects that field.
    let mut tv = TreeView::new(test_id());
    assert!(tv.can_focus());
    assert!(!tv.is_focused());
    tv.set_focused(true);
    assert!(tv.is_focused());
    tv.set_focused(false);
    assert!(!tv.is_focused());
}

#[test]
fn tree_view_can_focus_respects_disabled_and_invisible() {
    let mut tv = TreeView::new(test_id());
    tv.set_enabled(false);
    assert!(!tv.can_focus(), "disabled tree must not advertise focus");
    tv.set_enabled(true);
    tv.set_visible(false);
    assert!(!tv.can_focus(), "invisible tree must not advertise focus");
}

// ============================================================================
// TreeView event handling
// ============================================================================

#[test]
fn tree_view_disabled_ignores_events() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(TreeNode::new("a".to_string(), "A".to_string()));
    tv.layout(Rect::new(0, 0, 200, 200), &theme);
    tv.set_enabled(false);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    assert_eq!(tv.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_invisible_ignores_events() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(TreeNode::new("a".to_string(), "A".to_string()));
    tv.layout(Rect::new(0, 0, 200, 200), &theme);
    tv.set_visible(false);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    assert_eq!(tv.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_click_outside_bounds_ignored() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(TreeNode::new("a".to_string(), "A".to_string()));
    tv.layout(Rect::new(10, 10, 100, 100), &theme);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(500, 500),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    assert_eq!(tv.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_click_on_node_selects_it_and_fires_callback() {
    // Capture selection callback via Rc<RefCell<Option<String>>>.
    let captured: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let cap = captured.clone();

    let theme = default_theme();
    let mut tv = TreeView::new(test_id())
        .with_root_nodes(vec![
            TreeNode::new("a".to_string(), "A".to_string()),
            TreeNode::new("b".to_string(), "B".to_string()),
        ])
        .with_on_selection_change(move |id| {
            *cap.borrow_mut() = Some(id.to_string());
        });
    // bounds at (0,0,200,200); item_height=24; row 0 covers y in [0,24).
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    // Click somewhere clearly inside row 1 (y=30 is in [24, 48), row 1 = "b").
    // Use x=80 to land in the text area, NOT on the expand button. Since
    // these nodes have no children, there is no expand button to land on.
    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(80, 30),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = tv.handle_event(&click, &theme);
    assert_eq!(res, EventResult::Consumed, "click on a row must be consumed");
    assert_eq!(*captured.borrow(), Some("b".to_string()));
    assert_eq!(tv.selected_id(), Some("b".to_string()));
}

#[test]
fn tree_view_click_on_expand_button_toggles_and_fires_expand_callback() {
    // An expand button exists for nodes with children. Geometry mirrors
    // the impl:
    //   arrow_x = bounds.x() + indent * 20 + 4
    //   button rect width = 16
    // For root node (indent=0), button covers [4, 20] in x at the row's
    // vertical center.
    let captured: Rc<RefCell<Option<(String, bool)>>> = Rc::new(RefCell::new(None));
    let cap = captured.clone();

    let theme = default_theme();
    let mut root = TreeNode::new("r".to_string(), "R".to_string());
    root.add_child(TreeNode::new("c".to_string(), "C".to_string()));
    let mut tv = TreeView::new(test_id())
        .with_root_nodes(vec![root])
        .with_on_expand(move |id, expanded| {
            *cap.borrow_mut() = Some((id.to_string(), expanded));
        });
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    // Click center of expand button: x=10, y=12 (row 0, item_height=24,
    // center=12).
    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 12),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = tv.handle_event(&click, &theme);
    assert_eq!(res, EventResult::Consumed);
    // Expand callback fired with expanded=true.
    assert_eq!(*captured.borrow(), Some(("r".to_string(), true)));
    // The root is now expanded.
    let r = tv.find_node_mut("r").expect("root must still exist");
    assert!(r.expanded, "click on expand button toggles to expanded");

    // Click again -> collapse.
    let res2 = tv.handle_event(&click, &theme);
    assert_eq!(res2, EventResult::Consumed);
    assert_eq!(*captured.borrow(), Some(("r".to_string(), false)));
    let r = tv.find_node_mut("r").expect("root still exists");
    assert!(!r.expanded);
}

#[test]
fn tree_view_clicking_node_clears_previous_selection() {
    // After clicking node B, node A's `selected` flag must be false.
    let theme = default_theme();
    let mut tv = TreeView::new(test_id()).with_root_nodes(vec![
        TreeNode::new("a".to_string(), "A".to_string()),
        TreeNode::new("b".to_string(), "B".to_string()),
    ]);
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    // Manually pre-mark A as selected to verify clear_selection runs.
    tv.find_node_mut("a").unwrap().selected = true;
    assert_eq!(tv.selected_id(), Some("a".to_string()));

    // Click row 1 (B) at y=30.
    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(80, 30),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let _ = tv.handle_event(&click, &theme);
    assert_eq!(tv.selected_id(), Some("b".to_string()));
    // A is no longer selected.
    assert!(!tv.find_node_mut("a").unwrap().selected);
}

#[test]
fn tree_view_mouse_wheel_outside_bounds_ignored() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.layout(Rect::new(0, 0, 100, 100), &theme);

    let wheel = Event::MouseWheel(MouseWheelEvent {
        delta: Point::new(0, -1),
        position: Point::new(500, 500),
        modifiers: Modifiers::empty(),
    });
    assert_eq!(tv.handle_event(&wheel, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_mouse_wheel_no_overflow_ignored() {
    // Wheel events INSIDE bounds but with no scroll overflow (total_height
    // <= bounds.height) must return Ignored, not Consumed. That lets the
    // scroll bubble up.
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    // 1 root, no expansion: total_height = 24 << 200.
    tv.add_root_node(TreeNode::new("r".to_string(), "R".to_string()));
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    let wheel = Event::MouseWheel(MouseWheelEvent {
        delta: Point::new(0, -1),
        position: Point::new(50, 50),
        modifiers: Modifiers::empty(),
    });
    assert_eq!(tv.handle_event(&wheel, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_ignores_unrelated_events() {
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.add_root_node(TreeNode::new("a".to_string(), "A".to_string()));
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    // The widget only responds to MouseButton(Left,pressed=true) and
    // MouseWheel. Everything else falls through to Ignored.
    let events = [
        Event::MouseMove(MouseMoveEvent {
            position: Point::new(50, 50),
            delta: Point::new(0, 0),
            modifiers: Modifiers::empty(),
        }),
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Right,
            position: Point::new(50, 12),
            pressed: true,
            modifiers: Modifiers::empty(),
        }),
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(50, 12),
            pressed: false, // release, not press
            modifiers: Modifiers::empty(),
        }),
        Event::KeyPress(KeyPressEvent {
            key: Key::Down,
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
            tv.handle_event(ev, &theme),
            EventResult::Ignored,
            "TreeView must ignore {:?}",
            ev
        );
    }
}

#[test]
fn tree_view_empty_event_handling_does_not_panic() {
    // Smoke: hand a populated set of events to an empty (no nodes) tree.
    let theme = default_theme();
    let mut tv = TreeView::new(test_id());
    tv.layout(Rect::new(0, 0, 200, 200), &theme);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(10, 10),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    // Empty tree -> node_at_position returns None -> Ignored.
    assert_eq!(tv.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn tree_view_set_bounds_round_trips() {
    let mut tv = TreeView::new(test_id());
    tv.set_bounds(Rect::new(5, 5, 150, 80));
    assert_eq!(tv.bounds().x(), 5);
    assert_eq!(tv.bounds().width(), 150);
}

// ============================================================================
// TreeView -- keyboard navigation
// ============================================================================
//
// TreeView's keyboard handler is gated on `self.state.focused`. Without
// focus it returns Ignored so siblings (e.g. a text input nested in a
// tree row) keep their keys. With focus, Up/Down/Home/End traverse the
// visible (depth-first, expanded-only) order; Left collapses or
// ascends to parent; Right expands or descends to first child;
// Enter/Space re-fire the selection callback as an "activate" gesture.

fn key_press(key: Key) -> Event {
    Event::KeyPress(KeyPressEvent {
        key,
        modifiers: Modifiers::empty(),
        repeat: false,
    })
}

fn make_kbd_tree() -> TreeView {
    // Build a fixed tree shape used across the kbd-nav tests:
    //   root_a (expanded)
    //     a_child_1 (collapsed leaf)
    //     a_child_2 (expanded)
    //       a_grandchild_1
    //   root_b (collapsed)
    //     b_child_1 (HIDDEN -- root_b is collapsed)
    //   root_c (leaf)
    let mut a_child_2 = TreeNode::new("a_child_2".to_string(), "A.2".to_string());
    a_child_2.expanded = true;
    a_child_2.add_child(TreeNode::new(
        "a_grandchild_1".to_string(),
        "A.2.1".to_string(),
    ));

    let mut root_a = TreeNode::new("root_a".to_string(), "A".to_string());
    root_a.expanded = true;
    root_a.add_child(TreeNode::new("a_child_1".to_string(), "A.1".to_string()));
    root_a.add_child(a_child_2);

    let mut root_b = TreeNode::new("root_b".to_string(), "B".to_string());
    // root_b stays collapsed; its child is not visible.
    root_b.add_child(TreeNode::new("b_child_1".to_string(), "B.1".to_string()));

    let root_c = TreeNode::new("root_c".to_string(), "C".to_string());

    let mut tv = TreeView::new(test_id());
    tv.add_root_node(root_a);
    tv.add_root_node(root_b);
    tv.add_root_node(root_c);
    tv
}

#[test]
fn tree_view_down_arrow_advances_visible_selection_when_focused() {
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // No prior selection -> first Down lands on the first visible node.
    let r1 = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(r1, EventResult::Consumed);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));

    // Subsequent Downs walk the visible-DFS order.
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("a_child_1"));
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("a_child_2"));
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("a_grandchild_1"));
}

#[test]
fn tree_view_up_arrow_clamps_at_first() {
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // Land on root_a, then Up should clamp at root_a (first visible).
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));
    let r = tv.handle_event(&key_press(Key::Up), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));
}

#[test]
fn tree_view_down_arrow_skips_collapsed_descendants() {
    // root_b is collapsed; its child "b_child_1" must NOT appear in the
    // visible-DFS order. From "a_grandchild_1", Down should land on
    // "root_b" (next visible root), then Down again on "root_c".
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // Walk to a_grandchild_1.
    for _ in 0..4 {
        let _ = tv.handle_event(&key_press(Key::Down), &theme);
    }
    assert_eq!(tv.selected_id().as_deref(), Some("a_grandchild_1"));

    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(
        tv.selected_id().as_deref(),
        Some("root_b"),
        "collapsed root_b's child must be skipped"
    );

    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_c"));
}

#[test]
fn tree_view_right_arrow_expands_collapsed_parent() {
    // Select root_b (collapsed), Right -> expand. Selection stays on
    // root_b but its children become visible.
    let expand_log: Rc<RefCell<Vec<(String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = expand_log.clone();
    let theme = default_theme();
    let mut tv = make_kbd_tree().with_on_expand(move |id, expanded| {
        cap.borrow_mut().push((id.to_string(), expanded));
    });
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // Walk to root_b.
    for _ in 0..5 {
        let _ = tv.handle_event(&key_press(Key::Down), &theme);
    }
    assert_eq!(tv.selected_id().as_deref(), Some("root_b"));

    let r = tv.handle_event(&key_press(Key::Right), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(
        expand_log.borrow().as_slice(),
        &[("root_b".to_string(), true)],
        "Right on collapsed parent fires on_expand(_, true)"
    );
    // Selection unchanged; node is now expanded -> Down reveals child.
    assert_eq!(tv.selected_id().as_deref(), Some("root_b"));
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("b_child_1"));
}

#[test]
fn tree_view_right_arrow_descends_into_expanded_parent() {
    // root_a is already expanded; Right from root_a should move
    // selection to first child a_child_1 (NOT toggle expand).
    let expand_log: Rc<RefCell<Vec<(String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = expand_log.clone();
    let theme = default_theme();
    let mut tv = make_kbd_tree().with_on_expand(move |id, expanded| {
        cap.borrow_mut().push((id.to_string(), expanded));
    });
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));

    let _ = tv.handle_event(&key_press(Key::Right), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("a_child_1"));
    assert!(
        expand_log.borrow().is_empty(),
        "Right on already-expanded parent must NOT fire on_expand"
    );
}

#[test]
fn tree_view_left_arrow_collapses_expanded_parent() {
    let expand_log: Rc<RefCell<Vec<(String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = expand_log.clone();
    let theme = default_theme();
    let mut tv = make_kbd_tree().with_on_expand(move |id, expanded| {
        cap.borrow_mut().push((id.to_string(), expanded));
    });
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));

    let r = tv.handle_event(&key_press(Key::Left), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(
        expand_log.borrow().as_slice(),
        &[("root_a".to_string(), false)],
        "Left on expanded parent fires on_expand(_, false)"
    );
    // After collapse, Down should now skip the descendants and land on
    // root_b directly.
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_b"));
}

#[test]
fn tree_view_left_arrow_ascends_to_parent_when_already_collapsed() {
    // From a_child_1 (a leaf), Left should NOT toggle expand (it has
    // no children) and should move selection to its parent root_a.
    let expand_log: Rc<RefCell<Vec<(String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = expand_log.clone();
    let theme = default_theme();
    let mut tv = make_kbd_tree().with_on_expand(move |id, expanded| {
        cap.borrow_mut().push((id.to_string(), expanded));
    });
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // root_a -> a_child_1.
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("a_child_1"));

    let _ = tv.handle_event(&key_press(Key::Left), &theme);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));
    assert!(
        expand_log.borrow().is_empty(),
        "Left on a leaf must not fire on_expand"
    );
}

#[test]
fn tree_view_home_jumps_to_first() {
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    // Walk a few steps in.
    for _ in 0..3 {
        let _ = tv.handle_event(&key_press(Key::Down), &theme);
    }
    assert_ne!(tv.selected_id().as_deref(), Some("root_a"));

    let r = tv.handle_event(&key_press(Key::Home), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(tv.selected_id().as_deref(), Some("root_a"));
}

#[test]
fn tree_view_end_jumps_to_last_visible() {
    // Last visible in the fixed tree is root_c.
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    let r = tv.handle_event(&key_press(Key::End), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(tv.selected_id().as_deref(), Some("root_c"));
}

#[test]
fn tree_view_enter_fires_selection_callback() {
    // Enter / Space "activate" the selected node by re-firing the
    // selection callback. The arrow-key navigation already fires the
    // callback once when changing selection, so an Enter on a stable
    // selection produces another callback invocation.
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = log.clone();
    let theme = default_theme();
    let mut tv = make_kbd_tree()
        .with_on_selection_change(move |id| cap.borrow_mut().push(id.to_string()));
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    tv.set_focused(true);

    let _ = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(log.borrow().as_slice(), &["root_a".to_string()]);

    let r = tv.handle_event(&key_press(Key::Enter), &theme);
    assert_eq!(r, EventResult::Consumed);
    assert_eq!(
        log.borrow().as_slice(),
        &["root_a".to_string(), "root_a".to_string()],
        "Enter re-fires selection callback for the current node"
    );

    let r2 = tv.handle_event(&key_press(Key::Space), &theme);
    assert_eq!(r2, EventResult::Consumed);
    assert_eq!(log.borrow().len(), 3, "Space behaves like Enter");
}

#[test]
fn tree_view_keys_ignored_when_unfocused() {
    // Without focus the kbd handler must return Ignored so sibling
    // widgets keep their keys. Selection must NOT change.
    let theme = default_theme();
    let mut tv = make_kbd_tree();
    tv.layout(Rect::new(0, 0, 200, 400), &theme);
    // tv is NOT focused.
    assert!(!tv.is_focused());

    let before = tv.selected_id();
    let r = tv.handle_event(&key_press(Key::Down), &theme);
    assert_eq!(r, EventResult::Ignored);
    assert_eq!(tv.selected_id(), before, "selection unchanged when unfocused");

    // Same for Home/End/Enter.
    assert_eq!(tv.handle_event(&key_press(Key::Home), &theme), EventResult::Ignored);
    assert_eq!(tv.handle_event(&key_press(Key::End), &theme), EventResult::Ignored);
    assert_eq!(tv.handle_event(&key_press(Key::Enter), &theme), EventResult::Ignored);
}

// ============================================================================
// DockPanel
// ============================================================================
//
// DockPanel hides its DockNode tree and panels HashMap behind opaque
// methods. Externally we can:
//   - construct, layout, draw (draw not tested here -- CPU-only rule)
//   - add_panel / remove_panel and observe via callbacks
//   - splitter dragging via simulated MouseButton+MouseMove events
//   - focus discipline (wave-2 commit 2242794 made can_focus real)
//
// We can NOT directly observe:
//   - the dock-tree shape
//   - the panel-id -> position mapping
//   - splitter ratios (only that they survive a drag without panicking)
//   - active tab index
// So most tests below are "did the call complete without panic and
// did the callback fire?" rather than deep state assertions.

fn boxed_label(text: &str) -> Box<dyn Widget> {
    Box::new(Label::new(test_id(), text))
}

#[test]
fn dock_panel_creation_defaults() {
    let dp = DockPanel::new(test_id());
    assert!(dp.is_visible());
    assert!(dp.is_enabled());
    assert!(!dp.is_focused());
    // wave-2 commit 2242794 made this real: enabled+visible -> can_focus.
    assert!(dp.can_focus(), "default DockPanel is enabled+visible -> focusable");
}

#[test]
fn dock_panel_layout_sets_bounds() {
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.layout(Rect::new(10, 20, 800, 600), &theme);
    let b = dp.bounds();
    assert_eq!(b.x(), 10);
    assert_eq!(b.y(), 20);
    assert_eq!(b.width(), 800);
    assert_eq!(b.height(), 600);
}

#[test]
fn dock_panel_measure_returns_bounded_size() {
    let theme = default_theme();
    let dp = DockPanel::new(test_id());
    let s = dp.measure(&LayoutConstraints::bounded(640, 480), &theme);
    assert_eq!(s.width, 640);
    assert_eq!(s.height, 480);
    // Unbounded: defaults to 800x600.
    let s = dp.measure(&LayoutConstraints::UNBOUNDED, &theme);
    assert_eq!(s.width, 800);
    assert_eq!(s.height, 600);
}

#[test]
fn dock_panel_visibility_and_enabled_round_trip() {
    let mut dp = DockPanel::new(test_id());
    dp.set_visible(false);
    assert!(!dp.is_visible());
    dp.set_enabled(false);
    assert!(!dp.is_enabled());
}

#[test]
fn dock_panel_set_focused_round_trips() {
    // Wave-2 (commit 2242794) rectified focus discipline for this widget.
    // The widget_tests.rs file has equivalent tests; mirror them here so
    // a refactor of THIS file's coverage is self-contained.
    let mut dp = DockPanel::new(test_id());
    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", boxed_label("a")),
        DockPosition::Center,
    );
    assert!(!dp.is_focused());
    dp.set_focused(true);
    assert!(dp.is_focused());
    dp.set_focused(false);
    assert!(!dp.is_focused());
}

#[test]
fn dock_panel_can_focus_respects_disabled_and_invisible() {
    let mut dp = DockPanel::new(test_id());
    dp.set_enabled(false);
    assert!(!dp.can_focus(), "disabled DockPanel must not advertise focus");
    dp.set_enabled(true);
    dp.set_visible(false);
    assert!(!dp.can_focus(), "invisible DockPanel must not advertise focus");
}

#[test]
fn dock_panel_disabled_or_invisible_ignores_events() {
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", boxed_label("a")),
        DockPosition::Center,
    );
    dp.layout(Rect::new(0, 0, 800, 600), &theme);

    let click = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(50, 50),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    dp.set_visible(false);
    assert_eq!(dp.handle_event(&click, &theme), EventResult::Ignored);
    dp.set_visible(true);
    dp.set_enabled(false);
    assert_eq!(dp.handle_event(&click, &theme), EventResult::Ignored);
}

#[test]
fn dock_panel_empty_handles_events_without_panic() {
    // No panels -> root_node is Empty -> handle_node_event returns Ignored.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.layout(Rect::new(0, 0, 400, 300), &theme);

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
        Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position: Point::new(10, 10),
            pressed: false,
            modifiers: Modifiers::empty(),
        }),
        Event::KeyPress(KeyPressEvent {
            key: Key::Tab,
            modifiers: Modifiers::empty(),
            repeat: false,
        }),
        Event::Update,
    ];
    for ev in &events {
        // No assertion on Consumed vs Ignored -- the floating-window pass
        // and tab-control delegation choose. The point is "no panic."
        let _ = dp.handle_event(ev, &theme);
    }
}

#[test]
fn dock_panel_add_panel_fires_layout_change_callback() {
    let count: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    let cap = count.clone();

    let theme = default_theme();
    let mut dp = DockPanel::new(test_id())
        .with_on_layout_change(move || {
            *cap.borrow_mut() += 1;
        });
    dp.layout(Rect::new(0, 0, 800, 600), &theme);

    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", boxed_label("a")),
        DockPosition::Center,
    );
    assert_eq!(*count.borrow(), 1);

    dp.add_panel(
        DockablePanel::new("p1", "Panel 1", boxed_label("b")),
        DockPosition::Right,
    );
    assert_eq!(*count.borrow(), 2);

    dp.add_panel(
        DockablePanel::new("p2", "Panel 2", boxed_label("c")),
        DockPosition::Floating,
    );
    assert_eq!(*count.borrow(), 3, "even Floating placement fires layout change");
}

#[test]
fn dock_panel_remove_panel_returns_panel_and_fires_callback() {
    let count: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    let cap = count.clone();

    let theme = default_theme();
    let mut dp = DockPanel::new(test_id())
        .with_on_layout_change(move || {
            *cap.borrow_mut() += 1;
        });
    dp.layout(Rect::new(0, 0, 800, 600), &theme);

    dp.add_panel(
        DockablePanel::new("p0", "Panel 0", boxed_label("a")),
        DockPosition::Center,
    );
    let added_calls = *count.borrow(); // 1
    let removed = dp.remove_panel("p0");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "p0");
    assert_eq!(*count.borrow(), added_calls + 1, "remove fires layout-change too");

    // Removing an absent id returns None and does NOT fire the callback,
    // because the function exits early at panels.remove(id)?.
    let before = *count.borrow();
    let res = dp.remove_panel("does-not-exist");
    assert!(res.is_none());
    assert_eq!(*count.borrow(), before, "absent id must not fire callback");
}

#[test]
fn dock_panel_add_in_each_position_does_not_panic() {
    // Exercise all five docked positions plus Floating. Each branch in
    // add_to_dock_tree mutates root_node in a different way; verify they
    // all complete and a follow-up layout works.
    //
    // Order matters less now that Center-after-Split descends into the
    // first leaf-Tabs (see `dock_panel_center_after_split_descends_to_first_leaf`),
    // but we still start with Center on Empty -> Tabs root, then wrap
    // with the other positions. The Floating add at the end exercises
    // the floating-windows code path, separate from the dock tree.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);

    let positions = [
        ("center", DockPosition::Center),
        ("right", DockPosition::Right),
        ("left", DockPosition::Left),
        ("top", DockPosition::Top),
        ("bottom", DockPosition::Bottom),
        ("floating", DockPosition::Floating),
    ];
    for (id, pos) in positions {
        dp.add_panel(DockablePanel::new(id, id, boxed_label(id)), pos);
    }
    // Layout again with all six panels in the tree -- the layout pass
    // recurses through every Split branch.
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);
}

#[test]
fn dock_panel_center_after_split_descends_to_first_leaf() {
    // Adding any non-Floating non-Center position to a non-empty root
    // wraps the existing tree into a Split. Once the root is a Split,
    // a follow-up Center add must land somewhere — historically (Mojo
    // port) it silently leaked, then it panicked with a clear message,
    // and now it descends into Split.first recursively until it finds
    // a Tabs leaf and appends. This matches VS Code / JetBrains /
    // Eclipse: the "main" panel area is conventionally the first leaf,
    // and hosts that want a specific destination pass an explicit
    // Right/Left/Top/Bottom/Floating position.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);

    // First add: Center on Empty -> root becomes Tabs.
    dp.add_panel(
        DockablePanel::new("c", "C", boxed_label("c")),
        DockPosition::Center,
    );
    // Second add: Right on Tabs -> root becomes Split { first: Tabs[c], second: Tabs[r] }.
    dp.add_panel(
        DockablePanel::new("r", "R", boxed_label("r")),
        DockPosition::Right,
    );
    // Third add: Center on Split -> descend into first (Tabs[c]),
    // append "c2". Tree becomes Split { first: Tabs[c, c2], second: Tabs[r] }.
    dp.add_panel(
        DockablePanel::new("c2", "C2", boxed_label("c2")),
        DockPosition::Center,
    );

    let first_leaf = dp.first_leaf_panels_for_test();
    assert_eq!(
        first_leaf,
        vec!["c".to_string(), "c2".to_string()],
        "Center add to Split root must append to first leaf-Tabs in order"
    );

    // Layout completes without panic.
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);
}

#[test]
fn dock_panel_center_descends_through_nested_splits() {
    // Two nested splits -- Center should descend Split.first.first until
    // it lands on a Tabs leaf. Verifies the recursion isn't depth-1.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);

    dp.add_panel(
        DockablePanel::new("a", "A", boxed_label("a")),
        DockPosition::Center,
    );
    dp.add_panel(
        DockablePanel::new("b", "B", boxed_label("b")),
        DockPosition::Right,
    );
    // After this Top, root = Split[V] { first: Tabs[ttop], second: Split[H] { first: Tabs[a], second: Tabs[b] } }
    // Wait: Top wraps the whole root, so root = Split[V] { first: Tabs[ttop], second: <previous root> }.
    // The first-leaf chain now points at "ttop", not "a".
    dp.add_panel(
        DockablePanel::new("ttop", "T", boxed_label("ttop")),
        DockPosition::Top,
    );
    // Center add descends Split.first -> Tabs[ttop]. Appends "c".
    dp.add_panel(
        DockablePanel::new("c", "C", boxed_label("c")),
        DockPosition::Center,
    );

    assert_eq!(
        dp.first_leaf_panels_for_test(),
        vec!["ttop".to_string(), "c".to_string()],
        "Center descends Split.first.first... to the first reachable Tabs leaf"
    );
}

#[test]
fn dock_panel_splitter_drag_does_not_panic() {
    // Add a left dock and a center dock to create a Horizontal splitter.
    // Then click on the splitter and drag. The path-based ratio update
    // (replacing the deleted *mut DockNode -- see wave-2 commit 2242794)
    // must complete without panic and without UB.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.add_panel(
        DockablePanel::new("center", "Center", boxed_label("c")),
        DockPosition::Center,
    );
    dp.add_panel(
        DockablePanel::new("left", "Left", boxed_label("l")),
        DockPosition::Left,
    );
    // Layout to populate splitter_rects. Bounds chosen large enough that
    // the splitter rect at ratio=0.3 is at x ~= 300.
    let bounds = Rect::new(0, 0, 1000, 600);
    dp.layout(bounds, &theme);

    // The horizontal splitter (Left vs the rest) is at split_x ~= 300,
    // splitter_size=4 so its rect is roughly Rect(298, 0, 4, 600).
    // Click on (300, 300) which is well within the splitter.
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(300, 300),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = dp.handle_event(&press, &theme);
    assert_eq!(res, EventResult::Consumed, "splitter press should be consumed");

    // Drag to a new position. The ratio update must not panic.
    let drag = Event::MouseMove(MouseMoveEvent {
        position: Point::new(500, 300),
        delta: Point::new(200, 0),
        modifiers: Modifiers::empty(),
    });
    let res = dp.handle_event(&drag, &theme);
    assert_eq!(res, EventResult::Consumed, "drag while splitter active is consumed");

    // Release.
    let release = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(500, 300),
        pressed: false,
        modifiers: Modifiers::empty(),
    });
    let _ = dp.handle_event(&release, &theme);
    // After release, a follow-up MouseMove must NOT keep dragging the
    // splitter. The splitter is no longer active.
    let stray_move = Event::MouseMove(MouseMoveEvent {
        position: Point::new(700, 300),
        delta: Point::new(200, 0),
        modifiers: Modifiers::empty(),
    });
    // Whatever the tab/content handlers return, we just want no panic.
    let _ = dp.handle_event(&stray_move, &theme);

    // Re-layout after the drag -- this exercises the ratio path. Must
    // not panic, even with the new ratio in effect.
    dp.layout(bounds, &theme);
}

#[test]
fn dock_panel_floating_window_titlebar_drag_does_not_panic() {
    // A floating window starts at bounds (100,100,400,300) per the impl.
    // Click in the title bar (top 25 px) and drag.
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());
    dp.add_panel(
        DockablePanel::new("f", "Floater", boxed_label("x")),
        DockPosition::Floating,
    );
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);

    // Title bar: Rect(100, 100, 400, 25). Click center of titlebar at
    // (300, 110).
    let press = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(300, 110),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = dp.handle_event(&press, &theme);
    assert_eq!(
        res,
        EventResult::Consumed,
        "floating-window titlebar press is consumed"
    );

    // Drag the floating window. The handler updates window.bounds.
    let drag = Event::MouseMove(MouseMoveEvent {
        position: Point::new(400, 200),
        delta: Point::new(100, 90),
        modifiers: Modifiers::empty(),
    });
    let res = dp.handle_event(&drag, &theme);
    assert_eq!(res, EventResult::Consumed, "drag while floating-window dragging");

    // Release.
    let release = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(400, 200),
        pressed: false,
        modifiers: Modifiers::empty(),
    });
    let _ = dp.handle_event(&release, &theme);

    // Follow-up layout must still work -- no UB from the drag, no
    // dangling pointers (the wave-2 cleanup deleted DropTarget's *mut).
    dp.layout(Rect::new(0, 0, 1024, 768), &theme);
}

#[test]
fn dockable_panel_builders_round_trip() {
    let p = DockablePanel::new("id", "Title", boxed_label("body"));
    assert_eq!(p.id, "id");
    assert_eq!(p.title, "Title");
    assert!(p.can_close, "default can_close is true");
    assert!(p.can_float, "default can_float is true");
    assert_eq!(p.icon, None);

    let p2 = DockablePanel::new("a", "A", boxed_label("body"))
        .with_icon("file")
        .with_can_close(false)
        .with_can_float(false);
    assert_eq!(p2.icon.as_deref(), Some("file"));
    assert!(!p2.can_close);
    assert!(!p2.can_float);
}

#[test]
fn dock_position_equality() {
    // DockPosition derives PartialEq; locking variants in.
    assert_eq!(DockPosition::Left, DockPosition::Left);
    assert_ne!(DockPosition::Left, DockPosition::Right);
    assert_ne!(DockPosition::Top, DockPosition::Bottom);
    assert_ne!(DockPosition::Center, DockPosition::Floating);
}

#[test]
fn dock_panel_remove_collapses_split_when_one_side_emptied() {
    // Add Left + Center to get a horizontal split. Remove the Left
    // panel. The dock tree should collapse the split into the surviving
    // Center subtree. We can't read the tree directly, but a follow-up
    // layout + add must keep working without panic, and the layout-change
    // callback must fire on the remove.
    let count: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    let cap = count.clone();
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id())
        .with_on_layout_change(move || {
            *cap.borrow_mut() += 1;
        });
    dp.layout(Rect::new(0, 0, 800, 600), &theme);

    dp.add_panel(
        DockablePanel::new("center", "Center", boxed_label("c")),
        DockPosition::Center,
    );
    dp.add_panel(
        DockablePanel::new("left", "Left", boxed_label("l")),
        DockPosition::Left,
    );
    let before = *count.borrow();
    let removed = dp.remove_panel("left");
    assert!(removed.is_some());
    assert_eq!(*count.borrow(), before + 1);

    // Re-layout collapses cleanly.
    dp.layout(Rect::new(0, 0, 800, 600), &theme);

    // Add another panel after the collapse to verify the tree is sane.
    dp.add_panel(
        DockablePanel::new("right", "Right", boxed_label("r")),
        DockPosition::Right,
    );
    dp.layout(Rect::new(0, 0, 800, 600), &theme);
}

#[test]
fn dock_panel_tab_strip_click_switches_active_panel() {
    // HANDOFF_2026-04-28 #2: clicking the "secondary.rs" tab in Tab 5
    // of kitchen_sink doesn't switch panels. Reproduce the same dock
    // tree (Center "main" -> Right -> Bottom -> Center "secondary") and
    // simulate a click on the second tab in the first leaf-Tabs node.
    // The active_index of that leaf must flip from 0 to 1, AND the
    // change must survive a subsequent layout (which calls set_tabs).
    let theme = default_theme();
    let mut dp = DockPanel::new(test_id());

    dp.add_panel(
        DockablePanel::new("main", "main.rs", boxed_label("main")),
        DockPosition::Center,
    );
    dp.add_panel(
        DockablePanel::new("sidebar", "Sidebar", boxed_label("side")),
        DockPosition::Right,
    );
    dp.add_panel(
        DockablePanel::new("console", "Console", boxed_label("con")),
        DockPosition::Bottom,
    );
    dp.add_panel(
        DockablePanel::new("secondary", "secondary.rs", boxed_label("sec")),
        DockPosition::Center,
    );

    // Mirror kitchen_sink's Containers tab layout: dock placed at a
    // non-zero origin inside the content area. Reproduces the actual
    // user-facing case where bounds are not (0,0).
    let bounds = Rect::new(8, 88, 600, 500);
    dp.layout(bounds, &theme);

    assert_eq!(
        dp.first_leaf_panels_for_test(),
        vec!["main".to_string(), "secondary".to_string()],
        "precondition: first leaf has [main, secondary]"
    );
    assert_eq!(
        dp.first_leaf_active_index_for_test(),
        Some(0),
        "precondition: starts on the first tab"
    );

    // First leaf-Tabs lives at: V-split ratio 0.7 of 500 = 350 high
    // (top half), H-split ratio 0.7 of 600 = 420 wide (left half).
    // First leaf rect = (8, 88, 420, 350). Tab strip is at top with
    // tab_height = font + pad*2 = 14 + 16 = 30. Two tabs: tab_width =
    // (420/2).max(84).min(196) = 196. tab[1] starts at x=8+196=204.
    // Click at (250, 100) hits the second tab.
    //
    // Issue the mouse-move first to cache hover position; the actual
    // GUI loop does this and downstream hover_close gating depends
    // on it.
    let move_to_secondary = Event::MouseMove(MouseMoveEvent {
        position: Point::new(250, 100),
        delta: Point::ZERO,
        modifiers: Modifiers::empty(),
    });
    let _ = dp.handle_event(&move_to_secondary, &theme);

    let click_secondary = Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(250, 100),
        pressed: true,
        modifiers: Modifiers::empty(),
    });
    let res = dp.handle_event(&click_secondary, &theme);
    assert_eq!(
        res,
        EventResult::Consumed,
        "tab-strip click on secondary.rs must be consumed"
    );
    assert_eq!(
        dp.first_leaf_active_index_for_test(),
        Some(1),
        "click on second tab must switch active_index to 1"
    );

    // Now re-layout (mimics what kitchen_sink does after every event).
    // The handoff suspects this loses the active-tab change because
    // set_tabs gets called with the same items each frame. Verify
    // active_index survives.
    dp.layout(bounds, &theme);
    assert_eq!(
        dp.first_leaf_active_index_for_test(),
        Some(1),
        "active_index must survive a layout (set_tabs round-trip)"
    );
}

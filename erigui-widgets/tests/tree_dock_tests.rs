//! Integration tests for TreeView (and -- in a follow-up commit -- DockPanel).
//!
//! TreeView had zero integration tests before this file. Same bar as
//! the wave-3 sibling files (simple_widgets_tests.rs,
//! modal_widgets_tests.rs): "no widget has zero tests anymore" -- not
//! exhaustive coverage. Where the public API doesn't expose enough
//! state for a real round-trip assertion, the test degenerates to
//! "construction + call doesn't panic," which is still a regression
//! net.
//!
//! TreeView is a hierarchical list with an explicit
//! `root_nodes: Vec<TreeNode>` field that's `pub`, so most state is
//! observable directly.

use erigui_core::{
    Event, EventResult, FocusEvent, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, MouseWheelEvent, Point, Rect, TextInputEvent, Theme, Widget,
    WidgetId,
};
use erigui_widgets::{TreeNode, TreeView};
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

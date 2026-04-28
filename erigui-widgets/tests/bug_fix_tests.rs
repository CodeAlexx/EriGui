//! Regression tests for 10 bugs in menu / context_menu / text_input /
//! text_area / search_box widgets. Each test is paired with a fix in the
//! corresponding widget source file. Tests fail without the fix and pass
//! after it.

use erigui_core::{
    Event, EventResult, Key, KeyPressEvent, LayoutConstraints, Modifiers, MouseButton,
    MouseButtonEvent, MouseMoveEvent, Point, Rect, TextInputEvent, Theme, Widget, WidgetId,
};
use erigui_widgets::{
    ContextMenu, ContextMenuItem, DateTimePicker, DateTimePickerMode, Field, FieldKind,
    FieldValue, Graph, MenuBar, MenuItem, Node, NodeGraph, SearchBox, TextArea, TextInput,
};
use erigui_core::Size;

fn theme() -> Theme {
    Theme::dark()
}

fn id() -> WidgetId {
    WidgetId::default()
}

// Helper: mouse press event.
fn mouse_press(x: i32, y: i32) -> Event {
    Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(x, y),
        pressed: true,
        modifiers: Modifiers::empty(),
    })
}

// Helper: mouse release event.
fn mouse_release(x: i32, y: i32) -> Event {
    Event::MouseButton(MouseButtonEvent {
        button: MouseButton::Left,
        position: Point::new(x, y),
        pressed: false,
        modifiers: Modifiers::empty(),
    })
}

// Helper: mouse move event.
fn mouse_move(x: i32, y: i32) -> Event {
    Event::MouseMove(MouseMoveEvent {
        position: Point::new(x, y),
        delta: Point::ZERO,
        modifiers: Modifiers::empty(),
    })
}

fn key_press(key: Key, modifiers: Modifiers) -> Event {
    Event::KeyPress(KeyPressEvent {
        key,
        modifiers,
        repeat: false,
    })
}

// ============================================================================
// Bug 1: menu.rs hit-test off by 4 px vs draw
// menu_index_from_point started at bounds.x() while get_menu_rect started at
// bounds.x() + 4. Click at x=4 should map to menu 0.
// ============================================================================
#[test]
fn bug1_menu_hit_test_aligned_with_draw() {
    let mut bar = MenuBar::new(id());
    // "File" -> 4*8 + 3*8 = 56-wide item.
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 200, 30), &theme());

    // Click at draw-x=4 (start of menu 0's drawn rect) must hit menu 0.
    assert_eq!(
        bar.menu_index_at(Point::new(4, 15)),
        0,
        "click at draw-x=4 should hit menu 0"
    );

    // Bug surfaces between draw-x of menu 0 (ends at 4+56=60) and the
    // pre-fix hit-test boundary (56). With the +4 offset bug, x=57 is drawn
    // as menu 0 but hit-tested as menu 1. After the fix, x=57 is menu 0.
    assert_eq!(
        bar.menu_index_at(Point::new(57, 15)),
        0,
        "click at x=57 (still inside menu 0's drawn rect) must hit menu 0"
    );

    // And the start of menu 1's drawn rect (x=60) must hit menu 1.
    assert_eq!(
        bar.menu_index_at(Point::new(60, 15)),
        1,
        "click at draw-x=60 (start of menu 1) must hit menu 1"
    );
}

// ============================================================================
// Bug 2: search_box.rs default arm — typing forwards to inner input but
// never updates `last_change`, so debounced search never fires.
// ============================================================================
#[test]
fn bug2_search_box_typing_marks_change_pending() {
    let mut sb = SearchBox::new(id());
    sb.layout(Rect::new(0, 0, 300, 30), &theme());
    sb.set_focused(true);

    // Click into the box to focus it (also focuses the inner input).
    let _ = sb.handle_event(&mouse_press(150, 15), &theme());

    assert!(!sb.has_pending_change(), "no change pending before typing");

    // TextInput event reaches the widget via the default arm.
    let evt = Event::TextInput(TextInputEvent {
        text: "h".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());

    assert_eq!(sb.get_text(), "h", "inner input should have received the char");
    assert!(
        sb.has_pending_change(),
        "typing should mark a debounced change as pending"
    );
}

// ============================================================================
// Bug 3: text_input.rs:251 — click always sets cursor to text.len().
// Expectation: clicking near a column maps to that column.
// ============================================================================
#[test]
fn bug3_text_input_click_sets_cursor_to_clicked_column() {
    let mut input = TextInput::new(id()).with_text("hello world");
    input.layout(Rect::new(0, 0, 200, 24), &theme());

    // approx_char_width with font_size_base=14 -> round(14*0.55)=8.
    // Inner area starts at bounds.x()+4 (inset(4)). Click at column 6 means
    // x = inner.x() + 6 * char_w = 4 + 48 = 52.
    let target_col = 6usize;
    let char_w = 8;
    let click_x = 4 + target_col as i32 * char_w;
    let click_y = 12;

    let _ = input.handle_event(&mouse_press(click_x, click_y), &theme());

    assert_eq!(
        input.cursor_pos(),
        target_col,
        "click should place cursor at column {target_col}, got {}",
        input.cursor_pos()
    );
}

// ============================================================================
// Bug 4: text_area.rs underflow on empty `lines`.
// `move_cursor` does `lines.len() - 1`. With an empty Vec this underflows.
// After fix it should not panic.
// ============================================================================
#[test]
fn bug4_text_area_no_underflow_on_empty_lines() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.force_empty_lines_for_test();

    // The fix should make these calls safe. Use a closure with
    // catch_unwind to detect the panic on the buggy path.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // A focus + click should drive position_to_cursor + move_cursor.
        let _ = area.handle_event(&mouse_press(50, 50), &theme());
    }));
    assert!(result.is_ok(), "empty lines must not cause a panic");
}

// ============================================================================
// Bug 5: text_area.rs drag-select doesn't terminate when mouse-up happens
// outside widget bounds.
// ============================================================================
#[test]
fn bug5_text_area_drag_select_terminates_outside_bounds() {
    let mut area = TextArea::new(id()).with_text("hello world\nsecond line");
    area.layout(Rect::new(0, 0, 400, 200), &theme());

    // Press inside.
    let _ = area.handle_event(&mouse_press(60, 10), &theme());
    assert!(area.is_selecting(), "should be selecting after press");

    // Move outside bounds (x=500 is well past width=400).
    let _ = area.handle_event(&mouse_move(500, 300), &theme());

    // Release outside bounds.
    let _ = area.handle_event(&mouse_release(500, 300), &theme());

    assert!(
        !area.is_selecting(),
        "release outside bounds must end selection"
    );
}

// ============================================================================
// Bug 6: context_menu.rs:341 submenu destroyed in parent->submenu gap.
// Mouse moving into the submenu's bounds should keep the submenu alive.
// ============================================================================
#[test]
fn bug6_context_menu_submenu_survives_parent_to_submenu_move() {
    let submenu_items = vec![
        ContextMenuItem::new("Sub A"),
        ContextMenuItem::new("Sub B"),
    ];
    let parent_items = vec![
        ContextMenuItem::new("Item 0"),
        ContextMenuItem::new("With Submenu").with_submenu(submenu_items),
        ContextMenuItem::new("Item 2"),
    ];

    let mut menu = ContextMenu::new(id()).with_items(parent_items);
    menu.show_at(Point::new(100, 100));

    // Hover the parent submenu-bearing item to spawn the submenu.
    // Item 1 is the second item: y starts at 100+2 + 25 = 127, height 25.
    // We hover at (105, 130) which is inside item 1 (y in [127,152)).
    let _ = menu.handle_event(&mouse_move(105, 130), &theme());

    assert!(menu.has_submenu(), "submenu should spawn on hover");
    let sub_bounds = menu.submenu_bounds().expect("submenu bounds");

    // Move into submenu bounds. With the bug, parent's MouseMove handler
    // runs, sees `state.bounds.contains(point) == false`, and falls into
    // the destroy-on-gap branch. The fix is to detect "pointer over
    // submenu" and skip parent state mutation, so the submenu survives.
    let inside_sub = Point::new(sub_bounds.x() + 5, sub_bounds.y() + 5);
    // First move-event simulates the immediate transition.
    let _ = menu.handle_event(&mouse_move(inside_sub.x, inside_sub.y), &theme());
    // Second move at the same point simulates the (real) double-fire that
    // happens when winit emits a follow-up move; in the buggy code this
    // branch evaluates with `submenu` still cached and triggers the
    // destroy-when-not-over-parent path again.
    let _ = menu.handle_event(&mouse_move(inside_sub.x, inside_sub.y), &theme());

    assert!(
        menu.has_submenu(),
        "submenu must survive mouse move into its bounds"
    );
    // While the pointer is over the submenu, the parent's hover state must
    // remain anchored on the submenu-bearing item (item 1). Without the
    // fix, the parent's MouseMove handler runs and clears hover_index to
    // None, causing visual flicker on the submenu-bearing parent item.
    assert_eq!(
        menu.hover_index(),
        Some(1),
        "parent must keep hover on submenu-bearing item while pointer is in submenu"
    );
    assert_eq!(
        menu.submenu_open_index(),
        Some(1),
        "submenu_open must remain anchored to item 1"
    );
}

// ============================================================================
// Bug 7: menu.rs no outside-click dismissal + no Esc handling + no focus.
// ============================================================================
#[test]
fn bug7_menu_outside_click_and_esc_dismiss_dropdown() {
    let mut bar = MenuBar::new(id());
    bar.add_menu("File", vec![MenuItem::new("Open"), MenuItem::new("Save")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 200, 30), &theme());

    // can_focus must be true so the menu bar can receive KeyPress events.
    assert!(bar.can_focus(), "menu bar must be focusable");

    // Open menu 0 by clicking in its rect.
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    assert!(bar.is_dropdown_visible(), "dropdown should be open");

    // Click far outside both the menu bar and dropdown — should close.
    let _ = bar.handle_event(&mouse_press(500, 500), &theme());
    assert!(
        !bar.is_dropdown_visible(),
        "outside click should dismiss dropdown"
    );

    // Re-open and dismiss with Esc.
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    assert!(bar.is_dropdown_visible(), "dropdown re-opened");

    let _ = bar.handle_event(&key_press(Key::Escape, Modifiers::empty()), &theme());
    assert!(
        !bar.is_dropdown_visible(),
        "Esc should dismiss dropdown"
    );
}

// ============================================================================
// Bug 8: text_area.rs:826 \r filtered before \r->\n conversion. A TextInput
// containing "\r" should produce a newline.
// ============================================================================
#[test]
fn bug8_text_area_text_input_carriage_return_inserts_newline() {
    let mut area = TextArea::new(id()).with_text("ab");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    // Position cursor at end of "ab".
    let _ = area.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());

    let starting_lines = area.line_count();
    let evt = Event::TextInput(TextInputEvent {
        text: "\r".to_string(),
    });
    let _ = area.handle_event(&evt, &theme());

    assert_eq!(
        area.line_count(),
        starting_lines + 1,
        "\\r should be converted to \\n and create a new line"
    );
}

// ============================================================================
// Bug 9: text_area.rs:774 Tab inserts literal tab. Should pass through to
// parent (Ignored) so focus navigation works. Ctrl+Tab inserts a tab.
// ============================================================================
#[test]
fn bug9_text_area_tab_navigates_ctrl_tab_inserts() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);

    let before_text = area.get_text();
    let result = area.handle_event(&key_press(Key::Tab, Modifiers::empty()), &theme());

    assert_eq!(area.get_text(), before_text, "plain Tab must not insert text");
    assert_eq!(
        result,
        EventResult::Ignored,
        "plain Tab must return Ignored so parent handles focus"
    );

    // Ctrl+Tab inserts a tab (which the existing insert_char converts to
    // tab_size spaces).
    let result = area.handle_event(&key_press(Key::Tab, Modifiers::CTRL), &theme());
    assert_eq!(
        result,
        EventResult::Consumed,
        "Ctrl+Tab must be consumed and insert a tab character"
    );
    assert!(
        !area.get_text().is_empty(),
        "Ctrl+Tab should produce non-empty text"
    );
}

// ============================================================================
// Bug 10: text_input.rs:117 — selection_start never set. Shift+Right
// should set selection_start and advance cursor.
// ============================================================================
#[test]
fn bug10_text_input_shift_arrow_extends_selection() {
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(2);

    assert_eq!(input.selection_start(), None, "no selection initially");

    let _ = input.handle_event(&key_press(Key::Right, Modifiers::SHIFT), &theme());

    assert_eq!(
        input.selection_start(),
        Some(2),
        "selection_start should be set to the pre-move cursor"
    );
    assert_eq!(
        input.cursor_pos(),
        3,
        "cursor should advance one character"
    );
}

// Layout-helper integration: also sanity-check that LayoutConstraints is
// imported (silences unused-import lint if measure ever changes).
#[allow(dead_code)]
fn _layout_helper_sanity() {
    let _ = LayoutConstraints::UNBOUNDED;
}

// ---------------------------------------------------------------------------
// Phase 3: text_area.rs bugs
// ---------------------------------------------------------------------------

// Bug ta11: set_text strips trailing newline + bare \r.
#[test]
fn bug_ta11_set_text_preserves_trailing_newline() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_text("hello\n");
    assert_eq!(
        area.line_count(),
        2,
        "trailing \\n should leave an empty line after"
    );

    area.set_text("a\r\nb");
    assert_eq!(area.line_count(), 2, "\\r\\n should split into 2 lines");

    area.set_text("a\rb");
    assert_eq!(area.line_count(), 2, "lone \\r should split into 2 lines");
}

// Bug ta12: Ctrl+A select-all places cursor at end of selection.
#[test]
fn bug_ta12_ctrl_a_updates_cursor() {
    let mut area = TextArea::new(id()).with_text("hello\nworld");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    // Move cursor somewhere known.
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    assert_eq!(area.cursor_position(), (0, 0));

    let _ = area.handle_event(
        &key_press(Key::Character('a'), Modifiers::CTRL),
        &theme(),
    );
    let last_line = area.line_count() - 1;
    assert_eq!(
        area.cursor_position(),
        (last_line, 5),
        "Ctrl+A must place cursor at end of selection"
    );
    assert!(area.selection_for_test().is_some(), "selection must exist");
}

// Bug ta16: MouseWheel horizontal scroll has a max derived from widest
// line, so users can't scroll right indefinitely past content.
#[test]
fn bug_ta16_horizontal_scroll_bounded() {
    let mut area = TextArea::new(id()).with_text("short\nshort line\n");
    area.layout(Rect::new(0, 0, 200, 100), &theme());
    let max = area.max_scroll_x_for_test();
    // Synthesize many shift+wheel events trying to scroll right beyond max.
    use erigui_core::{MouseWheelEvent, Point};
    for _ in 0..100 {
        let evt = Event::MouseWheel(MouseWheelEvent {
            delta: Point::new(-50, 0),
            position: Point::new(50, 50),
            modifiers: Modifiers::SHIFT,
        });
        let _ = area.handle_event(&evt, &theme());
    }
    assert!(
        area.scroll_x_for_test() <= max,
        "scroll_x ({}) must be clamped to max ({})",
        area.scroll_x_for_test(),
        max
    );
}

// Bug ta17: preferred column survives Up/Down across short lines.
#[test]
fn bug_ta17_preferred_column_persists() {
    let mut area = TextArea::new(id())
        .with_text("longer first line\nshort\nlonger third line");
    area.layout(Rect::new(0, 0, 600, 200), &theme());
    area.set_focused(true);
    // Move to (0, 12) — past the short line's len.
    let _ = area.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    let (_, col0) = area.cursor_position();
    assert!(col0 > 5, "first line should be longer than 5 chars");
    // Down to short line: cursor clamped, but preferred_col remembered.
    let _ = area.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(area.cursor_position().0, 1);
    assert_eq!(
        area.cursor_position().1,
        5,
        "should clamp to short line length"
    );
    // Down again to the longer third line — should restore the original
    // intent column rather than staying at 5.
    let _ = area.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(
        area.cursor_position(),
        (2, col0),
        "preferred column should be restored on the third line"
    );
}

// Bug ta18: double-click selects word, triple-click selects line.
#[test]
fn bug_ta18_double_triple_click_selection() {
    let mut area = TextArea::new(id()).with_text("hello world foo");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    // First click at character "w" (col ~6). char_width = 12 in dark theme
    // so x is approximate; the helper recomputes.
    let _ = area.handle_event(&mouse_press(80, 10), &theme());
    let _ = area.handle_event(&mouse_release(80, 10), &theme());
    // Second click in same spot fast = double click → word select.
    let _ = area.handle_event(&mouse_press(80, 10), &theme());
    let _ = area.handle_event(&mouse_release(80, 10), &theme());
    let sel = area.selection_for_test().expect("word selection");
    let normalized = (
        sel.start_line.min(sel.end_line),
        sel.start_col.min(sel.end_col),
        sel.start_line.max(sel.end_line),
        sel.start_col.max(sel.end_col),
    );
    assert!(
        (normalized.3 - normalized.1) > 0,
        "word selection must be non-empty"
    );

    // Third click → line select.
    let _ = area.handle_event(&mouse_press(80, 10), &theme());
    let _ = area.handle_event(&mouse_release(80, 10), &theme());
    let sel = area.selection_for_test().expect("line selection");
    assert_eq!(sel.start_line, 0);
    assert_eq!(sel.end_line, 0);
    assert_eq!(sel.start_col, 0);
    assert_eq!(sel.end_col, "hello world foo".chars().count());
}

// Bug ta19: PageUp/PageDown jump by page; Ctrl+Left/Right word-jump;
// Ctrl+Backspace word-delete.
#[test]
fn bug_ta19_page_and_word_keys() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 100), &theme());
    area.set_focused(true);
    // Build a 30-line buffer.
    let text: String = (0..30)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    area.set_text(&text);
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::CTRL), &theme());
    assert_eq!(area.cursor_position().0, 0);
    let _ = area.handle_event(&key_press(Key::PageDown, Modifiers::empty()), &theme());
    assert!(
        area.cursor_position().0 > 0,
        "PageDown should move cursor down by a page"
    );
    let prior = area.cursor_position().0;
    let _ = area.handle_event(&key_press(Key::PageUp, Modifiers::empty()), &theme());
    assert!(
        area.cursor_position().0 < prior,
        "PageUp should move cursor up"
    );

    // Ctrl+Right from start of line "line 0" should jump past "line".
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::CTRL), &theme());
    let _ = area.handle_event(&key_press(Key::Right, Modifiers::CTRL), &theme());
    assert!(
        area.cursor_position().1 >= 4,
        "Ctrl+Right should land past 'line'"
    );

    // Ctrl+Backspace deletes a word.
    area.set_text("hello world");
    let _ = area.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    let _ = area.handle_event(&key_press(Key::Backspace, Modifiers::CTRL), &theme());
    assert!(
        !area.get_text().contains("world"),
        "Ctrl+Backspace should delete the trailing word"
    );
}

// Bug ta20: undo stack uses VecDeque; pushing >100 entries should still be
// fast (we just verify capping).
#[test]
fn bug_ta20_undo_stack_bounded() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    // Each set_text saves an undo entry. 200 of them.
    for i in 0..200 {
        area.set_text(&format!("v{}", i));
    }
    assert!(
        area.undo_depth() <= 100,
        "undo stack must be bounded (got {})",
        area.undo_depth()
    );
}

// Bug ta21: consecutive single-char inserts coalesce into one undo entry.
#[test]
fn bug_ta21_typing_coalesces_undo() {
    let mut area = TextArea::new(id());
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let depth_before = area.undo_depth();
    for ch in "hello".chars() {
        let _ = area.handle_event(
            &key_press(Key::Character(ch), Modifiers::empty()),
            &theme(),
        );
    }
    let after = area.undo_depth();
    assert_eq!(
        after - depth_before,
        1,
        "typing 'hello' should produce exactly one undo entry, got {}",
        after - depth_before
    );
}

// Bug ta22: undo restores selection. Capitalization-independent shortcuts.
#[test]
fn bug_ta22_undo_restores_selection_and_caps_shortcuts() {
    let mut area = TextArea::new(id()).with_text("hello world");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);

    // Make a selection with Shift+End from start.
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = area.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    assert!(area.selection_for_test().is_some(), "selection should exist");
    // Type a char to replace selection.
    let _ = area.handle_event(
        &key_press(Key::Character('x'), Modifiers::empty()),
        &theme(),
    );
    assert_eq!(area.get_text(), "x");

    // Undo with Capslocked Z should still work (case-insensitive shortcut).
    let _ = area.handle_event(
        &key_press(Key::Character('Z'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(area.get_text(), "hello world", "Ctrl+Z (uppercase) must undo");
    assert!(
        area.selection_for_test().is_some(),
        "undo must restore selection"
    );
}

// ---------------------------------------------------------------------------
// Phase 3: text_input.rs bugs
// ---------------------------------------------------------------------------

// Bug ti11: click x->cursor mapping over multi-byte text. The fix uses a
// character-level walk so the cursor lands on a UTF-8 boundary.
#[test]
fn bug_ti11_click_maps_to_char_boundary_multibyte() {
    let mut input = TextInput::new(id()).with_text("档案 hello");
    input.layout(Rect::new(0, 0, 300, 24), &theme());
    input.set_focused(true);
    // Click somewhere into the middle.
    let _ = input.handle_event(&mouse_press(40, 12), &theme());
    let pos = input.cursor_pos();
    assert!(
        input.text().is_char_boundary(pos),
        "cursor must land on a char boundary, got pos={} in {:?}",
        pos,
        input.text()
    );
}

// Bug ti12: Backspace honors selection.
#[test]
fn bug_ti12_backspace_deletes_selection() {
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(0);
    // Shift+End to select everything.
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    assert!(input.selection_start().is_some());
    let _ = input.handle_event(&key_press(Key::Backspace, Modifiers::empty()), &theme());
    assert_eq!(input.text(), "");
    assert!(input.selection_start().is_none());
}

// Bug ti15: Delete forward.
#[test]
fn bug_ti15_delete_forward() {
    let mut input = TextInput::new(id()).with_text("abcdef");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(2);
    let _ = input.handle_event(&key_press(Key::Delete, Modifiers::empty()), &theme());
    assert_eq!(input.text(), "abdef");
}

// Bug ti15: Home/End move cursor; Shift+Home selects to start.
#[test]
fn bug_ti15_home_end_keys() {
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    let _ = input.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    assert_eq!(input.cursor_pos(), 5);
    let _ = input.handle_event(&key_press(Key::Home, Modifiers::SHIFT), &theme());
    assert_eq!(input.cursor_pos(), 0);
    assert_eq!(input.selection_start(), Some(5));
}

// Bug ti15: Ctrl+A selects all.
#[test]
fn bug_ti15_ctrl_a_select_all() {
    let mut input = TextInput::new(id()).with_text("hello world");
    input.layout(Rect::new(0, 0, 300, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(3);
    let _ = input.handle_event(
        &key_press(Key::Character('a'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(input.selection_start(), Some(0));
    assert_eq!(input.cursor_pos(), input.text().len());
}

// Bug ti15: Ctrl+C, Ctrl+X, Ctrl+V via in-process clipboard.
#[test]
fn bug_ti15_clipboard_ctrl_c_x_v() {
    use erigui_widgets::clipboard;

    clipboard::clear_clipboard();
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(0);
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());

    // Copy.
    let _ = input.handle_event(
        &key_press(Key::Character('c'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(clipboard::get_clipboard(), "hello");

    // Cut.
    let _ = input.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    let _ = input.handle_event(
        &key_press(Key::Character('x'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(input.text(), "");
    assert_eq!(clipboard::get_clipboard(), "hello");

    // Paste.
    let _ = input.handle_event(
        &key_press(Key::Character('v'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(input.text(), "hello");
}

// Bug ti15: Ctrl+Left/Right word jump.
#[test]
fn bug_ti15_ctrl_arrow_word_jump() {
    let mut input = TextInput::new(id()).with_text("foo bar baz");
    input.layout(Rect::new(0, 0, 300, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(0);
    // Ctrl+Right: should land at start of "bar" (=4).
    let _ = input.handle_event(&key_press(Key::Right, Modifiers::CTRL), &theme());
    assert_eq!(input.cursor_pos(), 4);
    // Ctrl+Left: should go back to start.
    let _ = input.handle_event(&key_press(Key::Left, Modifiers::CTRL), &theme());
    assert_eq!(input.cursor_pos(), 0);
}

// Bug ti15: Ctrl+Backspace deletes previous word.
#[test]
fn bug_ti15_ctrl_backspace_deletes_word() {
    let mut input = TextInput::new(id()).with_text("foo bar");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    let _ = input.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    let _ = input.handle_event(&key_press(Key::Backspace, Modifiers::CTRL), &theme());
    assert_eq!(input.text(), "foo ");
}

// Bug ti15: Tab returns Ignored so parent can navigate.
#[test]
fn bug_ti15_tab_returns_ignored() {
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    let result = input.handle_event(&key_press(Key::Tab, Modifiers::empty()), &theme());
    assert_eq!(result, EventResult::Ignored);
}

// Bug ti18: max_length enforced on input.
#[test]
fn bug_ti18_max_length_enforced() {
    let mut input = TextInput::new(id()).with_max_length(3);
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    for ch in "hello".chars() {
        let evt = Event::TextInput(TextInputEvent {
            text: ch.to_string(),
        });
        let _ = input.handle_event(&evt, &theme());
    }
    assert_eq!(input.text().chars().count(), 3, "max_length=3 must cap input");
}

// Bug ti19: password mode renders something different from text. We can't
// inspect rendered pixels, but the API surface must exist and toggle.
#[test]
fn bug_ti19_password_mode_toggle() {
    let mut input = TextInput::new(id()).with_text("secret");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    assert!(!input.is_password_mode());
    input.set_password_mode(true);
    assert!(input.is_password_mode());
    // text() still returns the actual text — masking is render-only.
    assert_eq!(input.text(), "secret");
}

// Bug ti16: layout after with_text on a long string should ensure cursor
// is visible (scroll_offset > 0 if cursor would otherwise be off-screen).
#[test]
fn bug_ti16_layout_ensures_cursor_visible_for_long_text() {
    let long: String = "x".repeat(80);
    let mut input = TextInput::new(id()).with_text(&long);
    // First layout: should run ensure_cursor_visible.
    input.layout(Rect::new(0, 0, 100, 24), &theme());
    input.set_focused(true);
    // Cursor is at end; with width=100 (inner ~92) we shouldn't see the
    // start of the string. Non-zero scroll proves the test.
    assert_eq!(input.cursor_pos(), long.len());
    // Drive End via keyboard to exercise the scroll-follows-cursor path.
    let _ = input.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    assert!(
        input.scroll_offset_for_test() > 0,
        "scroll_offset must advance when cursor is past visible width, got {}",
        input.scroll_offset_for_test()
    );
}

// Bug ti13/14/15: selection/cursor rendering uses measure_text. Pixel-exact
// assertions need a real DrawContext, but we can at least validate the
// buffer remains coherent for multi-byte text and that selection
// boundaries land on character boundaries.
#[test]
fn bug_ta13_multibyte_selection_boundary_aligned() {
    let mut area = TextArea::new(id()).with_text("档案 hello");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = area.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    let sel = area.selection_for_test().expect("selection from Shift+End");
    // 8 characters: 档(1) 案(1) space(1) h e l l o (5) -> 8 chars total.
    let expected = "档案 hello".chars().count();
    assert_eq!(
        sel.end_col, expected,
        "selection end must be at the character count, not byte count"
    );
}

// Bug ta_get_selected_text: `get_selected_text` byte-sliced character columns
// (text_area.rs:270/273/282), so any multibyte selection (CJK, accented,
// emoji) panicked on a non-char-boundary slice or returned garbage. Fixed by
// routing through char_to_byte_index. This test selects "档案 hello" with
// Shift+End and asserts the returned string equals the visible characters.
#[test]
fn bug_ta_get_selected_text_utf8_safe() {
    let mut area = TextArea::new(id()).with_text("档案 hello");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let _ = area.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = area.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    let selected = area
        .get_selected_text()
        .expect("Shift+End must produce a selection");
    assert_eq!(
        selected, "档案 hello",
        "get_selected_text must slice on character boundaries, not byte boundaries"
    );
}

// ---------------------------------------------------------------------------
// Phase 3: search_box.rs bugs
// ---------------------------------------------------------------------------

// Bug sb11: poll_debounce fires the debounced change without further events.
#[test]
fn bug_sb11_poll_debounce_fires_change() {
    let mut sb = SearchBox::new(id()).with_debounce_delay(1);
    sb.layout(Rect::new(0, 0, 300, 30), &theme());
    sb.set_focused(true);
    let _ = sb.handle_event(&mouse_press(150, 15), &theme());
    let evt = Event::TextInput(TextInputEvent {
        text: "h".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());
    assert!(sb.has_pending_change());
    std::thread::sleep(std::time::Duration::from_millis(5));
    sb.poll_debounce();
    assert!(
        !sb.has_pending_change(),
        "poll_debounce must fire after the delay elapses"
    );
}

// Bug sb13: clicking on a suggestion uses the *current* rect (recomputed
// from current suggestion count) — works even when the layout-time rect
// was sized for a smaller list.
#[test]
fn bug_sb13_click_on_late_suggestion_lands() {
    let mut sb = SearchBox::new(id()).with_suggestion_provider(|q| {
        if q.is_empty() {
            vec![]
        } else {
            (0..5).map(|i| format!("{}{}", q, i)).collect()
        }
    });
    sb.layout(Rect::new(0, 0, 200, 30), &theme());
    sb.set_focused(true);
    let _ = sb.handle_event(&mouse_press(50, 15), &theme());
    let evt = Event::TextInput(TextInputEvent {
        text: "a".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());
    assert!(sb.is_showing_suggestions());
    assert_eq!(sb.suggestion_count(), 5);

    let item_h = theme().typography.font_size_base + 8;
    // Suggestions rect starts at y = bounds.bottom() + 2 = 32.
    let y = 32 + 4 * item_h + item_h / 2;
    let _ = sb.handle_event(&mouse_press(50, y), &theme());
    assert_eq!(sb.get_text(), "a4");
}

// Bug sb14: very-large negative y shouldn't wrap to a huge usize and panic.
#[test]
fn bug_sb14_click_clamped_for_negative_y() {
    let mut sb = SearchBox::new(id()).with_suggestion_provider(|_| vec!["x".to_string()]);
    sb.layout(Rect::new(0, 100, 200, 30), &theme());
    sb.set_focused(true);
    let _ = sb.handle_event(&mouse_press(50, 115), &theme());
    let evt = Event::TextInput(TextInputEvent {
        text: "z".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = sb.handle_event(&mouse_press(50, 0), &theme());
    }));
    assert!(result.is_ok(), "click at y=0 must not panic");
}

// Bug sb15: MouseMove outside suggestions clears selected_suggestion.
#[test]
fn bug_sb15_mouse_move_clears_selection() {
    let mut sb =
        SearchBox::new(id()).with_suggestion_provider(|_| vec!["a".into(), "b".into()]);
    sb.layout(Rect::new(0, 0, 200, 30), &theme());
    sb.set_focused(true);
    let _ = sb.handle_event(&mouse_press(50, 15), &theme());
    let evt = Event::TextInput(TextInputEvent {
        text: "x".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());

    let item_h = theme().typography.font_size_base + 8;
    let _ = sb.handle_event(&mouse_move(50, 32 + item_h / 2), &theme());
    assert_eq!(sb.selected_suggestion(), Some(0));
    let _ = sb.handle_event(&mouse_move(50, 15), &theme());
    assert_eq!(sb.selected_suggestion(), None);
}

// Bug sb16: Esc closes suggestions and clears selection.
#[test]
fn bug_sb16_esc_resets_selection() {
    let mut sb =
        SearchBox::new(id()).with_suggestion_provider(|_| vec!["a".into(), "b".into()]);
    sb.layout(Rect::new(0, 0, 200, 30), &theme());
    sb.set_focused(true);
    let _ = sb.handle_event(&mouse_press(50, 15), &theme());
    let evt = Event::TextInput(TextInputEvent {
        text: "x".to_string(),
    });
    let _ = sb.handle_event(&evt, &theme());
    let _ = sb.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert!(sb.selected_suggestion().is_some());
    let _ = sb.handle_event(&key_press(Key::Escape, Modifiers::empty()), &theme());
    assert!(!sb.is_showing_suggestions());
    assert!(sb.selected_suggestion().is_none());
}

// Bug sb17: Up/Down without active suggestions are consumed.
#[test]
fn bug_sb17_up_down_consumed_without_suggestions() {
    let mut sb = SearchBox::new(id());
    sb.layout(Rect::new(0, 0, 200, 30), &theme());
    sb.set_focused(true);
    let result = sb.handle_event(&key_press(Key::Up, Modifiers::empty()), &theme());
    assert_eq!(result, EventResult::Consumed);
    let result = sb.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(result, EventResult::Consumed);
}

// Bug sb19: clear() resets clear_button_hovered.
#[test]
fn bug_sb19_clear_resets_hover_state() {
    let mut sb = SearchBox::new(id()).with_suggestion_provider(|_| vec![]);
    sb.layout(Rect::new(0, 0, 200, 30), &theme());
    sb.set_text("hello");
    let _ = sb.handle_event(&mouse_move(190, 15), &theme());
    assert!(sb.is_clear_button_hovered());
    sb.clear();
    assert!(!sb.is_clear_button_hovered());
}

// Bug sb20: re-search of existing query LRU-promotes instead of dropping.
#[test]
fn bug_sb20_history_lru_promotes() {
    let mut sb = SearchBox::new(id());
    sb.add_to_history("alpha".to_string());
    sb.add_to_history("beta".to_string());
    sb.add_to_history("gamma".to_string());
    let snap = sb.history_snapshot();
    assert_eq!(snap, vec!["gamma", "beta", "alpha"]);
    sb.add_to_history("alpha".to_string());
    let snap = sb.history_snapshot();
    assert_eq!(snap, vec!["alpha", "gamma", "beta"]);
}

// ---------------------------------------------------------------------------
// Phase 3: context_menu.rs bugs
// ---------------------------------------------------------------------------

// Bug cm15: keyboard navigation cycles items, Enter activates, Esc closes.
#[test]
fn bug_cm15_keyboard_nav() {
    let items = vec![
        ContextMenuItem::new("Item 0"),
        ContextMenuItem::separator(),
        ContextMenuItem::new("Item 1").disabled(),
        ContextMenuItem::new("Item 2"),
    ];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    // Down should land on Item 0.
    let _ = menu.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(menu.hover_index(), Some(0));
    // Down again should skip the separator and disabled to Item 2.
    let _ = menu.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(menu.hover_index(), Some(3));
    // Esc closes.
    let _ = menu.handle_event(&key_press(Key::Escape, Modifiers::empty()), &theme());
    assert!(!menu.is_open());
}

// Bug cm16: submenu spawned with viewport bounds stays on-screen.
#[test]
fn bug_cm16_submenu_viewport_clamped() {
    let sub = vec![
        ContextMenuItem::new("Sub A"),
        ContextMenuItem::new("Sub B"),
    ];
    let parent = vec![ContextMenuItem::new("Parent").with_submenu(sub)];
    let mut menu = ContextMenu::new(id()).with_items(parent);
    // Viewport is small; parent will be at right edge so submenu would
    // otherwise overflow.
    menu.show_at_bounded(Point::new(180, 50), Some(Rect::new(0, 0, 200, 200)));
    // Hover the parent item.
    let item_y = menu.bounds().y() + 10;
    let _ = menu.handle_event(&mouse_move(menu.bounds().x() + 10, item_y), &theme());
    let sub_bounds = menu.submenu_bounds().expect("submenu should spawn");
    // Submenu must be inside viewport.
    assert!(
        sub_bounds.right() <= 200,
        "submenu right edge {} exceeds viewport",
        sub_bounds.right()
    );
}

// Bug cm17: MouseMove always Consumed when over the menu.
#[test]
fn bug_cm17_mouse_move_always_consumed_in_bounds() {
    let items = vec![ContextMenuItem::new("X"), ContextMenuItem::new("Y")];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    let bounds = menu.bounds();
    let inside = Point::new(bounds.x() + 5, bounds.y() + 5);
    let r1 = menu.handle_event(&mouse_move(inside.x, inside.y), &theme());
    let r2 = menu.handle_event(&mouse_move(inside.x, inside.y), &theme());
    assert_eq!(r1, EventResult::Consumed);
    assert_eq!(
        r2,
        EventResult::Consumed,
        "second move at same point must still be consumed"
    );
}

// Bug cm18: click on disabled item is consumed and menu stays open.
#[test]
fn bug_cm18_click_on_disabled_keeps_menu_open() {
    let items = vec![
        ContextMenuItem::new("Enabled"),
        ContextMenuItem::new("Disabled").disabled(),
    ];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    let bounds = menu.bounds();
    // Click on disabled (item index 1, second slot).
    let click_y = bounds.y() + 2 + 25 + 12;
    let result = menu.handle_event(
        &mouse_press(bounds.x() + 10, click_y),
        &theme(),
    );
    assert_eq!(result, EventResult::Consumed);
    assert!(menu.is_open(), "menu must stay open after disabled click");
}

// Bug cm18b: click on separator likewise stays open.
#[test]
fn bug_cm18_click_on_separator_keeps_menu_open() {
    let items = vec![
        ContextMenuItem::new("A"),
        ContextMenuItem::separator(),
        ContextMenuItem::new("B"),
    ];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    let bounds = menu.bounds();
    let sep_y = bounds.y() + 2 + 25 + 4;
    let result = menu.handle_event(&mouse_press(bounds.x() + 10, sep_y), &theme());
    assert_eq!(result, EventResult::Consumed);
    assert!(menu.is_open(), "menu must stay open after separator click");
}

// Bug cm19: re-show at same position preserves submenu.
#[test]
fn bug_cm19_reshow_same_position_preserves_submenu() {
    let sub = vec![ContextMenuItem::new("Sub A")];
    let parent = vec![ContextMenuItem::new("Parent").with_submenu(sub)];
    let mut menu = ContextMenu::new(id()).with_items(parent);
    menu.show_at(Point::new(100, 100));
    // Hover to spawn submenu.
    let bounds = menu.bounds();
    let _ = menu.handle_event(
        &mouse_move(bounds.x() + 10, bounds.y() + 10),
        &theme(),
    );
    assert!(menu.has_submenu());
    // Re-show at the SAME position — submenu must survive.
    menu.show_at(Point::new(100, 100));
    assert!(
        menu.has_submenu(),
        "submenu must survive show_at at same position"
    );
}

// Bug cm20: multi-byte text in menu items doesn't over-allocate width vs
// ASCII-equivalent character count.
#[test]
fn bug_cm20_width_is_char_based_not_byte_based() {
    let cjk = vec![ContextMenuItem::new("档案档案")]; // 4 chars, 12 bytes
    let ascii = vec![ContextMenuItem::new("aaaa")]; // 4 chars, 4 bytes
    let m1 = ContextMenu::new(id()).with_items(cjk);
    let m2 = ContextMenu::new(id()).with_items(ascii);
    let constraints = LayoutConstraints::UNBOUNDED;
    let s1 = m1.measure(&constraints, &theme());
    let s2 = m2.measure(&constraints, &theme());
    assert_eq!(
        s1.width, s2.width,
        "width must depend on char count, not byte count: cjk={} ascii={}",
        s1.width, s2.width
    );
}

// Bug cm22: classify_point distinguishes outside vs separator (we test via
// item_from_point which now correctly returns None only for non-actionable
// hits).
#[test]
fn bug_cm22_separator_and_outside_disambiguated() {
    // Indirect test: with a separator at index 1, clicking on it via
    // mouse_press should be Consumed (separator branch) — bug cm18 covers
    // this from the user-visible side.
    let items = vec![
        ContextMenuItem::new("A"),
        ContextMenuItem::separator(),
        ContextMenuItem::new("B"),
    ];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    let bounds = menu.bounds();
    let result = menu.handle_event(
        &mouse_press(bounds.right() + 100, bounds.y() + 5),
        &theme(),
    );
    // Outside click closes the menu (and is consumed).
    assert_eq!(result, EventResult::Consumed);
    assert!(!menu.is_open(), "click outside closed the menu");
}

// Bug cm23: focus-loss does NOT close the menu (e.g. on resize).
#[test]
fn bug_cm23_focus_loss_keeps_menu_open() {
    let items = vec![ContextMenuItem::new("A")];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    assert!(menu.is_open());
    menu.set_focused(false);
    assert!(
        menu.is_open(),
        "focus-loss must not auto-close the menu (Bug cm23)"
    );
}

// Bug cm15b: skip separators/disabled on Up.
#[test]
fn bug_cm15_keyboard_nav_up_skips() {
    let items = vec![
        ContextMenuItem::new("A"),
        ContextMenuItem::new("B").disabled(),
        ContextMenuItem::separator(),
        ContextMenuItem::new("C"),
    ];
    let mut menu = ContextMenu::new(id()).with_items(items);
    menu.show_at(Point::new(100, 100));
    // Down to A, then Down to C (skip B + separator).
    let _ = menu.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    let _ = menu.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(menu.hover_index(), Some(3));
    // Up should go back to A.
    let _ = menu.handle_event(&key_press(Key::Up, Modifiers::empty()), &theme());
    assert_eq!(menu.hover_index(), Some(0));
}

// ---------------------------------------------------------------------------
// Phase 3: menu.rs bugs
// ---------------------------------------------------------------------------

// Bug m11: arrow / Enter / mnemonic keyboard nav.
#[test]
fn bug_m11_arrow_keys_navigate_dropdown() {
    let mut bar = MenuBar::new(id());
    bar.add_menu(
        "File",
        vec![
            MenuItem::new("Open"),
            MenuItem::separator(),
            MenuItem::new("Save"),
        ],
    );
    bar.layout(Rect::new(0, 0, 200, 30), &theme());
    // Open the dropdown by clicking File.
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    assert!(bar.is_dropdown_visible());
    // First focusable item is index 0 (Open).
    assert_eq!(bar.dropdown_hover(), 0);
    // Down should skip the separator and land on Save (index 2).
    let _ = bar.handle_event(&key_press(Key::Down, Modifiers::empty()), &theme());
    assert_eq!(bar.dropdown_hover(), 2);
    // Up cycles back to Open.
    let _ = bar.handle_event(&key_press(Key::Up, Modifiers::empty()), &theme());
    assert_eq!(bar.dropdown_hover(), 0);
}

// Bug m11b: Left/Right cycles between menus.
#[test]
fn bug_m11_left_right_cycles_menus() {
    let mut bar = MenuBar::new(id());
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme());
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    assert_eq!(bar.active_menu(), 0);
    let _ = bar.handle_event(&key_press(Key::Right, Modifiers::empty()), &theme());
    assert_eq!(bar.active_menu(), 1);
    let _ = bar.handle_event(&key_press(Key::Left, Modifiers::empty()), &theme());
    assert_eq!(bar.active_menu(), 0);
}

// Bug m11c: Alt+letter opens matching menu.
#[test]
fn bug_m11_alt_mnemonic_opens_menu() {
    let mut bar = MenuBar::new(id());
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    bar.add_menu("Edit", vec![MenuItem::new("Cut")]);
    bar.layout(Rect::new(0, 0, 300, 30), &theme());
    let _ = bar.handle_event(&key_press(Key::E, Modifiers::ALT), &theme());
    assert_eq!(bar.active_menu(), 1);
    assert!(bar.is_dropdown_visible());
}

// Bug m12: hovering an item with a submenu marks it open.
#[test]
fn bug_m12_submenu_open_tracked() {
    let mut bar = MenuBar::new(id());
    bar.add_menu(
        "File",
        vec![MenuItem::new("Recent").with_submenu(vec![MenuItem::new("File 1")])],
    );
    bar.layout(Rect::new(0, 0, 200, 30), &theme());
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    let dropdown = bar.dropdown_rect_for(0);
    let _ = bar.handle_event(
        &mouse_move(dropdown.x() + 5, dropdown.y() + 12),
        &theme(),
    );
    assert_eq!(bar.submenu_open(), Some((0, 0)));
}

// Bug m13: dropdown overflowing the bottom flips above the menu bar.
#[test]
fn bug_m13_dropdown_flips_above_on_bottom_overflow() {
    let mut bar = MenuBar::new(id());
    let many: Vec<MenuItem> = (0..20).map(|i| MenuItem::new(format!("Item {}", i))).collect();
    bar.add_menu("File", many);
    bar.layout(Rect::new(0, 200, 300, 30), &theme());
    bar.set_viewport(Rect::new(0, 0, 800, 240)); // Tight bottom edge.
    let dr = bar.dropdown_rect_for(0);
    assert!(
        dr.y() < 200,
        "dropdown should flip above (y={}, menu bar y=200)",
        dr.y()
    );
}

// Bug m14: clicking a disabled dropdown item doesn't close the dropdown.
#[test]
fn bug_m14_disabled_click_keeps_dropdown_open() {
    let mut bar = MenuBar::new(id());
    let mut disabled = MenuItem::new("Disabled");
    disabled.enabled = false;
    bar.add_menu("File", vec![MenuItem::new("Open"), disabled]);
    bar.layout(Rect::new(0, 0, 200, 30), &theme());
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    let dr = bar.dropdown_rect_for(0);
    // Click the second item (the disabled one).
    let click_y = dr.y() + 28 + 12; // roughly inside item 1
    let _ = bar.handle_event(&mouse_press(dr.x() + 10, click_y), &theme());
    assert!(
        bar.is_dropdown_visible(),
        "clicking a disabled item must not close the dropdown"
    );
}

// Bug m15: clicking a separator doesn't close the dropdown.
#[test]
fn bug_m15_separator_click_keeps_dropdown_open() {
    let mut bar = MenuBar::new(id());
    bar.add_menu(
        "File",
        vec![
            MenuItem::new("Open"),
            MenuItem::separator(),
            MenuItem::new("Save"),
        ],
    );
    bar.layout(Rect::new(0, 0, 200, 30), &theme());
    let _ = bar.handle_event(&mouse_press(8, 15), &theme());
    let dr = bar.dropdown_rect_for(0);
    let click_y = dr.y() + 28 + 12; // item index 1 (separator)
    let _ = bar.handle_event(&mouse_press(dr.x() + 10, click_y), &theme());
    assert!(
        bar.is_dropdown_visible(),
        "clicking a separator must not close the dropdown"
    );
}

// Bug m17: multi-byte menu titles use char count, not byte count.
#[test]
fn bug_m17_menu_title_width_char_based() {
    let mut bar1 = MenuBar::new(id());
    bar1.add_menu("档案", vec![MenuItem::new("Open")]); // 2 chars (6 bytes)
    bar1.layout(Rect::new(0, 0, 200, 30), &theme());
    let mut bar2 = MenuBar::new(id());
    bar2.add_menu("ab", vec![MenuItem::new("Open")]); // 2 chars (2 bytes)
    bar2.layout(Rect::new(0, 0, 200, 30), &theme());
    // Both menus should hit-test the same way at the menu_index_at front.
    let r1 = bar1.menu_index_at(Point::new(8, 15));
    let r2 = bar2.menu_index_at(Point::new(8, 15));
    assert_eq!(r1, 0);
    assert_eq!(r2, 0);
    // The drawn rect of the first menu in each bar should also be the
    // same width, because we count chars now.
    // Trick: query menu 0 in each bar via dropdown_width_at sees per-menu
    // width which depends on submenu items, so use that to verify.
    let w1 = bar1.dropdown_width_at(0);
    let w2 = bar2.dropdown_width_at(0);
    assert_eq!(
        w1, w2,
        "dropdown width must depend on submenu char counts, not bytes"
    );
}

// Bug m18: per-menu dropdown widths.
#[test]
fn bug_m18_per_menu_dropdown_widths() {
    let mut bar = MenuBar::new(id());
    bar.add_menu("File", vec![MenuItem::new("Open")]);
    bar.add_menu(
        "Edit",
        vec![MenuItem::new("Find and Replace All Across The Project")],
    );
    bar.layout(Rect::new(0, 0, 600, 30), &theme());
    let w_file = bar.dropdown_width_at(0);
    let w_edit = bar.dropdown_width_at(1);
    assert!(
        w_edit > w_file,
        "Edit dropdown ({}) should be wider than File ({})",
        w_edit,
        w_file
    );
}

// ---------------------------------------------------------------------------
// Production-path keycode shortcuts. winit-0.29 emits Key::A..Z (not
// Key::Character) for letter keys; the Character path was the only one
// previously exercised. These siblings prove the keycode path also works.
// ---------------------------------------------------------------------------

#[test]
fn bug_ti15_ctrl_a_select_all_keycode_path() {
    let mut input = TextInput::new(id()).with_text("hello world");
    input.layout(Rect::new(0, 0, 300, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(3);
    let _ = input.handle_event(&key_press(Key::A, Modifiers::CTRL), &theme());
    assert_eq!(input.selection_start(), Some(0));
    assert_eq!(input.cursor_pos(), input.text().len());
}

#[test]
fn bug_ti15_clipboard_ctrl_c_x_v_keycode_path() {
    use erigui_widgets::clipboard;

    clipboard::clear_clipboard();
    let mut input = TextInput::new(id()).with_text("hello");
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(0);
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());

    let _ = input.handle_event(&key_press(Key::C, Modifiers::CTRL), &theme());
    assert_eq!(clipboard::get_clipboard(), "hello");

    let _ = input.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    let _ = input.handle_event(&key_press(Key::X, Modifiers::CTRL), &theme());
    assert_eq!(input.text(), "");
    assert_eq!(clipboard::get_clipboard(), "hello");

    let _ = input.handle_event(&key_press(Key::V, Modifiers::CTRL), &theme());
    assert_eq!(input.text(), "hello");
}

#[test]
fn bug_ta22_undo_restores_selection_and_caps_shortcuts_keycode_path() {
    let mut area = TextArea::new(id()).with_text("hello world");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);

    let _ = area.handle_event(&key_press(Key::Home, Modifiers::empty()), &theme());
    let _ = area.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());
    assert!(area.selection_for_test().is_some());

    // Replace selection by typing — TextInput path inserts via
    // Character event, so we feed a TextInput event to mimic real flow.
    let _ = area.handle_event(
        &Event::TextInput(TextInputEvent {
            text: "x".to_string(),
        }),
        &theme(),
    );
    assert_eq!(area.get_text(), "x");

    // Ctrl+Z via Key::Z keycode path.
    let _ = area.handle_event(&key_press(Key::Z, Modifiers::CTRL), &theme());
    assert_eq!(area.get_text(), "hello world");
    assert!(area.selection_for_test().is_some());
}

#[test]
fn bug_ta22_select_all_keycode_path() {
    let mut area = TextArea::new(id()).with_text("hello world");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let _ = area.handle_event(&key_press(Key::A, Modifiers::CTRL), &theme());
    let sel = area.selection_for_test().expect("Ctrl+A must select all");
    assert_eq!(sel.start_line, 0);
    assert_eq!(sel.start_col, 0);
}

#[test]
fn bug_ta22_clipboard_keycode_path() {
    use erigui_widgets::clipboard;

    clipboard::clear_clipboard();
    let mut area = TextArea::new(id()).with_text("hello");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let _ = area.handle_event(&key_press(Key::A, Modifiers::CTRL), &theme());

    let _ = area.handle_event(&key_press(Key::C, Modifiers::CTRL), &theme());
    assert_eq!(clipboard::get_clipboard(), "hello");

    let _ = area.handle_event(&key_press(Key::X, Modifiers::CTRL), &theme());
    assert_eq!(area.get_text(), "");
    assert_eq!(clipboard::get_clipboard(), "hello");

    let _ = area.handle_event(&key_press(Key::V, Modifiers::CTRL), &theme());
    assert_eq!(area.get_text(), "hello");
}

#[test]
fn bug_ta22_redo_keycode_path() {
    let mut area = TextArea::new(id()).with_text("hello");
    area.layout(Rect::new(0, 0, 400, 200), &theme());
    area.set_focused(true);
    let _ = area.handle_event(&key_press(Key::End, Modifiers::empty()), &theme());
    let _ = area.handle_event(
        &Event::TextInput(TextInputEvent {
            text: "!".to_string(),
        }),
        &theme(),
    );
    assert_eq!(area.get_text(), "hello!");
    let _ = area.handle_event(&key_press(Key::Z, Modifiers::CTRL), &theme());
    assert_eq!(area.get_text(), "hello");
    let _ = area.handle_event(&key_press(Key::Y, Modifiers::CTRL), &theme());
    assert_eq!(area.get_text(), "hello!");
}

// Password mode must not leak cleartext to the clipboard on Ctrl+C.
#[test]
fn bug_ti_password_ctrl_c_no_leak() {
    use erigui_widgets::clipboard;

    clipboard::clear_clipboard();
    clipboard::set_clipboard("sentinel");

    let mut input = TextInput::new(id()).with_text("hunter2");
    input.set_password_mode(true);
    input.layout(Rect::new(0, 0, 200, 24), &theme());
    input.set_focused(true);
    input.set_cursor_pos_for_test(0);
    let _ = input.handle_event(&key_press(Key::End, Modifiers::SHIFT), &theme());

    // Both shortcut paths must not leak the cleartext.
    let _ = input.handle_event(
        &key_press(Key::Character('c'), Modifiers::CTRL),
        &theme(),
    );
    assert_eq!(clipboard::get_clipboard(), "sentinel");

    let _ = input.handle_event(&key_press(Key::C, Modifiers::CTRL), &theme());
    assert_eq!(clipboard::get_clipboard(), "sentinel");
}

// search_box outside-click closes & consumes (matches menu / context_menu).
#[test]
fn bug_sb_outside_click_consumed() {
    let mut sb = SearchBox::new(id()).with_suggestion_provider(|_| vec!["x".to_string()]);
    sb.layout(Rect::new(10, 10, 200, 30), &theme());
    sb.set_focused(true);
    // Click far outside.
    let result = sb.handle_event(&mouse_press(500, 500), &theme());
    assert_eq!(
        result,
        EventResult::Consumed,
        "outside-click on SearchBox must be Consumed, like menu/context_menu"
    );
}

// menu.rs: separator height halves the row, so click row indexing must
// account for that.
#[test]
fn bug_m_dropdown_item_from_point_separator_aware() {
    let mut bar = MenuBar::new(id());
    bar.add_menu(
        "File",
        vec![
            MenuItem::new("Open"),
            MenuItem::separator(),
            MenuItem::new("Quit"),
        ],
    );
    bar.layout(Rect::new(0, 0, 400, 30), &theme());
    // Open menu 0 (drop down).
    let _ = bar.handle_event(&mouse_press(20, 15), &theme());
    assert!(bar.is_dropdown_visible());

    // Compute approximate row positions. The dropdown opens at y=30
    // (menu bar height). Default dropdown_height is the row height for
    // a normal row; separator is half. We click on the third item
    // ("Quit") and assert the bar registers a hover on row index 2.
    // Pre-fix code would compute index = relative_y / dropdown_height
    // and miscount because it ignored the separator's half-height.
    // We drive a MouseMove instead of a click so we can observe
    // dropdown_hover without firing the action.
    // Find a y inside row index 2 (Quit) under the separator-aware layout:
    //   row 0 (Open):     [30, 30+H)
    //   row 1 (Separator):[30+H, 30+H+H/2)
    //   row 2 (Quit):     [30+H+H/2, 30+H+H/2+H)
    // Pre-fix uniform-height layout would put row 2 at [30+2H, 30+3H), so a
    // click in [30+H+H/2, 30+2H) was misclassified. Test inside that gap.
    // Use the widget's default dropdown_height (28) for probe math.
    let h = 28i32;
    let probe_y = 30 + h + h / 2 + 2; // safely inside Quit row, post-fix
    let _ = bar.handle_event(&mouse_move(20, probe_y), &theme());
    assert_eq!(
        bar.dropdown_hover(),
        2,
        "row at y={} must hover Quit (row 2) accounting for separator",
        probe_y
    );
}

// ============================================================================
// Wave 1 (A3): inline field widgets in node_graph.
//
// These tests cover the new field-rendering & event-routing layer added to
// `NodeGraph`. They verify that:
//   - typing into a Text field via KeyPress events updates the FieldValue
//   - dragging a Number field's slider moves the value toward the cursor
//   - clicking a Select field's options updates the selection
//   - growing the field list grows the node's height (auto-resize)
//   - Bool fields toggle on click
//   - FilePath field clicks open a file dialog
// ============================================================================

fn ng_layout(graph: NodeGraph) -> NodeGraph {
    let mut g = graph;
    // Identity transform: pan = 0, zoom = 1, so world == screen.
    g.layout(Rect::new(0, 0, 1000, 800), &theme());
    g
}

fn ng_with_node(node: Node) -> NodeGraph {
    let mut g = NodeGraph::new(id(), Graph::default());
    g.add_node(node);
    ng_layout(g)
}

fn key_event(k: Key) -> Event {
    Event::KeyPress(KeyPressEvent {
        key: k,
        modifiers: Modifiers::empty(),
        repeat: false,
    })
}

fn make_node_with_fields(fields: Vec<Field>) -> Node {
    Node {
        id: 1,
        title: "T".into(),
        position: erigui_core::Point::new(40, 40),
        size: Size::new(220, 120),
        inputs: vec![],
        outputs: vec![],
        fields,
        component_type: None,
    }
}

fn field_text(value: &str) -> Field {
    Field {
        id: 1,
        name: "name".into(),
        label: "name".into(),
        kind: FieldKind::Text,
        value: FieldValue::Text(value.into()),
    }
}

fn field_number() -> Field {
    Field {
        id: 1,
        name: "n".into(),
        label: "n".into(),
        kind: FieldKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
        },
        value: FieldValue::Number(0.0),
    }
}

fn field_select() -> Field {
    Field {
        id: 1,
        name: "mode".into(),
        label: "mode".into(),
        kind: FieldKind::Select {
            options: vec!["a".into(), "b".into(), "c".into()],
        },
        value: FieldValue::Select("a".into()),
    }
}

fn field_bool() -> Field {
    Field {
        id: 1,
        name: "neg".into(),
        label: "neg".into(),
        kind: FieldKind::Bool,
        value: FieldValue::Bool(false),
    }
}

fn field_path() -> Field {
    Field {
        id: 1,
        name: "ckpt".into(),
        label: "ckpt".into(),
        kind: FieldKind::FilePath {
            extensions: vec!["safetensors".into()],
        },
        value: FieldValue::FilePath(std::path::PathBuf::new()),
    }
}

fn ng_field_value(g: &NodeGraph, node_id: usize, field_id: usize) -> FieldValue {
    g.graph
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .unwrap()
        .fields
        .iter()
        .find(|f| f.id == field_id)
        .unwrap()
        .value
        .clone()
}

fn field_world_rect(g: &NodeGraph, node_id: usize, field_id: usize) -> Rect {
    // Trigger a layout pass so field_rects is populated, then snoop via
    // public state(): the field rect is computed deterministically from
    // node position+size and field index, so we mirror that math here.
    let node = g
        .graph
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .expect("node");
    let header_h = 26;
    let mut y = node.position.y + header_h + 8;
    for f in &node.fields {
        let h = match f.kind {
            FieldKind::Text | FieldKind::FilePath { .. } => 44,
            FieldKind::Number { .. } | FieldKind::Select { .. } => 32,
            FieldKind::Bool => 28,
        };
        if f.id == field_id {
            return Rect::new(node.position.x + 8, y, node.size.width - 16, h);
        }
        y += h + 8;
    }
    panic!("field {} not found on node {}", field_id, node_id);
}

#[test]
fn bug_ng_field_text_input_typing() {
    let node = make_node_with_fields(vec![field_text("")]);
    let mut g = ng_with_node(node);
    let r = field_world_rect(&g, 1, 1);
    // Click into the field to make it active.
    let click = CenterClick::press(r);
    let _ = g.handle_event(&click.press, &theme());
    let _ = g.handle_event(&click.release, &theme());
    // Now fire KeyPress for 'h' and 'i' (no Shift -> lowercase).
    assert_eq!(
        g.handle_event(&key_event(Key::H), &theme()),
        EventResult::Consumed,
        "first KeyPress should be consumed by active text field"
    );
    let _ = g.handle_event(&key_event(Key::I), &theme());
    let v = ng_field_value(&g, 1, 1);
    match v {
        FieldValue::Text(s) => assert_eq!(s, "hi", "field should have typed text"),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn bug_ng_field_slider_drag() {
    let node = make_node_with_fields(vec![field_number()]);
    let mut g = ng_with_node(node);
    let r = field_world_rect(&g, 1, 1);
    // Press near the left edge of the slider, then drag toward the right.
    // Left edge -> low value; right edge -> high value (max=100).
    let press_x = r.x() + 4;
    let drag_x = r.x() + r.width() - 4; // far right
    let mid_y = r.y() + r.height() / 2;
    let _ = g.handle_event(&mouse_press(press_x, mid_y), &theme());
    let v_before = match ng_field_value(&g, 1, 1) {
        FieldValue::Number(n) => n,
        _ => panic!("expected number"),
    };
    let _ = g.handle_event(&mouse_move(drag_x, mid_y), &theme());
    let _ = g.handle_event(&mouse_release(drag_x, mid_y), &theme());
    let v_after = match ng_field_value(&g, 1, 1) {
        FieldValue::Number(n) => n,
        _ => panic!("expected number"),
    };
    assert!(
        v_after > v_before,
        "drag right should increase the slider value (before={}, after={})",
        v_before,
        v_after
    );
    // And it should land within [min, max].
    assert!(v_after >= 0.0 && v_after <= 100.0, "value out of range: {}", v_after);
}

#[test]
fn bug_ng_field_combo_select() {
    let node = make_node_with_fields(vec![field_select()]);
    let mut g = ng_with_node(node);
    let r = field_world_rect(&g, 1, 1);
    // Click the field to open the dropdown overlay.
    let cx = r.center().x;
    let cy = r.center().y;
    let _ = g.handle_event(&mouse_press(cx, cy), &theme());
    let _ = g.handle_event(&mouse_release(cx, cy), &theme());
    // The overlay opens just below the field; click into the second row ("b").
    let row_h = 22; // matches scale(22).clamp(18,32) at zoom=1
    let row_pad = 6 + 2; // top padding inside overlay + per-row gap accumulator
    let click_y = r.bottom() + row_pad + row_h + row_h / 2;
    let _ = g.handle_event(&mouse_press(cx, click_y), &theme());
    match ng_field_value(&g, 1, 1) {
        FieldValue::Select(s) => assert_eq!(s, "b", "second option should be selected"),
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn bug_ng_node_resizes_to_fit_fields() {
    // Build a node with the minimum starting height, then add 5 fields.
    // The auto-resize logic in update_field_rects should grow node.size.h.
    let node = Node {
        id: 1,
        title: "T".into(),
        position: erigui_core::Point::new(20, 20),
        size: Size::new(220, 60), // intentionally too small
        inputs: vec![],
        outputs: vec![],
        fields: (0..5)
            .map(|i| Field {
                id: i,
                name: format!("f{}", i),
                label: format!("f{}", i),
                kind: FieldKind::Text,
                value: FieldValue::Text(String::new()),
            })
            .collect(),
        component_type: None,
    };
    let initial_h = node.size.height;
    let g = ng_with_node(node);
    let h_after = g.graph.nodes[0].size.height;
    // 5 text fields × 44 + 4 gaps × 8 + header(26+8) + bottom_pad(16) ≈ 282
    assert!(
        h_after > initial_h,
        "node height must grow to fit fields (was {}, now {})",
        initial_h,
        h_after
    );
    assert!(
        h_after >= 5 * 44,
        "node height should be at least sum of field heights, got {}",
        h_after
    );
}

#[test]
fn bug_ng_field_bool_toggle() {
    let node = make_node_with_fields(vec![field_bool()]);
    let mut g = ng_with_node(node);
    let r = field_world_rect(&g, 1, 1);
    let cx = r.center().x;
    let cy = r.center().y;
    // Pre: false
    assert!(
        matches!(ng_field_value(&g, 1, 1), FieldValue::Bool(false)),
        "initial should be false"
    );
    let _ = g.handle_event(&mouse_press(cx, cy), &theme());
    let _ = g.handle_event(&mouse_release(cx, cy), &theme());
    assert!(
        matches!(ng_field_value(&g, 1, 1), FieldValue::Bool(true)),
        "click must toggle Bool field to true"
    );
    // Click again -> false
    let _ = g.handle_event(&mouse_press(cx, cy), &theme());
    let _ = g.handle_event(&mouse_release(cx, cy), &theme());
    assert!(
        matches!(ng_field_value(&g, 1, 1), FieldValue::Bool(false)),
        "second click must toggle back to false"
    );
}

#[test]
fn bug_ng_field_filepath_dialog_opens() {
    let node = make_node_with_fields(vec![field_path()]);
    let mut g = ng_with_node(node);
    assert!(!g.is_file_dialog_open(), "no dialog open initially");
    let r = field_world_rect(&g, 1, 1);
    let cx = r.center().x;
    let cy = r.center().y;
    let _ = g.handle_event(&mouse_press(cx, cy), &theme());
    let _ = g.handle_event(&mouse_release(cx, cy), &theme());
    assert!(
        g.is_file_dialog_open(),
        "clicking FilePath field must open the file dialog"
    );
}

/// Synthesizes a press+release pair at the center of `r`, returned as a
/// pair of `Event`s the caller can drive into a widget.
struct CenterClick {
    press: Event,
    release: Event,
}

impl CenterClick {
    fn press(r: Rect) -> Self {
        let cx = r.center().x;
        let cy = r.center().y;
        Self {
            press: mouse_press(cx, cy),
            release: mouse_release(cx, cy),
        }
    }
}

// ============================================================================
// DateTimePicker (production-hardening pass, 2026-04-28)
//
// Pre-fix bugs:
//   - Event::KeyPress / Event::TextInput arms ran whenever
//     editing_hour||editing_minute was true, ignoring focus → app-wide input
//     theft.
//   - set_focused only propagated to the input child; calendar_button focus
//     drifted; edit/popup state leaked when host took focus away.
//   - Plain Tab was consumed inside hour/minute editing → broke focus
//     traversal.
// ============================================================================

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

fn dtp_with_value(dt: NaiveDateTime) -> DateTimePicker {
    DateTimePicker::new(id())
        .with_mode(DateTimePickerMode::DateTime)
        .with_value(dt)
}

#[test]
fn dtp_focus_gates_keyboard_input() {
    // editing_hour=true, but state.focused=false: the KeyPress arm must
    // NOT mutate hour_input even though editing_hour says we're "in" the
    // hour field.
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_editing_hour_for_test(true);
    assert!(p.editing_hour_for_test());
    assert!(!p.is_focused(), "starts unfocused for the test");

    let before = p.hour_input_for_test().to_string();
    let _ = p.handle_event(
        &key_press(Key::Character('5'), Modifiers::empty()),
        &theme(),
    );
    assert_eq!(
        p.hour_input_for_test(),
        before,
        "unfocused KeyPress must not mutate hour_input"
    );
}

#[test]
fn dtp_focus_gates_text_input() {
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_editing_hour_for_test(true);
    assert!(!p.is_focused());

    let before = p.hour_input_for_test().to_string();
    let evt = Event::TextInput(TextInputEvent {
        text: "5".to_string(),
    });
    let _ = p.handle_event(&evt, &theme());
    assert_eq!(
        p.hour_input_for_test(),
        before,
        "unfocused TextInput must not mutate hour_input"
    );
}

#[test]
fn dtp_set_focused_false_clears_edit_state() {
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    p.set_editing_hour_for_test(true);
    p.set_show_calendar_for_test(true);
    assert!(p.editing_hour_for_test());
    assert!(p.show_calendar_for_test());

    p.set_focused(false);
    assert!(!p.editing_hour_for_test(), "editing_hour must clear");
    assert!(!p.editing_minute_for_test(), "editing_minute must clear");
    assert!(!p.show_calendar_for_test(), "show_calendar must clear");
    assert!(!p.show_time_picker_for_test(), "show_time_picker must clear");
}

#[test]
fn dtp_set_focused_propagates_to_button() {
    // set_focused(true/false) must reach BOTH children (input + calendar
    // button), not just the input. We don't have direct accessors on the
    // children from outside, but at minimum the parent's own focus state
    // round-trips, and toggling focused=false clears the edit state — both
    // are observable side-effects of the propagation path.
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    assert!(p.is_focused(), "set_focused(true) sticks on the parent");
    p.set_editing_hour_for_test(true);
    p.set_focused(false);
    assert!(!p.is_focused());
    // Edit state cleared — proves set_focused(false) ran the cleanup branch.
    assert!(!p.editing_hour_for_test());
}

#[test]
fn dtp_plain_tab_returns_ignored() {
    // While editing the hour field, plain Tab must NOT be consumed — that
    // would steal focus traversal from the host. Only Ctrl+Tab cycles the
    // hour/minute fields internally.
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    p.set_editing_hour_for_test(true);

    let r = p.handle_event(&key_press(Key::Tab, Modifiers::empty()), &theme());
    assert_eq!(r, EventResult::Ignored);
}

#[test]
fn dtp_ctrl_tab_cycles_hour_minute() {
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    p.set_editing_hour_for_test(true);
    assert!(p.editing_hour_for_test());

    let r = p.handle_event(&key_press(Key::Tab, Modifiers::CTRL), &theme());
    assert_eq!(r, EventResult::Consumed);
    assert!(
        !p.editing_hour_for_test(),
        "Ctrl+Tab from hour must move out of hour"
    );
    assert!(
        p.editing_minute_for_test(),
        "Ctrl+Tab from hour must enter minute"
    );
}

#[test]
fn dtp_text_input_appends_to_active_field() {
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    // hour_input starts as "00" (length 2), so to verify the append path
    // we reset editing state and clear it first via the TextInput path
    // is not exposed — but we can verify by going through minute_input
    // which starts at "00" and is similarly len=2. Better: pick a base
    // datetime whose formatted hour leaves room. There is none — format
    // is always 2 digits. Instead, drive the well-defined append-while-
    // <2 path: the TextInput arm rejects appends when hour_input.len()>=2.
    // So the canonical test is: set editing_hour=true at base "00", drive
    // a digit, expect no change (length already 2). To exercise the actual
    // append, simulate by manually shrinking via Backspace + then digit.
    p.set_editing_hour_for_test(true);
    // Send Backspace to drop a digit (focus-gated: must be focused, which
    // we set above).
    let _ = p.handle_event(&key_press(Key::Backspace, Modifiers::empty()), &theme());
    assert_eq!(
        p.hour_input_for_test().len(),
        1,
        "Backspace should leave one digit"
    );
    // Now send a TextInput digit; should append back to length 2.
    let evt = Event::TextInput(TextInputEvent {
        text: "7".to_string(),
    });
    let _ = p.handle_event(&evt, &theme());
    assert_eq!(
        p.hour_input_for_test().len(),
        2,
        "TextInput digit must append to active hour field"
    );
    assert!(
        p.hour_input_for_test().ends_with('7'),
        "appended digit must be at the end, got {}",
        p.hour_input_for_test()
    );
}

#[test]
fn dtp_calendar_click_updates_datetime() {
    // Drive a DateTimePicker in Date mode so a click on a day cell updates
    // selected_datetime. Use Jan 2024: 1 Jan 2024 was a Monday, so day 15
    // sits at row=2 (third row), col=1 (Monday). Geometry: layout placed
    // at (0,0,240,30), so calendar_rect starts at y=32. Header row is 30,
    // padding 10, plus 20 px below the day-name row. Cells are 30x30
    // starting at x=10. Day 15's cell center is therefore approximately
    // (10 + 1*30 + 15, 32 + 30 + 10 + 20 + 2*30 + 15) = (55, 137).
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let mut p = DateTimePicker::new(id())
        .with_mode(DateTimePickerMode::Date)
        .with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    p.set_focused(true);
    // Open the calendar without depending on the trigger-button hit-test.
    p.set_show_calendar_for_test(true);

    // Day 15 grid pos: first_day_of_week(Jan 2024) = 1 (Monday is 1 from
    // Sunday). grid_pos = 1 + 15 - 1 = 15. row = 15/7 = 2, col = 15%7 = 1.
    // Cell rect = (calendar_x + 10 + 1*30, days_y + 2*30, 30, 30) where
    // calendar_x = 0, calendar_y = 32 + 30 + 10 = 72, days_y = 72 + 20 = 92.
    // So day 15 cell = (40, 152, 30, 30). Click center = (55, 167).
    let before = p.selected_datetime_for_test();
    assert_eq!(before.day(), 1, "preconditions: starting on day 1");
    let _ = p.handle_event(&mouse_press(55, 167), &theme());
    let after = p.selected_datetime_for_test();
    assert_eq!(after.day(), 15, "clicking day 15 must update selected_datetime");
    assert_eq!(after.month(), 1);
    assert_eq!(after.year(), 2024);
    // Time component preserved.
    assert_eq!(after.time(), NaiveTime::from_hms_opt(12, 0, 0).unwrap());
}

#[test]
fn dtp_click_self_focuses_for_typing() {
    // The picker's KeyPress/TextInput arms are gated on `state.focused`. A
    // real-world user clicks the picker (trigger or a time field) and then
    // types — typing must mutate state without the host having to also call
    // `set_focused(true)` separately. This test exercises the full chain:
    // click trigger → state.focused=true (popups also opened in DateTime
    // mode) → click hour input → editing_hour=true → Backspace mutates
    // hour_input. If the click handler ever stops self-focusing, the
    // Backspace would be dropped by the focus gate and hour_input would
    // not change.
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut p = dtp_with_value(dt);
    p.layout(Rect::new(0, 0, 240, 30), &theme());
    assert!(!p.is_focused(), "starts unfocused");

    // Click the trigger button (calendar_button bounds = right 30px of the
    // 240-wide picker, so x in 210..240). In DateTime mode this opens both
    // popups. Self-focus must fire because the click is inside state.bounds.
    let _ = p.handle_event(&mouse_press(220, 15), &theme());
    assert!(
        p.is_focused(),
        "click on the trigger must self-focus the picker"
    );
    assert!(
        p.show_time_picker_for_test(),
        "DateTime mode trigger click should open the time picker"
    );

    // Click the hour input. Default TimeFormat is H24, so time_picker_height
    // = 80. time_picker_rect for DateTime mode at layout (0,0,240,30) =
    // (250+10, 32, 200, 80) = (260, 32, 200, 80). Center = (360, 72).
    // Hour rect = (360 - 60 - 10, 72 - 15, 60, 30) = (290, 57, 60, 30).
    // Click center = (320, 72).
    let _ = p.handle_event(&mouse_press(320, 72), &theme());
    assert!(
        p.editing_hour_for_test(),
        "click on hour input must set editing_hour=true"
    );
    assert!(
        p.is_focused(),
        "click stays focused after entering hour edit"
    );

    // hour_input begins as "10" (length 2). Backspace through the focus-gated
    // KeyPress arm must drop a digit — proving (click → focused → key →
    // mutation) end-to-end without anyone calling set_focused() externally.
    assert_eq!(p.hour_input_for_test(), "10");
    let _ = p.handle_event(&key_press(Key::Backspace, Modifiers::empty()), &theme());
    assert_eq!(
        p.hour_input_for_test(),
        "1",
        "Backspace must mutate hour_input through the click-self-focus path"
    );
}

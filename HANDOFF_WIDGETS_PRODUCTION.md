# EriGui widget library — production-readiness handoff

> Focus: get `erigui-widgets` good enough that EriGui apps stop having
> rage-inducing UX bugs. Pure CPU work — no GPU/runtime testing required
> for any of the items below. Everything verifiable with `cargo test`.

## What's actually solid right now

After the overnight pass (commits `46bdf19`..`e741185`):

- **Text editing in node-graph fields** — the library `TextInput` widget
  is now the editor (not a parallel `FieldEditState` hack). Click in the
  middle of text, cursor lands there. Arrows, selection, undo, clipboard
  all work.
- **Keyboard accessibility** — Slider, Checkbox, Button, ListView, Dialog
  all respond to standard keys when focused. Slider focus state actually
  round-trips (was a permanent lie).
- **AltGr no longer eats characters** — Ctrl-shortcut detection now
  requires `CTRL && !ALT` so AltGr+letter (Linux: types @, €, etc.)
  doesn't trigger Ctrl-shortcuts.
- **Production keyboard input** — combo_box, spin_box, color_picker,
  date_time_picker now handle `Event::TextInput`. Previously they only
  handled the legacy `Key::Character` form which never fires in real
  GUIs (winit 0.29).
- **ListView usability** — wheel-scroll, auto-scroll-to-selected on
  keyboard nav, visible scrollbar when content overflows.

309 tests pass workspace-wide, 0 failures. 48 widget_tests covering the
new behaviors.

## Hard production blockers (do these first)

### 1. HiDPI / font scaling — **DONE 2026-04-28**
Shipped path (a). `Theme::with_scale(f32)` builder added to
`erigui-core/src/theme.rs`. It scales all pixel-valued integer fields
(typography font sizes, spacing, borders) — not just fonts, because
14px font scaled to 28px inside 8px-padded boxes broke layouts.
`erigui-app/src/main.rs` now constructs the theme as
`Theme::alex_jammin().with_scale(ui_scale)` where `ui_scale` is the
primary monitor's `scale_factor` (already queried for window sizing).
Tests stay at scale 1.0 since `Theme::dark()` / `Theme::light()`
default to identity. 8 new tests in `theme::tests::with_scale_*`,
covering identity, 2.0×, 1.5× rounding, color/family preservation,
zero/negative clamping, and 1px-border floor at extreme scales.
Workspace: **317 passed**, 0 failed (was 309). Cross-monitor changes
not handled — explicit user decision.

### 2. Native file picker (no GTK)
**Symptom**: file_dialog widget has ugly fonts, awkward layout, no
visible scrollbar — user complained explicitly. The user-flagged "GTK
is unacceptable" rules out `rfd`.

**Fix**: replace in-app FileDialog widget with `tinyfiledialogs` calls
in the host (erigui-app's open/save handlers). The crate wraps
`zenity` (Linux), native dialogs (mac/windows). No GTK link.

```toml
tinyfiledialogs = "3"
```

In `erigui-app/src/main.rs`:
```rust
let path = tinyfiledialogs::open_file_dialog(
    "Load workflow",
    "",
    Some((&["*.json"], "Workflow JSON")),
);
```

Doesn't touch the widget library — purely host-side. Keeps the in-app
FileDialog widget around as a fallback for environments without zenity.

### 3. TabControl not focusable
**Symptom**: `can_focus()` returns `false`, `set_focused` is a no-op.
Hosts can't direct focus to it, so Ctrl+Tab style tab navigation is
impossible.

**Fix**: same pattern as Slider/Checkbox/ListView — make focus state
real, add Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+1..9 keyboard handling.

### 4. ScrollBar drag-to-scroll
**Symptom**: ListView shows a scrollbar but the thumb isn't draggable
— mouse wheel and arrow keys are the only way to scroll.

**Fix**: in `list_view.rs` `handle_event`:
- On MouseButton press inside the thumb rect: start dragging, track
  `drag_start_y` and `drag_start_offset`.
- On MouseMove while dragging: compute delta, scale by
  `content_height / track_height`, update `scroll_offset`, clamp.
- On MouseButton release: stop dragging.

About 40 lines, all CPU-testable.

## Soft blockers (production-quality polish)

### 5. RadioButton group has no keyboard nav
Arrow keys should move the selection within a group. Currently the
global `RadioGroupManager` singleton tracks state but radio buttons
don't receive arrow keys. Partial fix: instead of arrows-on-individual-
button, the host's keyboard nav layer (Tab traversal) could focus the
selected button and Up/Down within the focused group could be a
host concern. Either path works.

### 6. Tooltip widget is passive
No event handling — relies on host to call `show()`/`hide()`. Standard
expectation: hover-to-show after delay, mouse-leave-to-hide. Add a
`hover_start: Option<Instant>` field and check elapsed in
`Event::MouseMove`/`Event::MouseLeave`. Plus an `Event::Update` tick.

### 7. ScrollView is a stub
`scroll_view.rs` has placeholder comments like "In a real implementation,
we'd need access to the widget manager here". Not load-bearing — only
`widget_gallery.rs` example uses it. Either:
- Delete it (no consumers).
- Refactor it to wrap a single `Box<dyn Widget>` child directly (no
  WidgetManager indirection). ~2 hours.

### 8. Accordion has no keyboard nav
Up/Down to switch panels, Enter/Space to toggle. Same pattern as
ListView. ~30 min.

### 9. Untested widgets
21 widgets currently have zero integration tests:
accordion, breadcrumb, color_picker, container, date_time_picker,
dialog, dock_panel, drag_drop, file_dialog, file_manager, icon,
keyboard_nav, label, notification, radio_button, scroll_view,
spin_box, status_bar, tab_control, toolbar, tooltip, tree_view.

For production confidence, each should have at least:
- Construction test (new() doesn't panic)
- Schema/state test (basic invariants)
- Event smoke test (handle_event doesn't panic on unrelated events)

About 5-10 min per widget, ~3 hours total.

## Architecture cleanups (eventually, not blockers)

- **Field/Port struct shape**: `Field { id, name, label, kind, value }`
  has `name` (added overnight) — but the API still exposes `id` (usize)
  AND `name` (String). The `id` is used for layout caches, `name` for
  executor lookup. Could be cleaner.
- **Modal handling**: dialog manages its own `is_open`, file_dialog
  similar, but they don't share infrastructure. A single `ModalHost`
  abstraction would dedupe ~100 lines.
- **Theme not applied consistently**: some widgets compute font sizes
  with `theme.typography.font_size_base`, others hardcode. Audit pass
  would unify.

## Testing notes

All overnight tests use the pattern:
```rust
let theme = Theme::dark();
let mut w = SomeWidget::new(test_id());
w.layout(Rect::new(0, 0, W, H), &theme);
w.set_focused(true); // for kbd tests
let event = Event::KeyPress(KeyPressEvent { ... });
let result = w.handle_event(&event, &theme);
assert_eq!(result, EventResult::Consumed);
```

This works without any GUI/GPU because `Widget::handle_event` only
mutates internal state — it doesn't draw. Pure CPU. Keep this pattern
for new tests.

For widgets that need `Theme::dark()`'s field defaults, use that. The
`Theme::default()` impl doesn't exist — common gotcha.

## Concrete next-session task list (priority order)

1. ~~**HiDPI fonts**~~ — done (commit on this push). All px-valued
   Theme fields scale, not just fonts.
2. **Native file picker** (~30 min): swap rfd usage out for
   tinyfiledialogs in `erigui-app/src/main.rs::open_save_dialog` and
   `open_load_dialog`. Keep the in-app widget for fallback.
3. **ScrollBar drag** (~30 min): MouseButton+Move+Release routing in
   `list_view.rs`. Test: simulate drag, verify scroll_offset moved.
4. **TabControl focus + kbd** (~30 min): same pattern as Slider.
5. **Tooltip auto-show/hide** (~30 min): timer-based hover detection.
6. **Untested widget smoke** (~3 hours): 21 widgets × ~10 min each.
7. **Accordion kbd** (~30 min): if there's time.

Total remaining: ~5-6 hours of CPU work.

## Quick reference

```bash
cd /home/alex/EriGui/rust-gui

# Run all widget tests
LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:$LD_LIBRARY_PATH \
  cargo test -p erigui-widgets --release

# Workspace check (no run — verifies all crates compile)
LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:$LD_LIBRARY_PATH \
  cargo check --workspace --all-targets

# Build the app (still works, but app launch needs GPU)
LD_LIBRARY_PATH=/home/alex/libs/libtorch/lib:$LD_LIBRARY_PATH \
  cargo build --release -p erigui-app
```

erigui-widgets test count baseline (as of overnight push):
- 27 lib unit + 74 bug_fix + 5 progress + 48 widget_tests = **154 widget tests**
- Plus 6 erigui-runtime, ~80 elsewhere = **309 total** workspace tests passing

Don't add tests that need GPU/CUDA. Anything that calls `flame_core::Tensor`
or `global_cuda_device` makes test runs require `LD_LIBRARY_PATH=libcudnn`
and breaks the "pure widget library" promise. The library tests today are
clean of GPU dependencies (the integration tests under `erigui-app`/
`erigui-nodes` have CUDA tests, that's expected).

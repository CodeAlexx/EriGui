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

### 2. Native file picker (no GTK) — **DONE 2026-04-28**
Shipped: `tinyfiledialogs = "3"` in `erigui-app/Cargo.toml`,
`open_save_dialog` / `open_load_dialog` in main.rs now call
`save_file_dialog_with_filter` / `open_file_dialog` synchronously.
The in-app `FileDialog` widget code is untouched in the library
(left as fallback for headless envs). Removed from `App`: the
`file_dialog` field, `dialog_purpose`, `pending_path`,
`dialog_cancelled`, `DialogPurpose` enum, `FILE_DIALOG_W/H`
constants, `layout_dialog_only`, `complete_dialog`, the
modal-priority block in `handle_event`, the dialog branch in
`layout`, and the translucent-backdrop branch in `draw`. The
synchronous-blocking model matches what every native app does for
File→Save. `Color` import dropped (was only used by the backdrop).

Workspace stayed at **317 passed, 0 failed**. zenity confirmed
present on this Linux box. Cross-platform: Win32
`GetOpenFileNameW`, macOS `NSOpenPanel`, Linux
`zenity`/`kdialog`/`qarma` — picked over `rfd` because rfd drags
GTK.

### 3. ScrollBar drag-to-scroll — **DONE 2026-04-28** (visual TODO)
**Visual verification still pending** — needs a ListView in the live
app populated with enough items to overflow the viewport. The CPU
tests confirm the math but no human has dragged the thumb yet. Try
again once a long ListView appears in the UI (long add-node menu,
file dialog inside a populated dir, etc.).


Shipped: `ListView` now has a `ScrollbarDrag` state field. On
MouseButton-Left press inside the thumb rect, drag captures
`(start_mouse_y, start_offset)`; subsequent MouseMove deltas map back
to `scroll_offset` via `dy_mouse * (max_scroll / max_thumb_y)` and
clamp; release clears the drag. The scrollbar geometry computation
was factored into a single helper `scrollbar_geometry()` shared
between `draw` and the hit-test so they can't drift. Thumb hit-test
runs BEFORE item-click so a press on the thumb doesn't double-fire
selection underneath. Pressing in the track but outside the thumb is
still a no-op (page-jump deferred). 5 new tests covering: drag
scrolls, drag-then-release stops tracking, drag past end clamps to
max_scroll, no-overflow lists ignore drag, track-but-not-thumb press
doesn't engage. Workspace: **322 passed**, 0 failed.

### 4. TabControl focus + keyboard — **DONE 2026-04-28**
Shipped: `is_focused`/`set_focused`/`can_focus` now read/write
`WidgetState::focused` (was hardcoded false / no-op). Mouse click on
a tab also sets focus. Keyboard nav when focused: **Ctrl+Tab** moves
to next enabled tab, **Ctrl+Shift+Tab** to previous, **Ctrl+1..9**
jumps to that index. Wrap-around at both ends. Disabled tabs are
skipped via `next_enabled_tab` so Ctrl+Tab can't get stuck — without
this, a single disabled tab between two enabled ones would have made
forward navigation impossible. Plain Tab (no Ctrl) is intentionally
ignored so the host's tab-traversal layer keeps working. 11 tests
covering all of the above plus unfocused-keys-ignored and
out-of-range Ctrl+9. Workspace **333 passed**, 0 failed.

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

1. ~~**HiDPI fonts**~~ — done.
2. ~~**Native file picker**~~ — done.
3. ~~**ScrollBar drag**~~ — done (visual verify pending).
4. ~~**TabControl focus + kbd**~~ — done. Ctrl+Tab/Shift+Tab/Ctrl+1..9
   with disabled-tab skipping.
5. **Tooltip auto-show/hide** (~30 min): timer-based hover detection.
6. **Untested widget smoke** (~3 hours): 21 widgets × ~10 min each.
7. **Accordion kbd** (~30 min): if there's time.

Total remaining: ~3.5 hours of CPU work.

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

# Handoff — kitchen_sink debug session

> Session-end state, 2026-04-28 evening. Pick up here next session.
> The kitchen_sink demo (`cargo run --release --example kitchen_sink -p
> erigui-examples`) is the verification harness for wave-2/3 widget
> work. This session went deep on a sequence of bugs the demo
> exposed; this doc records what landed, what's still broken, and
> the diagnostic threads to follow.

## What the session shipped

Commits on `master` since handoff start (`f99ab75`):

| Commit | Layer | Fix |
|--------|-------|-----|
| `5e62cde` | examples | Initial kitchen_sink demo (1534 LOC, 6 tabs) |
| `54e5aac` | rendering | Zero-bitmap glyphs (space, tab) now advance cursor — words no longer ran together |
| `c2cfde3` | widgets+examples | TabControl theme-aware (`tab_height` / `min_tab_width` / `max_tab_width`) + kitchen_sink theme-driven layout metrics across all 6 tab layouts + App tab/status heights |
| `f15d975` | examples/kitchen_sink | `request_redraw()` after every event + `WindowEvent::RedrawRequested` arm. Wayland was deferring paints. |
| `a59a7f3` | examples/kitchen_sink | **Persistent `EventTranslator`**. The legacy `convert_window_event` helper makes a fresh translator per call — `last_cursor` always `Point::ZERO` → every click reported as (0,0). This was the actual root cause of "nothing worked" |
| `0150fc2` | rendering | (initial direction) draw_text treats position.y as top-of-text |
| `74ef99f` | widgets+examples | RadioButton snap+sync (no longer needs Event::Update pumping) + ListView 50→200 + ComboBox draws after TextInput (z-order) + ProgressBar tick |
| `8456180` | widgets | Accordion `header_height` + DockPanel `tab_height`/`title_height` theme-driven; ListView scrollbar contrast (border → text_secondary) |
| `6417a93` | widgets | **Standardized 7 widgets on top-of-text positioning** (accordion / tooltip / breadcrumb / search_box / tree_view / radio_button — were `center.y + font/2 - 2`, now `center.y - font/2`) + NotificationManager `notification_width`/`height`/`margin`/`spacing` theme-scaled + Dialog `title_height`/`button_height` theme-scaled |
| `0b0128f` | widgets/accordion | Click on header now sets `focused_panel = Some(i)` so subsequent Space/Enter actually fire |

## Architectural patterns surfaced

### 1. Mojo-port hardcoded layout metrics

Many widgets store layout heights/widths as raw-px constants in their
constructors (`tab_height: 30`, `header_height: 40`, `notification_width: 350`,
`button_size: 16`, etc.). At HiDPI / `Theme::with_scale` these don't scale
and the widget chrome doesn't fit the doubled font.

**Fix pattern**: in `Widget::layout(rect, theme)`, recompute these from
`theme.typography.font_size_base + theme.spacing.padding.top * N`. At
scale 1.0 the values match the old constants closely (verified for
TabControl). At scale 2x they double. Already applied to:

- TabControl (`c2cfde3`)
- Accordion (`8456180`)
- DockPanel `tab_height` + `title_height` (`8456180`)
- NotificationManager (`6417a93`)
- Dialog (`6417a93`)

**Still need this treatment** (suspected from grep, not visually confirmed):

- `radio_button.rs:78` `button_size: 16`
- `spin_box.rs:148-178` `button_width: 20` (hardcoded inside `get_text_rect`/`get_up_button_rect`/`get_down_button_rect`)
- `tree_view.rs` `item_height` (hardcoded — visually small at HiDPI)
- `list_view.rs` `item_height: 24` (hardcoded — same)
- `dock_panel.rs:153` `splitter_size`/`drag_threshold` likely hardcoded
- `dialog.rs:230` `close_size: 20`
- `dialog.rs:177` `min_width: 300` (the Dialog box itself)
- `progress_bar.rs` likely has fixed text-area heights

### 2. Top-of-text vs baseline-y in `draw_text`

The renderer's `draw_text(position, ...)` was previously ambiguous —
some widgets passed top-of-text (TextInput, TabControl, ListView,
ComboBox, Dialog, etc.) and some passed baseline (Accordion, Tooltip,
Breadcrumb, SearchBox, TreeView, RadioButton). Wave's session
standardized on **top-of-text** in `0150fc2` (renderer) + `6417a93`
(widget call sites).

**Convention going forward**: caller passes `center.y - font_size / 2`
as the y-coordinate for vertically-centered text. Renderer treats
`position.y` as the top of the text block.

### 3. EventTranslator must persist

`erigui_rendering::convert_window_event(event, viewport)` is a free
function that internally constructs a fresh `EventTranslator` per
call. The translator caches `last_cursor` from `CursorMoved` events
and uses it in subsequent `MouseInput` translation (winit's
MouseInput does NOT carry position). With a fresh translator,
`last_cursor` is always `Point::ZERO` → every click reports as (0,0).

**`convert_window_event` is broken by design.** Other examples
(`widget_gallery.rs`, `dock_panel_demo.rs`, `radio_button_demo.rs`,
`text_demo.rs`, etc.) still use it and have the same latent bug.
The kitchen_sink fix is in `a59a7f3` — instantiate
`let mut translator = EventTranslator::new(viewport);` at startup,
mirror `set_window_size(...)` on Resize, call
`translator.translate(&event)` per event.

**TODO**: deprecate `convert_window_event` and migrate the other
example demos.

### 4. Wayland needs explicit `request_redraw()`

`ControlFlow::Poll` + `AboutToWait` does not reliably trigger paints
on Wayland. The kitchen_sink (and `erigui-app/src/main.rs`) call
`renderer.window().request_redraw()` after every state-changing event
and on Resize, then paint inside `WindowEvent::RedrawRequested`. See
`f15d975` for the pattern.

## Bugs still open at session end

### Reported and confirmed

1. **SpinBox can't enter edit mode** (Tab 1).
   - Up/Down arrows work — clicks on those buttons are received.
   - But clicking the value area ("42") doesn't visibly enter edit mode.
   - Diagnostic open: does the box border turn `theme.colors.primary`
     (blue) on click? `spin_box.rs:251-256` makes that happen when
     `is_editing == true`. If border doesn't change, the click isn't
     reaching the `else if get_text_rect().contains(...)` branch at
     `spin_box.rs:369`.
   - Possible cause: `get_text_rect()` uses hardcoded `button_width: 20`,
     which at HiDPI may be smaller than the visible up/down strip,
     making clicks land in odd zones.
   - Fix path: theme-scale the button_width (same pattern as
     TabControl); add a visible cursor indicator in the value area
     when `is_editing`.

2. **DockPanel `secondary.rs` tab doesn't switch on click** (Tab 5).
   - Tab labels render correctly (post `8456180`).
   - Click on `secondary.rs` does nothing — content stays as `main.rs`.
   - Suspect: `dock_panel.rs:475` calls `tab_control.set_tabs(tab_items)`
     on EVERY layout call. set_tabs (`tab_control.rs:129-139`) clears
     `self.tabs` and rebuilds — but layout fires after every event,
     so the user's click might race with set_tabs's clear-and-rebuild
     and lose the active_tab change.
   - Fix path: only call `set_tabs` when the panel list actually
     changed (compare lengths or panel ids); not unconditionally.
   - Alternative: have DockPanel track its own active_index and stop
     using TabControl as a sub-widget (it's not really earning its
     keep here).

3. **TextInput "extra chars" on type** (Tab 1).
   - Typing one keystroke produces multiple characters.
   - Most likely cause: OS keyboard autorepeat (Wayland fires multiple
     KeyboardInput events when key is held briefly).
   - **Need confirmation**: tap a single key WITHOUT holding (sub-50ms),
     do you see exactly 1 char or exactly 2? If 1: it's autorepeat,
     not a bug. If 2 (or more, deterministically): it's a dual-event
     bug where both Event::TextInput and Event::KeyPress(Key::Char)
     route to insert_char.
   - Diagnostic: add `eprintln!("evt: {:?}", event)` at top of
     TextInput::handle_event for one repro session, count events per
     keypress.

4. **TreeView selected-row text drawing into next row** — partially
   addressed in `6417a93` (text_y formula fixed) but Alex's last
   screenshot still shows "tests/" appearing dim/black below
   "util.rs". Probably also affected by hardcoded `item_height` (see
   #1 in pending Mojo-port list above).

### Reported but not yet investigated

5. **Checkbox black artifact below highlight** — visible in earlier
   screenshots. Probably the bounds rect is taller than the actual
   checkbox glyph due to scaled `row_h` and the row's background fill
   doesn't extend.

6. **TreeView focus indicator** — Alex: "hard to tell what's in focus".
   No visual cue (border, highlight, etc.) when `state.focused = true`.
   Skipped this session.

7. **FileDialog letters cut off + Cancel doesn't close** — reported
   earlier. The Cancel button's callback wiring in kitchen_sink's
   ModalsTab may not flip `show_file_dialog = false`. Glyph cutoff is
   probably the same Mojo-port hardcoded layout issue.

## Files touched this session (for review)

```
erigui-rendering/src/font_freetype.rs    (zero-bitmap glyph + baseline)
erigui-widgets/src/accordion.rs          (header_height + focused_panel)
erigui-widgets/src/breadcrumb.rs         (text-y standardization)
erigui-widgets/src/dialog.rs             (title/button height theme)
erigui-widgets/src/dock_panel.rs         (tab/title height theme)
erigui-widgets/src/list_view.rs          (scrollbar thumb contrast)
erigui-widgets/src/notification.rs       (width/height/margin theme)
erigui-widgets/src/radio_button.rs       (animation_progress sync + text-y)
erigui-widgets/src/search_box.rs         (text-y standardization)
erigui-widgets/src/tab_control.rs        (theme-aware metrics in layout)
erigui-widgets/src/tooltip.rs            (text-y standardization)
erigui-widgets/src/tree_view.rs          (text-y standardization)
erigui-examples/Cargo.toml               (kitchen_sink registration)
erigui-examples/examples/kitchen_sink.rs (the demo itself, ~1600 LOC)
KNOWN_ISSUES.md                          (added FileManager rebuild TODO)
```

## Test status at session end

`cargo test -p erigui-widgets --release`: **all green** at end of every
commit. `cargo test --workspace --release` is **blocked upstream** —
`erigui-nodes` won't compile against the current EriDiffusion
`inference-flame` API (3 errors, tracked in `KNOWN_ISSUES.md`).

## Next-session pickup order

1. Fix SpinBox edit mode (most user-blocking from this session).
2. Fix DockPanel `set_tabs` race (visible interaction bug).
3. Confirm TextInput autorepeat-vs-dual-event with a one-tap experiment.
4. Theme-scale the remaining Mojo-port hardcoded sizes (RadioButton
   `button_size`, SpinBox button_width, TreeView/ListView item_height,
   Dialog `close_size`/`min_width`, ProgressBar text area).
5. TreeView focus indicator visual cue.
6. Migrate other example demos off `convert_window_event` to a
   persistent `EventTranslator`.

## Architectural lesson for the audit

This session's bug stream came almost entirely from **Mojo-port
hardcoded sizes** and one renderer convention mismatch. The
wave-2/3 audit caught the singletons and focus-state contradictions,
but missed the layout-metric class. Next audit pass should grep
specifically for:

```
$ grep -rE 'self\.\w+(_height|_width|_size): \w+ = \d+' erigui-widgets/src/
$ grep -rE '\b(let|const)\s+\w+(_size|_height|_width|_padding)\s*=\s*\d+' erigui-widgets/src/
```

Every match is a candidate Mojo-port hardcoded constant that should
either be derived from theme in `layout()` or annotated as
deliberately fixed (e.g., 1px borders).

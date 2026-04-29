# Handoff — kitchen_sink + menu_demo debug session, night of 2026-04-28

> Pickup from `HANDOFF_2026-04-28_KITCHEN_SINK_DEBUG.md` (commit `7353112`).
> 14 commits land in this session (`8379878` → `211e1eb`). Several
> bugs are fixed, several are partially fixed, and a few are still
> open. **Caveat:** every fix on this branch was driven by user-side
> visual inspection of `kitchen_sink` and `menu_demo` on a Wayland +
> NVIDIA + HiDPI box. The unit-test suite agrees with each fix in
> isolation; the demos are the only end-to-end check we have, and
> they are still buggy.

## What this session shipped

| Commit | Layer | Fix |
|--------|-------|-----|
| `8379878` | spin_box | Visible edit caret + theme-scaled `button_width`. Caret was the actual symptom of "doesn't visibly enter edit mode" — clicks always reached the text branch (verified by test); the only feedback was a subtle border-color change. Caret is unambiguous. |
| `5a81799` | tab_control + dock_panel | Idempotent `set_tabs`. Skips clear/rebuild when items match. The handoff suspected a race; I could not reproduce it in unit tests even with non-zero-origin bounds. Defensive fix landed anyway + added `first_leaf_active_index_for_test()` for future tab-routing tests. |
| `ecedea1` | text_input | Regression test: TextInput inserts exactly once per typing event; non-Ctrl `Key::Character` does NOT insert. Locks the no-double-insert contract. |
| `86959fc` | list_view + tree_view + radio_button + dialog | Theme-scale `item_height` / `button_size` / `close_size` / `min_width` / dialog height. Mojo-port hardcoded constants didn't scale at HiDPI. Dialog test renamed to use a range, not a hard 300. |
| `c990ea4` | tree_view + list_view | Focus border switches to `border_focus` when `state.focused`. Previously focus was a silent state — Up/Down/Enter looked dead even when wired. |
| `dcfceb3` | rendering + 4 demos | `#[deprecated]` `convert_window_event`. Migrated `widget_gallery`, `dock_panel_demo`, `radio_button_demo`, `text_demo` to persistent `EventTranslator`. The free helper instantiates a fresh translator per call → `last_cursor` always `Point::ZERO` → every `MouseInput` reports as `(0,0)`. |
| `169bc11` | 14 demos | Swept the rest of the demos off `convert_window_event`. Build is now warning-clean. |
| `4060c4e` | spin_box + tree_view | SpinBox accepts `Key::Num*` as digit fallback (some Wayland configs deliver KeyboardInput without `event.text`). TreeView toggles expand on full-row click for nodes-with-children — chevron's 16px hit zone was hard to land on; users were clicking the folder name. |
| `44e24e8` | rendering + text_input + text_area + dialog + tab_control | Translator special-cases Space: emits `KeyPress(Key::Space)` instead of `TextInput(" ")` so Accordion/Checkbox/Button activation works. TextInput + TextArea added `Key::Space` arms to insert a literal `' '`. Dialog draws multi-line messages by splitting on `\n`. Tab strip stretches to fill (removed `max_tab_width` clamp) — the dock secondary tab "didn't switch" because users were clicking in the dead space at the right of the strip, not the actual tab rect. |
| `4f568c4` | spin_box + notification | SpinBox commits typed value before applying arrow-click increment/decrement. NotificationManager wraps draw block in clip rect + bumped width to `font*30` so long titles don't bleed off-screen. |
| `b8fe8d0` | accordion + text_input | Accordion auto-focuses first panel when host gives it focus. Toggle (Space/Enter) ignores autorepeat. TextInput caret thickened to 2px in `colors.primary` for visibility. |
| `cd1a324` | rendering | Translator drops printable-text events when `event.repeat == true`. Native toolkits typically don't insert text on autorepeat; only the initial press inserts. Held-key autorepeat (`hhhhhhh...`) is now suppressed for typing. Backspace / arrows still autorepeat. |
| `5856b60` | menu_demo | Dropdowns now render via second-pass `draw_dropdown_only` walk. `begin_frame` uses `theme.colors.background` (was hardcoded 30,30,30 against `Theme::light()`). Menu bar height = `font + 12` instead of hardcoded 25. |
| `211e1eb` | menu_demo + menu | Switched all `Theme::light()` to `Theme::dark()`. MenuBar recomputes `dropdown_widths` from `font` in `Widget::layout` so HiDPI doesn't pack the shortcut column into the label column. |

## What's still broken at session end

The user spent an evening clicking through `kitchen_sink` and
`menu_demo`. The following symptoms were reported as still
present at the end of the session — they are NOT covered by the
fixes above, or the fixes were partial.

### menu_demo
> "still buggy"

User did not enumerate. Confirmed working from the screenshots
earlier in the session: dropdowns render, menu items list with
shortcuts, dark background. What's left unclear:
- **Body label position**: user said "menu text on way bottom" in
  the Theme::light() screenshot. After switching to dark theme +
  wider dropdowns, user said "still buggy". The body labels
  ("Menu Demo", "Click on the menu items above…", "The menu uses
  integer coordinates…") sit at the top of the content area
  bunched with 50 px stride; the rest of the window is dead space.
  Probably they want vertical centering or the labels distributed
  across the body area. **Investigate next session.**
- Submenu (second-level) rendering not exercised by the demo.
- Sub-menu hover-to-open delay not exercised.

### Accordion
> "accordian still mess up, leave it buggy but note it"

Auto-focus-first-panel on host `set_focused(true)` landed (`b8fe8d0`).
Autorepeat-toggle suppression landed. User still sees a problem.
Hypotheses:
- Maybe Space is consumed by another widget BEFORE reaching the
  accordion when the user clicks "outside" the accordion. The
  kitchen_sink ContainersTab calls `accordion.set_focused(true)`
  on any click within `accordion.bounds()`, but other widgets in
  the same tab also see the click and may update their own focus
  state in a way that matters.
- Maybe `panel_animations` blocks toggling during a transition.
  `toggle_panel` doesn't gate on animation state, but the visible
  result might look "no change" until the animation finishes.
- Maybe the user clicks the panel CONTENT (not header) and Space
  gets routed to the content widget instead.
- Maybe `Modifiers::is_empty()` fails on this user's Wayland
  session because some modifier (CapsLock, NumLock?) is reported
  in `last_mods` even when not pressed.

`ERIGUI_DEBUG_ACC=1` env var dumps each KeyPress that reaches the
accordion's keyboard arm. Use this to confirm the event is or
isn't arriving with `state.focused == true`.

### TextInput "loses place"
> "test edit, still puts in extra stuff or losses place"

After `cd1a324` (autorepeat suppression for printable text), the
"extra stuff" symptom (typing one 'h' producing 35 h's) should be
gone. The "loses place" half is unclear. Possible meanings:
- Cursor jumps to wrong position after typing.
- Cursor is invisible (now `colors.primary`, 2px wide — should be
  visible but user can confirm).
- Focus is lost between keystrokes.

Not reproduced under unit test. Need user-side confirmation after
re-running with `cd1a324`.

### Dialog overflow
> "dialog still overlaps and goes off screen"

The user said this earlier in the session, after I split the
dialog message on `\n` and recomputed `min_width` from longest
line. They have not retested since `44e24e8`. The screenshot they
showed afterward was the multi-line `\n` rendering as boxes — that
is fixed.

### Tab-strip dead space
Reproduced and **fixed** in `44e24e8`. The dock diagnostic
(`ERIGUI_DEBUG_DOCK=1`) confirmed the secondary tab DID switch on
clicks at `(296, 200)` and `(464, 200)` — the user's earlier
clicks at `(1764, 210)`, `(1902, 202)`, `(1088, 201)` were in the
dead space at the right of the strip, where each tab was clamped
to `max_tab_width = 392` even though the strip was 1439 wide.
Removing the clamp lets tabs stretch to fill.

## Diagnostic env vars added this session

- `ERIGUI_DEBUG_DOCK=1` — `dock_panel.rs` prints click coords,
  tab list, tab control bounds, and active-index transitions for
  every MouseButton press that reaches a `DockNode::Tabs`.
- `ERIGUI_DEBUG_SPIN=1` — `spin_box.rs` prints the editing-state
  view of every TextInput / KeyPress event. (Diagnostic was
  trimmed back in `4f568c4` once the typed-value-not-committed
  bug was identified; current state of the diagnostic in HEAD
  prints nothing — the variable is no longer wired.)
- `ERIGUI_DEBUG_ACC=1` — `accordion.rs` prints each KeyPress that
  enters the keyboard-nav arm with current `focused_panel` and
  `repeat`.

If you re-instrument SpinBox in the next session, follow the same
pattern (gate behind `ERIGUI_DEBUG_SPIN`).

## Architectural patterns that surfaced

### 1. Mojo-port hardcoded constants (continuation)

`HANDOFF_2026-04-28_KITCHEN_SINK_DEBUG.md` flagged this. This
session swept:

- `list_view.rs` `item_height: 24` → `font + pad*2`
- `tree_view.rs` `item_height: 24` → `font + pad*2`
- `radio_button.rs` `button_size: 16` → `font.max(16)`
- `dialog.rs` `close_size: 20` (duplicated in 2 places) → field,
  `font.max(16)` in layout
- `dialog.rs` `min_width: 300` (in measure) → `font*22` floor, also
  uses longest-line char count not total
- `notification.rs` `notification_width: font*24` (already scaled)
  → bumped to `font*30`
- `spin_box.rs` `button_width: 20` (duplicated in 3 places) →
  field, `font + pad` in layout
- `tab_control.rs` `max_tab_width` clamp → removed (let tabs fill)
- `menu_demo.rs` `menu_height: 25` → `font + 12`
- `menu.rs` dropdown widths recomputed from theme in layout

**Still hardcoded** (deferred — not user-flagged):
- `dock_panel.rs` `splitter_size: 4` (intentional thin splitter)
- `progress_bar.rs` `stripe_width: 20` (cosmetic)
- `file_dialog.rs` `header_height: 80`, `footer_height: 50`,
  `breadcrumb_height: 30`, `input_width: 300`, `button_width: 80`,
  `filter_width: 150` (file_dialog has separate issues per the
  prior handoff #7)
- `tree_view.rs` `scrollbar_width: 12`, `button_size: 16` for the
  expand glyph (still in draw, mirrored in click hit-test)
- `combo_box.rs` `item_height: 25`, `arrow_size: 8`
- `accordion.rs` `icon_size: 16`
- `color_picker.rs` `sv_size: 200`, `hue_height: 20`, etc.
- `date_time_picker.rs` `input_width: 200`, `button_width: 30`

### 2. Space-as-activation vs Space-as-text

The translator now special-cases `Key::Space` to emit
`KeyPress(Key::Space)` instead of `TextInput(" ")`. This breaks
the assumption that all printable text comes through `TextInput`.
TextInput and TextArea handle `Key::Space` to insert `' '`.
**Other widgets that consume printable text via `Event::TextInput`
will silently drop Space presses** — they need a `Key::Space` arm.
Audit candidates:
- `combo_box.rs` `Event::TextInput` filter (line 413) — Space
  presses while combo dropdown is open won't filter.
- `search_box.rs` — wraps TextInput, so it inherits the fix.
- `file_dialog.rs` — has filename input boxes.

### 3. Autorepeat for printable text

Per `cd1a324`, the translator drops `event.repeat == true` events
that would produce printable text. This breaks the convention that
holding a letter key types repeatedly. **If a future user complains
that holding 'a' doesn't repeat in TextInput**, this is why. The
trade-off was made because the user's Wayland session was firing
autorepeat at a rate that produced ~35 chars per held tap, which
is unusable. Backspace / Delete / arrows still autorepeat through
their KeyPress paths.

### 4. Dropdown z-order

`MenuBar::draw` draws ONLY the strip. Dropdown is rendered via
`draw_dropdown_only`, which the host MUST call after the rest of
the tree to get correct z-order. Same pattern is used by
`tab_control` (active page content drawn separately) and
`combo_box` (dropdown popup). If a host writes a generic
`walk_tree_and_draw_each(widget)` without considering this, menu
dropdowns will be hidden behind sibling widgets. `kitchen_sink`
and `menu_demo` both now do a second pass.

## Tests added or modified

- `widget_tests::spin_box_click_on_value_area_enters_edit_mode`
- `widget_tests::spin_box_click_then_type_replaces_value`
- `widget_tests::text_input_text_input_event_inserts_exactly_one_char`
- `tree_dock_tests::dock_panel_tab_strip_click_switches_active_panel`
- `modal_widgets_tests::dialog_measure_floors_width_for_short_message`
  (renamed from `dialog_measure_floors_width_at_300`, now a range
  not an absolute)
- `dock_panel.first_leaf_active_index_for_test()` test accessor

Pre-existing 5 failures in `modal_widgets_tests::manager_*` are
unchanged. They predate this session and are unrelated to any of
the fixes here. They should be a separate triage pass.

## Pickup priorities for next session

1. **Reproduce the still-buggy accordion in isolation.** Build a
   minimal accordion-only example (no other widgets) and Space the
   focused panel. If Space toggles on first press, the bug is
   environmental in `kitchen_sink` (some widget is intercepting).
   If Space still doesn't toggle, instrument the Modifiers state
   and check for `Modifiers::is_empty()` returning false on this
   user's session.

2. **Re-test SpinBox + TextInput after `cd1a324`** with the user.
   The autorepeat suppression should fix "extra stuff". The
   "loses place" symptom needs concrete description.

3. **menu_demo body label layout.** Probably want vertical-center
   distribution within the content container, or larger labels.

4. **Sweep ComboBox + FileDialog** for `Key::Space` so the user
   can type spaces in the filter / filename input.

5. **Pre-existing 5 manager_* test failures** — separate triage.

6. **inference-flame API drift / erigui-nodes won't compile** —
   blocked upstream. See `KNOWN_ISSUES.md`.

## Files touched this session (for review)

```
erigui-rendering/src/window.rs           (translator: Space + autorepeat + #[deprecated])
erigui-widgets/src/accordion.rs          (auto-focus + repeat-toggle suppress)
erigui-widgets/src/dialog.rs             (multi-line + theme-scaled + close_size field)
erigui-widgets/src/dock_panel.rs         (idempotent set_tabs hint + diagnostic + accessor)
erigui-widgets/src/list_view.rs          (item_height + focus border)
erigui-widgets/src/menu.rs               (dropdown widths in layout, theme-scaled)
erigui-widgets/src/notification.rs       (clip + width)
erigui-widgets/src/radio_button.rs       (button_size in layout)
erigui-widgets/src/spin_box.rs           (caret + button_width + Key::Num* + commit-on-arrow)
erigui-widgets/src/tab_control.rs        (idempotent set_tabs + remove max_tab_width clamp)
erigui-widgets/src/text_area.rs          (Key::Space arm)
erigui-widgets/src/text_input.rs         (Key::Space arm + thicker primary caret)
erigui-widgets/src/tree_view.rs          (item_height + focus border + full-row toggle)
erigui-widgets/tests/widget_tests.rs     (spin_box + text_input regressions)
erigui-widgets/tests/tree_dock_tests.rs  (dock_panel tab-routing test)
erigui-widgets/tests/modal_widgets_tests.rs (dialog measure range)
erigui-examples/examples/*.rs            (16 demos: 14 migrated off convert_window_event,
                                          menu_demo: dark + dropdown render + scaled menu bar)
```

## Test status at session end

- `cargo test -p erigui-widgets --release`: 5 pre-existing
  `modal_widgets_tests::manager_*` failures, all other tests pass
  (~290 passing, 5 failing).
- `cargo build --release --examples -p erigui-examples`: clean,
  zero errors, zero deprecation warnings.
- `cargo build --workspace`: still blocked upstream on
  `erigui-nodes` / `inference-flame` API drift per
  `KNOWN_ISSUES.md`. Out of scope for this session.

## End-state notes

The user said "we call it a night" and asked for this handoff.
They are aware of the open accordion / TextInput-loses-place /
menu_demo body-position items. Pickup is clean — every commit
builds and runs without panics. Pre-existing test failures are
isolated to the notification manager and predate this session.

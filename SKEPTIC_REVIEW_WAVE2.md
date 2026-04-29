# Skeptic review of wave 2 (commits 703a1fa..HEAD)

> Reviewer: Claude (Opus 4.7), 2026-04-28. Read-only re-audit of the 16
> wave-2 commits. Workspace test suite confirmed: 401 passed / 0 failed
> on `cargo test --workspace --release`. Verdict: 1 real bug, 1 mistitled
> test, 2 latent concerns. Build/test integrity intact.

## Verdict per commit

**703a1fa — docs: a11y/kbd_nav/drag_drop singleton justification**
Clean. Documentation-only change in already-deferred files.

**43b2f61 — toolbar tooltip retrofit**
Mostly clean. `with_tooltip_state` and `with_last_tooltip` work; `item_draw_rects` provides one source of geometry truth for tooltip dispatch and draw. Press-dismisses-before-action ordering verified. Disable/hide hooks correct (line 692-707 hides all per-item tooltips when toolbar disabled; line 717-731 hides per-item tooltip when individual item disabled). One latent concern (see Concerns §1) about geometry mismatch between tooltip rect and click-hit rect.

**603e7a2 — checkbox tooltip retrofit**
Clean. Mirrors Button exactly. No `set_enabled` hook in checkbox (the widget has no early-reset block to put one in), so tooltip stays visible until the next event after disable — minor (the disable hook in `handle_event` line 200-202 catches it on the very next event).

**5866fbe — slider tooltip retrofit**
Clean. `set_enabled(false)` calls `tooltip.hide()` (line 383-385) per the audit recommendation. Existing reset block lifted correctly.

**3e0297b — text_input tooltip retrofit**
Behavior change unflagged. The agent added a top-level early-return guard `if !visible || !enabled { ... return Ignored }` at lines 518-523 because TextInput previously had no such guard. Pre-fix, clicking on a disabled TextInput would set `state.focused = true` (line 541) but per-branch checks then suppressed mutation. Post-fix, clicks on disabled inputs are fully ignored. This is **a strictly correct** change but the commit message understates it — it's a behavioral fix, not just tooltip plumbing. No test in the suite caught it because no existing test covered "clicking a disabled TextInput". Acceptable, but should have been called out in the commit message.

**acf467a — spin_box tooltip retrofit**
Clean. `set_enabled(false)` hooks `tooltip.hide()` (line 487-493).

**224a7b8 — combo_box tooltip retrofit**
Clean. `set_enabled(false)` hooks `tooltip.hide()` (line 451-458).

**d3d2c62 — text_area UTF-8 fix in `get_selected_text`**
Clean. All three slice sites (lines 270 → 278, 273 → 283, 282 → 294) routed through `Self::char_to_byte_index`. New test `bug_ta_get_selected_text_utf8_safe` correctly exercises the single-line path with "档案 hello" and Shift+End. The multi-line path is fixed but not directly tested — the pattern mirrors `delete_selection` so confidence is high. `char_to_byte_index` is safe for out-of-range char_col (returns line.len()).

**5c6ad83 — focus-gate KeyPress and TextInput**
Clean as a defensive change. Both arms correctly gate on `self.state.focused && (editing_hour || editing_minute)`. **However, this introduces a real interactive regression**: see Bugs §1 below. The focus gate by itself is correct per the audit; the regression is the missing complementary self-focus on click.

**2a6068b — set_focused propagates and clears edit state**
Clean. Propagates to BOTH `self.input` and `self.calendar_button` (lines 1121-1122). Resets ALL FIVE flags on focus loss: `editing_hour`, `editing_minute`, `show_calendar`, `show_time_picker`, `hovered_day` (lines 1127-1133). Matches audit spec exactly.

**d2255cb — plain Tab returns Ignored**
Clean. Imports `Modifiers`, destructures `KeyPressEvent { key, modifiers, .. }`, gates the consume path on `modifiers.contains(Modifiers::CTRL)` (line 1053). Mirrors `text_area.rs` pattern.

**fff1b31 — calendar shadow drawn before background**
Clean. Three-line move; background and border now occlude shadow correctly; no other reorderings smuggled in.

**9ad526d — geometry constants lifted to module level**
Clean. All 12 consts at module top (lines 40-53) match audit-recommended values exactly (`CALENDAR_PADDING=10`, `CALENDAR_HEADER_HEIGHT=30`, `CALENDAR_CELL_SIZE=30`, `CALENDAR_ARROW_SIZE=16`, `CALENDAR_WIDTH=250`, `CALENDAR_HEIGHT=280`, `TIME_PICKER_PADDING=20`, `TIME_PICKER_INPUT_WIDTH=60`, `TIME_PICKER_INPUT_HEIGHT=30`, `TIME_PICKER_WIDTH=200`, `TIME_PICKER_AM_PM_WIDTH=35`, `TIME_PICKER_AM_PM_HEIGHT=25`). The `let input_width = 200` at `measure()` line 812 was correctly preserved as a separate concept (inner TextInput width, not popup width — they happen to be equal but are unrelated values).

**0065d2d — test accessors + 8 regression tests**
Mostly clean, one test name oversells. All 8 tests are present and run as part of the 401 passing tests. Test accessors gated behind `#[doc(hidden)]` correctly. **However**, `dtp_set_focused_propagates_to_button` (line 2021) does NOT actually verify that the calendar_button got focused — the test comment admits this ("we don't have direct accessors on the children from outside") and asserts only on parent focus + edit-state cleanup. The test name promises something the test doesn't deliver. See Bugs §2.

The `dtp_calendar_click_updates_datetime` test (line 2130) does exercise real click coordinates via `mouse_press(55, 167)` against `handle_calendar_click` — the test comment computes geometry rigorously and the click maps to day 15's cell. Real, not a `select_day` bypass.

**0814faa — MenuItem.on_click → Box<dyn FnMut()>**
Clean. `#[derive(Clone)]` correctly dropped. `grep -rn "menu_item.*clone\|MenuItem.*clone"` returns nothing in production. Both call sites (line 645-672 mouse-click, line 754-784 Enter-handler) correctly split into immutable-borrow-for-gates + mutable-borrow-for-callback. Test `menu_item_callback_can_capture_mut_state` captures `Rc<RefCell<u32>>`, fires twice, asserts count=2 — proves FnMut + multi-fire works.

**e9a7201 — node_graph draw_edges _theme rename**
Clean. Verified `EDGE_COLOR` (line 1047) and `EDGE_HOVER` (line 1048) are local consts inside `draw_edges`, hardcoded to amber/gold. The function legitimately doesn't consume any theme tokens; renaming to `_theme` is honest. The trait-conforming signature shape is justified.

## Bugs to fix

### 1. `date_time_picker.rs` — focus gate regresses interactive use; picker never self-focuses on click

**File:line**: `erigui-widgets/src/date_time_picker.rs:906-953` (the `Event::MouseButton` arm in `handle_event`).

**What's wrong**: Wave 2 added `self.state.focused &&` to the gates on `Event::TextInput` (line 997) and `Event::KeyPress` (line 1020). This correctly stops sibling input theft. **But no code in `handle_event`'s click path ever sets `self.state.focused = true`.** Consequence: a real-world user clicks the trigger button → calendar opens, or clicks a time field → `editing_hour=true`, then types — typing does nothing because `state.focused` is still false. The host has to call `picker.set_focused(true)` separately. `Button` (button.rs:310) and `TextInput` (text_input.rs:541) both self-focus on click; the picker should match that pattern. Tests pass only because every dtp_ test that types calls `p.set_focused(true)` explicitly, bypassing the bug.

**How to fix in 1 sentence**: In the picker's `MouseButton` arm, set `self.state.focused = true` whenever the click hits any of the three regions (calendar_button.bounds, calendar_rect, time_picker_rect) before returning Consumed; mirror Button's `self.state.focused = true` at button.rs:310.

### 2. `bug_fix_tests.rs:2021` — test name `dtp_set_focused_propagates_to_button` doesn't verify what it claims

**File:line**: `erigui-widgets/tests/bug_fix_tests.rs:2021-2040`.

**What's wrong**: The test name says "propagates to button" but the test only checks the parent's `is_focused()` and that `editing_hour` clears on focus loss. The actual propagation to `self.calendar_button.set_focused(focused)` (date_time_picker.rs:1122) is unobservable from outside without an accessor. The test passes trivially as long as `set_focused(false)` runs the cleanup branch — which would still happen even if line 1122 were deleted.

**How to fix in 1 sentence**: Add a `#[doc(hidden)] pub fn calendar_button_focused_for_test(&self) -> bool { self.calendar_button.is_focused() }` accessor and assert on it inside the test, so the test would actually fail if line 1122 regressed.

## Concerns (not bugs but worth flagging)

### 1. `toolbar.rs` — tooltip rect ≠ click-hit rect

`item_draw_rects` (line 230-266) returns vertically-centered rects of size `item_size` (the button's natural size). The `MouseMove` arm at line 887-893 and `MouseButton` arm at line 762-774 both build hit rects with `height = self.state.bounds.height()` (full toolbar height). For toolbars whose buttons are shorter than the toolbar, hovering in the padding area above/below a button will set `hovered_item = Some(i)` (full-height check) but tooltip's `update_on_event` will see `bounds.contains == false` (centered check) and never start the timer. In the common case `item_size.height == bounds.height()` so the rects coincide — this is latent, not present.

Fix would be to use `item_draw_rects` for hit-testing in `MouseMove`/`MouseButton` too, removing the duplicate geometry walk at lines 750-806/870-915 entirely.

### 2. `date_time_picker.rs:1121` — set_focused(true) cascades to inner TextInput, which now races with `update_input_text`

The picker now calls `self.input.set_focused(focused)` and `self.calendar_button.set_focused(focused)` from `set_focused`. The inner TextInput is display-only but isn't read-only (audit deferred item #6). When the picker is focused, the inner TextInput is also focused, which means sibling event dispatch (line 902 `self.input.handle_event(event, theme)`) processes Backspace/keys against the visible date string. That mutation gets clobbered the next time `update_input_text` runs (e.g. on `update_time_from_inputs`). Net effect: visible flicker but eventually consistent. Pre-existing latent issue made more apparent by wave 2's broader propagation.

Resolves cleanly when the deferred TextInput-readonly UX decision lands. Not a wave-2 bug.

### 3. `text_input.rs:518-523` — top-level early-return semantics changed

The new `if !visible || !enabled { ... return Ignored }` block now blocks ALL events on disabled/hidden TextInputs, including `MouseButton` clicks. Pre-fix, a click on a disabled input would set `state.focused = true` (then per-branch gates would block mutation). Post-fix, the disabled input is unfocused-on-click. This is **strictly correct** — disabled widgets shouldn't capture focus — but the commit message frames it as tooltip-suppression plumbing only, when it's also a focus-discipline fix. No tests broke; passing 401/0 confirms safety.

## Quality of work overall

These three agents were reliable. The work is consistently scoped to the audit, idiomatic, and test-backed. Across 16 commits and ~1200 added lines, I found one real interactive regression (the dtp focus gate without self-focus on click), one mistitled test, and three latent concerns — none of them blockers.

Where they cut corners: the dtp focus gate change (5c6ad83) was implemented exactly as the audit specified, but neither the audit nor the agent noticed the missing complementary self-focus. This is partly an audit defect, but a calibrated agent should have flagged "the gate requires `state.focused` but no code path in this file sets it from a click — does that introduce a regression?" The dtp test suite passes because every test that types calls `set_focused(true)` explicitly; the agent never ran the trainer-style "user-clicks-then-types" scenario mentally to catch this.

Where they exceeded the spec: the toolbar retrofit (43b2f61) introduced `item_draw_rects` as a single source of geometry truth — a refactor the audit didn't request but which prevents future drift between draw and hit-test. The text_input retrofit (3e0297b) honestly named that it was adding a top-level early-return guard, even if it understated the behavior change implication. The constants lift (9ad526d) preserved the value `200` in `measure()` as a separate concept, correctly distinguishing inner-TextInput width from popup width.

Calibration data for future waves: agent assumptions follow the audit literally without cross-checking interaction flow. When the audit specifies a focus gate, ensure the audit also specifies the corresponding self-focus path. Tests that verify *propagation through to a child widget* must either expose a child-state accessor or explicitly observe a side effect that REQUIRES the propagation (the cleanup branch alone is insufficient because it runs on the parent regardless). Behavioral changes piggybacked on cosmetic plumbing (text_input early-return) should be called out in commit messages so reviewers can audit them separately.

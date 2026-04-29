# Deep-read audit — text_area.rs and date_time_picker.rs

> Reviewer: Claude (Opus 4.7), 2026-04-28. Read-only deep-read of the two
> largest under-audited widgets. Both were skimmed in the original
> AUDIT_MOJO_PORT_2026-04-28.md and flagged as needing skeptic re-read.
> Scope here is "what's actually wrong, line by line." Citations are
> verified line numbers.

## text_area.rs (1453 LOC)

### What the widget does
Multi-line plain-text editor with selection, undo/redo, line numbers,
horizontal/vertical scroll, mouse-drag select, double/triple-click
word/line select, full Ctrl+arrow/Home/End/PageUp/PageDown navigation,
Ctrl+C/X/V/Z/Y/A clipboard+history shortcuts, and an in-process
clipboard via `crate::clipboard`. Cursor positions are stored in
character columns and converted to byte indices through a single
`char_to_byte_index` helper.

### Mojo-port artifacts found

1. **text_area.rs:270 / :273 / :282 — `get_selected_text` byte-slices
   character columns.** `start_col`/`end_col` come from `cursor_col`,
   which the entire rest of the file treats as a *character* column
   (proven by `Self::char_to_byte_index` calls at lines 363, 400, 410,
   416, 433, 435, 457, 458, 475, 476, 480, 484, 864, 865, 877, 914,
   956). But `get_selected_text` does:
   ```
   self.lines[start_line][start_col..end_col].to_string()
   ```
   On any non-ASCII selection (CJK, accented chars, emoji), this either
   panics on a non-char-boundary slice or splices garbage. Same problem
   at lines 273 and 282. Existing test `bug_ta13_multibyte_selection_boundary_aligned`
   (bug_fix_tests.rs:839) only checks `sel.end_col` is a char count —
   it never calls `get_selected_text()`, so the bug is not caught.
   **Fix: route through `char_to_byte_index` like every other slice
   site does.** This is the single most serious correctness defect
   in either file. (a) mechanical.

2. **text_area.rs:1257-1311 vs :1312-1357 — duplicate Ctrl+letter
   handler paths, but they are NOT redundant.** The skeptic flagged
   this for consolidation. Verified the two paths cover different
   inputs:
   - `Key::Character('c'|'C')` — only emitted by *headless tests* and
     legacy backends. Production winit (erigui-rendering/src/window.rs:99-104)
     filters control characters into a separate `Event::TextInput` and
     never produces `Key::Character`.
   - `Key::C` — production winit emits this from `KeyCode::KeyC`
     (window.rs:183).

   Both paths are reachable depending on host. Deduplicating is fair
   game (helper fn `apply_clipboard_shortcut(self, ch: char, shift)`)
   but it is **not** a Mojo-port artifact — it is real defensive code.
   Deduping is polish, not Rustification. (c) acceptable as-is, but a
   helper would shave ~50 LOC.

3. **text_area.rs:1307-1308 — fall-through `Key::Character(*ch)` calls
   `insert_char`.** This is the same headless-test-only path as the
   Ctrl arm above. In production this branch never fires (winit emits
   `Event::TextInput`, see window.rs:103-104, which is handled at
   text_area.rs:1382). Defensive fallback for tests. (c) acceptable.

4. **text_area.rs:1442-1444 — `can_focus`, `is_focused`, `set_focused`
   correct.** Real `self.state.focused` reads/writes; `can_focus`
   gates on `enabled && visible`. Matches TabControl pattern. (c).

5. **text_area.rs:1097-1099 — `KeyPress` correctly gates on
   `self.state.focused`.** No key stealing from siblings. (c).

6. **text_area.rs:81-82 / :12 — callback signature is `FnMut`.**
   `type ChangeCallback = Box<dyn FnMut(&str)>` matches the recently
   converged sweep. (c) acceptable.

7. **text_area.rs:209-259 — `#[doc(hidden)] pub fn ..._for_test`
   accessors.** Unusual pattern: 7 test-only accessors are public-but-
   hidden. Idiomatic Rust would expose these via `#[cfg(test)]` only,
   or via a `#[cfg(any(test, feature = "test-support"))]` gate. Right
   now they're in production binaries. Acceptable but could trim once
   the test suite is moved into the same crate (currently
   integration-test under `tests/`, hence `pub` is required). (c).

8. **text_area.rs:1257-1311 — Ctrl+A path (lines 1268-1285) and
   :1324-1338 — Ctrl+A path (Key::A arm) are byte-for-byte duplicates
   of select-all logic.** Same as point 2: defensive coverage of two
   key paths, not a Mojo artifact. Could factor into a `select_all()`
   method. (c) acceptable / polish.

9. **text_area.rs:807-812 — `measure(...)` returns the constraint max,
   not a content-derived size.** A TextArea returning 600x400 by
   default ignores its `lines` content. This is acceptable for a
   resizable editor (the host gives bounds), but `measure` should
   probably return *minimum* sensible size, not max. Mojo idiom was
   to return-the-max because Mojo had no Option-typed constraints.
   Low-priority. (b) design-needed: should `measure` reflect lines?

10. **text_area.rs:553-554 — hardcoded `cursor_x = self.cursor_col *
    self.char_width` for `ensure_cursor_visible`.** This uses a fixed
    `char_width` for scroll-target calculation, but `draw` uses
    `context.measure_text` for proportional fonts (lines 866, 878,
    957). Mismatch: ensure_cursor_visible may scroll wrong amounts on
    proportional fonts. (b) design-needed: switch to measure_text or
    accept that the editor is monospace-only.

11. **text_area.rs:587-596 — `position_to_cursor` divides pixel x by
    `self.char_width`.** Same fixed-width assumption as #10. With a
    proportional font, mouse-clicks land on the wrong column. (b)
    design-needed, paired with #10.

12. **text_area.rs:182-203 — `split_lines` correctly handles `\n`,
    `\r`, `\r\n`.** Good. (c).

13. **No singletons. No `OnceLock<Mutex<...>>`. No parallel
    `TextAreaManager`. No raw pointers. No `unwrap()` (only
    `unwrap_or`). No "in a real implementation" stubs.** Verified by
    grep across whole file. (c).

### Already Rust-idiomatic
- Focus state real and correctly gated.
- Callbacks use `FnMut`.
- Undo/redo bounded at 100 entries via `VecDeque` (line 342).
- AltGr fix at line 1105: `ctrl && !alt` correctly handles Linux
  AltGr-as-Ctrl-Alt without stealing typed `@`/`€`/etc.
- Typing-coalesce undo grouping (lines 380-395) is well thought out.
- `preferred_col` for vertical motion (lines 53, 1136-1188) is
  correctly preserved through Up/Down/PageUp/PageDown, reset on
  horizontal motion.
- Drag-select cleanup on out-of-bounds release (lines 1033-1036).
- Multi-click word/line selection via `select_word_at` (line 691)
  and `select_line_at` (line 737) with a 350ms / 4px gate.
- Defensive empty-`lines` guard in `move_cursor_internal` (lines
  526-528).

### Triage
- **(a) mechanical agent-friendly**:
  - **#1 UTF-8 byte-slice bug in `get_selected_text`** (lines
    270, 273, 282). Single-function fix. Add a multibyte test that
    exercises copy/paste of "档案 hello".
- **(b) design-needed**:
  - **#10/#11 monospace-only assumption** in `ensure_cursor_visible`
    and `position_to_cursor`. Question for Alex: is TextArea intended
    to be monospace-only, or should it support proportional fonts
    (matching the cursor/selection draw which already uses
    `measure_text`)?
  - **#9 `measure` ignores content**. Question for Alex: should
    `measure` return `(content_width, line_count * line_height)` so
    auto-layout gives sensible defaults?
- **(c) acceptable**: everything else, including the duplicate
  Ctrl-letter arms (defensive against two real input paths), the
  `#[doc(hidden)]` test accessors, and the headless-test fallback
  for `Key::Character`.

---

## date_time_picker.rs (1009 LOC)

### What the widget does
Composite widget combining a `TextInput` (display-only) and a `Button`
trigger that opens a popup calendar and/or a popup time picker.
Calendar shows month grid with nav arrows, clickable days, month/year
header. Time picker shows hour/minute fields and AM/PM toggle (H12
mode). Datetime is held in `chrono::NaiveDateTime`. Three modes:
Date / Time / DateTime. Three date formats (YMD/DMY/MDY) and two
time formats (H24/H12).

### Mojo-port artifacts found

1. **date_time_picker.rs:905-954 — `Event::KeyPress` arm runs whenever
   `editing_hour || editing_minute`, NOT gated on
   `self.state.focused`.** Same shape as the focus contradictions the
   skeptic catalogued elsewhere. If a sibling widget is focused but
   this picker still has `editing_hour=true` (set on click at
   line 626), keypresses anywhere in the app will edit the hour
   field. **Fix: gate the entire match on `self.state.focused`.**
   (a) mechanical.

2. **date_time_picker.rs:885-904 — `Event::TextInput` arm has the
   same defect.** No focus check. Any TextInput event in the app
   while `editing_hour||editing_minute` is true mutates the hour
   field. (a) mechanical, same fix as #1.

3. **date_time_picker.rs:993-996 — `set_focused` propagates focus to
   `self.input` but not to `self.calendar_button`.** Inconsistent.
   Should mirror to both child components. Also: no clearing of
   `editing_hour`/`editing_minute`/`show_calendar`/`show_time_picker`
   when the host hands focus away. **Fix: on `set_focused(false)`,
   also reset edit/popup flags.** (a) mechanical.

4. **date_time_picker.rs:932-941 — `Key::Tab` is hijacked from focus
   traversal when editing time fields.** Returns `Consumed` for plain
   Tab, breaking app-level Tab navigation. Convention elsewhere in
   this codebase (text_area.rs:1245-1252) is "plain Tab = focus nav,
   Ctrl+Tab = widget-internal." Should follow that pattern: only
   consume Tab if Ctrl is held; let plain Tab fall through to
   `Ignored`. (a) mechanical.

5. **date_time_picker.rs:911-919 — broken indent + mis-grouped
   `if/else if` chain in the `Key::Character` arm.** The brace
   structure is:
   ```
   if self.editing_hour {
       if self.hour_input.len() < 2 {
           self.hour_input.push(*ch);
           self.update_time_from_inputs();
       }
   } else if self.editing_minute
       && self.minute_input.len() < 2 {
           self.minute_input.push(*ch);
           self.update_time_from_inputs();
       }
   return EventResult::Consumed;
   ```
   The `return EventResult::Consumed` sits *outside* both branches at
   the same indent level, which means it always fires — not actually
   buggy, but the indentation is misleading and the `&&`-flattened
   else-if differs from the hour branch's nested-if style. Pure
   stylistic Mojo-port artifact. (a) mechanical.

6. **date_time_picker.rs:166-167 — `update_input_text` overwrites
   `self.input.set_text(text)` every time the model changes.** The
   inner `TextInput` has no read-only mode (verified: no `readonly`/
   `read_only` field anywhere in `text_input.rs`). User typing into
   the visible input is silently lost on the next `update_input_text`
   call (e.g. opening the calendar and clicking a day). **Either**:
   make TextInput read-only (add a `with_readonly(bool)` builder), or
   parse user input on focus loss and round-trip back into
   `selected_datetime`. Currently the input is misleading: it looks
   editable but isn't usefully so. (b) design-needed: pick a UX.

7. **date_time_picker.rs:85-86 — `TextInput::new(WidgetId::default())`
   and `Button::new(WidgetId::default(), ...)` use the **null
   slotmap key**.** Confirmed via erigui-core/src/types.rs:277
   (`pub type WidgetId = slotmap::DefaultKey`). All DateTimePicker
   instances share the null id for their inner widgets. If any host
   code dispatches by id (e.g. `widget_manager.get(input_id)`), this
   collides. In practice these inner widgets aren't registered, so
   the smell is latent. (b) design-needed: should `DateTimePicker`
   accept a `WidgetIdAllocator` and mint real ids, or stay null and
   document the limitation?

8. **date_time_picker.rs:274-275 vs :523-524 vs :853-855 — the same
   constants `padding=10, header_height=30, cell_size=30` are
   re-declared in three sites** (`draw_calendar`, `handle_calendar_click`,
   `MouseMove` hover-day update). If one site changes, hit testing or
   hover detection silently breaks. Same shape repeats for
   `padding=20, input_width=60, input_height=30` between
   `draw_time_picker` (lines 411-413) and `handle_time_picker_click`
   (lines 610-612). **Fix: lift to module-level `const` items or
   private struct fields populated in `layout`.** (a) mechanical.

9. **date_time_picker.rs:313 / :388 — text centering uses
   `String::len()` (byte length), not `chars().count()` or
   `measure_text`.** `header_text.len() as i32 * 4` and
   `day_text.len() as i32 * 3`. For English month names and 1-31 day
   numbers this is fine, but for any localized month name this
   miscentres. Also doesn't use `context.measure_text` like text_area
   does, so font changes break the layout. (b) design-needed: enforce
   `measure_text`-based centering across the codebase, or accept this
   as ASCII-only for now.

10. **date_time_picker.rs:266-272 — shadow drawn AFTER background and
    border**, not before. Visual artifact: shadow is on top of the
    border for the calendar popup. Reading order in `draw_calendar`
    is `fill_rect(bg)` → `draw_rect(border)` → `fill_rect(shadow)`,
    but shadow conventionally renders first/under. Likely a Mojo
    port-paste error. (a) mechanical, 3-line move.

11. **date_time_picker.rs:707-713 — `measure` returns hardcoded
    `200 + 30 + 4` regardless of mode/format.** Time-only mode would
    be visibly narrower; H12 needs more space than H24. Acceptable as
    a default but ignores `_constraints`. (c) acceptable; matches the
    Mojo "fixed measure" idiom but isn't actively wrong.

12. **date_time_picker.rs:798-799 — input and button event-handle
    BEFORE the picker's own match.** Both child widgets see every
    event first and may consume it (especially the button's hover
    state on `MouseMove`). The picker's match arm at 802 then re-
    inspects the same event. Net effect: the click handling is
    correct (button records "I was clicked", picker also toggles
    `show_calendar`), but every `MouseMove` runs twice through the
    button's hover-state machine. Performance: cheap. Correctness:
    OK because the picker's button has no `on_click` callback set in
    `new()`. If a host called `with_on_click` on the button somehow
    (no public API to do so currently), it would double-fire with
    the picker's own toggle. (b) design-needed: pre-route only the
    relevant event to each child.

13. **date_time_picker.rs:828-837 — chained `if-and-method-call`
    pattern works but reads oddly:**
    ```
    if self.show_calendar && self.calendar_rect.contains(*position)
        && self.handle_calendar_click(*position) {
            return EventResult::Consumed;
        }
    ```
    `handle_calendar_click` mutates state AND returns bool —
    side-effect-with-return-value is hard to grep. Rust idiom would
    be to mutate inside if let or to split. (c) acceptable.

14. **date_time_picker.rs:669-670 — `parse().unwrap_or(0)`**. Safe
    fallback for non-numeric `hour_input`/`minute_input`, which can
    never happen because the typing path filters `is_numeric()`
    (line 889) and the constructor formats numerics (line 82). The
    `unwrap_or(0)` is defensive against future code changes — fine.
    No `panic!`/`unwrap()` anywhere in the file. (c).

15. **date_time_picker.rs:241-250 — `first_day_of_week` returns
    `Sunday=0` on invalid month input via `unwrap_or(0)`.** Defensive,
    safe. The `(1..=12).contains(&month)` early return is also
    defensive. No `unwrap()` panic risk. (c).

16. **date_time_picker.rs:998-1000 — `can_focus = enabled && visible`,
    real `is_focused`/`set_focused` reading/writing
    `self.state.focused`.** Correct shape. (c).

17. **date_time_picker.rs:61 — callback is `Box<dyn FnMut(NaiveDateTime)>`.**
    Already converged with the FnMut sweep. (c).

18. **No singletons. No `OnceLock<Mutex<...>>`. No parallel
    `DateTimePickerManager`. No raw pointers. No `unwrap()` (only
    `unwrap_or`). No "in a real implementation" stubs. No bare
    `fn()` callbacks.** Verified by grep. (c).

19. **date_time_picker.rs — calendar has no keyboard navigation.**
    No Up/Down/Left/Right/Enter handling for day selection when the
    calendar popup is open. Discoverability: poor. **Missing
    feature**, not a Mojo-port artifact. (b) design-needed: scope
    of "production quality" — does the user want full kbd nav for
    the calendar, or is mouse-only acceptable?

20. **date_time_picker.rs — no Escape handler to close popups.**
    Standard UX expectation: Esc dismisses an open popover. Currently
    the only way to close is to click the trigger button again or
    click outside. (b) design-needed, same scope question as #19.

### Already Rust-idiomatic
- `selected_datetime: NaiveDateTime`, no stringly-typed date.
- Mode/Format are real enums (`DateTimePickerMode`, `DateFormat`,
  `TimeFormat`) — no `kind: String`.
- `chrono::NaiveDate::from_ymd_opt` / `NaiveTime::from_hms_opt`
  used everywhere (lines 247, 588-589, 693). No date-validation
  unwrap panics — the README claim holds for this file. (Skeptic
  was right to verify; verified.)
- `set_focused` propagates to `self.input` (partially — see #3).
- Bounded inputs (`.min(23)`, `.clamp(1, 12)`, `.min(59)`) prevent
  out-of-range time values (lines 673-690).
- `show_calendar`/`show_time_picker` toggle exclusively (line
  812-822) so two popups never stack.

### Triage
- **(a) mechanical agent-friendly**:
  - #1 + #2 + #3 — focus gating and propagation. All three are
    one PR. ~30 LOC. Tests required: `key_press_when_unfocused_does_not_edit_time`,
    `text_input_when_unfocused_does_not_edit_time`, `set_focused_false_clears_edit_state`.
  - #4 — Tab traversal fix. ~5 LOC.
  - #5 — re-indent the Key::Character arm. Pure cosmetic. ~10 LOC.
  - #8 — lift constants to module-level. ~30 LOC. No behaviour
    change but eliminates drift risk.
  - #10 — shadow draw order. 3-line move.
- **(b) design-needed**:
  - #6 TextInput readonly UX. Question: should the trigger's
    inner TextInput be read-only?
  - #7 Inner WidgetId nullness. Question: do we need real ids
    here?
  - #9 String::len-based centering. Question: enforce
    measure_text everywhere?
  - #11 measure fixed at 234px. Question: scale by mode?
  - #12 child-event pre-dispatch. Question: route per-region?
  - #19 / #20 calendar kbd nav and Esc. Question: scope of
    production quality.
- **(c) acceptable**: everything else.

---

## Cross-cutting findings

1. **Both files use `crate::clipboard`** (text_area.rs:1, indirectly
   in date_time_picker via TextInput/Button). The clipboard module is
   already audited as (c) acceptable with documented limitation
   (process-local `RwLock<String>`). No duplicate clipboard
   re-implementation in either file. Good.

2. **Both files correctly use `Box<dyn FnMut(...)>`** for callbacks.
   No need for the FnMut sweep on these two files — they're already
   converged. (text_area.rs:12, date_time_picker.rs:61).

3. **Both files have correct focus state shape** — real
   `self.state.focused`, `can_focus = enabled && visible`,
   `set_focused` writes through. **The focus *gating* on the keyboard
   handler differs:** text_area.rs:1097 correctly returns Ignored when
   not focused; date_time_picker.rs:905-954 does NOT, and runs key
   handling whenever `editing_hour||editing_minute` is true. This is
   the most consequential cross-file inconsistency.

4. **Both files use `#[doc(hidden)] pub fn ..._for_test`** -ish
   patterns (text_area.rs:209-259) for integration test access. None
   in date_time_picker — its tests in
   `widget_tests.rs:342` explicitly call out that "internal state is
   private; smoke-level test that the fix compiles is the most we can
   do." Cross-file consistency: would be cleaner to either expose
   test accessors uniformly (text_area pattern) or keep all internal
   (date_time_picker pattern). Preference depends on test ergonomics
   wanted. (c).

5. **Both files use `chrono`/numeric/byte-index helpers without
   panic**. Verified: every `.unwrap()` in either file is `unwrap_or`.
   Both files honour the README's "no unwrap panics" claim.

6. **Hardcoded pixel values** in date_time_picker (8 sites cited
   above) versus theme-driven rendering in text_area (which uses
   `theme.typography.font_size_base`/`_small`/`_large` and
   `theme.colors.*` everywhere). text_area.rs *does* still hardcode
   `tab_size: 4`, `gutter_width: 50`, scroll-margin `20`, undo cap
   `100` — but those are user-visible behaviours, not theme tokens.
   The cross-cutting smell is "calendar/time-picker hardcoded
   geometry doesn't scale with theme font size." (b) design-needed.

7. **Neither file has a `#[cfg(test)]` module of its own** — all
   tests live in `erigui-widgets/tests/bug_fix_tests.rs` and
   `widget_tests.rs`. text_area has 30+ regression tests there;
   date_time_picker has 0 (only a comment in widget_tests.rs:342
   noting tests can't reach internals). **Date picker test coverage
   is the biggest evidence gap of the two files**, and any
   mechanical fix should add tests at the same time — exposing
   `editing_hour`/`editing_minute`/`show_calendar` via
   `#[doc(hidden)]` accessors mirroring text_area's pattern.

---

## Recommended next-agent brief

**Title**: Production-harden DateTimePicker focus + clean up text_area
UTF-8 + lift DateTimePicker constants

**Files**:
- `/home/alex/EriGui/rust-gui/erigui-widgets/src/text_area.rs`
- `/home/alex/EriGui/rust-gui/erigui-widgets/src/date_time_picker.rs`
- `/home/alex/EriGui/rust-gui/erigui-widgets/tests/bug_fix_tests.rs` (or
  add a `date_time_picker_tests.rs` next to it)

**Changes**:

1. **text_area.rs:270 / :273 / :282 — fix UTF-8 byte-slice in
   `get_selected_text`.** Replace direct `[start_col..end_col]`
   slicing with `Self::char_to_byte_index`-mediated slicing, mirroring
   the `delete_selection` pattern at lines 472-491. Add a test that
   selects a multibyte string ("档案 hello") with Shift+End, calls
   `get_selected_text()`, and asserts the returned `String` matches
   the visible characters.

2. **date_time_picker.rs:905 — gate the entire `Event::KeyPress` arm
   on `self.state.focused` before checking `editing_hour||editing_minute`.**
   Same for the `Event::TextInput` arm at line 885.

3. **date_time_picker.rs:993-996 — `set_focused(focused)`** must:
   - propagate `focused` to BOTH `self.input.set_focused(focused)`
     AND `self.calendar_button.set_focused(focused)`
   - if `focused == false`, reset
     `editing_hour=false, editing_minute=false, show_calendar=false,
     show_time_picker=false, hovered_day=None`.

4. **date_time_picker.rs:932-941 — Tab handling**: only consume Tab if
   Ctrl modifier is set; otherwise `return EventResult::Ignored` so
   focus traversal can proceed. Match text_area.rs:1245-1252 pattern.

5. **date_time_picker.rs:266-272 — move shadow `fill_rect` to BEFORE
   the background `fill_rect(self.calendar_rect)` call.** Three-line
   re-order. Visual fix.

6. **date_time_picker.rs — lift `padding=10, header_height=30,
   cell_size=30, arrow_size=16, padding=20 (time), input_width=60,
   input_height=30, calendar_width=250, calendar_height=280,
   time_picker_width=200, am_pm_button_width=35, am_pm_button_height=25`
   to module-level `const` items**, single source of truth. Update
   call sites at lines 274-275, 320, 411-413, 523-524, 535, 610-612,
   708-709, 718, 743-744, 749-750, 853-855.

7. **Add test accessors to date_time_picker.rs** mirroring text_area's
   `#[doc(hidden)] pub fn`:
   - `editing_hour_for_test() -> bool`
   - `editing_minute_for_test() -> bool`
   - `show_calendar_for_test() -> bool`
   - `show_time_picker_for_test() -> bool`
   - `selected_datetime_for_test() -> NaiveDateTime`

   Add 8 tests to bug_fix_tests.rs (or a new file):
   - `dtp_focus_gates_keyboard_input`
   - `dtp_focus_gates_text_input`
   - `dtp_set_focused_false_clears_edit_state`
   - `dtp_set_focused_propagates_to_button`
   - `dtp_plain_tab_returns_ignored`
   - `dtp_ctrl_tab_cycles_hour_minute`
   - `dtp_text_input_appends_to_active_field`
   - `dtp_calendar_click_updates_datetime`

   Plus 1 test for the text_area UTF-8 fix:
   - `bug_ta_get_selected_text_utf8_safe`

**Expected test count delta**: +9 tests, all should pass.

**Out of scope**: TextInput read-only (#6 above), calendar keyboard
nav (#19), Esc-closes-popup (#20), TextArea proportional-font support
(#10/#11), shared-WidgetId (#7), and the duplicate Ctrl-letter handler
in text_area.rs (it's defensive, not a port artifact).

**LOC estimate**: ~50 added, ~30 modified, ~10 reorganised. Single
agent, single PR, low risk.

---

## Triage counts

- text_area.rs findings: 13
  - (a) mechanical: 1 (UTF-8 byte-slice)
  - (b) design-needed: 3 (proportional font support [2], measure
    semantics)
  - (c) acceptable: 9
- date_time_picker.rs findings: 20
  - (a) mechanical: 6 (focus gating x2, propagation, Tab, indent,
    shadow order, constants)
  - (b) design-needed: 7 (readonly UX, null id, string len centering,
    measure scaling, child pre-dispatch, calendar kbd, Esc)
  - (c) acceptable: 7
- Cross-cutting: 7 observations.

**Total: 33 findings + 7 cross-cutting = 40 items catalogued.**

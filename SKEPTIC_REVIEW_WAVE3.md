# Skeptic review of wave 3 (test-coverage agents)

> Reviewer: Claude (Opus 4.7), 2026-04-28. Read-only audit of wave-3 test
> coverage commits `9be50c8..97519d5` (10 commits). Test count
> verified: `cargo test -p erigui-widgets --release` reports
> **452 passed / 0 failed / 1 ignored** across the 8 test sections,
> matching the brief's claimed +213 wave-3 tests over the wave-2
> baseline of 239. The 1 ignored is `node_graph::preview` GL test
> (pre-existing, unrelated). Verdict: tests are mostly sound; **3
> tests with name-vs-body oversells**, **5 tests deliberately
> lock in real bugs** (agent 4 flagged these correctly), **2
> agent-flagged bugs were over-called**, **1 new bug missed by
> all four agents**. Calibration data at the end.

## Verdict on the 4 test files

**`simple_widgets_tests.rs` (Agent 1, 63 tests)** — Clean. Label/Container/StatusBar/Icon coverage is honest about its limitations: where state is private and there's no observable side-effect, tests degenerate to "construction + measure doesn't panic," and the file says so explicitly in module comments. The Icon `IconRecorder` DrawContext stub is a fair design — counts calls per draw routine. Two concerns: (a) `icon_gear_draws_polygon_and_center_hole` and `icon_palette_draws_two_ellipses` only count `fill_ellipse` calls — they do NOT verify the alpha-zero "punch a hole" trick actually erases pixels (it doesn't; bug 3/4 are real). The tests pass while the rendered output is wrong. (b) `container_layout_sets_bounds` (line 319) confirms bounds-set but the agent didn't add a test that would FAIL when `Container::layout` doesn't lay out children — locking in bug 2.

**`modal_widgets_tests.rs` (Agent 2, 47 tests)** — Mostly clean. Dialog tests mirror the private button-rect geometry constants in helper `button_click_position_n` (line 408-420) — fragile, but the constants are stable. Notification/Manager tests use `update_animations(10.0)` to settle slide-in deterministically, which is a nice trick. Concern: `dialog_open_modal_consumes_unrelated_clicks` (line 296) and `dialog_set_focused_false_does_not_panic` (line 391) explicitly LOCK IN questionable contracts — modal-click-outside doesn't close, and `is_focused` is wired to `is_open` ignoring the `focused` arg. These are design calls, not bugs the agent should have inverted, but the tests should be written to FAIL if the contract changes (which they are). Geometry-mirroring tests at line 234 (close button) and line 692-704 (notification close-button) are the same fragility pattern wave-2 flagged for date_time_picker — solvable with `#[doc(hidden)] pub fn ..._for_test_geometry()` accessors.

**`tree_dock_tests.rs` (Agent 3, 47 tests)** — Solid on TreeView, thin on DockPanel. TreeView click-on-row, click-on-expand-button, and `selected_id` traversal tests are real round-trips with observable state. **Concern: `dock_panel_remove_collapses_split_when_one_side_emptied` (line 976) test name oversells** — same shape as wave-2's `dtp_set_focused_propagates_to_button`. The test calls `remove_panel("left")`, asserts the layout-change callback fires, then re-layouts. It does NOT verify the dock tree actually collapsed (which it can't, since the tree is private). The test would pass even if `remove_from_dock_tree` were a no-op as long as the callback fired. **Concern: `dock_panel_add_in_each_position_does_not_panic`** (line 808) iterates DockPosition::Center after Left was added — silently masking bug 16 (Center on a Split-root no-ops). The test doesn't observe panel reachability.

**`picker_widgets_tests.rs` (Agent 4, 56 tests)** — Highest-quality wave-3 file. Agent 4 honestly flagged its own locked-in tests in module comments. ColorPicker tests reach into popup geometry (sv_size, hue_y, alpha_y, hex_y) and assert real color round-trips through HSV — those are solid. **The five locked-in tests Agent 4 flagged ARE real bug-locking** (see next section). The geometry-mirroring concern from modal_widgets applies here too (sv_rect, hue_rect, alpha_rect, hex_rect mirror private layout constants).

## Bugs locked into tests (need test inversion or comment)

Agent 4 explicitly flagged these as locked-in. Triage:

| Test | Locks in | Triage |
|------|----------|--------|
| `color_picker_set_color_fires_callback_each_call` (picker:107) | `set_color` fires callback even when color unchanged (bug 19) | **Invert** — change to `assert_eq!(v.len(), 2)` once bug-fixer adds equality short-circuit. The test's current form actively prevents that fix. |
| `color_picker_with_color_does_not_fire_callback` (picker:128) | `with_color` builder skips callback while `set_color` doesn't (bug 20) | **Keep + comment** — "with_color is a builder, callbacks not yet installed" is a defensible Rust pattern. Document the asymmetry. |
| `file_filter_matches_extension_case_sensitive` (picker:704) | `*.JPG` excluded by `*.jpg` filter (bug 22) | **Invert** — fix `matches()` to lowercase both sides; this test breaks cross-platform (Windows is case-insensitive on FAT/NTFS). |
| `file_filter_wildcard_matches_anything_with_extension` (picker:725) | `Makefile` excluded by `*` filter (bug 23) | **Invert** — `*` should match all paths. Native pickers do. |
| `file_dialog_escape_key_fires_cancel_callback` (picker:937) | Escape always cancels even when filename_input is focused (bug 25) | **Design call** — see triage. If kept, add comment "modal contract: Escape always closes". |

Modal_widgets has additional locked-in:
| `manager_clear_does_not_panic_and_does_not_fire_close_callback` (modal:653) | `clear()` skips callback, `remove()` fires it (bug 8) | **Invert** — fix `clear()` to fire callbacks per-item. The test comment honestly says "if someone changes clear() to fire callbacks, they'll have to intentionally update this test." That's wave-3's problem to fix. |
| `manager_press_event_does_not_remove_notifications` (modal:921) | Close-button fires on release-only (bug 9) | **Keep + comment** — release-only is defensible (matches button.rs:310 self-focus on click but action-on-release pattern). Document. |
| `dialog_set_focused_false_does_not_panic` (modal:391) | `is_focused` wired to `is_open`, `set_focused` arg ignored (bug 6) | **Design call** — Dialog's `is_focused` representing "modal active" is defensible. The set_focused side-effect (clearing is_dragging when focused=false) is fine. |
| `dialog_open_modal_consumes_unrelated_clicks` (modal:296) | Modal click-outside does NOT close (bug 7) | **Design call** — explicit contract in test comment. macOS/Windows native modals also don't auto-close on click-outside. Defensible. |

Simple_widgets icon tests:
| `icon_gear_draws_polygon_and_center_hole` (simple:818) | Counts `fill_ellipse` only; alpha-zero "hole" doesn't actually erase (bug 3) | **Keep + comment** — full visual verification needs a pixel-level test infra that doesn't exist yet. Add `// FIXME: alpha-zero does not punch a hole; bug 3 in source.` |
| `icon_palette_draws_two_ellipses` (simple:826) | Same as above (bug 4) | Same. |

## New bugs the test agents missed

1. **`file_dialog.rs:239` — index-out-of-bounds panic on `with_filters(Vec::new())` followed by user navigation.**
   `selected_filter` is `usize` defaulting to 0. `with_filters(empty_vec)` replaces filters with empty, but doesn't reset `selected_filter`. The next `navigate_to_path` (e.g., user clicks a directory) calls `refresh_file_list`, which indexes `self.filters[self.selected_filter]` at line 239. **Panic on the first non-dir file.** The test `file_dialog_with_filters_empty_does_not_panic` at picker:841 only exercises the constructor + chain; no subsequent navigation. (M) — fix at file_dialog.rs:238 by guarding `if !self.filters.is_empty() && !is_dir && ...`.

2. **`status_bar.rs:152` separator gap is always 2px regardless of style — bug 5 promoted to confirmed.** When `SeparatorStyle::None` is selected, `calculate_panel_rects` still reserves 2px between panels, leaving visible gaps with nothing drawn. Visual mismatch. (M).

3. **`color_picker.rs:679-688` color flickers through nonsensical values while typing partial hex.** Bug 29 mischaracterized by Agent 4 — there's no "swallowed parse failure" because `is_ascii_hexdigit` filters non-hex. The real issue: typing "F" → "FF" → "FF0" → "FF00" updates color on every keystroke, parsing "F" as `Color::from_hex(0x0000000F)` = `rgb(0,0,15)`, then "FF" as `rgb(0,0,255)`, etc. Flicker is real but bug shape is "intermediate parses are visible," not "silently swallowed". Fix: only call `update_color_from_hex` when `hex_input.len() == 6`. (M).

## Agent-flagged bugs that were over-called

1. **Bug 13 — `tree_view.rs:244-283` `node_at_position` scroll-math drift when bounds.y ≠ 0 — DOES NOT REPRODUCE.** I traced through with bounds.y=10, scroll_offset=20: `adjusted_y = position.y + scroll_offset` correctly converts viewport-y back to tree-y, then `current_y` starts at `bounds.y()` which is the absolute tree-y of the first row. Math is correct. The agent flagged it without a worked example. (C) Already correct.

2. **Bug 14 — `tree_view.rs:378-413` depth/y-tracking on click is buggy for deep nodes — DOES NOT REPRODUCE.** The recursive `find_node_info` correctly threads `current_depth + 1` through descents. `node_y` (current_y) IS mishandled (only incremented between siblings, not pre-recursion), but `node_y` is only read at line 412 to populate `node_depth` from depth, and the post-call code at line 416 uses ONLY `node_depth` and `position.x`/`position.y`, never `node_y`. The y-mishandling is dead code. (C) Already correct (or at least not impactful).

3. **Bug 21 — `color_picker.rs:445-456` Compact-mode hit rect uses bounds.width() not preview_size — INTENTIONAL.** The compact widget DRAWS preview_size square + dropdown arrow at `preview_rect.right() + 4` (line 217). The hit rect spanning bounds.width() (=preview_size + 12) correctly covers BOTH preview and arrow. This is correct UX. (C) Intentional.

## Triaged bug list

**(M) = mechanical fix, route to bug-fixer.** **(D) = design call, ask Alex.** **(C) = intentional or already correct.**

| # | File:line | Description | Triage | 1-sentence fix |
|---|-----------|-------------|--------|----------------|
| 1 | container.rs:228-230 | Empty `if !visible {}` block missing `return` | (M) | Add `return;` inside the guard. |
| 2 | container.rs:222-226 | `Container::layout` no-ops; doesn't lay out children | (D) | Decide: (a) implement child-layout via TODO note's "widget manager access," or (b) confirm "Container is a marker, layout is external" and document. Currently silently broken. |
| 3 | icon.rs:343-350 | `draw_gear` uses `with_alpha(0)` to "punch hole"; doesn't erase | (M) | Re-fill with theme background color instead of `color.with_alpha(0)`. |
| 4 | icon.rs:371-379 | Same alpha-zero pattern in `draw_palette` | (M) | Same fix as #3. |
| 5 | status_bar.rs:152 vs :158 | Separator gap always +2 even when style=None or Line | (M) | In `calculate_panel_rects`, derive separator width from `self.separator_style` (0 for None, 1 for Line, 2 for Raised/Sunken). |
| 6 | dialog.rs:418-420 | `is_focused` returns `is_open`; `set_focused` arg ignored | (D) | Dialog's "focus = modal active" is defensible. Decide: keep (document), or add real `WidgetState::focused` field. |
| 7 | dialog.rs:369-373 | Modal click-outside consumes but doesn't close | (C) | Match macOS/Windows modal contract. Already documented in test. Keep. |
| 8 | notification.rs:200-202 | `clear()` skips `on_notification_closed` callback | (M) | Iterate `self.notifications.drain(..)` and fire callback for each. |
| 9 | notification.rs:543 | Close-button fires on release-only, not press | (D) | Defensible (matches release-action UX), but inconsistent with checkbox/button which fire on press. Decide: keep release-only (document), or align with rest of toolkit. |
| 10 | notification.rs:91 | Only `with_progress` builder; no `set_progress` setter | (M) | Add `pub fn set_progress(&mut self, progress: f32)` that mirrors `with_progress` clamp + assignment. |
| 11 | dock_panel.rs:289 | `remove_from_node` borrow recursion smell | (C) | Compiles, behaves correctly. Refactor candidate, not a bug. Skip. |
| 12 | dock_panel.rs:280 | Literal `// TODO: Implement proper tree cleanup` comment | (M) | Either remove the TODO (function is functionally correct per visual test) or expand the cleanup. The comment is misleading. |
| 13 | tree_view.rs:244-283 | `node_at_position` scroll math drifts when bounds.y ≠ 0 | (C) | False alarm; math is correct. Skip. |
| 14 | tree_view.rs:378-413 | Depth/y-tracking buggy on click for deep nodes | (C) | False alarm; depth is correctly threaded, the y-tracking is dead. Skip. |
| 15 | tree_view.rs:108-129 | `selected_id` skips selected nodes under collapsed parents | (C) | Defensible "only visible selection" contract; test pins this. Keep. |
| 16 | dock_panel.rs:158-184 | `add_panel(_, Center)` after Split-root silently no-ops | (M) | In `add_to_dock_tree`, when root is Split and position is Center, descend to first leaf-Tabs and append, OR `panic!("add Center to Split root not yet supported")` to surface the issue. |
| 17 | dock_panel.rs | Drag-and-drop docking missing post-DropTarget removal | (D) | Wave-1 deferred; needs Alex on whether drag-to-dock is a Q3 feature. |
| 18 | tree_view.rs:364 | TreeView has zero keyboard nav | (D) | Defer (consistent with audit's "keyboard_nav globally deferred"). |
| 19 | color_picker.rs:97-99 | `set_color` fires callback even when color unchanged | (M) | Wrap in `if self.color != color { ... callback(color) }`. |
| 20 | color_picker.rs:59-68 vs 89-100 | `with_color` builder skips callback, `set_color` fires it | (C) | Defensible — builders run before user-visible install. Document. |
| 21 | color_picker.rs:445-456 | Compact hit rect spans bounds.width(), preview is square | (C) | Intentional; arrow is part of click target. Skip. |
| 22 | file_dialog.rs:43 | `FileFilter::matches` is case-sensitive | (M) | Lowercase both `ext_str` and the comparison entry: `e.eq_ignore_ascii_case(ext_str)`. |
| 23 | file_dialog.rs:36-48 | `*` wildcard requires extension; `Makefile` excluded | (M) | When extensions list contains `"*"`, return `true` immediately before the `path.extension()` check. |
| 24 | file_dialog.rs:239 | `with_filters(Vec::new())` → panic on next navigate | (M) | Guard at line 238: `if !is_dir && self.mode != FileDialogMode::SelectFolder && !self.filters.is_empty()`. |
| 25 | file_dialog.rs:712-717 | Escape always fires `on_cancel`, ignoring focused child | (D) | Decide: keep (modal contract — Escape always closes), or check `if !self.filename_input.is_focused()` first. |
| 26 | file_dialog.rs:621-630 | Every event re-runs ALL child handlers | (D) | Performance smell; refactor to dispatch to single focused child. Out-of-scope for bug-fixer (architectural). |
| 27 | file_dialog.rs:660-684 | "Double-click" comment, but no double-click detection | (M) | Either rename comment to "single-click navigates" (current behavior) and remove the misleading comment, or implement double-click via `last_click_time: Option<Instant>` field with 500ms threshold. The current behavior (single-press navigates) is broken UX — selecting a directory then clicking it once enters it without a second click. |
| 28 | color_picker.rs:660 | Duplicate `Key::Character` and `Event::TextInput` paths | (C) | Intentional — line 658-659 comment documents headless-fallback. Verified pattern matches `text_area.rs:1257-1357`. Keep. |
| 29 | color_picker.rs:679-688 | Color flickers through nonsensical intermediate values during hex typing | (M) | Add `if self.hex_input.len() == 6` guard before `update_color_from_hex` call. |
| **NEW** | container.rs:228-230 | (covered by #1) — note that test `container_*` does not catch the missing return | — | — |
| **NEW** | (skeptic-found) | Tree-dock test `dock_panel_remove_collapses_split_when_one_side_emptied` name oversells | — | Either rename (`dock_panel_remove_keeps_layout_callback_consistent`) or extend with a `#[doc(hidden)] pub fn root_node_kind_for_test()` accessor that returns "tabs|split|empty" so the test can verify collapse. |
| **NEW** | (skeptic-found) | Modal test `dialog_close_button_closes` mirrors private close_size=20 / 5px inset | (D) | Same fragility pattern as wave-2 dtp. Add `#[doc(hidden)] pub fn close_button_rect_for_test(&self) -> Rect` to dialog.rs. |
| **NEW** | (skeptic-found) | Picker test `color_picker_compact_popup_*` mirrors padding=10, sv_size=200, hue_y=256, alpha_y=286, hex_y=316 | (D) | Same. Add `#[doc(hidden)] pub fn compact_popup_rects_for_test(&self) -> CompactPopupRects` to color_picker.rs. |

## Recommended bug-fixer scope for wave 3

The (M) list has 12 mechanical fixes. Doing all twelve in one bug-fixer run is too much — pick by leverage and risk.

**Tier 1 (highest leverage, low risk, do these now):**
1. **#1** container.rs:228-230 missing `return;` — 1-line fix, untangles a reachable code path.
2. **#22** file_dialog.rs:43 case-sensitive extension match — fix `e.eq_ignore_ascii_case(ext_str)`. Cross-platform breakage.
3. **#23** file_dialog.rs:36-48 `*` requires extension — `if extensions.contains(&"*".to_string()) { return true; }` early. Affects native-picker parity.
4. **#24** file_dialog.rs:239 panic on `with_filters(empty)` — guard the index access. Real crash.
5. **#19** color_picker.rs:97-99 callback flood on same-color `set_color` — wrap in equality check. Inverts test #19, route to test fix in same PR.
6. **#27** file_dialog.rs:660-684 single-press navigates as if double-click — implement `last_click_time: Option<Instant>` (~30 LOC). Real UX bug.

**Tier 2 (do if Tier 1 lands cleanly, same agent):**
7. **#3, #4** icon.rs alpha-zero "hole" — replace with theme.colors.surface. May need theme-passing in `Icon::draw_gear`/`Icon::draw_palette` signatures, or document as "no fallback hole; fixed background only." Check signature breakage first.
8. **#10** notification.rs add `set_progress` setter — 5 LOC. Same shape as `set_color`/`set_text` patterns.
9. **#29** color_picker.rs hex flicker — add `if self.hex_input.len() == 6` guard. 1-line fix.
10. **#5** status_bar.rs separator gap — derive width from style. ~10 LOC.

**Tier 3 (skip until next wave or design pass):**
- **#8** notification.rs `clear()` callback symmetry — needs test inversion in same PR; coordinate.
- **#16** dock_panel.rs Center-on-Split silent no-op — add either descent or panic. Needs Alex on UX intent.
- **#12** dock_panel.rs:280 misleading TODO comment — either delete comment or implement; needs Alex.

**Don't fix in wave 3:**
- (D) items: Alex must decide first. Do not have the bug-fixer guess.
- (C) items: false alarms or intentional contracts. No action.
- #11 (borrow recursion) and #26 (event-fanout) — architectural, not mechanical.

**Recommended Tier-1 + Tier-2 = 10 fixes**, ~150 LOC across 4 files (icon.rs, container.rs, file_dialog.rs, color_picker.rs, notification.rs, status_bar.rs). One agent, one day. Tests for #19/#22/#23 must be inverted in the same PR to avoid the "passing tests lock in the bug" trap.

## Quality of work overall

These four agents were **better calibrated than wave-2**. The main quality signals:

- **Module comments are honest about coverage limitations.** Agent 1's "where the public API doesn't expose enough state, the test degenerates to construction + measure doesn't panic" is exactly the kind of self-deprecating accuracy you want. Agent 4 went further and explicitly listed its own locked-in tests in the "Bugs to file" report — that's calibrated humility.
- **Geometry-mirroring is acknowledged.** All four agents wrote helper comments explaining where they pulled private constants from (`button_height=30, button_width=80, gap=10` in modal:411-412; sv_size=200, padding=10 in picker:299-300). The wave-2 skeptic pushed for `#[doc(hidden)] pub fn ..._for_test_geometry()` accessors; wave-3 didn't add them but DID document the fragility. Half a step forward.
- **Locked-in tests are flagged.** Agent 4's flagging of `color_picker_set_color_fires_callback_each_call`, `file_filter_*_case_sensitive`, `file_dialog_escape_*` is correct. The agent could have chosen to invert one (e.g., file_filter case-sensitivity) and write the test against the desired behavior, but that's not the agent's job — it's the bug-fixer's. Agent 4's role was "write tests against current behavior + flag bugs", and it did that.

**Where they cut corners:**

- **Three agents missed bug 24** (`with_filters(Vec::new())` → panic on navigate). The test `file_dialog_with_filters_empty_does_not_panic` exercises only the constructor chain, not a subsequent user-driven navigate. A "what's the next API call after this construction?" check would have caught it.
- **Agent 3 over-called bugs 13 and 14** (TreeView scroll-math, depth-tracking). Both are false alarms. The skeptic should expect 1-2 false positives per agent in a 25-bug list — that's normal calibration noise. Don't penalize.
- **`dock_panel_remove_collapses_split_when_one_side_emptied`** test name oversells (Agent 3). Same shape as wave-2's `dtp_set_focused_propagates_to_button`. The wave-2 calibration data was already in `SKEPTIC_REVIEW_WAVE2.md` and Agent 3 didn't apply the lesson. The fix is simple: any test name that promises observation of private state needs a `#[doc(hidden)]` accessor.
- **Agents 2 and 4 didn't propose `#[doc(hidden)]` accessors despite clearly needing them** for Dialog's button rects, ColorPicker's popup geometry, and Notification's close/action button rects. This is a systemic test-architecture gap that should be raised across the board before wave 4.

**Where they exceeded the spec:**

- Agent 1's `IconRecorder` DrawContext stub is a clean reusable artifact. Future icon visual-regression tests can extend it (capture full call-sequences, not just counts).
- Agent 2's notification eviction test (`manager_max_visible_evicts_oldest_with_callback`, modal:605) cleverly uses the close-callback as the SOLE observable signal of eviction. Without that callback the test would have to inspect a private queue.
- Agent 4's `with_initial_path` to `Cargo.toml` for verifying `selected_file` round-trip (picker:1207-1226) uses `env!("CARGO_MANIFEST_DIR")` for hermetic file resolution — better than hardcoding `/tmp` for that one test.

**Calibration data for wave 4:**

- Continue accepting "no widget has zero tests anymore" as the bar. Coverage depth comes from later waves.
- Demand `#[doc(hidden)] pub fn ..._for_test_geometry()` accessors on ANY widget where tests mirror private layout constants. Wave-2 shipped them for date_time_picker; wave-3 should retroactively add them to Dialog, ColorPicker, FileDialog, NotificationManager. ~50 LOC across 4 files.
- When an agent flags a "locked-in" test, it must either (a) propose the inversion in the same task transcript, or (b) explicitly mark the test with a `// FIXME(bug-N):` comment so future readers can grep them. Wave-3 did (b) inconsistently.
- Skeptic should expect ~10% false-positive rate on agent bug lists. Don't penalize agents for false alarms; do penalize for missed real bugs (like #24 for FileDialog).
- The "test-as-mechanical-regression-net" framing is the right one for this stage. Don't push agents to write integration tests against the rendered pixels — that infrastructure doesn't exist yet and the temptation to mirror geometry is the result.

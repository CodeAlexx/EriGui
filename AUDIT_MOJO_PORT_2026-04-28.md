# Mojo→Rust port audit, erigui-widgets

> Author: Claude (Opus 4.7), 2026-04-28.
> Trigger: Tooltip task (#5) revealed the auto-show/hide logic exists
> but lives behind a global `OnceLock<Mutex<TooltipManager>>` singleton
> that no widget actually uses. Alex confirmed: "the original code was
> simply a port of mojo code to rust. mojo had limitations and it
> carried to this repo. now we are making erigui a rust working lib."
>
> This audit lists every spot in `erigui-widgets/src/` that smells
> Mojo-shaped, classifies each as **(a) mechanical** (safe to delegate
> to a rewrite agent), **(b) design-needed** (decide first), or
> **(c) acceptable** (smell but justified). Rewrite agents fire only
> after Alex triages.
>
> A skeptic agent will re-audit this document to catch what the author
> missed.

## Inventory

40 .rs files, ~22.9k lines under `erigui-widgets/src/`. Largest:
`node_graph/mod.rs` (3823), `text_area.rs` (1453), `date_time_picker.rs`
(1009), `dock_panel.rs` (879). Smallest: `clipboard.rs` (36).

## Patterns searched for

- `OnceLock<Mutex<...>>` / `static <NAME>: <Lock>` globals
- `pub struct \w+Manager` parallel to widget structs
- Hardcoded `false` returns in `is_focused` / `can_focus`
- No-op `fn set_focused(&mut self, _: bool) {}`
- `_id: WidgetId` underscore-prefixed unused fields
- `panic!(`, `unimplemented!(`, `todo!(`, `FIXME`, `XXX`
- "In a real implementation" stub markers
- `pub struct \w+Ext` free-standing extension types alongside trait

---

## Findings — by file

### tooltip.rs (319 LOC) — **(a) mechanical**, scope known
- Global `OnceLock<Mutex<TooltipManager>>` at line 279.
- Parallel `TooltipManager` struct with full update() and draw() that
  no widget ever drives — auto-show/hide exists but is unreachable.
- `_id: WidgetId` field on Tooltip is dead (line 15).
- `TooltipWidget` extension struct (line 295) is itself a Mojo-shape —
  Rust would put `with_tooltip(text)` on the widgets that need one.
- **Agent brief**: drop the singleton + `TooltipWidget` extension.
  Introduce a `TooltipState { hover_start: Option<Instant>, visible:
  bool, text: String, delay_ms: u32 }` helper that any widget
  composes. Add `Button::with_tooltip(text)` builder + per-frame
  `update_on_event(event, bounds)` + draw. Delete dead `_id`. Delete
  the global. Update callers (Tooltip is currently exported but no
  widget calls into it — verify with `grep -r tooltip_manager`).
  ~250 LOC change. Tests required: hover_start fires after delay,
  hide on mouse-leave, hide on click.

### radio_button.rs (377 LOC) — **(a) mechanical**, scope clear
- `static INSTANCE: OnceLock<Arc<Mutex<RadioGroupManager>>>` at
  line 353 buried inside an `impl` block. Used to coordinate which
  radio in a group is selected.
- The handoff doc (#5 RadioButton group has no keyboard nav) named
  this — "global RadioGroupManager singleton tracks state."
- **Agent brief**: replace the global with a host-owned
  `RadioGroup` struct that the host instantiates and passes to each
  RadioButton via `with_group(&group_handle)` (Rc<RefCell<...>> at
  the call site is fine — local sharing, not global). Tests: two
  RadioButtons sharing a group, clicking one deselects the other;
  two groups don't cross-talk. ~150 LOC change. Likely unblocks the
  later "RadioButton kbd nav" item too.

### accessibility.rs (326 LOC) — **(b) design-needed**
- `static ACCESSIBILITY_MANAGER: OnceLock<Mutex<AccessibilityManager>>`
  at line 313.
- This one is real-world ambiguous: OS accessibility APIs (AT-SPI on
  Linux, UIAutomation on Windows, NSAccessibility on macOS) are
  inherently process-singleton — there's one a11y bus per app. So a
  global *might* be justified once it actually integrates with an OS.
  But right now it doesn't integrate — it just stores a screen-reader
  text buffer in memory. If the goal is "real a11y eventually,"
  global-with-real-OS-bridge is fine; if the goal is "host-managed
  in-app a11y," refactor.
- **Decision needed from Alex**: do we plan to wire OS a11y eventually?
  If yes, leave it. If no, refactor like Tooltip.

### keyboard_nav.rs (262 LOC) — **(b) design-needed**
- `static KEYBOARD_NAV_MANAGER: OnceLock<Mutex<KeyboardNavigationManager>>`
  at line 247.
- Tab traversal across widgets is naturally a host concern — the
  host knows the widget tree and tab order. A global manager fights
  Rust's ownership model and forces all widgets to register/unregister
  themselves.
- **Decision needed**: refactor to host-owned (Rust way) or accept
  the global because tab traversal needs to know about every widget?
  My read is host-owned is the right answer; agent brief is small
  but the API change ripples.

### drag_drop.rs (170 LOC) — **(b) design-needed**
- `static DRAG_DROP_MANAGER: OnceLock<Mutex<DragDropManager>>`
  at line 147.
- Drag-and-drop has process-wide state (the current drag payload),
  but again that's a host concern. Rust idiom: host owns a single
  `DragState` and routes drag events through it.
- **Decision needed**: same shape as keyboard_nav — refactor to
  host-owned. But unlike keyboard_nav, OS DnD bridges may want a
  single registration point per process. Lower priority since DnD
  isn't actively used yet.

### scroll_view.rs (280 LOC) — **(b) decide: delete or implement**
- "In a real implementation, we'd need access to the widget manager
  here" at line 167.
- "In a real implementation, we'd translate the drawing context by
  -scroll_offset" at line 185.
- Hardcoded `is_focused -> false`, `can_focus -> false`,
  `set_focused -> no-op`.
- Used only by `widget_gallery.rs` (the example), per the handoff.
- **Decision needed**: delete (no consumers worth preserving) OR do
  a real implementation that wraps a single child widget and
  translates draw + events by the scroll offset. ~2 hours either way
  but the deletion option is the YAGNI-correct choice unless we
  imminently need a generic scroll wrapper.

### file_manager.rs (298 LOC) — **(b) decide: delete or implement**
- Three "In a real implementation, this would read the file system"
  comments at lines 61, 79, 111. The widget pretends to list files
  but everything is hardcoded mock data.
- `_path_input_id: WidgetId` is a dead field.
- We just shipped `tinyfiledialogs` for native open/save — the
  in-app file_manager has no live consumer.
- **Decision needed**: delete or implement. With native pickers
  shipped, file_manager is doubly dead.

### accordion.rs (585 LOC) — **(a) mechanical**, small fix
- Has real focus state (`self.state.focused`) but `can_focus` returns
  hardcoded `false` (line 575).
- That makes the focus state unreachable: nothing can focus an
  accordion, so its focus field never goes true, so its keyboard
  handlers (if any) never run.
- Same fix shape as TabControl item #4 just shipped.
- **Agent brief**: change `can_focus -> self.state.enabled &&
  self.state.visible`. Add Up/Down to switch panels and Enter/Space
  to toggle (per the original handoff item #8). One-paragraph fix.
  ~30 LOC + 4-5 tests.

### notification.rs (636 LOC) — **(c) acceptable, but cluttered**
- `is_focused -> false`, `set_focused -> no-op`, `can_focus -> false`
  all hardcoded.
- Notifications are passive (auto-dismiss, optional close button) so
  no-focus is correct.
- BUT also has a `NotificationManager` struct at line 120. Need to
  check if it's a singleton. (Confirmed: not a static singleton —
  it's a normal struct the host instantiates. False alarm on the
  Manager pattern alone.)
- **Verdict**: leave as-is. Hardcoded false is correct for passive
  widget. Manager is host-owned, that's fine.

### clipboard.rs (36 LOC) — **(c) acceptable, documented**
- `static CLIPBOARD: RwLock<String>` at line 12.
- Module docstring **explicitly** says: "This is *not* a system
  clipboard — it's a process-local `RwLock<String>`. Cross-process
  integration is a separate piece of work." The author knew and
  documented the limitation.
- **Verdict**: leave as-is. The smell is real but the design is
  intentional and the upgrade path (system clipboard via `arboard`)
  is documented. Future work, not Mojo-port debt.

### file_dialog.rs (793 LOC) — **(c) acceptable for now**
- We just stopped using this in erigui-app (tinyfiledialogs swap).
- Focus state is real (line 774). No singleton. Big widget but
  well-shaped per Rust idiom.
- **Verdict**: leave alone. It's the documented fallback per the
  tinyfiledialogs commit. If we eventually decide to delete the
  in-app FileDialog, that's a deletion task, not a Rustify task.

### Hardcoded `is_focused -> false` widgets where focus is irrelevant
- `label.rs` — display only ✓
- `progress_bar.rs` — display only ✓
- `notification.rs` — passive, see above ✓
- `status_bar.rs` — display + dropdown items ✓
- `container.rs` — pure layout, no events ✓
- `menu.rs` — interactive but uses `is_open` semantics, not focus ✓

These are not Mojo-port artifacts; they're correct hardcoded answers
for widgets whose semantics don't include focus. **Verdict (c).**

### lib.rs `WidgetManager` (line 81) — **(c) clean Rust**
- `pub struct WidgetManager { widgets: SlotMap<WidgetId, Box<dyn Widget>> }`
- Host-owned, no statics, no Mutex. This is the correct Rust shape.
- **Verdict**: leave alone. Not a Mojo artifact despite the "Manager"
  name.

### Less-explored areas — **(b) reach goal: skeptic agent reviews**
The audit didn't deep-read every widget. Skeptic should look hard at:
- `text_area.rs` (1453) — biggest non-graph widget; may have its own
  singleton/manager patterns.
- `date_time_picker.rs` (1009) — second-biggest.
- `dock_panel.rs` (879) — TODOs noted in handoff.
- `node_graph/mod.rs` (3823) — graph canvas; Stagehand-like
  complexity, hard to audit superficially.
- `menu.rs`, `context_menu.rs`, `dialog.rs` — modals often have
  global stack managers in Mojo ports.
- `color_picker.rs` (741) — large interactive; may have helpers.

---

## Triage summary

### (a) Mechanical — agent-delegatable

| File | Smell | Brief size |
|------|-------|------------|
| `tooltip.rs` | OnceLock singleton, `_id` dead, `TooltipWidget` ext | ~250 LOC |
| `radio_button.rs` | `RadioGroupManager` global singleton | ~150 LOC |
| `accordion.rs` | `can_focus -> false` despite real focus state | ~30 LOC |

These three are the "send agents" candidates. Each has a clear scope,
no architectural ambiguity, and a finite test list.

### (b) Design-needed — Alex decides first

| File | Question |
|------|----------|
| `accessibility.rs` | Will OS a11y bridges plug into this? Yes → keep singleton. No → refactor. |
| `keyboard_nav.rs` | Refactor to host-owned tab traversal? (My recommendation: yes.) |
| `drag_drop.rs` | Refactor to host-owned DnD state? (Lower priority — no live consumer.) |
| `scroll_view.rs` | Delete (YAGNI) or implement (~2h)? |
| `file_manager.rs` | Delete (now redundant with tinyfiledialogs) or implement? |

### (c) Acceptable — leave alone

`notification.rs`, `clipboard.rs` (documented), `file_dialog.rs` (doc'd
fallback), `label.rs`/`progress_bar.rs`/etc. (focus N/A by semantics),
`lib.rs` `WidgetManager` (clean Rust).

---

## Proposed next steps

1. **Skeptic agent audits this doc** — re-reads erigui-widgets/src/
   from scratch, flags missed smells and false positives, and
   especially deep-reads the large files (text_area, date_time_picker,
   dock_panel, node_graph/mod) that the author skimmed.
2. **Alex triages (a) and (b)** — picks which mechanical items to
   send to agents now and which design items to decide on.
3. **Spawn rewrite agents** in parallel for chosen (a) items, each
   with the scoped brief above.
4. **Verify** each agent's output before merging.

Nothing fires until Alex signs off.

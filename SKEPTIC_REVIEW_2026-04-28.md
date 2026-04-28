# Skeptic review of AUDIT_MOJO_PORT_2026-04-28.md

> Reviewer: Claude (Opus 4.7), 2026-04-28. Read-only audit. The auditor's
> writeup is broadly correct in shape but underplays scope on every (a)
> brief, missed two interactive widgets with broken focus, missed an
> ecosystem-wide callback-bound smell, and missed an unsound `*mut`
> raw pointer hiding behind `#[allow(dead_code)]` in `dock_panel.rs`.

---

## Verdict on the auditor's three (a) "send agent now" items

### tooltip.rs — agree, but the brief is wrong about scope

Auditor said "no widget calls into it — verify with `grep -r tooltip_manager`".
That grep does **not** come up empty. Live consumers exist:

- `erigui-examples/examples/tooltip_demo.rs` lines 35–91 (7×
  `register_tooltip` calls), line 179 (per-frame `update`), line 336
  (per-frame `draw`). Deleting the singleton breaks the demo.
- `WIDGET_GUIDE.md` line 1145 documents the singleton API as the
  public way to use tooltips.
- `rust-gui-fresh/` parallel tree contains a copy of all of this; we
  need to know whether that tree is still alive or we're patching a
  dead clone.

Also: the auditor's brief ignores existing infrastructure. `Button` already
has `with_tooltip(text)` at `button.rs:71` — it stores into
`WidgetState.tooltip: Option<String>` (defined in
`erigui-core/src/widget.rs:142`). The brief should be **"complete the
already-started Rustification by adding a read path,"** not "introduce a
TooltipState helper from scratch." Concretely:

1. `WidgetState.tooltip` already exists and Button writes to it.
2. No widget reads it back. Add a `TooltipState` *next to* the existing
   field (hover-start instant, visibility, cached target bounds), wire
   `update_on_event` into Button (and Toolbar buttons, which also have
   tooltip support per the existing `Toolbar` field structure).
3. Rewrite `tooltip_demo.rs` to drive widget-owned tooltips.
4. Update `WIDGET_GUIDE.md` line 1145.
5. Delete `TooltipManager`, `TooltipExt`, `TooltipWidget`,
   `tooltip_manager()`, the static at line 279, and the dead `_id`
   field at line 15.

Real LOC: ~250 LOC removed + ~150 LOC added in 4–5 files (tooltip.rs,
button.rs, toolbar.rs, examples/tooltip_demo.rs, WIDGET_GUIDE.md), not
"~250 LOC change in one file" as the auditor implied.

**Verdict: agree with delegation, but rewrite the brief — current text
underspecifies the demo + guide updates and the existing-button-field
reuse.**

### radio_button.rs — agree, but scope is bigger than claimed

The auditor undersold the smell. Three points it missed:

1. **Every `RadioButton::new()` registers in the global at the
   constructor** (`radio_button.rs:33`) and each `with_checked(true)`
   modifies it (line 55). Constructors that touch global mutable state
   are a Rust anti-pattern — they break `Default` ergonomics, prevent
   test isolation, and cross-pollute across processes.
2. **`Event::Update` polls the global every tick** (lines 277–287) to
   resync `self.checked`. That's a Mutex acquisition every frame for
   every radio button. Once the singleton is gone, the polling loop
   needs to be replaced with explicit "selected_id changed" notification
   from the group, otherwise check-state desync becomes possible.
3. **The example `erigui-examples/examples/radio_button_demo.rs` is
   the only consumer** (10× `RadioButton::new(.., group_id)` at lines
   39–71). The brief must include a rewrite of that demo to use the
   new `RadioGroup` host-owned struct.

There are **no tests** for radio_button (`grep RadioButton
erigui-widgets/tests/` returns nothing) — so test-compat is not a
constraint, but the agent must add tests. The auditor said "Tests:
two RadioButtons sharing a group, clicking one deselects the other; two
groups don't cross-talk." That's a reasonable starter set; I'd add:
"RadioButton::new() in test does not pollute state visible to other
tests" — currently this fails because of the singleton.

Real LOC: ~150 in radio_button.rs + 50 in the demo + 60 in new tests.

**Verdict: agree with delegation, but call out the per-frame
Mutex-poll and demo rewrite explicitly in the brief.**

### accordion.rs — agree, scope is roughly right

The audit's accordion fix is the smallest of the three and the brief
is the most accurate. The change is genuinely "flip `can_focus -> false`
to `can_focus -> self.state.enabled && self.state.visible` and add
Up/Down/Enter/Space handling."

One caveat the auditor glossed: the accordion's `handle_event` already
forwards events to expanded panel content (lines 528–535). Adding Up/Down
key handling at the accordion level needs to gate "consume vs forward"
correctly — if focus is on a text field inside an expanded panel,
Up/Down should belong to the text field, not the accordion. The fix is
to gate accordion keyboard handling on `self.state.focused` (same
pattern TabControl uses at `tab_control.rs:410`).

Real LOC: 30–60 with the focus-gate caveat handled.

**Verdict: agree, brief is fine, confirm the focus-gate before
forwarding-to-content.**

---

## Auditor missed these

### 1. breadcrumb.rs:490 — `can_focus -> false` (interactive, 501 LOC)

Same shape as accordion. Breadcrumb is interactive (clickable segments,
hover state at lines 396–451), and it has real focus state in its
`WidgetState`. `can_focus -> false` makes the focus state unreachable
just like accordion. Auditor's grep apparently caught the singletons
but not the focus contradiction in breadcrumb. **Should be (a)
mechanical — same agent brief as accordion.**

### 2. menu.rs:815-823 — contradictory focus, but **interactive**

The auditor classified menu as (c) "uses `is_open` semantics, not
focus." Wrong. menu.rs:

- Returns `is_focused -> false` (line 815) — hardcoded.
- Returns `can_focus -> true` (line 821) — wants focus.
- Has `set_focused -> {}` (line 819) — discards focus events.
- **Has full keyboard handling** (`grep KeyPress menu.rs` →
  lines 664–727: arrow keys, Enter, Escape, Alt-mnemonics) gated on
  `self.dropdown_visible`, not on focus.

This is broken for keyboard navigation: the host's tab-traversal will
ask "should I focus this MenuBar?" → "yes (`can_focus -> true`)" → call
`set_focused(true)` → menu silently drops it → next call to
`is_focused()` returns false → host thinks focus didn't land. Effect:
the MenuBar can't participate in Tab traversal even though it wants to.

The auditor's "uses `is_open` semantics" reasoning would justify
making it look like context_menu.rs (where `is_focused` aliases to
`is_open`, line 673). But menu has separate `dropdown_visible`,
`active_menu`, and the focus state in `WidgetState` — three orthogonal
concepts. Pick one and stick with it. The right Rust answer is what
TabControl does: `is_focused` reads the real field, `can_focus` returns
`enabled && visible`, `set_focused` accepts.

**Should be (a) mechanical — but it's a 30-LOC fix with a 5-test
ride-along.**

### 3. dock_panel.rs:153 — `*mut DockNode` raw pointer in `DropTarget`

```
struct DropTarget {
    rect: Rect,
    position: DockPosition,
    target_node: *mut DockNode,
}
```

This is the most Mojo-ish thing in the entire codebase. Mojo had
pointer-style references baked in; Rust does not. The struct is gated
behind `#[allow(dead_code)]` and the field `_drop_target` (line 132)
is dead (`grep target_node` confirms it's never assigned to). But the
type is still in the codebase. If any future maintainer un-deads
`DropTarget`, the `*mut` becomes load-bearing — and at that point
they'll also need to handle reborrow/lifetime invariants that don't
exist in Mojo's mental model. Better to delete `DropTarget` entirely
along with `_drop_target`, `_dragging_panel`, `_resize_edge` and
`get_drop_target` (line 568, "TODO: Calculate drop targets").

This is a **partial-rewrite target.** Drag-drop docking either gets
implemented properly (with a path-based DropTarget mirroring the
already-fixed splitter pattern at lines 583–609) or it gets deleted.
Same shape as the (b) `scroll_view`/`file_manager` decisions.

Auditor missed this entirely.

### 4. dock_panel.rs:820 — `can_focus -> false`

Same pattern as accordion. Dock panel is interactive (splitters drag,
panels click, tabs activate). Has real focus state. `can_focus -> false`
makes it unreachable. Same one-line fix as accordion.

### 5. file_manager.rs is **buggy**, not just stub

Auditor says "delete or implement." It's worse than that:

- Line 154: `if !self.state.visible { }` — empty if-branch in `draw()`.
  No-op. Should be `if !self.state.visible { return; }`. Currently
  invisible FileManager would still paint child widgets if they had
  paint logic.
- Lines 281–298: `create_file_manager` builds **two** FileManager
  instances — one inside `Box::new(FileManager::new(...))` that gets
  registered with the WidgetManager (line 282), and another `let
  file_manager = FileManager::new(...)` outside (line 281) whose
  `.id()` is returned. The returned id is from an unregistered orphan.
  Calling `widget_manager.get(returned_id)` crashes or returns None.

The "implement" path requires fixing both bugs. The "delete" path is
strongly preferred — tinyfiledialogs ships, and the in-app FileManager
has zero live consumers and is structurally broken. Auditor's verdict
"delete or implement" is technically right but the strength of the
delete recommendation should be much higher.

### 6. Callback-bound inconsistency: `Box<dyn Fn>` vs `Box<dyn FnMut>`

The auditor's grep didn't catch this. There are 30+ callback fields
across widgets, and they're inconsistent:

- `Box<dyn Fn>` (caller can't mutate captured state):
  `tab_control.rs:57-58` (`on_tab_changed`, `on_tab_closed`),
  `dialog.rs:41` (`on_button_clicked`),
  `slider.rs:23` (`on_value_changed`),
  `spin_box.rs:21` (`on_value_changed`),
  `combo_box.rs:9` (`SelectionChangedCallback`),
  `checkbox.rs:13` (`on_toggle`)
- `Box<dyn FnMut>` (Rust idiom for callbacks):
  everything else (~24 widgets).

This is a Mojo-port artifact: Mojo had `fn` (no captured environment)
not closures. Some Rust ports got blanket-converted to `Fn`, others
got the idiom-correct `FnMut`. The footgun: a host wanting to write
`with_on_change(|v| { self.value = v; })` works for half the widgets
and silently fails the borrow check for the other half, with no
guidance on which is which.

Mass-mechanical fix. Audit each callback for *whether the receiver
ever needs `&mut`*. In practice every interactive callback wants
`FnMut`. **Should be added to (a) as a single sweeping mechanical
change** — touches 6 widgets, ~30 LOC, near-zero risk of regression
(`FnMut` is a strict supertype of `Fn` for callback bounds).

### 7. menu.rs `MenuItem.on_click: Option<fn()>` — pure fn pointer

`menu.rs:22`: `pub on_click: Option<fn()>`. Not even `Box<dyn Fn>` —
a bare function pointer. This is the Mojo limitation in its purest
form: the menu callback can't capture *anything*. Hosts can't have
"File → Save" call into their app state because there's no closure
environment. This makes the menu widget unusable for real apps.

Should be `Option<Box<dyn FnMut()>>` like every other on_click
callback. ~5 LOC change but it changes the public API. **Goes in (b)
design-needed because callers of `MenuItem::with_on_click(handler:
fn())` have to update.** Realistically this is also blocking real
menu usage so it's high-leverage.

### 8. text_area.rs:1257-1357 — duplicate Ctrl+letter handling paths

text_area handles Ctrl+C/X/V/Z/Y/A in two parallel match arms:
- `Key::Character(ch) if ctrl => match ch.to_ascii_lowercase() { 'c' => ... }`
  at lines 1257-1310
- `Key::C if ctrl => ...`, `Key::X if ctrl => ...`, etc. at lines 1312-1357

Both branches reachable depending on whether the host emits
`Key::Character('c')` or `Key::C`. It's defensive — it's not Mojo-port
debt — but it's 100 LOC of duplicated logic that should be deduplicated
via a helper. Low-priority polish, not a rewrite candidate.

### 9. node_graph.rs:3315 — `can_focus -> true`, ungated

Minor inconsistency vs the canonical pattern. Should be
`self.state.enabled && self.state.visible` like TabControl. 1-line
fix, included in any focus-rectification sweep.

---

## Auditor overcalled these

### scroll_view — auditor's "delete YAGNI" call is correct but weakly argued

The auditor says "delete (no consumers worth preserving) OR do a real
implementation." I checked — `scroll_view` has stub comments at lines
167 and 185 ("In a real implementation, ..."), no live consumers
beyond the gallery example, and the entire concept of a generic scroll
wrapper is poorly motivated — text_area, list_view, tree_view all
implement their own scroll. **Delete is the strong answer, not
"either/or".** Goes to (a) mechanical (delete file + remove from
lib.rs + remove from gallery example).

### accessibility.rs and keyboard_nav.rs — auditor leaves both in (b)

I'd push both to (a). The auditor cites OS-bridge ambiguity for
accessibility — but the existing code doesn't bridge to anything; it
just stores screen-reader text in memory. If/when an OS bridge gets
written, it can introduce its own singleton (or use the OS-provided
one — a11y APIs hand you the bus, you don't create it). The current
in-memory stub is useless and confusing. Same for keyboard_nav: a
host-owned focus-tree is the only Rust answer; the global is a port
artifact; nothing real consumes it.

That said, these are bigger refactors than tooltip/radio_button. Not
"send agent now" priority — but **don't classify as design-needed**.
The design *is* known; it just hasn't been done. Move to (a) with
scoped briefs, schedule for after the easy wins.

### file_dialog (c) accept — fine, but note tinyfiledialogs lapse

Auditor says (c). Fine — but `erigui-app` no longer uses it. The
correct status is "deletion candidate, not Mojo-shape." Doesn't
change the audit conclusion but should be flagged.

---

## Recommended (a) list (if different from auditor's)

Ordered by leverage/risk ratio (high leverage, low risk first):

1. **accordion.rs** `can_focus` fix + Up/Down/Enter/Space (auditor's
   third (a) — keep). ~30 LOC. Risk: zero. Pattern: same as TabControl.
2. **breadcrumb.rs** `can_focus` fix (new, missed by auditor). ~5 LOC
   + maybe Left/Right arrow nav (~30 LOC). Same agent can do this in
   the same PR as accordion. **Highest leverage/risk ratio.**
3. **menu.rs** focus rectification (`is_focused` → real field,
   `set_focused` → real assignment, gate keyboard on focus). ~30 LOC.
   Same shape as #1, #2. Bundle in same agent run.
4. **dock_panel.rs** `can_focus` fix only (new, missed by auditor).
   ~5 LOC. Splitter / drag-drop scope is (b) territory. Bundle in
   same focus-rectification agent.
5. **node_graph/mod.rs** `can_focus` gate on enabled/visible. 1 line.
   Bundle in same agent.
6. **Callback bounds sweep**: `Fn` → `FnMut` across tab_control,
   dialog, slider, spin_box, combo_box, checkbox. ~30 LOC, near-zero
   risk. Bundle in same agent.
7. **tooltip.rs** Rustify with the corrected brief (auditor's first
   (a) — keep, but with the wider scope: button reuse + demo rewrite +
   guide update). ~250 LOC removed + 150 added across 5 files.
8. **radio_button.rs** Rustify with the corrected brief (auditor's
   second (a) — keep, but flag the per-frame Mutex-poll and demo
   rewrite). ~150 LOC + 50 demo + 60 tests.
9. **file_manager.rs delete** (decision is "delete," but the
   deletion is a clean (a). ~300 LOC removed + lib.rs export drop +
   any gallery references). Trivial and high-value.
10. **scroll_view.rs delete** (same shape — auditor's (b) is too
    weak). ~280 LOC removed. Trivial.

Items 1–6 are a single agent's day. Items 7+8 are bigger but
self-contained per agent. 9+10 are deletions, parallelizable with
anything.

What I'd **not** recommend sending now: accessibility.rs / keyboard_nav.rs
/ drag_drop.rs / dock_panel drag-drop. They're real Rustifications but
the design questions need Alex first. dock_panel splitters are already
fixed (path-based). The drag-drop dead code can wait until someone
actually wants drag-and-drop docking.

---

## Outstanding questions for Alex

1. **`rust-gui-fresh/` parallel tree** — is it dead or alive? It has
   its own tooltip.rs, examples/, WIDGET_GUIDE.md. If alive, every
   rewrite has to be applied twice. If dead, delete the tree before
   doing anything. **This affects every brief.**

2. **WIDGET_GUIDE.md line 1145** documents the global tooltip API.
   Does the guide get rewritten as part of the tooltip Rustification,
   or is it a separate doc-task? (My recommendation: bundle it.)

3. **MenuItem.on_click** — current type is `Option<fn()>` (bare fn
   pointer, no captured environment). Changing to `Option<Box<dyn
   FnMut()>>` is necessary for real menu use, but breaks any caller
   passing a `fn` literal. Is there a real menu consumer in
   `erigui-app` or examples that would be affected? (Quick grep
   suggests no, but should be confirmed.)

4. **accessibility.rs / keyboard_nav.rs / drag_drop.rs** — Alex's
   intent. My read: the architecturally-correct answer is "host
   owns these." Does Alex agree, or are there long-term plans for
   OS-a11y bridges that genuinely want a process-singleton?

5. **`Fn` vs `FnMut` callback sweep** — straightforward, but the
   change is API-breaking for any user passing an `Fn`-only closure
   (rare in practice, but possible). Does Alex want a single sweep
   or item-by-item?

6. **scroll_view + file_manager deletion** — the audit's "delete or
   implement" defers the decision. I think both are clear-deletes
   given current state. Confirm before the agent fires.

7. **dock_panel drag-drop** — the dead `*mut`/`#[allow(dead_code)]`
   path can be removed cleanly, but the question is whether
   drag-to-dock is on the eventual feature list. If yes, the dead
   code is a placeholder for something that should be implemented;
   if no, delete now.

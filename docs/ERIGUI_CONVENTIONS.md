# EriGui conventions and gotchas

> The patterns, naming rules, and dispatch tricks that take 3 grep
> rounds to figure out each session. Read once, save hours later.
>
> Format mirrors `flame-core/docs/FLAME_CONVENTIONS.md`.

---

## Widget shape

Every widget in `erigui-widgets` follows the same shape. Match it when
adding new ones — host code assumes it.

```rust
pub struct Foo {
    state: WidgetState,                      // id, bounds, visible, enabled, focused
    on_change: Option<Box<dyn FnMut(...)>>,  // FnMut, NOT Fn — must be Box<dyn ...>
    tooltip: Option<TooltipState>,           // optional; if present, drive in handle_event
    // widget-specific state ...
    // theme-driven metrics computed in layout(theme):
    button_width: i32,
    item_height: i32,
}

impl Widget for Foo {
    fn id(&self) -> WidgetId { self.state.id }
    fn measure(&self, _: &LayoutConstraints, theme: &Theme) -> Size { ... }
    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        // RECOMPUTE THEME-SCALED METRICS HERE
        let font = theme.typography.font_size_base;
        let pad  = theme.spacing.padding.top.max(8);
        self.button_width = font + pad;
        self.item_height  = font + pad * 2;
    }
    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) { ... }
    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled { return EventResult::Ignored; }
        if let Some(t) = &mut self.tooltip { t.update_on_event(event, self.state.bounds); }
        // ... dispatch on Event variant ...
    }
    // boilerplate: bounds, set_bounds, is_visible, set_visible, is_enabled,
    // set_enabled, is_focused, set_focused, can_focus, as_any, as_any_mut.
}
```

Three rules that aren't obvious from the trait signature:

1. **`measure` does NOT mutate state.** Some widgets are tempted to
   compute scaled metrics in `measure` because it sees the theme.
   Don't. Hosts call `measure` to ask "how big do you want to be?" and
   may discard the result. Only `layout` is guaranteed to be called
   before `draw`.

2. **`layout` is called every frame in kitchen_sink.** It must not
   allocate or do expensive work. Recompute scalar metrics from the
   theme; that's it.

3. **`handle_event` returns `Consumed` only when the event was
   *handled*.** Returning `Consumed` for a click outside your bounds
   blocks sibling widgets behind you. Hit-test first.

---

## Builder pattern

Widget constructors are spelled `Foo::new(WidgetId)` or
`Foo::new(WidgetId, primary_arg)`. Configuration is done via builder
methods that take `mut self` and return `Self`:

```rust
let btn = Button::new(Default::default(), "OK")
    .with_tooltip("Click to confirm")
    .with_on_click(|| set_status("clicked"));
```

After construction, runtime mutation goes through `set_*`:
```rust
btn.set_enabled(false);
btn.set_text("OK!".to_string());
```

`with_*` returns `Self` so you can chain in expressions; `set_*` returns
`()` and operates in place. Don't mix them.

---

## Callbacks are `Box<dyn FnMut(...)>`

All widget callbacks are `Box<dyn FnMut(...) + 'static>`, not
`Box<dyn Fn(...)>` and not bare `fn(...)`. This was a deliberate
convergence (commit 9624253) so hosts can capture `&mut state` in click
handlers — by far the dominant real-world use:

```rust
let counter = Rc::new(RefCell::new(0));
let c = counter.clone();
btn.with_on_click(move || *c.borrow_mut() += 1);
```

Callback types do NOT implement `Clone` (because `Box<dyn FnMut>` can't).
Don't try to clone widget state across hosts; rebuild fresh.

---

## Event flow

The host owns the event loop. Per winit `WindowEvent`, the host calls
`translator.translate(&event)` and passes the result (if any) into
`app.handle_event(&gui_event)`. The app then dispatches to widgets
either by walking a tree (`WidgetManager`) or by calling each
top-level widget directly (kitchen_sink does the latter).

```
winit::WindowEvent
    ↓ EventTranslator::translate
erigui::Event
    ↓ app.handle_event
ContainerWidget.handle_event  → dispatches to children, gets EventResult
    ↓
LeafWidget.handle_event       → reads bounds, decides Consumed | Ignored
```

Three things to remember:

1. **`EventTranslator` is stateful and MUST be persistent.** It caches
   `last_cursor` from `CursorMoved` so subsequent `MouseInput` events
   can be tagged with a position (winit doesn't include position on
   button events). Construct one at startup, hold it for the window's
   lifetime, mirror Resize via `set_window_size`. The free helper
   `convert_window_event` is `#[deprecated]` precisely because it
   instantiates a fresh translator per call → every click reports as
   `(0, 0)`.

2. **`Space` comes through as `KeyPress(Key::Space)`, not
   `TextInput(" ")`.** This is so activation widgets (Button /
   Checkbox / Accordion / Dialog buttons) can react to it. TextInput
   and TextArea explicitly handle `Key::Space` to insert a literal
   space. If you write a new widget that consumes printable text,
   *also* add a `Key::Space` arm.

3. **Autorepeat for printable text is suppressed at the translator.**
   `event.repeat == true && text.is_printable()` → the event is dropped.
   Backspace / Delete / arrows still autorepeat through their KeyPress
   path. If a future bug shows "holding 'a' doesn't repeat", this is
   why; the trade-off was made because Wayland on this user's NVIDIA
   box was firing autorepeat at unusable rates.

---

## Layout pass / draw pass / event pass — when does each run?

The kitchen_sink event loop is the canonical example:

```rust
// initial layout once before the event loop
app.layout(viewport, &theme);

event_loop.run(move |event, elwt| {
    match event {
        WinitEvent::WindowEvent { event, .. } => {
            match &event {
                WindowEvent::Resized(sz) => {
                    renderer.resize(sz.width, sz.height);
                    translator.set_window_size(...);
                    app.layout(new_size, &theme);
                    renderer.window().request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    app.layout(renderer.viewport_size(), &theme);
                    renderer.begin_frame(theme.colors.background);
                    app.draw(&mut renderer, &theme);
                    // SECOND PASS for overlay widgets:
                    app.draw_dropdowns(&mut renderer, &theme);
                    renderer.end_frame();
                }
                _ => {}
            }
            if let Some(gui_event) = translator.translate(&event) {
                app.handle_event(&gui_event, &theme);
                app.layout(renderer.viewport_size(), &theme);
                renderer.window().request_redraw();
            }
        }
        WinitEvent::AboutToWait => {
            renderer.window().request_redraw();
        }
        _ => {}
    }
});
```

Two non-obvious requirements on Wayland:

1. **Always `request_redraw()` after a state-changing event.** Without
   it, Wayland defers the next paint until the next incidental event,
   so checkbox toggles / typed text don't appear until you move the
   mouse. Fixed in commit `f15d975`.

2. **Layout must run before the first draw.** Widgets default to
   `bounds = (0,0,0,0)` at construction. `kitchen_sink` calls
   `app.layout(viewport, &theme)` once before `event_loop.run` and
   again on every redraw + after every translated event. Cheaper than
   it sounds because layout just does i32 arithmetic.

---

## Z-order: overlays via second-pass draw

`Widget::draw` is called per widget in tree order. For widgets that
overlay siblings (menu dropdowns, combo popups, tooltips), `draw` only
renders the base; the host calls a separate method during a second
pass after the rest of the tree has drawn:

| Widget | Method | When |
|---|---|---|
| `MenuBar` | `draw_dropdown_only(ctx, theme)` | After all other widgets drew |
| `ComboBox` | (drop-down embedded in `draw`, but drawn last in tab) | Constrain via tab z-order |
| `Tooltip` | drawn by the widget itself, last | Inside the same `draw` call |

`menu_demo.rs::App::draw_dropdowns` is the canonical walker — recurses
the widget tree, calls `draw_dropdown_only` on every `MenuBar` after
the main tree pass.

If a new generic host writes "for each widget in tree, call .draw()",
menu dropdowns and combo popups will be hidden. Add a second pass.

---

## Theme conventions

### Color slots

22 named slots in `ThemeColors`. Use the semantic name, not the
literal color:

| Slot | Meaning |
|---|---|
| `background` | Window root fill |
| `surface` | Card / panel fill |
| `surface_variant` | Secondary panel fill (popups, tab strip) |
| `text` | Primary text |
| `text_secondary` | Subtitle / hint text |
| `text_disabled` | Disabled-widget text |
| `primary` / `_hover` / `_active` / `_disabled` | Accent (buttons, sliders, scrollbar) |
| `secondary` / `_hover` / `_active` | Neutral accent (headers, separators when emphasized) |
| `success` / `warning` / `error` / `info` | Semantic (notifications, validation) |
| `border` / `_hover` / `_focus` | Widget chrome borders. Switch to `_focus` when `state.focused`. |
| `selection` | Selected text / list item highlight (usually `primary` with alpha) |
| `shadow` | Drop-shadow color (always alpha-blended) |
| `overlay` | Modal-dim layer behind a Dialog |

When porting palettes from another framework, follow the mapping at
the top of `Theme::serenity()` in `theme.rs`. ImGui-style palettes
tend to have separate `slider_grab` / `scrollbar_grab` / `button` —
collapse all three under `primary`. Slots without an upstream
equivalent (`success`, `info`, `border_focus`) get a palette-appropriate
default.

### HiDPI scaling

`Theme::with_scale(scale_factor)` multiplies typography + spacing +
border-width. Call **once** at startup with the monitor's scale
factor; do NOT call it per-frame — the multiplication compounds.
Widgets read post-scale values from the theme via their `layout(theme)`.

The scale factor comes from `winit::Window::scale_factor()` or, if you
want screen-height heuristics like serenity, you can detect:
```
height ≤ 1200  → 1.0
height ≤ 1600  → 1.15
otherwise      → 1.4
```
EriGui doesn't ship the heuristic; the host picks the value.

---

## File / module layout

### Where new widgets go

A new widget is a new file `erigui-widgets/src/foo.rs` plus an entry
in `erigui-widgets/src/lib.rs`:

```rust
mod foo;
pub use foo::Foo;
```

Tests go in `erigui-widgets/tests/widget_tests.rs` (the most-active
test file) or a sibling integration test if it warrants its own
fixture (`tree_dock_tests.rs`, `modal_widgets_tests.rs`).

### Where new examples go

`erigui-examples/examples/foo_demo.rs`. The `[[example]]` block in
`erigui-examples/Cargo.toml` is auto-discovered by cargo for
`examples/*.rs` — no extra entry needed unless you want a custom
description.

### Three places to learn the canonical patterns

- **`kitchen_sink.rs`** — tab-based demo, 6 pages, layout +
  event-pump pattern, second-pass draw for notifications.
- **`menu_demo.rs`** — second-pass `draw_dropdown_only` walker.
- **`widget_tests.rs`** — `default_theme()`, `test_id()`, `move_event`,
  `left_press_event` helpers; copy them into a new test file rather
  than reaching for `pub use` from the library.

---

## Tests

### Helpers

`erigui-widgets/tests/widget_tests.rs` defines the standard test
helpers:

```rust
fn default_theme() -> Theme { Theme::dark() }
fn test_id() -> WidgetId { WidgetId::default() }
fn move_event(p: Point) -> Event { ... }       // MouseMove
fn left_press_event(p: Point) -> Event { ... } // MouseButton(Left, pressed)
```

Sibling test files (`tree_dock_tests.rs`, `picker_widgets_tests.rs`,
`simple_widgets_tests.rs`, `modal_widgets_tests.rs`) **redeclare the
same helpers** rather than `use`ing them, because cargo treats each
integration test as a separate crate and the helpers aren't `pub`.
Keep them in sync if you change the canonical ones.

### When you can't unit-test

Some bugs only show in the actual GL window — DPI scaling visuals,
real Wayland event timing, font rendering. The pattern documented in
`HANDOFF_2026-04-28_KITCHEN_SINK_DEBUG.md` and
`HANDOFF_2026-04-28_NIGHT_DEBUG.md`:

1. Add an `eprintln!` gated by an env var (`ERIGUI_DEBUG_FOO=1`).
2. Have the user run the demo with the var set, click around, paste
   the stderr.
3. Reason from the trace.

The current diagnostic vars in HEAD: `ERIGUI_DEBUG_DOCK`,
`ERIGUI_DEBUG_ACC`. Re-instrument as needed.

### Pre-existing failing tests

5 tests in `modal_widgets_tests::manager_*` (NotificationManager
hover/click) have failed since before the 2026-04-28 fix passes. They
predate the work and need separate triage. The other ~290 tests pass.

---

## Things that look weird but are intentional

### `set_tabs` is idempotent

`TabControl::set_tabs(items)` early-returns when `items` matches the
existing tab list by length + title. The dock panel calls it on every
layout (which fires after every event), and rebuilding cleared
ancillary per-tab state. Fix landed in commit `5a81799`.

### `WindowEvent::RedrawRequested` arm in kitchen_sink

The outer `WinitEvent::WindowEvent { event, .. }` arm uses `match &event`
(borrow), not `match event` (move), so the redraw arm INSIDE that match
is reachable AND the same `event` can be passed to `translator.translate`
afterward. Without the borrow, the second use would not compile.

### The Mojo→Rust port heritage

EriGui was originally a Mojo project ported to Rust. Several widgets
still carry port artifacts: hardcoded pixel constants instead of
theme-driven metrics, singleton pattern for things that should be
per-widget state, dead extension methods. When something looks weird
in a widget, suspect port artifact rather than design.

The 2026-04-28 sweep theme-scaled the user-flagged constants
(`item_height`, `button_size`, `close_size`, `min_width`, etc.) but
many remain. Audit grep:

```
$ grep -rE 'self\.\w+(_height|_width|_size): \w+ = \d+' erigui-widgets/src/
$ grep -rE '\b(let|const)\s+\w+(_size|_height|_width|_padding)\s*=\s*\d+' erigui-widgets/src/
```

Each match is a candidate. Either theme-scale in `layout()` or
annotate why it's deliberately fixed (border widths, splitter
thickness, etc.).

### `convert_window_event` is `#[deprecated]`, not removed

Leaving the tombstone in `erigui-rendering` so out-of-tree code that
depends on EriGui still compiles with a loud warning. All in-tree
demos and `kitchen_sink` migrated in commits `dcfceb3` and `169bc11`.

---

## When you're stuck

1. Start with [`ERIGUI_INDEX.md`](./ERIGUI_INDEX.md) — flat symbol list.
   "Is there already a function/widget for X?"
2. Then [`ERIGUI_MODULES.md`](./ERIGUI_MODULES.md) — paragraph per
   crate. "Which crate does this live in?"
3. The `HANDOFF_*.md` files at the repo root — the most recent
   debug-session diaries. They have the user-side bug repros and
   what was tried.
4. `KNOWN_ISSUES.md` at the repo root — current upstream blockers.
5. Run `kitchen_sink` to see the change live. The demo is the
   canonical visual-regression harness.

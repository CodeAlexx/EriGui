# EriGui module map

> One paragraph per module / crate. Read once at session start to know
> where things live and which crate to touch. ⭐ marks crates a real
> host actually depends on, ⚠️ marks unfinished or upstream-blocked.
>
> Format mirrors `flame-core/docs/FLAME_MODULES.md`.

EriGui is a pure-Rust GUI framework targeting Wayland + OpenGL. It is
not immediate-mode (egui-style) — widgets are persistent objects that
implement a `Widget` trait, and the host drives `layout()` / `draw()` /
`handle_event()` per frame. Hit-testing is host-driven too: every
widget's `handle_event` checks its own `bounds` and decides whether to
consume.

The framework is organized into seven crates plus an example crate:

```
erigui-core         types, events, theme, Widget trait, layout primitives
erigui-rendering    OpenGL backend + winit integration + EventTranslator
erigui-widgets      30+ widget implementations (the bulk of the code)
erigui-runtime      reactive runtime (less mature; not used by demos)
erigui-workflow     workflow / signal-graph layer
erigui-app          app shell scaffolding
erigui-nodes ⚠️    node-graph editor (blocked on inference-flame drift)
erigui-examples     all the demos including kitchen_sink
```

`erigui-core` and `erigui-widgets` are the load-bearing crates. The
others build on top. `erigui-nodes` doesn't compile right now (see
`KNOWN_ISSUES.md`).

---

## ⭐ `erigui-core` — types, traits, events, theme

This is the foundation. Everything else depends on it. Five source
files, all small.

**`src/types.rs`** — `Point`, `Size`, `Rect`, `Color`, `Margins`. All
math is integer pixels (i32). `Color` is RGBA u8 with `to_gl_color()`
returning `[f32; 4]` for the GL backend. `Color::from_hex(0xRRGGBB)` is
the convenient constructor for theme palettes.

**`src/widget.rs`** — the `Widget` trait, the `EventResult` enum, the
`DrawContext` trait, `WidgetState`, `WidgetId`, `WidgetManager`. Every
widget in `erigui-widgets` implements `Widget`. `DrawContext` is the
narrow surface widgets use to draw — `set_color`, `fill_rect`,
`draw_rect`, `draw_line`, `draw_text`, `measure_text`, `push_clip_rect`,
`pop_clip_rect`. The OpenGL `Renderer` in `erigui-rendering` implements
it.

**`src/event.rs`** — `Event` enum + per-variant payload structs +
`Key` enum (full physical-key list including Numpad and `Character(char)`
fallback) + `Modifiers` bitflags. The translator in `erigui-rendering`
produces `Event` values from winit `WindowEvent`s and the host pumps
them into widgets.

**`src/theme.rs`** — `Theme` (colors / typography / spacing / borders),
`ThemeColors` (22 named slots), `ThemeManager`, and 10 built-in factory
methods: `dark`, `light`, `alex_jammin`, `serenity`, `moonlight`,
`monochrome`, `nord`, `cinder`, `blender`, `cyberpunk`. `Theme::all_named()`
returns a `Vec<Theme>` for theme-picker UIs; `Theme::by_name(&str)` is
the case-insensitive lookup. `Theme::with_scale(f32)` scales typography
+ spacing + border-width by a HiDPI factor — call once at startup, not
per-frame. The 7 serenity-port palettes were pulled from
`/home/alex/serenity/serenity/ui/theme.py` (DearPyGui) and translated
via the slot-mapping documented at the top of the `Theme` impl block.

**`src/layout.rs`** — `LayoutConstraints`, `LayoutMode`, `LayoutConfig`.
Used by `Container` and host layout passes.

---

## ⭐ `erigui-rendering` — OpenGL backend + window + event translation

Two responsibilities:

1. **`renderer.rs` + `gl.rs` + `gl_context.rs`** — the OpenGL context
   creation, the immediate-mode draw API, font glyph upload via FreeType
   (`font_freetype.rs`). Implements `DrawContext`. `Renderer::new` opens
   a winit window + GL context; `begin_frame(clear_color)` /
   `end_frame()` bracket every paint. `viewport_size()` returns the
   physical pixel size; `window()` exposes the `winit::Window` so the
   host can call `request_redraw()`.

2. **`window.rs`** — the `EventTranslator`. Translates winit
   `WindowEvent`s into erigui `Event`s. **Stateful** (caches
   `last_cursor` from `CursorMoved` so subsequent `MouseInput` events
   carry the right position). The free helper `convert_window_event`
   instantiates a fresh translator per call → every click reports as
   `(0, 0)`. It is `#[deprecated]`. Hosts must hold a persistent
   `EventTranslator` for the window's lifetime, mirror Resize via
   `set_window_size`, and call `translate(&event)`. See the kitchen_sink
   event loop for the canonical pattern.

   Two notable special-cases inside `translate()`:
   - **Space** is always emitted as `KeyPress(Key::Space)`, never
     `TextInput(" ")`. Activation widgets (Button / Checkbox / Accordion)
     need it as KeyPress. TextInput / TextArea handle `Key::Space`
     explicitly to insert a literal space.
   - **Autorepeat for printable text** is dropped. winit/Wayland fires
     `KeyboardInput { repeat: true, ... }` while a key is held, and on
     this user's NVIDIA + Wayland session the rate was high enough that
     a brief 'h' tap produced ~35 chars. Now: only the initial press
     produces a `TextInput` event. Backspace / arrows / Delete still
     autorepeat through their KeyPress paths so "hold backspace to
     delete fast" works.

---

## ⭐ `erigui-widgets` — the widget collection

30+ widgets, one per file. Most follow the same shape:

```
pub struct Foo {
    state: WidgetState,    // id, bounds, visible, enabled, focused
    // widget-specific fields ...
    on_change: Option<Box<dyn FnMut(...)>>,  // FnMut, NOT Fn
    tooltip: Option<TooltipState>,
}

impl Foo {
    pub fn new(id: WidgetId) -> Self { ... }
    pub fn with_*(...) -> Self { ... }   // builder
    pub fn set_*(&mut self, ...) { ... } // runtime mutation
}

impl Widget for Foo {
    fn measure(...) -> Size { ... }
    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;
        // recompute theme-scaled metrics here
    }
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) { ... }
    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult { ... }
    // boilerplate for is_visible/enabled/focused, set_*, can_focus
}
```

**Theme-driven layout.** Every widget that has internal metrics
(`tab_height`, `item_height`, `button_width`, `close_size`, etc.)
recomputes them in its `layout(rect, theme)` from
`theme.typography.font_size_base` + `theme.spacing.padding`. This is
how HiDPI scaling reaches the widget chrome. Pre-2026-04 the widgets
were a Mojo→Rust port and many had hardcoded constants ("Mojo-port
hardcoded constants"); a sweep landed in commits between
`c2cfde3` and `86959fc` to theme-scale the user-flagged ones.

**Z-order via host-driven second passes.** A few widgets render an
overlay that must paint *on top of* siblings. Their `Widget::draw`
intentionally only renders the base content; the host calls a separate
method during a second pass:

- `MenuBar::draw_dropdown_only(ctx, theme)` — the dropdown menu.
- `ComboBox` — the dropdown popup (similar pattern).
- `Tooltip` — drawn last by the widget itself when visible.

If a host writes a generic walk-tree-and-draw without considering this,
menu dropdowns and combo popups will be hidden behind sibling widgets.
See `kitchen_sink.rs` and `menu_demo.rs` for the canonical second-pass
walker.

**Categories** (paragraph each; full widget list in
[`ERIGUI_INDEX.md`](./ERIGUI_INDEX.md)):

- **Layout containers**: `Container` (host-driven, marker only),
  `TabControl`, `DockPanel`, `Accordion`. DockPanel uses a `DockNode`
  tree (`Tabs / Split / Empty`) and exposes test accessors
  (`first_leaf_panels_for_test`, `first_leaf_active_index_for_test`).
- **Inputs**: `Button`, `Checkbox`, `RadioButton`, `Slider`, `SpinBox`,
  `TextInput`, `TextArea`, `ComboBox`, `SearchBox`. Most accept
  `.with_tooltip(text)`.
- **Display / lists**: `Label`, `ListView`, `TreeView`, `Icon`,
  `ProgressBar`, `StatusBar`, `Separator`. ListView and TreeView both
  switch their border to `theme.colors.border_focus` when focused.
- **Modals / overlays**: `Dialog` (multi-line `\n` rendering),
  `FileDialog` (fallback; tinyfiledialogs is the live picker),
  `NotificationManager` (toast queue with clip-to-bounds), `Tooltip`.
- **Pickers**: `ColorPicker`, `DateTimePicker`.
- **Menus**: `MenuBar` (with the z-order caveat above), `MenuItem`,
  `ContextMenu`, `Toolbar`.
- **Misc**: `Breadcrumb`, `keyboard_nav` helper, `accessibility` (⚠️
  stub).

---

## `erigui-runtime` / `erigui-workflow` / `erigui-app`

Less mature scaffolding for reactive UI / signal graphs / app-shell
patterns. Not used by `kitchen_sink` or any of the demos that drive
the visual-regression harness. Consult their `Cargo.toml` and `lib.rs`
when you need them; the kitchen_sink path is host-driven layout +
manual event pumping, no reactive runtime in the loop.

---

## ⚠️ `erigui-nodes` — node-graph editor

Blocked. `cargo build -p erigui-nodes` fails with three errors against
the current `inference-flame` API (see `KNOWN_ISSUES.md` and the
2026-04-29 `graphinference` scaffold at
`/home/alex/EriDiffusion/graphinference/`). The intent is for nodes to
target `graphinference` instead of `inference-flame` directly, so the
node graph editor can compile while inference-flame is in the middle
of the TensorIterator port. The `graphinference` crate is currently
empty placeholder modules — the API surface is documented but not
wired.

---

## `erigui-examples` — demos

Twenty-ish examples. The relevant ones for a working session:

- **`kitchen_sink`** — six tabs covering 25+ widgets. This is the
  canonical visual-regression harness. Always run it after a widget
  change. Diagnostic env vars (`ERIGUI_DEBUG_DOCK`, etc.) gate stderr
  prints inside specific widgets.
- **`menu_demo`** — MenuBar + dropdowns. Demonstrates the second-pass
  draw pattern.
- **`widget_gallery`** — older mixed-widget reference.
- The rest are widget-isolated reproducers (`tree_view_demo`,
  `tooltip_demo`, etc.). Useful when bisecting a bug to one widget.

All demos hold a persistent `EventTranslator` (the migration completed
in commit `169bc11`). None use the deprecated `convert_window_event`.

---

## Tests

`cargo test -p erigui-widgets --release` is the working test suite.
~290 passing tests cover widget behavior in isolation:

- `widget_tests.rs` — most widgets.
- `simple_widgets_tests.rs` — Button / Checkbox / Slider / ProgressBar.
- `picker_widgets_tests.rs` — ComboBox / ColorPicker / DateTimePicker.
- `tree_dock_tests.rs` — TreeView + DockPanel.
- `modal_widgets_tests.rs` — Dialog + NotificationManager.
  *5 pre-existing `manager_*` failures predate the recent fix passes;
  separate triage needed.*

Workspace-wide `cargo test --workspace` is blocked by the
`erigui-nodes` upstream issue.

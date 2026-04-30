# EriGui symbol index

> Flat list of public symbols → `crate/file:approx-line` + 1-line description.
> First place to look when you need to know "where is X" or "is there
> already a function/widget for Y."
>
> Format mirrors `flame-core/docs/FLAME_INDEX.md`. ⭐ marks symbols that
> the kitchen_sink demo or a real host actually exercises today, ⚠️
> marks legacy / unfinished surface (`erigui-nodes` blocked upstream,
> `accessibility.rs` is a stub).

---

## Core types — `erigui-core::types` (`erigui-core/src/types.rs`)

| Symbol | Where | Notes |
|---|---|---|
| ⭐ `Point { x: i32, y: i32 }` | `types.rs:7` | Integer-pixel 2D point. `Point::ZERO`, `add/sub/distance`. |
| ⭐ `Size { width: i32, height: i32 }` | `types.rs:55` | Integer-pixel size. `Size::ZERO`. |
| ⭐ `Rect` | `types.rs:84` | `x/y/width/height` + helpers (`contains`, `intersect`, `inset`, `right`, `bottom`, `center`, `from_origin_size`). All math in i32 pixels. |
| ⭐ `Color { r, g, b, a: u8 }` | `types.rs:179` | RGBA. Constructors: `rgb(r,g,b)`, `rgba(r,g,b,a)`, `from_hex(0xRRGGBB)`, `with_alpha(a)`. Constants: `BLACK / WHITE / RED / GREEN / BLUE / TRANSPARENT`. `to_gl_color() -> [f32; 4]` for the GL backend. |
| ⭐ `Margins { top, right, bottom, left }` | `types.rs:231` | Used by `Theme::spacing.padding/margins`. `Margins::all(n)`. |
| `LayoutConstraints` | `layout.rs` | `min/max_width/height` + alignment hints. Passed to `Widget::measure`. |
| `LayoutMode { Vertical, Horizontal, Grid, ... }` | `layout.rs` | Layout enum used by `Container`. |
| `LayoutConfig` | `layout.rs` | Per-container layout options. |

---

## Widget trait + drawing — `erigui-core::widget` (`erigui-core/src/widget.rs`)

| Symbol | Where | Notes |
|---|---|---|
| ⭐ `trait Widget` | `widget.rs:4` | The trait every widget implements. Required: `id()`, `measure()`, `layout()`, `draw()`, `handle_event()`, `bounds()`, `set_bounds()`, `is_visible()` / `set_visible()`, `is_enabled()` / `set_enabled()`, `is_focused()` / `set_focused()`, `can_focus()`, `as_any()`, `as_any_mut()`. |
| ⭐ `enum EventResult { Consumed, Ignored }` | `widget.rs:63` | Return value of `handle_event`. `is_consumed()`. |
| ⭐ `trait DrawContext` | `widget.rs:78` | Backend interface widgets call: `set_color`, `fill_rect`, `draw_rect`, `draw_line`, `draw_text`, `measure_text`, `push_clip_rect`, `pop_clip_rect`. The OpenGL `Renderer` implements this. |
| ⭐ `WidgetState` | `widget.rs:136` | Common per-widget state every widget embeds: `id`, `bounds`, `visible`, `enabled`, `focused`. `WidgetState::new(id)`. |
| ⭐ `WidgetId` | `widget.rs` | Newtype `u64` widget identifier. `Default` produces a unique id (atomic counter). |
| `WidgetManager` | `widget.rs` | Heterogeneous widget collection: `add_widget`, `get`, `get_typed::<T>`, `get_typed_mut::<T>`. Used by demos that want a tree-walked draw/event pass. |
| `Module` (re-exported) | (alias of `Widget`) | Layer trait — same shape as `Widget`. |

---

## Events — `erigui-core::event` (`erigui-core/src/event.rs`)

| Symbol | Where | Notes |
|---|---|---|
| ⭐ `enum Event` | `event.rs:5` | `MouseMove / MouseButton / MouseWheel / KeyPress / KeyRelease / TextInput / Focus / Resize / DragDrop / Update`. |
| ⭐ `MouseMoveEvent` | `event.rs:19` | `position`, `delta`, `modifiers`. |
| ⭐ `MouseButtonEvent` | `event.rs:26` | `button: MouseButton`, `position`, `pressed: bool`, `modifiers`. |
| ⭐ `MouseWheelEvent` | `event.rs:34` | `delta: Point`, `position`, `modifiers`. |
| ⭐ `KeyPressEvent` | `event.rs:41` | `key: Key`, `modifiers`, `repeat: bool`. |
| ⭐ `KeyReleaseEvent` | `event.rs:48` | `key`, `modifiers`. |
| ⭐ `TextInputEvent { text: String }` | `event.rs:54` | Printable text from the OS layer. Translator dedupes with `KeyPress`. |
| ⭐ `FocusEvent` | `event.rs:59` | `gained: bool`. |
| `ResizeEvent` | `event.rs:64` | `old_size`, `new_size`. |
| ⭐ `enum MouseButton` | `event.rs:85` | `Left / Right / Middle / Extra1 / Extra2`. |
| ⭐ `enum Key` | `event.rs:110` | All physical keys: A-Z, Num0-9, F1-F24, NumpadNum0-9, NumpadAdd/Subtract/Multiply/Divide/Decimal/Enter, arrows, Home, End, PageUp/Down, Tab, Enter, Escape, Backspace, Delete, Space, plus `Character(char)` fallback for printable chars without a physical mapping. |
| `Modifiers` (bitflags) | `event.rs` | `CTRL / SHIFT / ALT / META / CAPSLOCK / NUMLOCK`. `Modifiers::empty()`, `.contains(Modifiers::CTRL)`. |

---

## Theme — `erigui-core::theme` (`erigui-core/src/theme.rs`)

| Symbol | Where | Notes |
|---|---|---|
| ⭐ `Theme` | `theme.rs:5` | `name`, `colors: ThemeColors`, `typography`, `spacing`, `borders`. Pass by reference to every `Widget::layout` and `Widget::draw`. |
| ⭐ `ThemeColors` | `theme.rs:14` | 22 named slots: `background`, `surface`, `surface_variant`, `text`, `text_secondary`, `text_disabled`, `primary`, `primary_hover`, `primary_active`, `primary_disabled`, `secondary`, `secondary_hover`, `secondary_active`, `success`, `warning`, `error`, `info`, `border`, `border_hover`, `border_focus`, `selection`, `shadow`, `overlay`. |
| ⭐ `Typography` | `theme.rs:54` | `font_family`, `font_size_base`, `font_size_small/large/xlarge`, `line_height`, `letter_spacing`. |
| ⭐ `Spacing` | `theme.rs:65` | `base`, `margins: Margins`, `padding: Margins`, `gap_small/medium/large`. |
| ⭐ `Borders` | `theme.rs:75` | `width`, `radius`, `radius_small/large`. |
| ⭐ `Theme::dark()` | `theme.rs:204` | Default dark theme. Used by every demo that doesn't pick another. |
| `Theme::light()` | `theme.rs:137` | Default light theme. |
| `Theme::alex_jammin()` | `theme.rs:83` | EriGui's signature purple/indigo. |
| ⭐ `Theme::serenity()` | `theme.rs:255` | Indigo + purple. Ported from `serenity/serenity/ui/theme.py:PALETTE_SERENITY`. |
| ⭐ `Theme::moonlight()` | `theme.rs:289` | Neutral gray with yellow accent. Ported. |
| ⭐ `Theme::monochrome()` | `theme.rs:323` | Cyan-on-black hacker terminal. Ported. |
| ⭐ `Theme::nord()` | `theme.rs:357` | Cool arctic blue/gray (popular). Ported. |
| ⭐ `Theme::cinder()` | `theme.rs:391` | Slate dark with red accent. Ported. |
| ⭐ `Theme::blender()` | `theme.rs:425` | Blender-3D-editor look (gray + blue). Ported. |
| ⭐ `Theme::cyberpunk()` | `theme.rs:459` | Neon cyan/magenta on near-black. Ported. |
| ⭐ `Theme::all_named()` | `theme.rs:493` | `Vec<Theme>` of all 10 built-in themes in display order. Use for theme-picker dropdowns. |
| ⭐ `Theme::by_name(&str)` | `theme.rs:514` | Case-insensitive lookup. Returns `None` for unknown names. |
| ⭐ `Theme::name(&self)` | `theme.rs:535` | Display name of a theme (string slice into `name` field). |
| ⭐ `Theme::with_scale(f32)` | `theme.rs:540` | Scales typography / spacing / border-width by `scale`. Use this once at startup with the monitor's HiDPI factor; widgets read post-scale values from the theme via `layout()`. |
| `ThemeManager` | `theme.rs:586` | Map `name → Theme`, current pointer. `new()`, `current()`, `set_theme(name)`, `add_theme(theme)`, `list_themes()`. Comes pre-populated with all 10 built-ins. |

---

## Rendering — `erigui-rendering` (`erigui-rendering/src/`)

| Symbol | Where | Notes |
|---|---|---|
| ⭐ `Renderer` | `renderer.rs` | OpenGL renderer + window. `Renderer::new(&event_loop, w, h, title)`, `begin_frame(clear_color)`, `end_frame()`, `viewport_size() -> Size`, `resize(w, h)`, `window() -> &Window` (winit handle for `request_redraw`). Implements `DrawContext`. |
| ⭐ `EventTranslator` | `window.rs:33` | Stateful winit `WindowEvent` → erigui `Event` translator. **MUST be persistent** — see CONVENTIONS. `new(window_size)`, `set_window_size(size)`, `translate(&event) -> Option<Event>`. |
| ⚠️ `convert_window_event(event, size)` | `window.rs:265` | `#[deprecated]`. Constructs a fresh translator every call → `MouseInput` clicks always report (0, 0). Use `EventTranslator` instead. |
| `font::Font` | `font.rs` / `font_freetype.rs` | FreeType-backed glyph rasterizer; cached in the renderer. |

---

## Widgets — `erigui-widgets` (`erigui-widgets/src/`)

> Each is an independent `pub struct` implementing `Widget`. Constructor
> is `Foo::new(WidgetId)` for most. Common pattern: `.with_*` builder
> methods for config, `set_*` getters/setters for runtime mutation.

### Layout containers

| Widget | File | One-liner |
|---|---|---|
| ⭐ `Container` | `container.rs` | Marker for host-driven layout — does not lay out children itself (kitchen_sink calls `child.layout(rect, theme)` per child). |
| ⭐ `TabControl` | `tab_control.rs` | Top-tab page organizer. `add_tab(TabPage)`, `set_tabs(Vec<TabItem>)` (idempotent), `set_active_tab(idx)`, `active_tab()`, `bounds()`. Tab strip stretches to fill width — no `max_tab_width` clamp anymore. Ctrl+Tab / Ctrl+1..9 navigation. |
| ⭐ `DockPanel` | `dock_panel.rs` | VS-Code-style dockable panels. `add_panel(DockablePanel, DockPosition)`, four positions (Center/Left/Right/Top/Bottom/Floating). Test accessors: `first_leaf_panels_for_test()`, `first_leaf_active_index_for_test()`. `ERIGUI_DEBUG_DOCK=1` dumps tab-strip clicks. |
| ⭐ `Accordion` | `accordion.rs` | Collapsible panel group. `add_panel(AccordionPanel)`, `with_on_toggle`. Auto-focuses first panel when host gives focus; Space/Enter toggles, Up/Down nav. Suppresses autorepeat-toggle. `ERIGUI_DEBUG_ACC=1`. |

### Inputs

| Widget | File | One-liner |
|---|---|---|
| ⭐ `Button` | `button.rs` | Standard push button. `with_text`, `with_on_click`, `with_tooltip`, `with_icon`. Activates on Enter/Space when focused. |
| ⭐ `Checkbox` | `checkbox.rs` | Toggle. `with_checked`, `with_on_toggle`. Space toggles when focused. |
| ⭐ `RadioButton` | `radio_button.rs` | Single-select within a `RadioGroup`. `button_size` theme-scaled in `layout()`. |
| ⭐ `Slider` | `slider.rs` | Numeric drag slider. `with_on_value_changed`. Arrow keys when focused. |
| ⭐ `SpinBox` | `spin_box.rs` | Numeric input with up/down arrows. Caret drawn at end of `text_value` when `is_editing`. Arrow click commits typed value first. Accepts `Key::Num*`, `NumpadNum*`, `Period`, `Minus` as digit fallback (Wayland configs sometimes drop `event.text`). |
| ⭐ `TextInput` | `text_input.rs` | Single-line text. 2px primary-color caret when focused. Ctrl+A/C/X/V. `Key::Space` arm inserts a literal space (translator emits Space as KeyPress not TextInput). |
| ⭐ `TextArea` | `text_area.rs` | Multi-line editor. Undo/redo, line numbers, word wrap, selection. `Key::Space` arm. |
| ⭐ `ComboBox` | `combo_box.rs` | Dropdown selector with type-to-filter. `with_items`, `with_selected`, `with_on_selection_changed`. |
| `SearchBox` | `search_box.rs` | TextInput + magnifier icon + history dropdown + suggestions. |

### Display / lists

| Widget | File | One-liner |
|---|---|---|
| ⭐ `Label` | `label.rs` | Static text. `with_align(TextAlign)`, `with_color(Color)`. |
| ⭐ `ListView` | `list_view.rs` | Scrollable single-select list. `item_height` theme-scaled. Border switches to `border_focus` when focused. |
| ⭐ `TreeView` | `tree_view.rs` | Hierarchical list. `item_height` theme-scaled. Single-click on a node-with-children row toggles expand (full-row hit area, not just the chevron). Border-focus indicator. |
| `Icon` | `icon.rs` | Pure-vector icon set: `draw_close`, `draw_check`, `draw_info`, `draw_warning`, `draw_error`, etc. Static methods on `Icon`. |
| ⭐ `ProgressBar` | `progress_bar.rs` | Determinate / indeterminate. `with_style(ProgressBarStyle)`. |
| `StatusBar` | `status_bar.rs` | Multi-panel bottom bar. `set_panel_text(idx, str)`. |
| `Separator` | (in `lib.rs`) | Horizontal/vertical rule. `SeparatorStyle`. |

### Modals / overlays

| Widget | File | One-liner |
|---|---|---|
| ⭐ `Dialog` | `dialog.rs` | Modal popup. `Dialog::new(id, title, message)`, `show()`, `close()`. Multi-line `\n` rendering, theme-scaled `min_width` (was hardcoded 300), close glyph in struct field. |
| `FileDialog` | `file_dialog.rs` | Open / Save / SelectFolder. `tinyfiledialogs` is the live picker; this widget is a fallback. |
| ⭐ `NotificationManager` | `notification.rs` | Toast queue. `show(Notification)`, `clear()`. Toasts are clipped to box bounds (long titles no longer overflow). Width = `font * 30`. |
| `Notification` | `notification.rs` | Single toast: `with_title`, `with_message`, `with_type(NotificationType::Info/Success/Warning/Error)`. |
| ⭐ `Tooltip` (`TooltipState`) | `tooltip.rs` | Hover-to-show overlay. `update_on_event(event, bounds)`, `draw(ctx, theme, bounds)`. Most input widgets accept `with_tooltip(text)` / `with_tooltip_state(TooltipState)`. |

### Pickers

| Widget | File | One-liner |
|---|---|---|
| `ColorPicker` | `color_picker.rs` | RGB/HSV picker, optional alpha, optional preview. `ColorPickerStyle`. |
| `DateTimePicker` | `date_time_picker.rs` | Calendar + clock. `DateTimePickerMode::Date / Time / DateTime`. |

### Menus

| Widget | File | One-liner |
|---|---|---|
| ⭐ `MenuBar` | `menu.rs` | Top menu bar. `add_menu(text, items)`. **Important**: `Widget::draw` only renders the strip — host MUST call `MenuBar::draw_dropdown_only` after the rest of the tree (z-order). See `kitchen_sink.rs` and `menu_demo.rs`. |
| `MenuItem` | `menu.rs:23` | `MenuItem::new(text)`, `with_shortcut`, `with_submenu(items)`, `with_on_click`, `MenuItem::separator()`. Note: `Box<dyn FnMut()>`, NOT `Clone`. |
| `ContextMenu` | `context_menu.rs` | Right-click menu. |
| `Toolbar` | `toolbar.rs` | Icon-button strip with optional overflow chevron. |

### Misc

| Widget | File | One-liner |
|---|---|---|
| `Breadcrumb` | `breadcrumb.rs` | Path segments with click navigation. |
| ⚠️ `accessibility.rs` | `accessibility.rs` | Stub. Screen-reader bridge not implemented. |
| `keyboard_nav.rs` | `keyboard_nav.rs` | Focus-cycling helper used by some demos. |

### Clipboard / drag-drop

| Symbol | Where | Notes |
|---|---|---|
| `clipboard::get_clipboard()` | `clipboard.rs` | Platform clipboard read. |
| `clipboard::set_clipboard(s)` | `clipboard.rs` | Platform clipboard write. |
| `clipboard::clear_clipboard()` | `clipboard.rs` | (Used by some demos.) |
| `drag_drop` | `drag_drop.rs` | Helper types: `DragData`, `DragDropEvent` (re-exported via core). |

---

## Examples — `erigui-examples/examples/` (`erigui-examples/examples/*.rs`)

| Example | One-liner |
|---|---|
| ⭐ `kitchen_sink` | The canonical visual-regression harness. 6 tabs covering most widgets. **Always run after a widget change.** |
| ⭐ `menu_demo` | MenuBar + dropdowns + submenus. Demonstrates the `draw_dropdown_only` z-order pattern. |
| `widget_gallery` | Old grid-of-widgets reference. |
| `dock_panel_demo` | DockPanel splits / floating. |
| `tree_view_demo` | TreeView keyboard nav demo. |
| `tooltip_demo` | Hover delay + auto-position. |
| `notification_demo` | Toast queue + close-button + actions. |
| `accordion_demo` | Toggle / kbd nav. |
| `radio_button_demo` | RadioGroup + animation. |
| `text_demo` | TextInput in isolation. |
| `text_area_demo` | TextArea in isolation. |
| `breadcrumb_demo` | Path nav. |
| `search_box_demo` | Filter + history. |
| `toolbar_demo` | Toolbar with overflow. |
| `color_picker_demo` | ColorPicker. |
| `date_time_picker_demo` | DateTimePicker. |
| `file_dialog_demo` | FileDialog (fallback path). |
| `enhanced_widgets_demo` | Mixed kbd-nav demo. |
| `widgets_demo` | Pre-strip mixed demo. |

All examples now hold a persistent `EventTranslator`. None still use the
deprecated `convert_window_event`.

---

## Diagnostic env vars

| Var | Effect |
|---|---|
| `ERIGUI_DEBUG_DOCK=1` | `dock_panel.rs` prints click coords + tab list + bounds + active-index transitions per MouseButton press. |
| `ERIGUI_DEBUG_ACC=1` | `accordion.rs` prints each KeyPress that enters the keyboard-nav arm with `focused_panel` and `repeat`. |
| `ERIGUI_DEBUG_SPIN=1` | (Wired in commits earlier; current HEAD only prints from MouseButton arms — see `spin_box.rs` if you need to re-instrument.) |

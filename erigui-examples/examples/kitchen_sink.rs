//! Kitchen-sink demo: visually exercise every wave-2/wave-3 widget fix
//! in a single launchable binary so a single hover/click pass confirms
//! the lot. Layout is host-driven (Container is a marker per wave-3
//! commit 61f8dd4) and tabs use the wave-2 keyboard-nav-enabled
//! TabControl as the page organizer. See the [[example]] block in
//! erigui-examples/Cargo.toml for the test plan.
//!
//! Cap: ~1000 LOC. No new public widget APIs, public state only.

use erigui_core::{
    Color, DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, Modifiers,
    MouseButton, MouseButtonEvent, Rect, Size, Theme, Widget,
};
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::{
    clear_clipboard, get_clipboard, Accordion, AccordionPanel, Breadcrumb, BreadcrumbItem, Button,
    Checkbox, ColorPicker, ColorPickerStyle, ComboBox, DateTimePicker, DateTimePickerMode, Dialog,
    DialogType, DockPanel, DockPosition, DockablePanel, FileDialog, FileDialogMode, Icon, Label,
    ListItem, ListView, Notification, NotificationManager, NotificationPosition, NotificationType,
    ProgressBar, ProgressBarStyle, RadioButton, RadioGroup, SeparatorStyle, Slider, SpinBox,
    StatusBar, StatusPanel, TabControl, TabPage, TextAlignment, TextArea, TextInput, TreeNode,
    TreeView, WidgetManager,
};
use std::cell::RefCell;
use std::rc::Rc;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

// ---------- shared "selection confirmed" channel (StatusBar feedback) ----------
//
// Every widget callback writes a status-bar message via this Rc<RefCell<String>>.
// The App reads it each frame and pushes it into status_bar.set_panel_text(0, ...).
type StatusMsg = Rc<RefCell<String>>;

fn make_status_msg() -> StatusMsg {
    Rc::new(RefCell::new("Ready. Click around to verify wave-2/3 fixes.".into()))
}

fn set_status(msg: &StatusMsg, text: impl Into<String>) {
    *msg.borrow_mut() = text.into();
}

// -- Theme picker request channels --------------------------------------------
//
// MiscTab hosts a ComboBox + ColorPicker that lets the user pick a theme by
// name or override the primary color. Both write to these Rc<RefCell> slots;
// the main event loop reads them after each handle_event pass and rebuilds
// the live `Theme` accordingly. Same shape as `StatusMsg` — keeps the widget
// callbacks `'static` without threading raw refs.
type ThemeRequest = Rc<RefCell<Option<String>>>;
type ColorRequest = Rc<RefCell<Option<Color>>>;

// ---------- shared layout metrics derived from the theme ----------
//
// The theme is already scaled by ui_scale at App start (Theme::with_scale,
// wave-1 a07a8f9). All sizes here scale with it; on a 4K display at scale
// 2x the rows roughly double, on a 1080p at 1x they're roughly the
// pre-fix hardcoded 32px. Avoid raw px constants in tab layouts.
struct LayoutMetrics {
    pad: i32,
    row_h: i32,
    row_gap: i32,
    btn_w: i32,
    btn_gap: i32,
    cb_w: i32,
    label_h: i32,
}

/// Minimum content body height (so the demo doesn't collapse when the
/// window is smaller than reasonable). Theme-scaled.
fn m_min_body(theme: &Theme) -> i32 {
    theme.typography.font_size_base * 12
}

fn metrics(theme: &Theme) -> LayoutMetrics {
    let font = theme.typography.font_size_base.max(12);
    let pad = theme.spacing.padding.left.max(8);
    LayoutMetrics {
        pad,
        // glyph + chrome: enough vertical room for the font + button border/padding.
        row_h: font + pad * 2 + 4,
        row_gap: theme.spacing.gap_medium.max(8),
        // ~10 chars wide — covers labels like "Open Dialog", "Toast Info", etc.
        btn_w: font * 10,
        btn_gap: theme.spacing.gap_medium.max(8),
        // ~16 chars wide — covers checkbox labels like "Enable feature A".
        cb_w: font * 16,
        label_h: font + 6,
    }
}

// ---------- tab 1: Basic Inputs (wave-2 TooltipState retrofit) ----------
//
// Verify: hover button/checkbox/slider/spinbox/combobox/text_input ~500ms ->
// tooltip appears. Click any -> tooltip dismisses. Disabled button: no
// tooltip (wave-2 fix; tooltip is hidden in set_enabled(false)).
struct BasicInputsTab {
    btn_ok: Button,
    btn_disabled: Button,
    btn_warn: Button,
    cb_a: Checkbox,
    cb_b: Checkbox,
    slider: Slider,
    spin: SpinBox,
    combo: ComboBox,
    text_input: TextInput,
    label_section: Label,
}

impl BasicInputsTab {
    fn new(status: StatusMsg) -> Self {
        let s1 = status.clone();
        let s2 = status.clone();
        let s3 = status.clone();
        let s4 = status.clone();
        let s5 = status.clone();
        let s6 = status.clone();
        let s7 = status.clone();
        let s8 = status.clone();

        let btn_ok = Button::new(Default::default(), "OK")
            .with_tooltip("Standard OK button. Hover ~500ms; tooltip dismisses on press.")
            .with_on_click(move || set_status(&s1, "btn_ok pressed"));

        let mut btn_disabled = Button::new(Default::default(), "Disabled")
            .with_tooltip("This tooltip should NEVER show -- button is disabled.");
        btn_disabled.set_enabled(false);

        let btn_warn = Button::new(Default::default(), "Warn")
            .with_tooltip("Hover here, then click on OK. Tooltip should hide on press.")
            .with_on_click(move || set_status(&s2, "btn_warn pressed"));

        let cb_a = Checkbox::new(Default::default(), "Enable feature A")
            .with_tooltip("Wave-2: per-widget TooltipState now lives on Checkbox.")
            .with_on_toggle(move |c| set_status(&s3, format!("cb_a -> {}", c)));

        let cb_b = Checkbox::new(Default::default(), "Enable feature B")
            .with_tooltip("Click to toggle; tooltip hides on press.")
            .with_checked(true)
            .with_on_toggle(move |c| set_status(&s4, format!("cb_b -> {}", c)));

        let slider = Slider::new(Default::default(), 0.0, 100.0, 50.0)
            .with_tooltip("Wave-2: Slider hides its tooltip in set_enabled(false).")
            .with_on_value_changed(move |v| set_status(&s5, format!("slider -> {:.1}", v)));

        let spin = SpinBox::new(Default::default(), 0.0, 1000.0, 42.0)
            .with_tooltip("Wave-2: SpinBox tooltip retrofit; hides on disable.")
            .with_on_value_changed(move |v| set_status(&s6, format!("spin -> {:.1}", v)));

        let combo = ComboBox::new(Default::default())
            .with_items(vec![
                "Apple".into(),
                "Banana".into(),
                "Cherry".into(),
                "Durian".into(),
                "Elderberry".into(),
            ])
            .with_selected(0)
            .with_tooltip("Wave-2: ComboBox tooltip retrofit. Click to open.")
            .with_on_selection_changed(move |i, t| {
                set_status(&s7, format!("combo -> {:?} ({:?})", i, t));
            });

        let text_input = TextInput::new(Default::default())
            .with_placeholder("Type here...")
            .with_tooltip("Wave-2: text_input early-return guard blocks events on disabled.")
            .with_on_change(move |t| set_status(&s8, format!("text_input -> {:?}", t)));

        let label_section = Label::new(
            Default::default(),
            "Tab 1: Basic Inputs (hover for tooltips ~500ms; click dismisses)",
        );

        Self {
            btn_ok,
            btn_disabled,
            btn_warn,
            cb_a,
            cb_b,
            slider,
            spin,
            combo,
            text_input,
            label_section,
        }
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let x0 = area.x() + m.pad;

        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(x0, y, area.width() - m.pad * 2, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        // Row 1: three buttons across
        self.btn_ok.layout(Rect::new(x0, y, m.btn_w, m.row_h), theme);
        self.btn_disabled.layout(
            Rect::new(x0 + m.btn_w + m.btn_gap, y, m.btn_w, m.row_h),
            theme,
        );
        self.btn_warn.layout(
            Rect::new(x0 + (m.btn_w + m.btn_gap) * 2, y, m.btn_w, m.row_h),
            theme,
        );
        y += m.row_h + m.row_gap;

        // Row 2: two checkboxes — separated enough that long labels don't overlap
        self.cb_a.layout(Rect::new(x0, y, m.cb_w, m.row_h), theme);
        self.cb_b.layout(
            Rect::new(x0 + m.cb_w + m.btn_gap, y, m.cb_w, m.row_h),
            theme,
        );
        y += m.row_h + m.row_gap;

        // Row 3: slider — wider than buttons for visible drag range
        self.slider
            .layout(Rect::new(x0, y, m.btn_w * 3, m.row_h), theme);
        y += m.row_h + m.row_gap;

        // Row 4: spinbox
        self.spin
            .layout(Rect::new(x0, y, m.btn_w + m.btn_gap, m.row_h), theme);
        y += m.row_h + m.row_gap;

        // Row 5: combobox
        self.combo
            .layout(Rect::new(x0, y, m.btn_w * 2, m.row_h), theme);
        y += m.row_h + m.row_gap;

        // Row 6: text input
        self.text_input
            .layout(Rect::new(x0, y, m.btn_w * 3, m.row_h), theme);
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.btn_ok.draw(ctx, theme);
        self.btn_disabled.draw(ctx, theme);
        self.btn_warn.draw(ctx, theme);
        self.cb_a.draw(ctx, theme);
        self.cb_b.draw(ctx, theme);
        self.slider.draw(ctx, theme);
        self.spin.draw(ctx, theme);
        // ComboBox last so its open dropdown overlays the TextInput below.
        // Earlier order drew TextInput on top of the dropdown popup, hiding
        // the lower options behind it.
        self.text_input.draw(ctx, theme);
        self.combo.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme) {
        let _ = self.btn_ok.handle_event(e, theme);
        let _ = self.btn_disabled.handle_event(e, theme);
        let _ = self.btn_warn.handle_event(e, theme);
        let _ = self.cb_a.handle_event(e, theme);
        let _ = self.cb_b.handle_event(e, theme);
        let _ = self.slider.handle_event(e, theme);
        let _ = self.spin.handle_event(e, theme);
        let _ = self.combo.handle_event(e, theme);
        let _ = self.text_input.handle_event(e, theme);
    }
}

// ---------- tab 2: Selection & Lists (wave-1 ListView scrollbar drag, wave-3 TreeView kbd nav) ----------
//
// Verify: 50-item ListView shows scrollbar; drag the thumb. RadioGroup
// uses host-owned Rc<RefCell<RadioGroup>> per wave-2 fe7f50f. TreeView:
// click to focus, then Up/Down/Left/Right/Enter/Space/Home/End -- wave-3
// commit 5370607 added these.
struct SelectionTab {
    label_section: Label,
    list: ListView,
    radio_a: RadioButton,
    radio_b: RadioButton,
    radio_c: RadioButton,
    radio_d: RadioButton,
    tree: TreeView,
    label_list: Label,
    label_radio: Label,
    label_tree: Label,
}

impl SelectionTab {
    fn new(status: StatusMsg, mgr: &mut WidgetManager) -> Self {
        let s1 = status.clone();
        let s2 = status.clone();

        let items: Vec<ListItem> = (1..=200)
            .map(|i| ListItem {
                id: format!("row{}", i),
                text: format!("Item {} (drag scrollbar to move me)", i),
                icon: None,
                selected: i == 1,
            })
            .collect();

        let list = ListView::new(Default::default())
            .with_items(items)
            .with_on_selection_change(move |idx| {
                set_status(&s1, format!("ListView -> {:?}", idx));
            });

        // Mint real, unique WidgetIds for the radios via a WidgetManager so
        // the host-owned RadioGroup actually distinguishes them. The existing
        // radio_button_demo.rs uses WidgetId::default() for all, which makes
        // every radio share the null key -- hence a click on any radio in a
        // group selects them all. Real ids dodge that latent bug.
        let id_a = mgr.add_widget(Box::new(Label::new(Default::default(), "")));
        let id_b = mgr.add_widget(Box::new(Label::new(Default::default(), "")));
        let id_c = mgr.add_widget(Box::new(Label::new(Default::default(), "")));
        let id_d = mgr.add_widget(Box::new(Label::new(Default::default(), "")));

        let group = Rc::new(RefCell::new(RadioGroup::new()));

        let s_a = status.clone();
        let s_b = status.clone();
        let s_c = status.clone();
        let s_d = status.clone();

        let radio_a = RadioButton::new(id_a, "Alpha")
            .with_group(group.clone())
            .with_on_change(move |c| set_status(&s_a, format!("Alpha -> {}", c)));
        let radio_b = RadioButton::new(id_b, "Bravo")
            .with_checked(true)
            .with_group(group.clone())
            .with_on_change(move |c| set_status(&s_b, format!("Bravo -> {}", c)));
        let radio_c = RadioButton::new(id_c, "Charlie")
            .with_group(group.clone())
            .with_on_change(move |c| set_status(&s_c, format!("Charlie -> {}", c)));
        let radio_d = RadioButton::new(id_d, "Delta")
            .with_group(group)
            .with_on_change(move |c| set_status(&s_d, format!("Delta -> {}", c)));

        // 3-level tree: root -> 3 branches -> 2-3 leaves each
        let root = TreeNode {
            id: "root".into(),
            text: "Project (focus me, then arrow keys)".into(),
            icon: None,
            expanded: true,
            selected: false,
            children: vec![
                TreeNode {
                    id: "src".into(),
                    text: "src/".into(),
                    icon: None,
                    expanded: true,
                    selected: false,
                    children: vec![
                        TreeNode {
                            id: "main.rs".into(),
                            text: "main.rs".into(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                        TreeNode {
                            id: "lib.rs".into(),
                            text: "lib.rs".into(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                        TreeNode {
                            id: "util.rs".into(),
                            text: "util.rs".into(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                    ],
                },
                TreeNode {
                    id: "tests".into(),
                    text: "tests/".into(),
                    icon: None,
                    expanded: false,
                    selected: false,
                    children: vec![
                        TreeNode {
                            id: "smoke.rs".into(),
                            text: "smoke.rs".into(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                        TreeNode {
                            id: "regress.rs".into(),
                            text: "regress.rs".into(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                    ],
                },
                TreeNode {
                    id: "docs".into(),
                    text: "docs/".into(),
                    icon: None,
                    expanded: false,
                    selected: false,
                    children: vec![TreeNode {
                        id: "README.md".into(),
                        text: "README.md".into(),
                        icon: None,
                        expanded: false,
                        selected: false,
                        children: vec![],
                    }],
                },
            ],
        };

        let tree = TreeView::new(Default::default())
            .with_root_nodes(vec![root])
            .with_on_selection_change(move |path| {
                set_status(&s2, format!("Tree selected: {}", path));
            });

        Self {
            label_section: Label::new(
                Default::default(),
                "Tab 2: Selection -- drag list scrollbar; tree kbd: Up/Down/Left/Right/Enter/Home/End",
            ),
            list,
            radio_a,
            radio_b,
            radio_c,
            radio_d,
            tree,
            label_list: Label::new(Default::default(), "ListView (50 items, scrollable):"),
            label_radio: Label::new(Default::default(), "RadioGroup (host-owned Rc<RefCell>):"),
            label_tree: Label::new(Default::default(), "TreeView (click to focus, then arrows):"),
        }
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(area.x() + m.pad, y, area.width() - m.pad * 2, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        // Three columns: list (left), radios (middle), tree (right).
        let col_w = (area.width() - m.pad * 4) / 3;
        let col1_x = area.x() + m.pad;
        let col2_x = col1_x + col_w + m.pad;
        let col3_x = col2_x + col_w + m.pad;
        let col_h = area.bottom() - y - m.pad;

        // List column
        let list_lbl_h = self
            .label_list
            .measure(&LayoutConstraints::default(), theme)
            .height
            .max(m.label_h);
        self.label_list.layout(Rect::new(col1_x, y, col_w, list_lbl_h), theme);
        self.list.layout(
            Rect::new(col1_x, y + list_lbl_h + m.row_gap / 2, col_w, col_h - list_lbl_h - m.row_gap / 2),
            theme,
        );

        // Radio column
        self.label_radio.layout(Rect::new(col2_x, y, col_w, list_lbl_h), theme);
        let radio_y = y + list_lbl_h + m.row_gap / 2;
        let radio_h = m.row_h;
        self.radio_a.layout(Rect::new(col2_x, radio_y, col_w, radio_h), theme);
        self.radio_b
            .layout(Rect::new(col2_x, radio_y + radio_h, col_w, radio_h), theme);
        self.radio_c
            .layout(Rect::new(col2_x, radio_y + radio_h * 2, col_w, radio_h), theme);
        self.radio_d
            .layout(Rect::new(col2_x, radio_y + radio_h * 3, col_w, radio_h), theme);

        // Tree column
        self.label_tree.layout(Rect::new(col3_x, y, col_w, list_lbl_h), theme);
        self.tree.layout(
            Rect::new(col3_x, y + list_lbl_h + m.row_gap / 2, col_w, col_h - list_lbl_h - m.row_gap / 2),
            theme,
        );
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.label_list.draw(ctx, theme);
        self.list.draw(ctx, theme);
        self.label_radio.draw(ctx, theme);
        self.radio_a.draw(ctx, theme);
        self.radio_b.draw(ctx, theme);
        self.radio_c.draw(ctx, theme);
        self.radio_d.draw(ctx, theme);
        self.label_tree.draw(ctx, theme);
        self.tree.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme) {
        let _ = self.list.handle_event(e, theme);
        let _ = self.radio_a.handle_event(e, theme);
        let _ = self.radio_b.handle_event(e, theme);
        let _ = self.radio_c.handle_event(e, theme);
        let _ = self.radio_d.handle_event(e, theme);
        // Tree gets focus on click; route the click first so focus updates,
        // then route subsequent kbd events while focused.
        if let Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position,
            pressed,
            ..
        }) = e
        {
            if *pressed && self.tree.bounds().contains(*position) {
                self.tree.set_focused(true);
            }
        }
        let _ = self.tree.handle_event(e, theme);
    }
}

// ---------- tab 3: Modals & Dialogs (Dialog, FileDialog double-click, Notification press-fire + clear callback) ----------
//
// Verify:
//  - "Open Dialog" -> Dialog opens; OK/Cancel each push a status msg.
//  - "Open File Picker" -> in-app FileDialog; double-click navigates dirs
//    (wave-3 commit, last_click_time threshold). Single-click selects only.
//  - "Toast Info/Warn/Err" -> Notification appears; close button fires on
//    PRESS (wave-3 commit 5d88d3c).
//  - "Clear Notifications" -> manager.clear() fires on_notification_closed
//    for every queued item (wave-3 commit 4e745a0).
struct ModalsTab {
    label_section: Label,
    btn_open_dialog: Button,
    btn_open_file: Button,
    btn_toast_info: Button,
    btn_toast_warn: Button,
    btn_toast_err: Button,
    btn_clear_toasts: Button,
    dialog: Dialog,
    file_dialog: FileDialog,
    show_dialog: bool,
    show_file_dialog: bool,
    next_toast_id: u32,
    notifications_closed: Rc<RefCell<u32>>,
}

impl ModalsTab {
    fn new(status: StatusMsg) -> Self {
        let s1 = status.clone();
        let s_open = status.clone();
        let s_toast_i = status.clone();
        let s_toast_w = status.clone();
        let s_toast_e = status.clone();

        let dialog = Dialog::new(
            Default::default(),
            "Confirm Action",
            "This dialog tests Dialog.show()/close() and the modal-button callback.\n\nClick OK or Cancel to dismiss.",
        )
        .with_standard_buttons(DialogType::Question)
        .with_on_button_clicked(move |btn| {
            set_status(&s1, format!("Dialog -> {:?}", btn));
        });

        let s_file_sel = status.clone();
        let s_file_cancel = status.clone();

        let initial_path = std::path::PathBuf::from("/tmp");
        let file_dialog = FileDialog::new(Default::default(), FileDialogMode::Open)
            .with_initial_path(&initial_path)
            .with_on_file_selected(move |path| {
                set_status(&s_file_sel, format!("Picked: {}", path.display()));
            })
            .with_on_cancel(move || set_status(&s_file_cancel, "FileDialog cancelled"));

        Self {
            label_section: Label::new(
                Default::default(),
                "Tab 3: Modals -- Dialog, FileDialog (double-click dir to navigate), Notifications (close-on-press, clear-fires-callbacks)",
            ),
            btn_open_dialog: Button::new(Default::default(), "Open Dialog").with_on_click(
                move || set_status(&s_open, "[click to open dialog]"),
            ),
            btn_open_file: Button::new(Default::default(), "Open File Picker"),
            btn_toast_info: Button::new(Default::default(), "Toast Info").with_on_click(move || {
                set_status(&s_toast_i, "[push info notification]");
            }),
            btn_toast_warn: Button::new(Default::default(), "Toast Warn").with_on_click(move || {
                set_status(&s_toast_w, "[push warn notification]");
            }),
            btn_toast_err: Button::new(Default::default(), "Toast Err").with_on_click(move || {
                set_status(&s_toast_e, "[push err notification]");
            }),
            btn_clear_toasts: Button::new(Default::default(), "Clear All"),
            dialog,
            file_dialog,
            show_dialog: false,
            show_file_dialog: false,
            next_toast_id: 0,
            notifications_closed: Rc::new(RefCell::new(0)),
        }
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let x0 = area.x() + m.pad;
        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(x0, y, area.width() - m.pad * 2, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        let btn_w = m.btn_w + m.btn_gap; // ~12 chars to fit "Open File Picker"
        // Row 1: Dialog + FilePicker
        self.btn_open_dialog.layout(Rect::new(x0, y, btn_w, m.row_h), theme);
        self.btn_open_file.layout(
            Rect::new(x0 + btn_w + m.btn_gap, y, btn_w, m.row_h),
            theme,
        );
        y += m.row_h + m.row_gap;

        // Row 2: Toasts
        self.btn_toast_info.layout(Rect::new(x0, y, btn_w, m.row_h), theme);
        self.btn_toast_warn.layout(
            Rect::new(x0 + btn_w + m.btn_gap, y, btn_w, m.row_h),
            theme,
        );
        self.btn_toast_err.layout(
            Rect::new(x0 + (btn_w + m.btn_gap) * 2, y, btn_w, m.row_h),
            theme,
        );
        self.btn_clear_toasts.layout(
            Rect::new(x0 + (btn_w + m.btn_gap) * 3, y, btn_w, m.row_h),
            theme,
        );

        // Center the dialog (when shown). Theme-scaled minimums.
        if self.show_dialog {
            let dlg_w = (m.btn_w * 5).min(area.width() - m.pad * 2);
            let dlg_h = m.row_h * 6;
            self.dialog.layout(
                Rect::new(
                    area.x() + (area.width() - dlg_w) / 2,
                    area.y() + (area.height() - dlg_h) / 2,
                    dlg_w,
                    dlg_h,
                ),
                theme,
            );
        }

        // FileDialog: large, centered.
        if self.show_file_dialog {
            let fd_w = (area.width() - m.pad * 2).min(m.btn_w * 8);
            let fd_h = (area.height() - m.pad * 2).min(m.row_h * 16);
            self.file_dialog.layout(
                Rect::new(
                    area.x() + (area.width() - fd_w) / 2,
                    area.y() + (area.height() - fd_h) / 2,
                    fd_w,
                    fd_h,
                ),
                theme,
            );
        }
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.btn_open_dialog.draw(ctx, theme);
        self.btn_open_file.draw(ctx, theme);
        self.btn_toast_info.draw(ctx, theme);
        self.btn_toast_warn.draw(ctx, theme);
        self.btn_toast_err.draw(ctx, theme);
        self.btn_clear_toasts.draw(ctx, theme);
        if self.show_dialog {
            self.dialog.draw(ctx, theme);
        }
        if self.show_file_dialog {
            self.file_dialog.draw(ctx, theme);
        }
    }

    fn handle_event(
        &mut self,
        e: &Event,
        theme: &Theme,
        status: &StatusMsg,
        notifs: &mut NotificationManager,
    ) {
        // Modal: when shown, route to dialog FIRST (it already consumes
        // unrelated clicks per the locked-in test in modal_widgets_tests).
        if self.show_dialog {
            let _ = self.dialog.handle_event(e, theme);
            // Dialog's `result()` flips after a button click. Use that to
            // know when to close.
            if !self.dialog.is_open() || self.dialog.result().is_some() {
                self.show_dialog = false;
                self.dialog.close();
            }
            return;
        }
        if self.show_file_dialog {
            let _ = self.file_dialog.handle_event(e, theme);
            // Heuristic close: Esc handling and OK/Cancel inside FileDialog
            // both run their callbacks; we close on any selected path
            // appearing OR by clicking outside the FileDialog rect with a
            // subsequent press. Simplest: close on Escape key.
            if let Event::KeyPress(KeyPressEvent {
                key: Key::Escape, ..
            }) = e
            {
                self.show_file_dialog = false;
            }
            // If the dialog produced a selected path on this event, close.
            if self.file_dialog.get_selected_path().is_some() {
                set_status(
                    status,
                    format!(
                        "Picked: {}",
                        self.file_dialog
                            .get_selected_path()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ),
                );
                self.show_file_dialog = false;
            }
            return;
        }

        let _ = self.btn_open_dialog.handle_event(e, theme);
        let _ = self.btn_open_file.handle_event(e, theme);
        let _ = self.btn_toast_info.handle_event(e, theme);
        let _ = self.btn_toast_warn.handle_event(e, theme);
        let _ = self.btn_toast_err.handle_event(e, theme);
        let _ = self.btn_clear_toasts.handle_event(e, theme);

        // Translate button clicks into stateful actions. Buttons run their
        // own on_click, but we also need to flip `show_dialog`, push toasts,
        // and call manager.clear() -- those need access to fields the
        // closures don't own.
        if let Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position,
            pressed: false,
            ..
        }) = e
        {
            if self.btn_open_dialog.bounds().contains(*position) {
                self.show_dialog = true;
                self.dialog.show();
                set_status(status, "Dialog opened (Click OK/Cancel)");
            } else if self.btn_open_file.bounds().contains(*position) {
                self.show_file_dialog = true;
                set_status(status, "FileDialog opened (Esc closes)");
            } else if self.btn_toast_info.bounds().contains(*position) {
                self.next_toast_id += 1;
                notifs.show(
                    Notification::new(
                        format!("toast{}", self.next_toast_id),
                        "Info",
                        "Wave-3: close-button fires on PRESS (commit 5d88d3c).",
                    )
                    .with_type(NotificationType::Info)
                    .with_duration(std::time::Duration::from_secs(60)),
                );
            } else if self.btn_toast_warn.bounds().contains(*position) {
                self.next_toast_id += 1;
                notifs.show(
                    Notification::new(
                        format!("toast{}", self.next_toast_id),
                        "Warning",
                        "Hover the (x) and press: notification removes immediately.",
                    )
                    .with_type(NotificationType::Warning)
                    .with_duration(std::time::Duration::from_secs(60)),
                );
            } else if self.btn_toast_err.bounds().contains(*position) {
                self.next_toast_id += 1;
                notifs.show(
                    Notification::new(
                        format!("toast{}", self.next_toast_id),
                        "Error",
                        "Wave-3 (4e745a0): clear() fires the close callback per item.",
                    )
                    .with_type(NotificationType::Error)
                    .with_duration(std::time::Duration::from_secs(60)),
                );
            } else if self.btn_clear_toasts.bounds().contains(*position) {
                let before = *self.notifications_closed.borrow();
                notifs.clear();
                let after = *self.notifications_closed.borrow();
                set_status(
                    status,
                    format!(
                        "clear() fired {} close callbacks (wave-3 4e745a0)",
                        after - before
                    ),
                );
            }
        }
    }
}

// ---------- tab 4: Pickers (ColorPicker hex, DateTimePicker focus, Breadcrumb) ----------
//
// Verify:
//  - ColorPicker compact: open popup, type hex chars one at a time. Color
//    only updates when len==6 (wave-3 hex flicker fix; commit logged in
//    SKEPTIC_REVIEW_WAVE3 #29 if applied).
//  - DateTimePicker: click hour/min field -> picker self-focuses on click
//    (wave-2 follow-up after 5c6ad83). Type digit -> appears.
//    Plain Tab returns Ignored (wave-2 d2255cb).
//  - Breadcrumb: focused -> Left/Right moves segment cursor; Enter
//    activates.
struct PickersTab {
    label_section: Label,
    color_picker: ColorPicker,
    dtp: DateTimePicker,
    breadcrumb: Breadcrumb,
    label_color: Label,
    label_dtp: Label,
    label_breadcrumb: Label,
}

impl PickersTab {
    fn new(status: StatusMsg) -> Self {
        let s1 = status.clone();
        let s2 = status.clone();
        let s3 = status.clone();

        let color_picker = ColorPicker::new(Default::default())
            .with_color(Color::rgb(255, 128, 64))
            .with_style(ColorPickerStyle::Compact)
            .with_on_change(move |c| {
                set_status(
                    &s1,
                    format!("Color -> #{:02X}{:02X}{:02X}", c.r, c.g, c.b),
                );
            });

        let dtp = DateTimePicker::new(Default::default())
            .with_mode(DateTimePickerMode::DateTime)
            .with_on_change(move |dt| {
                set_status(&s2, format!("DateTime -> {}", dt));
            });

        let breadcrumb = Breadcrumb::new(Default::default())
            .with_items(vec![
                BreadcrumbItem::new("home", "0"),
                BreadcrumbItem::new("alex", "1"),
                BreadcrumbItem::new("EriGui", "2"),
                BreadcrumbItem::new("rust-gui", "3"),
                BreadcrumbItem::new("erigui-examples", "4"),
            ])
            .with_on_navigate(move |id, idx| {
                set_status(&s3, format!("Breadcrumb -> {} idx={}", id, idx));
            });

        Self {
            label_section: Label::new(
                Default::default(),
                "Tab 4: Pickers -- ColorPicker (hex stays stable until 6 chars), DateTimePicker, Breadcrumb (focus + Left/Right)",
            ),
            color_picker,
            dtp,
            breadcrumb,
            label_color: Label::new(
                Default::default(),
                "ColorPicker (compact; click swatch, type hex):",
            ),
            label_dtp: Label::new(
                Default::default(),
                "DateTimePicker (click hour, type digits):",
            ),
            label_breadcrumb: Label::new(
                Default::default(),
                "Breadcrumb (click to focus, then Left/Right/Enter):",
            ),
        }
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let x0 = area.x() + m.pad;
        let full_w = area.width() - m.pad * 2;
        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(x0, y, full_w, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        // ColorPicker row
        self.label_color.layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        // Compact picker — preview swatch + dropdown arrow. Square-ish.
        self.color_picker
            .layout(Rect::new(x0, y, m.row_h * 3, m.row_h), theme);
        y += m.row_h + m.row_gap;

        // DateTimePicker row
        self.label_dtp.layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        self.dtp
            .layout(Rect::new(x0, y, m.btn_w * 3, m.row_h), theme);
        y += m.row_h + m.row_gap;

        // Breadcrumb row
        self.label_breadcrumb
            .layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        self.breadcrumb
            .layout(Rect::new(x0, y, full_w, m.row_h), theme);
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.label_color.draw(ctx, theme);
        self.color_picker.draw(ctx, theme);
        self.label_dtp.draw(ctx, theme);
        self.dtp.draw(ctx, theme);
        self.label_breadcrumb.draw(ctx, theme);
        self.breadcrumb.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme) {
        // Click-to-focus for kbd-nav widgets. ColorPicker, DTP, Breadcrumb
        // all benefit from explicit focus on click so subsequent keypresses
        // route correctly (wave-2 audit findings).
        if let Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position,
            pressed: true,
            ..
        }) = e
        {
            if self.color_picker.bounds().contains(*position) {
                self.color_picker.set_focused(true);
            }
            if self.dtp.bounds().contains(*position) {
                self.dtp.set_focused(true);
            }
            if self.breadcrumb.bounds().contains(*position) {
                self.breadcrumb.set_focused(true);
            }
        }
        let _ = self.color_picker.handle_event(e, theme);
        let _ = self.dtp.handle_event(e, theme);
        let _ = self.breadcrumb.handle_event(e, theme);
    }
}

// ---------- tab 5: Containers (DockPanel auto-descent, Accordion kbd) ----------
//
// Verify:
//  - DockPanel: 4 panels added in order Center "main", Right "sidebar",
//    Bottom "console", Center "secondary" -- the LAST add triggers wave-3
//    auto-descent (commits 165feef + 5d48953). secondary should appear
//    as a tab in the main panel area, not panic and not silently no-op.
//  - Accordion: focused -> Up/Down switches panel; Enter/Space toggles
//    expansion. Plain arrows when unfocused do NOT cycle.
struct ContainersTab {
    label_section: Label,
    dock: DockPanel,
    accordion: Accordion,
    label_dock: Label,
    label_accordion: Label,
}

impl ContainersTab {
    fn new(status: StatusMsg) -> Self {
        let s1 = status.clone();
        let s2 = status.clone();

        let mut dock = DockPanel::new(Default::default())
            .with_on_layout_change(move || {
                set_status(&s1, "Dock layout changed");
            })
            .with_on_panel_close(move |id| {
                set_status(&s2, format!("Panel closed: {}", id));
            });

        // Order matters per wave-3 dock fix: Center first makes root a
        // Tabs node, then Right/Bottom create Splits, and the FINAL Center
        // exercises the auto-descent path (165feef): root is Split, so
        // descend to first leaf-Tabs and append "secondary".
        dock.add_panel(
            DockablePanel::new(
                "main",
                "main.rs",
                Box::new(Label::new(
                    Default::default(),
                    "Main editor panel (Center, added first)",
                )),
            )
            .with_can_close(true),
            DockPosition::Center,
        );
        dock.add_panel(
            DockablePanel::new(
                "sidebar",
                "Sidebar",
                Box::new(Label::new(Default::default(), "Right sidebar (Right add)")),
            )
            .with_can_close(true),
            DockPosition::Right,
        );
        dock.add_panel(
            DockablePanel::new(
                "console",
                "Console",
                Box::new(Label::new(Default::default(), "Console output (Bottom add)")),
            )
            .with_can_close(true),
            DockPosition::Bottom,
        );
        // Wave-3 auto-descent test: Center on Split-root.
        dock.add_panel(
            DockablePanel::new(
                "secondary",
                "secondary.rs",
                Box::new(Label::new(
                    Default::default(),
                    "Secondary editor (Center add to Split root -> auto-descent to first leaf-Tabs).",
                )),
            )
            .with_can_close(true),
            DockPosition::Center,
        );

        let s_acc = status;

        let mut accordion = Accordion::new(Default::default()).with_on_toggle(move |idx, open| {
            set_status(&s_acc, format!("Accordion panel {} -> open={}", idx, open));
        });
        accordion.add_panel(
            AccordionPanel::new(
                "Section 1: Wave-1 kbd nav gate",
                Box::new(Label::new(
                    Default::default(),
                    "Focused arrows cycle; unfocused arrows are ignored.",
                )),
            )
            .with_expanded(true),
        );
        accordion.add_panel(AccordionPanel::new(
            "Section 2: Up/Down moves cursor",
            Box::new(Label::new(
                Default::default(),
                "Click on the accordion header to focus, then Up/Down.",
            )),
        ));
        accordion.add_panel(AccordionPanel::new(
            "Section 3: Enter/Space toggles",
            Box::new(Label::new(
                Default::default(),
                "Pressing Enter or Space on the focused panel toggles expansion.",
            )),
        ));

        Self {
            label_section: Label::new(
                Default::default(),
                "Tab 5: Containers -- DockPanel auto-descent (4th add), Accordion gated kbd",
            ),
            dock,
            accordion,
            label_dock: Label::new(
                Default::default(),
                "DockPanel (look for 'secondary.rs' tab in main area):",
            ),
            label_accordion: Label::new(
                Default::default(),
                "Accordion (click header to focus, then Up/Down/Enter):",
            ),
        }
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(area.x() + m.pad, y, area.width() - m.pad * 2, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        // Two columns: dock (left, 60%), accordion (right, 40%)
        let col1_w = (area.width() - m.pad * 3) * 6 / 10;
        let col2_w = area.width() - m.pad * 3 - col1_w;
        let col1_x = area.x() + m.pad;
        let col2_x = col1_x + col1_w + m.pad;

        self.label_dock.layout(Rect::new(col1_x, y, col1_w, m.label_h), theme);
        self.label_accordion
            .layout(Rect::new(col2_x, y, col2_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        let col_h = area.bottom() - y - m.pad;

        self.dock.layout(Rect::new(col1_x, y, col1_w, col_h), theme);
        self.accordion
            .layout(Rect::new(col2_x, y, col2_w, col_h), theme);
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.label_dock.draw(ctx, theme);
        self.dock.draw(ctx, theme);
        self.label_accordion.draw(ctx, theme);
        self.accordion.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme) {
        // Focus Accordion on click for kbd-nav verification.
        if let Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position,
            pressed: true,
            ..
        }) = e
        {
            if self.accordion.bounds().contains(*position) {
                self.accordion.set_focused(true);
            }
        }
        let _ = self.dock.handle_event(e, theme);
        let _ = self.accordion.handle_event(e, theme);
    }
}

// ---------- tab 6: Misc (Icon alpha-zero hole fix, ProgressBar, TextArea UTF-8 copy) ----------
//
// Verify:
//  - Icon::draw_gear/palette: now take a bg_color so the inner hole is
//    actually painted (wave-3 commits #3/#4 from skeptic list, applied
//    in icon.rs by passing the surface color through).
//  - TextArea: select "档案 hello" with Shift+End and Ctrl+C; clipboard
//    contains the actual visible chars (wave-2 d3d2c62 UTF-8 fix).
//    Status bar shows "Copied: <text>".
struct MiscTab {
    label_section: Label,
    progress: ProgressBar,
    text_area: TextArea,
    label_icons: Label,
    label_progress: Label,
    label_textarea: Label,
    icons_rect: Rect,
    /// Advanced once per redraw tick so the bar visibly moves. Wraps at 100.
    progress_phase: f32,

    // Theme + color picker row.
    label_theme_section: Label,
    label_theme: Label,
    theme_combo: ComboBox,
    label_primary: Label,
    primary_picker: ColorPicker,
    reset_btn: Button,
}

impl MiscTab {
    fn new(
        _status: StatusMsg,
        theme_request: ThemeRequest,
        primary_request: ColorRequest,
        initial_theme_name: String,
        initial_primary: Color,
    ) -> Self {
        let progress = ProgressBar::new(Default::default())
            .with_range(0.0, 100.0)
            .with_value(50.0)
            .with_text_visible(true)
            .with_style(ProgressBarStyle::Solid);

        // Pre-fill with multi-script content. Hand-select Shift+End on line 1
        // and Ctrl+C, then check the StatusBar for the copied chars.
        let text_area = TextArea::new(Default::default())
            .with_text("档案 hello world\nLine 2 with éñ accents\nLine 3 ASCII only")
            .with_line_numbers(true);

        // Theme picker — every built-in theme by display name. Selection writes
        // to the shared ThemeRequest channel; main reads after handle_event and
        // rebuilds the live Theme.
        let theme_names: Vec<String> = Theme::all_named()
            .into_iter()
            .map(|t| t.name().to_string())
            .collect();
        let initial_idx = theme_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(&initial_theme_name))
            .unwrap_or(0);
        let req = theme_request.clone();
        let names_for_combo = theme_names.clone();
        let theme_combo = ComboBox::new(Default::default())
            .with_items(theme_names)
            .with_selected(initial_idx)
            .with_tooltip("Live theme switch. Same names as Ctrl+T cycle.")
            .with_on_selection_changed(move |idx, _text| {
                if let Some(i) = idx {
                    if let Some(name) = names_for_combo.get(i) {
                        *req.borrow_mut() = Some(name.clone());
                    }
                }
            });

        // Primary-color override. ColorPicker preview shows the active primary;
        // changes write the new color to the shared ColorRequest. Reset button
        // clears the override and restores the theme's default primary.
        let pick_req = primary_request.clone();
        let primary_picker = ColorPicker::new(Default::default())
            .with_color(initial_primary)
            .with_style(ColorPickerStyle::Compact)
            .with_on_change(move |c| {
                *pick_req.borrow_mut() = Some(c);
            });

        // The reset button writes Color::TRANSPARENT as a sentinel. Main
        // interprets `Some(Color::TRANSPARENT)` as "clear the primary
        // override and rebuild the theme from scratch". Any other Color
        // value is a normal override request.
        let reset_req = primary_request.clone();
        let reset_btn = Button::new(Default::default(), "Reset")
            .with_tooltip("Clear primary-color override and restore the active theme's default.")
            .with_on_click(move || {
                *reset_req.borrow_mut() = Some(Color::TRANSPARENT);
            });

        Self {
            label_section: Label::new(
                Default::default(),
                "Tab 6: Misc -- Icons (gear/palette show inner hole), ProgressBar, TextArea (UTF-8 copy)",
            ),
            progress,
            text_area,
            label_icons: Label::new(
                Default::default(),
                "Icons (wave-3: gear/palette now paint hole in surface color):",
            ),
            label_progress: Label::new(
                Default::default(),
                "ProgressBar (animates 0..100 each frame to verify redraw is alive):",
            ),
            label_textarea: Label::new(
                Default::default(),
                "TextArea: select 'archive hello' on line 1, Ctrl+C; status shows clipboard.",
            ),
            icons_rect: Rect::default(),
            progress_phase: 0.0,
            label_theme_section: Label::new(
                Default::default(),
                "Theme & color (live):",
            ),
            label_theme: Label::new(Default::default(), "Theme:"),
            theme_combo,
            label_primary: Label::new(Default::default(), "Primary:"),
            primary_picker,
            reset_btn,
        }
    }

    /// External update: keep the theme combo's selected index and the
    /// color picker's preview in sync with whatever the main loop just
    /// applied (e.g., after Ctrl+T cycled the theme outside this tab).
    fn sync_to_theme(&mut self, theme_name: &str, primary: Color) {
        let names: Vec<String> = Theme::all_named()
            .into_iter()
            .map(|t| t.name().to_string())
            .collect();
        if let Some(idx) = names.iter().position(|n| n.eq_ignore_ascii_case(theme_name)) {
            self.theme_combo.set_selected(Some(idx));
        }
        self.primary_picker.set_color(primary);
    }

    /// Advance the progress bar by ~1% per call. Called from the
    /// redraw loop so the bar visibly moves; wraps to 0 at 100.
    fn tick_progress(&mut self) {
        self.progress_phase = (self.progress_phase + 1.0) % 100.0;
        self.progress.set_value(self.progress_phase);
    }

    fn layout(&mut self, area: Rect, theme: &Theme) {
        let m = metrics(theme);
        let mut y = area.y() + m.pad;
        let x0 = area.x() + m.pad;
        let full_w = area.width() - m.pad * 2;
        let lbl_size = self
            .label_section
            .measure(&LayoutConstraints::default(), theme);
        self.label_section.layout(
            Rect::new(x0, y, full_w, lbl_size.height.max(m.label_h)),
            theme,
        );
        y += lbl_size.height.max(m.label_h) + m.row_gap;

        // Icons row: reserve a band tall enough for theme-scaled icons
        let icon_band_h = m.row_h * 2;
        self.label_icons.layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        self.icons_rect = Rect::new(x0, y, full_w, icon_band_h);
        y += icon_band_h + m.row_gap;

        self.label_progress
            .layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        self.progress
            .layout(Rect::new(x0, y, m.btn_w * 3, m.row_h), theme);
        y += m.row_h + m.row_gap;

        self.label_textarea
            .layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        // Reserve space for the theme-picker row at the bottom: section
        // label + (label_theme | combo | label_primary | picker | reset).
        let theme_row_h = m.row_h.max(m.label_h) + m.row_gap / 2 + m.row_h + m.row_gap;
        let ta_h = (area.bottom() - y - m.pad - theme_row_h).max(m.row_h * 3);
        self.text_area.layout(Rect::new(x0, y, full_w, ta_h), theme);
        y += ta_h + m.row_gap;

        // Theme & color row.
        self.label_theme_section
            .layout(Rect::new(x0, y, full_w, m.label_h), theme);
        y += m.label_h + m.row_gap / 2;
        let mut x = x0;
        let label_w_short = m.btn_w / 2;
        self.label_theme
            .layout(Rect::new(x, y, label_w_short, m.row_h), theme);
        x += label_w_short + m.btn_gap;
        let combo_w = m.btn_w * 2;
        self.theme_combo
            .layout(Rect::new(x, y, combo_w, m.row_h), theme);
        x += combo_w + m.btn_gap * 2;
        self.label_primary
            .layout(Rect::new(x, y, label_w_short, m.row_h), theme);
        x += label_w_short + m.btn_gap;
        let pick_w = m.btn_w;
        self.primary_picker
            .layout(Rect::new(x, y, pick_w, m.row_h), theme);
        x += pick_w + m.btn_gap;
        self.reset_btn
            .layout(Rect::new(x, y, m.btn_w, m.row_h), theme);
    }

    fn draw(&self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.label_section.draw(ctx, theme);
        self.label_icons.draw(ctx, theme);

        // Manually paint the icons. Pass the surface color as bg_color so
        // the wave-3 hole-fix is observable.
        let bg = theme.colors.surface;
        let fg = theme.colors.text;
        let r = self.icons_rect;
        // Three sizes of gear, side by side, then palette.
        let sizes = [32, 48, 60];
        let mut x = r.x();
        for size in &sizes {
            let icon_rect = Rect::new(x, r.y() + (r.height() - size) / 2, *size, *size);
            Icon::draw_gear(ctx, icon_rect, fg, bg);
            x += size + 12;
        }
        // Palette
        let pal_rect = Rect::new(x, r.y() + (r.height() - 48) / 2, 64, 48);
        Icon::draw_palette(ctx, pal_rect, Color::rgb(120, 200, 90), bg);

        self.label_progress.draw(ctx, theme);
        self.progress.draw(ctx, theme);
        self.label_textarea.draw(ctx, theme);
        self.text_area.draw(ctx, theme);

        // Theme & color row. Combo's open dropdown is drawn last so it
        // overlays everything below.
        self.label_theme_section.draw(ctx, theme);
        self.label_theme.draw(ctx, theme);
        self.label_primary.draw(ctx, theme);
        self.primary_picker.draw(ctx, theme);
        self.reset_btn.draw(ctx, theme);
        // ComboBox last so its dropdown popup paints over the picker / button.
        self.theme_combo.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme, status: &StatusMsg) {
        // Focus the text_area on click so kbd events route to it.
        if let Event::MouseButton(MouseButtonEvent {
            button: MouseButton::Left,
            position,
            pressed: true,
            ..
        }) = e
        {
            if self.text_area.bounds().contains(*position) {
                self.text_area.set_focused(true);
            } else {
                self.text_area.set_focused(false);
            }
        }
        let _ = self.text_area.handle_event(e, theme);

        // Theme picker row. ComboBox + ColorPicker + Reset button each get
        // every event; their callbacks write into the shared request slots,
        // which the main loop drains.
        let _ = self.theme_combo.handle_event(e, theme);
        let _ = self.primary_picker.handle_event(e, theme);
        let _ = self.reset_btn.handle_event(e, theme);

        // After handling, check whether a Ctrl+C just landed by inspecting
        // the in-process clipboard. Update status if it changed.
        if let Event::KeyPress(KeyPressEvent { key, modifiers, .. }) = e {
            if (matches!(key, Key::C) || matches!(key, Key::X))
                && modifiers.contains(Modifiers::CTRL)
            {
                let cb = get_clipboard();
                if !cb.is_empty() {
                    set_status(status, format!("Clipboard: {:?}", cb));
                }
            }
        }
    }
}

// ---------- top-level app ----------
enum ActiveTab {
    Basic,
    Selection,
    Modals,
    Pickers,
    Containers,
    Misc,
}

struct App {
    tabs: TabControl,
    status_bar: StatusBar,
    notifications: NotificationManager,
    notif_close_count: Rc<RefCell<u32>>,
    status_msg: StatusMsg,

    basic: BasicInputsTab,
    selection: SelectionTab,
    modals: ModalsTab,
    pickers: PickersTab,
    containers: ContainersTab,
    misc: MiscTab,

    // Theme & primary-color picker write into these from MiscTab; main reads
    // them after every event pass.
    pub theme_request: ThemeRequest,
    pub primary_request: ColorRequest,

    // Persisted across ticks for the StatusBar refresh and idle redraw.
    viewport: Size,
}

impl App {
    fn new(viewport: Size, initial_theme_name: String, initial_primary: Color) -> Self {
        let status_msg = make_status_msg();
        let theme_request: ThemeRequest = Rc::new(RefCell::new(None));
        let primary_request: ColorRequest = Rc::new(RefCell::new(None));
        let mut widget_mgr = WidgetManager::new();

        let mut tabs = TabControl::new(Default::default()).with_on_tab_changed({
            let s = status_msg.clone();
            move |i| set_status(&s, format!("Active tab -> {}", i))
        });
        tabs.add_tab(TabPage::new("Basic Inputs"));
        tabs.add_tab(TabPage::new("Selection"));
        tabs.add_tab(TabPage::new("Modals"));
        tabs.add_tab(TabPage::new("Pickers"));
        tabs.add_tab(TabPage::new("Containers"));
        tabs.add_tab(TabPage::new("Misc"));

        let mut status_bar = StatusBar::new(Default::default())
            .with_separator_style(SeparatorStyle::Sunken);
        status_bar.add_panel(StatusPanel::new("Ready").with_spring_width());
        status_bar.add_panel(
            StatusPanel::new("Tab: 1/6")
                .with_fixed_width(120)
                .with_alignment(TextAlignment::Right),
        );

        let notif_close_count = Rc::new(RefCell::new(0u32));
        let count_clone = notif_close_count.clone();
        let s_close = status_msg.clone();
        let notifications = NotificationManager::new(Default::default())
            .with_position(NotificationPosition::TopRight)
            .with_max_visible(5)
            .with_on_notification_closed(move |id| {
                *count_clone.borrow_mut() += 1;
                set_status(&s_close, format!("Notification closed: {}", id));
            });

        let basic = BasicInputsTab::new(status_msg.clone());
        let selection = SelectionTab::new(status_msg.clone(), &mut widget_mgr);
        let mut modals = ModalsTab::new(status_msg.clone());
        modals.notifications_closed = notif_close_count.clone();
        let pickers = PickersTab::new(status_msg.clone());
        let containers = ContainersTab::new(status_msg.clone());
        let misc = MiscTab::new(
            status_msg.clone(),
            theme_request.clone(),
            primary_request.clone(),
            initial_theme_name,
            initial_primary,
        );

        // widget_mgr only used to mint unique RadioButton ids; not needed
        // post-construction (radios own their own state). Drop it.
        drop(widget_mgr);

        Self {
            tabs,
            status_bar,
            notifications,
            notif_close_count,
            status_msg,
            basic,
            selection,
            modals,
            pickers,
            containers,
            misc,
            theme_request,
            primary_request,
            viewport,
        }
    }

    fn active(&self) -> ActiveTab {
        match self.tabs.active_tab() {
            0 => ActiveTab::Basic,
            1 => ActiveTab::Selection,
            2 => ActiveTab::Modals,
            3 => ActiveTab::Pickers,
            4 => ActiveTab::Containers,
            _ => ActiveTab::Misc,
        }
    }

    fn layout(&mut self, viewport: Size, theme: &Theme) {
        self.viewport = viewport;
        // TabControl auto-scales its tab strip in layout(theme); mirror the
        // formula here so we can compute the content area's y-offset
        // before the tabs widget exposes its own value publicly.
        let tab_h = theme.typography.font_size_base + theme.spacing.padding.top.max(8) * 2;
        let status_h = (theme.typography.font_size_base + theme.spacing.padding.top.max(8)).max(24);
        let full = Rect::new(0, 0, viewport.width, viewport.height);

        // Single layout pass — TabControl owns its full vertical extent
        // (tab strip + body), then the host paints content directly into
        // the body region.
        let real_tabs_h = (viewport.height - status_h).max(tab_h + m_min_body(theme));
        self.tabs
            .layout(Rect::new(0, 0, full.width(), real_tabs_h), theme);

        // Content area = everything below the tab strip and above the status bar.
        let content = Rect::new(
            0,
            tab_h,
            viewport.width,
            (viewport.height - tab_h - status_h).max(m_min_body(theme)),
        );

        match self.active() {
            ActiveTab::Basic => self.basic.layout(content, theme),
            ActiveTab::Selection => self.selection.layout(content, theme),
            ActiveTab::Modals => self.modals.layout(content, theme),
            ActiveTab::Pickers => self.pickers.layout(content, theme),
            ActiveTab::Containers => self.containers.layout(content, theme),
            ActiveTab::Misc => self.misc.layout(content, theme),
        }

        // Status bar at the bottom.
        self.status_bar.layout(
            Rect::new(0, viewport.height - status_h, viewport.width, status_h),
            theme,
        );

        // Notification overlay covers full viewport (so manager can place
        // them in TopRight relative to the window).
        self.notifications.layout(full, theme);
    }

    fn refresh_status_bar(&mut self) {
        let msg = self.status_msg.borrow().clone();
        self.status_bar.set_panel_text(0, msg);
        let tab_label = format!("Tab: {}/{}", self.tabs.active_tab() + 1, 6);
        self.status_bar.set_panel_text(1, tab_label);
    }

    fn draw(&mut self, ctx: &mut dyn DrawContext, theme: &Theme) {
        self.refresh_status_bar();

        self.tabs.draw(ctx, theme);

        match self.active() {
            ActiveTab::Basic => self.basic.draw(ctx, theme),
            ActiveTab::Selection => self.selection.draw(ctx, theme),
            ActiveTab::Modals => self.modals.draw(ctx, theme),
            ActiveTab::Pickers => self.pickers.draw(ctx, theme),
            ActiveTab::Containers => self.containers.draw(ctx, theme),
            ActiveTab::Misc => self.misc.draw(ctx, theme),
        }

        self.status_bar.draw(ctx, theme);
        // Notifications drawn LAST so they overlay all tab content.
        self.notifications.draw(ctx, theme);
    }

    fn handle_event(&mut self, e: &Event, theme: &Theme) {
        // Let the TabControl see global Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+1..9.
        let _ = self.tabs.handle_event(e, theme);

        // Notifications' close-button gets first dibs so a press inside a
        // toast fires the close (wave-3 5d88d3c) and doesn't fall through
        // to the underlying tab's button.
        let notif_consumed = self.notifications.handle_event(e, theme) == EventResult::Consumed;
        if notif_consumed {
            return;
        }

        match self.active() {
            ActiveTab::Basic => self.basic.handle_event(e, theme),
            ActiveTab::Selection => self.selection.handle_event(e, theme),
            ActiveTab::Modals => self
                .modals
                .handle_event(e, theme, &self.status_msg, &mut self.notifications),
            ActiveTab::Pickers => self.pickers.handle_event(e, theme),
            ActiveTab::Containers => self.containers.handle_event(e, theme),
            ActiveTab::Misc => self.misc.handle_event(e, theme, &self.status_msg),
        }

        // The close-callback writes notif_close_count via Rc; mirror to the
        // ModalsTab so its "Clear All" status string can show the delta.
        // (Alternatively, we already share the Rc.)
        let _ = &self.notif_close_count;
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    let (win_w, win_h, ui_scale) = match event_loop.primary_monitor() {
        Some(m) => {
            let scale = m.scale_factor().max(1.0);
            let phys = m.size();
            let w = ((phys.width as f64) * 0.85 / scale) as i32;
            let h = ((phys.height as f64) * 0.85 / scale) as i32;
            (w.max(1100), h.max(720), scale as f32)
        }
        None => (1400, 900, 1.0_f32),
    };

    let mut renderer =
        Renderer::new(&event_loop, win_w, win_h, "EriGui Kitchen Sink (wave-2/3 verify)")?;

    // ERIGUI_THEME=<name> picks a starting theme by name (case-insensitive
    // via Theme::by_name). Default is Dark. Ctrl+T cycles through every
    // built-in theme at runtime so all 10 can be previewed in one session.
    let theme_names: Vec<String> = Theme::all_named()
        .into_iter()
        .map(|t| t.name().to_string())
        .collect();
    let initial_name = std::env::var("ERIGUI_THEME").ok();
    let mut theme_idx: usize = initial_name
        .as_deref()
        .and_then(|n| {
            let n_lower = n.trim().to_ascii_lowercase();
            theme_names
                .iter()
                .position(|tn| tn.to_ascii_lowercase() == n_lower)
        })
        .unwrap_or_else(|| {
            theme_names
                .iter()
                .position(|n| n == "Dark")
                .unwrap_or(0)
        });
    let mut theme = Theme::by_name(&theme_names[theme_idx])
        .unwrap_or_else(Theme::dark)
        .with_scale(ui_scale);
    // Active primary-color override (None = use theme's default).
    let mut primary_override: Option<Color> = None;
    let apply_primary_override = |theme: &mut Theme, color: Color| {
        // Mirror the primary slot AND the hover/active variants so the
        // override is consistent across button states. derive_hover/active
        // shifts the lightness slightly; cheap approximation here.
        theme.colors.primary = color;
        theme.colors.primary_hover = color.with_alpha(255);
        theme.colors.primary_active = color.with_alpha(255);
        theme.colors.border_focus = color;
        theme.colors.selection = Color::rgba(color.r, color.g, color.b, 100);
    };
    eprintln!(
        "[theme] starting with {} (Ctrl+T cycles, picker in tab 6)",
        theme_names[theme_idx]
    );
    clear_clipboard();

    let viewport = renderer.viewport_size();
    let mut app = App::new(
        viewport,
        theme_names[theme_idx].clone(),
        theme.colors.primary,
    );
    app.layout(viewport, &theme);

    // Persistent translator — caches last_cursor across calls so MouseInput
    // events get the actual click position. The free `convert_window_event`
    // helper instantiates a fresh EventTranslator per call, which silently
    // reports every click at (0, 0) and is the reason this demo's clicks
    // weren't reaching widgets at all in the previous build.
    let mut translator = EventTranslator::new(viewport);

    event_loop.run(move |event, elwt| {
        // Poll so the per-widget tooltip 500ms timers tick forward even when
        // the cursor is sitting still (the same trick tooltip_demo.rs uses).
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        let new_size = Size::new(
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
                        translator.set_window_size(new_size);
                        app.layout(new_size, &theme);
                        renderer.window().request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        // Wayland delivers RedrawRequested after we call
                        // request_redraw(); paint here. This is the canonical
                        // spot for the GL frame, mirroring erigui-app.
                        app.notifications.update_animations(1.0 / 60.0);
                        // Advance the Tab 6 ProgressBar so it visibly moves
                        // per frame instead of being stuck at its initial
                        // value. Verifies the redraw cycle is alive.
                        app.misc.tick_progress();
                        app.layout(renderer.viewport_size(), &theme);
                        renderer.begin_frame(theme.colors.background);
                        app.draw(&mut renderer, &theme);
                        renderer.end_frame();
                    }
                    _ => {}
                }

                if let Some(gui_event) = translator.translate(&event) {
                    // Ctrl+T cycles to the next built-in theme. Intercept
                    // BEFORE the app sees the event so widgets don't get
                    // a stray "T" keypress while we're swapping the theme
                    // out from under them. The next layout pass picks up
                    // the new theme.
                    let mut consumed_for_theme = false;
                    if let Event::KeyPress(KeyPressEvent { key, modifiers, .. }) = &gui_event {
                        if matches!(key, Key::T) && modifiers.contains(Modifiers::CTRL) {
                            theme_idx = (theme_idx + 1) % theme_names.len();
                            theme = Theme::by_name(&theme_names[theme_idx])
                                .unwrap_or_else(Theme::dark)
                                .with_scale(ui_scale);
                            if let Some(c) = primary_override {
                                apply_primary_override(&mut theme, c);
                            }
                            app.misc.sync_to_theme(
                                &theme_names[theme_idx],
                                theme.colors.primary,
                            );
                            eprintln!("[theme] {}", theme_names[theme_idx]);
                            consumed_for_theme = true;
                        }
                    }
                    if !consumed_for_theme {
                        app.handle_event(&gui_event, &theme);
                    }

                    // Drain MiscTab's theme-picker requests. The combo writes
                    // a theme name; the color picker writes Some(Color); the
                    // Reset button writes the sentinel Color::TRANSPARENT to
                    // mean "clear the override".
                    if let Some(req) = app.theme_request.borrow_mut().take() {
                        if let Some(idx) = theme_names
                            .iter()
                            .position(|n| n.eq_ignore_ascii_case(&req))
                        {
                            theme_idx = idx;
                            theme = Theme::by_name(&theme_names[theme_idx])
                                .unwrap_or_else(Theme::dark)
                                .with_scale(ui_scale);
                            if let Some(c) = primary_override {
                                apply_primary_override(&mut theme, c);
                            }
                            app.misc.sync_to_theme(
                                &theme_names[theme_idx],
                                theme.colors.primary,
                            );
                            eprintln!("[theme] {}", theme_names[theme_idx]);
                        }
                    }
                    if let Some(req) = app.primary_request.borrow_mut().take() {
                        if req == Color::TRANSPARENT {
                            // Sentinel = reset.
                            primary_override = None;
                            theme = Theme::by_name(&theme_names[theme_idx])
                                .unwrap_or_else(Theme::dark)
                                .with_scale(ui_scale);
                            app.misc.sync_to_theme(
                                &theme_names[theme_idx],
                                theme.colors.primary,
                            );
                            eprintln!("[theme] reset primary override");
                        } else {
                            primary_override = Some(req);
                            apply_primary_override(&mut theme, req);
                            eprintln!(
                                "[theme] primary -> ({}, {}, {})",
                                req.r, req.g, req.b
                            );
                        }
                    }

                    // After every event, push an animation tick to the
                    // notification manager so slide-in/out animations
                    // settle. 16ms ~= 60Hz frame budget.
                    app.notifications.update_animations(1.0 / 60.0);
                    app.layout(renderer.viewport_size(), &theme);
                    // Without this, on Wayland the state mutation (e.g.
                    // checkbox toggled, text typed) is not visible until
                    // the NEXT incidental event triggers a paint.
                    renderer.window().request_redraw();
                }
            }
            WinitEvent::AboutToWait => {
                // Idle tick so tooltip timers + notification animations
                // advance even with no input. Triggers a redraw via the
                // RedrawRequested path so paint logic stays in one place.
                app.notifications.update_animations(1.0 / 60.0);
                renderer.window().request_redraw();
            }
            _ => {}
        }
    })?;
    Ok(())
}


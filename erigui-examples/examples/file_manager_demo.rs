use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use image::imageops::FilterType;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::SystemTime;

static VIEW_MODE_CMD: AtomicI32 = AtomicI32::new(-1);
static FILE_CMD: AtomicI32 = AtomicI32::new(-1);
const FILE_CMD_OPEN: i32 = 0;

fn load_png_icon(name: &str) -> Option<ButtonIcon> {
    let path = Path::new("assets/icons32").join(name);
    if !path.exists() {
        return None;
    }
    let mut img = image::open(path).ok()?.to_rgba8();

    // Force the icon pixels to white while keeping the source alpha so they read on the blue buttons.
    for p in img.pixels_mut() {
        let alpha = p[3];
        p[0] = 255;
        p[1] = 255;
        p[2] = 255;
        p[3] = alpha;
    }

    let img = image::imageops::resize(&img, 28, 28, FilterType::Lanczos3);
    let (w, h) = img.dimensions();
    Some(ButtonIcon::Bitmap {
        width: w as i32,
        height: h as i32,
        data: img.into_raw(),
    })
}

fn set_view_icons() {
    VIEW_MODE_CMD.store(0, Ordering::SeqCst);
}

fn draw_link_icon(context: &mut dyn DrawContext, rect: Rect, color: Color) {
    let inset = 6;
    let chain_width = rect.width() - inset * 2;
    let chain_height = rect.height() - inset * 2;
    let left = Rect::new(
        rect.x() + inset,
        rect.y() + inset,
        chain_width / 2,
        chain_height,
    );
    let right = Rect::new(
        rect.x() + inset + chain_width / 2,
        rect.y() + inset,
        chain_width / 2,
        chain_height,
    );

    context.set_color(color.with_alpha(200));
    context.draw_rounded_rect(left, 6);
    context.draw_rounded_rect(right, 6);

    let connector_y = rect.y() + rect.height() / 2;
    context.draw_line(
        Point::new(left.right() - 4, connector_y),
        Point::new(right.x() + 4, connector_y),
        2,
    );
}

fn set_view_list() {
    VIEW_MODE_CMD.store(1, Ordering::SeqCst);
}

fn set_view_details() {
    VIEW_MODE_CMD.store(2, Ordering::SeqCst);
}

fn trigger_open_command() {
    FILE_CMD.store(FILE_CMD_OPEN, Ordering::SeqCst);
}
use winit::{
    dpi::PhysicalPosition,
    event::{Event as WinitEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
};

struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
    selected: bool,
}

#[derive(Clone, Copy)]
enum ClosePromptAction {
    Save,
    DontSave,
    Cancel,
}

#[derive(Clone)]
struct PromptButton {
    rect: Rect,
    label: &'static str,
    action: ClosePromptAction,
}

#[derive(Clone, Copy)]
enum EditorAction {
    None,
    Close,
}

struct EditorState {
    text_area: TextArea,
    path: PathBuf,
    status: String,
    clipboard: String,
    dirty: bool,
    save_button: Rect,
    close_button: Rect,
    show_close_prompt: bool,
    prompt_buttons: Vec<PromptButton>,
    show_context_menu: bool,
    context_menu_rect: Rect,
    context_menu_hovered: Option<usize>,
    context_menu_items: Vec<EditorContextMenuItem>,
}

impl EditorState {
    fn new(path: PathBuf, content: String) -> Self {
        let text_area = TextArea::new(WidgetId::default())
            .with_line_numbers(true)
            .with_word_wrap(false);
        let mut state = Self {
            text_area,
            path,
            status: "Editing".to_string(),
            clipboard: String::new(),
            dirty: false,
            save_button: Rect::new(0, 0, 0, 0),
            close_button: Rect::new(0, 0, 0, 0),
            show_close_prompt: false,
            prompt_buttons: Vec::new(),
            show_context_menu: false,
            context_menu_rect: Rect::new(0, 0, 0, 0),
            context_menu_hovered: None,
            context_menu_items: Vec::new(),
        };
        state.text_area.set_text(&content);
        state.text_area.set_focused(true);
        state
    }

    fn layout_components(&mut self, panel_rect: Rect, theme: &Theme) {
        let padding = 16;
        let toolbar_height = 42;
        let text_rect = Rect::new(
            panel_rect.x() + padding,
            panel_rect.y() + padding + toolbar_height,
            panel_rect.width() - padding * 2,
            panel_rect.height() - toolbar_height - padding * 2,
        );
        self.text_area.layout(text_rect, theme);

        let button_width = 110;
        let button_height = 26;
        let spacing = 10;
        self.save_button = Rect::new(
            panel_rect.right() - padding - button_width * 2 - spacing,
            panel_rect.y() + padding / 2,
            button_width,
            button_height,
        );
        self.close_button = Rect::new(
            panel_rect.right() - padding - button_width,
            panel_rect.y() + padding / 2,
            button_width,
            button_height,
        );
    }

    fn draw_button(&self, context: &mut dyn DrawContext, theme: &Theme, rect: Rect, label: &str) {
        context.set_color(theme.colors.primary);
        context.fill_rounded_rect(rect, 4);
        context.set_color(theme.colors.border);
        context.draw_rounded_rect(rect, 4);
        context.set_color(theme.colors.surface);
        let text_pos = Point::new(
            rect.x() + 10,
            rect.center().y - (theme.typography.font_size_base / 2),
        );
        context.draw_text(label, text_pos, theme.typography.font_size_base);
    }

    fn draw_close_prompt(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        let prompt_width = 320;
        let prompt_height = 150;
        let prompt_rect = Rect::new(
            bounds.x() + (bounds.width() - prompt_width) / 2,
            bounds.y() + (bounds.height() - prompt_height) / 2,
            prompt_width,
            prompt_height,
        );
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(prompt_rect);
        context.set_color(theme.colors.border);
        context.draw_rect(prompt_rect);

        let message = "Save changes before closing?";
        context.set_color(theme.colors.text);
        context.draw_text(
            message,
            Point::new(prompt_rect.x() + 16, prompt_rect.y() + 20),
            theme.typography.font_size_base,
        );

        let button_width = 80;
        let button_height = 26;
        let spacing = 12;
        let start_x =
            prompt_rect.x() + (prompt_rect.width() - (button_width * 3 + spacing * 2)) / 2;
        let y = prompt_rect.y() + prompt_height - button_height - 16;

        self.prompt_buttons = vec![
            PromptButton {
                rect: Rect::new(start_x, y, button_width, button_height),
                label: "Save",
                action: ClosePromptAction::Save,
            },
            PromptButton {
                rect: Rect::new(
                    start_x + button_width + spacing,
                    y,
                    button_width,
                    button_height,
                ),
                label: "Don't Save",
                action: ClosePromptAction::DontSave,
            },
            PromptButton {
                rect: Rect::new(
                    start_x + (button_width + spacing) * 2,
                    y,
                    button_width,
                    button_height,
                ),
                label: "Cancel",
                action: ClosePromptAction::Cancel,
            },
        ];

        for button in &self.prompt_buttons {
            context.set_color(theme.colors.primary_active);
            context.fill_rounded_rect(button.rect, 4);
            context.set_color(theme.colors.border);
            context.draw_rounded_rect(button.rect, 4);
            context.set_color(theme.colors.surface);
            let text_pos = Point::new(
                button.rect.x() + 8,
                button.rect.center().y - (theme.typography.font_size_base / 2),
            );
            context.draw_text(button.label, text_pos, theme.typography.font_size_base);
        }
    }

    fn show_context_menu_at(&mut self, origin: Point) {
        const CONTEXT_ACTIONS: &[(EditorContextAction, &str)] = &[
            (EditorContextAction::Undo, "Undo"),
            (EditorContextAction::Redo, "Redo"),
            (EditorContextAction::Cut, "Cut"),
            (EditorContextAction::Copy, "Copy"),
            (EditorContextAction::Paste, "Paste"),
            (EditorContextAction::Save, "Save"),
            (EditorContextAction::Close, "Close Editor"),
        ];

        let bounds = self.text_area.bounds();
        let menu_width = 180;
        let item_height = 26;
        let padding = 6;
        let total_height = item_height * CONTEXT_ACTIONS.len() as i32 + padding * 2;

        let mut x = origin.x;
        let mut y = origin.y;
        if x + menu_width > bounds.right() {
            x = bounds.right() - menu_width;
        }
        if y + total_height > bounds.bottom() {
            y = bounds.bottom() - total_height;
        }
        x = x.max(bounds.x());
        y = y.max(bounds.y());

        self.context_menu_rect = Rect::new(x, y, menu_width, total_height);
        self.context_menu_items.clear();
        let mut item_y = y + padding;
        for (action, label) in CONTEXT_ACTIONS {
            let rect = Rect::new(x + padding, item_y, menu_width - padding * 2, item_height);
            self.context_menu_items.push(EditorContextMenuItem {
                rect,
                label,
                action: *action,
            });
            item_y += item_height;
        }
        self.show_context_menu = true;
        self.context_menu_hovered = None;
    }

    fn hide_context_menu(&mut self) {
        self.show_context_menu = false;
        self.context_menu_hovered = None;
    }

    fn draw_context_menu(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.show_context_menu {
            return;
        }

        context.set_color(theme.colors.surface_variant);
        context.fill_rounded_rect(self.context_menu_rect, 4);
        context.set_color(theme.colors.border);
        context.draw_rounded_rect(self.context_menu_rect, 4);

        for (idx, item) in self.context_menu_items.iter().enumerate() {
            let hovered = self.context_menu_hovered == Some(idx);
            context.set_color(if hovered {
                theme.colors.primary.with_alpha(180)
            } else {
                theme.colors.surface
            });
            context.fill_rounded_rect(item.rect, 3);
            context.set_color(if hovered {
                theme.colors.surface
            } else {
                theme.colors.text
            });
            context.draw_text(
                item.label,
                Point::new(
                    item.rect.x() + 8,
                    item.rect.y() + item.rect.height() / 2 + 4,
                ),
                theme.typography.font_size_base - 1,
            );
        }
    }

    fn context_menu_hit(&self, pos: Point) -> Option<usize> {
        if !self.show_context_menu {
            return None;
        }
        self.context_menu_items
            .iter()
            .enumerate()
            .find(|(_, item)| item.rect.contains(pos))
            .map(|(idx, _)| idx)
    }

    fn handle_context_action(&mut self, action: EditorContextAction) -> EditorAction {
        match action {
            EditorContextAction::Undo => {
                self.text_area.undo();
                self.status = "Undo".to_string();
                self.dirty = true;
                EditorAction::None
            }
            EditorContextAction::Redo => {
                self.text_area.redo();
                self.status = "Redo".to_string();
                self.dirty = true;
                EditorAction::None
            }
            EditorContextAction::Cut => {
                if let Some(text) = self.text_area.cut_selection() {
                    self.clipboard = text;
                    self.status = "Cut".to_string();
                    self.mark_dirty();
                }
                EditorAction::None
            }
            EditorContextAction::Copy => {
                if let Some(text) = self.text_area.copy_selection() {
                    self.clipboard = text;
                    self.status = "Copied".to_string();
                }
                EditorAction::None
            }
            EditorContextAction::Paste => {
                if !self.clipboard.is_empty() {
                    self.text_area.paste_text(&self.clipboard);
                    self.status = "Pasted".to_string();
                    self.mark_dirty();
                }
                EditorAction::None
            }
            EditorContextAction::Save => {
                self.save();
                EditorAction::None
            }
            EditorContextAction::Close => self.handle_close_request(),
        }
    }

    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            self.status = "Edited".to_string();
        }
    }

    fn handle_mouse_button(
        &mut self,
        button: MouseButton,
        pressed: bool,
        position: Point,
        theme: &Theme,
    ) -> EditorAction {
        if button == MouseButton::Right {
            if pressed && !self.show_close_prompt {
                if self.text_area.bounds().contains(position) {
                    self.show_context_menu_at(position);
                } else {
                    self.hide_context_menu();
                }
            }
            return EditorAction::None;
        } else if button != MouseButton::Left {
            let event = Event::MouseButton(MouseButtonEvent {
                button,
                position,
                pressed,
                modifiers: Modifiers::empty(),
            });
            self.text_area.handle_event(&event, theme);
            return EditorAction::None;
        }

        if self.show_context_menu {
            if pressed {
                if let Some(index) = self.context_menu_hit(position) {
                    let action = self.context_menu_items[index].action;
                    self.hide_context_menu();
                    let result = self.handle_context_action(action);
                    return result;
                } else if !self.context_menu_rect.contains(position) {
                    self.hide_context_menu();
                }
            }
        }

        if self.show_close_prompt {
            if pressed {
                for prompt in &self.prompt_buttons {
                    if prompt.rect.contains(position) {
                        match prompt.action {
                            ClosePromptAction::Save => {
                                self.hide_context_menu();
                                self.save();
                                self.show_close_prompt = false;
                                return EditorAction::Close;
                            }
                            ClosePromptAction::DontSave => {
                                self.hide_context_menu();
                                self.show_close_prompt = false;
                                return EditorAction::Close;
                            }
                            ClosePromptAction::Cancel => {
                                self.show_close_prompt = false;
                                return EditorAction::None;
                            }
                        }
                    }
                }
            }
            return EditorAction::None;
        }

        if pressed {
            if self.save_button.contains(position) {
                self.hide_context_menu();
                self.save();
                return EditorAction::None;
            }

            if self.close_button.contains(position) {
                self.hide_context_menu();
                if self.dirty {
                    self.show_close_prompt = true;
                } else {
                    return EditorAction::Close;
                }
                return EditorAction::None;
            }
        }

        let event = Event::MouseButton(MouseButtonEvent {
            button,
            position,
            pressed,
            modifiers: Modifiers::empty(),
        });
        self.text_area.handle_event(&event, theme);
        EditorAction::None
    }

    fn draw(&mut self, context: &mut dyn DrawContext, theme: &Theme, window_bounds: Rect) {
        context.set_color(Color::rgba(5, 8, 15, 220));
        context.fill_rect(window_bounds);

        let panel_width = (window_bounds.width() as f32 * 0.75) as i32;
        let panel_height = (window_bounds.height() as f32 * 0.75) as i32;
        let panel_rect = Rect::new(
            window_bounds.x() + (window_bounds.width() - panel_width) / 2,
            window_bounds.y() + (window_bounds.height() - panel_height) / 2,
            panel_width,
            panel_height,
        );

        context.set_color(theme.colors.surface);
        context.fill_rect(panel_rect);
        context.set_color(theme.colors.border);
        context.draw_rect(panel_rect);

        self.layout_components(panel_rect, theme);
        self.text_area.draw(context, theme);

        let title = format!("Editing: {}", self.path.display());
        context.set_color(theme.colors.text);
        context.draw_text(
            &title,
            Point::new(panel_rect.x() + 16, panel_rect.y() + 20),
            theme.typography.font_size_large,
        );

        self.draw_button(
            context,
            theme,
            self.save_button,
            if self.dirty { "Save *" } else { "Save" },
        );
        self.draw_button(context, theme, self.close_button, "Close");

        let hint = if self.dirty {
            "Ctrl+S to save, Esc to close"
        } else {
            "Esc to close"
        };
        context.draw_text(
            hint,
            Point::new(panel_rect.x() + 16, panel_rect.y() + panel_height - 16),
            theme.typography.font_size_small,
        );
        context.draw_text(
            &self.status,
            Point::new(panel_rect.x() + 16, panel_rect.y() + panel_height - 32),
            theme.typography.font_size_small,
        );

        if self.show_context_menu {
            self.draw_context_menu(context, theme);
        }

        if self.show_close_prompt {
            self.draw_close_prompt(context, theme, window_bounds);
        }
    }

    fn handle_pointer_move(&mut self, position: Point) {
        if self.show_context_menu {
            self.context_menu_hovered = self.context_menu_hit(position);
        }
    }

    fn forward_event(&mut self, event: &Event, theme: &Theme) -> bool {
        let consumed = self.text_area.handle_event(event, theme).is_consumed();
        match event {
            Event::TextInput(_) => self.mark_dirty(),
            Event::KeyPress(key_event) => {
                if matches!(
                    key_event.key,
                    Key::Backspace | Key::Delete | Key::Enter | Key::Tab
                ) {
                    self.mark_dirty();
                }
            }
            _ => {}
        }
        consumed
    }

    fn handle_shortcut_key(&mut self, key: &Key, modifiers: Modifiers) -> (bool, bool) {
        if !modifiers.contains(Modifiers::CTRL) {
            return (false, false);
        }
        match key {
            Key::Character('z') => {
                self.text_area.undo();
                self.status = "Undo".to_string();
                self.dirty = true;
                (true, false)
            }
            Key::Character('y') => {
                self.text_area.redo();
                self.status = "Redo".to_string();
                self.dirty = true;
                (true, false)
            }
            Key::Character('c') => {
                if let Some(text) = self.text_area.copy_selection() {
                    self.clipboard = text;
                    self.status = "Copied".to_string();
                }
                (true, false)
            }
            Key::Character('x') => {
                if let Some(text) = self.text_area.cut_selection() {
                    self.clipboard = text;
                    self.status = "Cut".to_string();
                    self.mark_dirty();
                }
                (true, false)
            }
            Key::Character('v') => {
                if !self.clipboard.is_empty() {
                    self.text_area.paste_text(&self.clipboard);
                    self.status = "Pasted".to_string();
                    self.mark_dirty();
                }
                (true, false)
            }
            Key::Character('s') => {
                self.save();
                (true, false)
            }
            Key::Character('w') => {
                let should_close = matches!(self.handle_close_request(), EditorAction::Close);
                (true, should_close)
            }
            _ => (false, false),
        }
    }

    fn handle_text_input(&mut self, text: String, theme: &Theme) -> bool {
        let event = Event::TextInput(TextInputEvent { text });
        self.mark_dirty();
        self.forward_event(&event, theme)
    }

    fn handle_mouse_wheel(&mut self, delta: Point, position: Point, theme: &Theme) -> bool {
        if self.show_close_prompt {
            return true;
        }
        let event = Event::MouseWheel(MouseWheelEvent {
            delta,
            position,
            modifiers: Modifiers::empty(),
        });
        self.forward_event(&event, theme)
    }

    fn save(&mut self) {
        match fs::write(&self.path, self.text_area.get_text()) {
            Ok(_) => self.status = "Saved".to_string(),
            Err(err) => self.status = format!("Save failed: {}", err),
        }
        self.dirty = false;
        self.show_close_prompt = false;
    }

    fn handle_close_request(&mut self) -> EditorAction {
        if self.show_close_prompt {
            self.show_close_prompt = false;
            EditorAction::None
        } else if self.dirty {
            self.hide_context_menu();
            self.show_close_prompt = true;
            EditorAction::None
        } else {
            EditorAction::Close
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LinkTargetKind {
    Url,
    Path,
}

struct LinkEntrySeed {
    label: &'static str,
    target: &'static str,
    kind: LinkTargetKind,
}

struct LinkCategorySeed {
    title: &'static str,
    entries: &'static [LinkEntrySeed],
}

const LINK_LIBRARY: &[LinkCategorySeed] = &[
    LinkCategorySeed {
        title: "Text Encoders",
        entries: &[
            LinkEntrySeed {
                label: "gwen_2.5_vl_7b_fp8_scaled.safetensors",
                target: "https://github.com/itsibitzi/foundry/releases/download/latest/gwen_2.5_vl_7b_fp8_scaled.safetensors",
                kind: LinkTargetKind::Url,
            },
            LinkEntrySeed {
                label: "byt5_small_glyphxl_fp16.safetensors",
                target: "https://github.com/itsibitzi/foundry/releases/download/latest/byt5_small_glyphxl_fp16.safetensors",
                kind: LinkTargetKind::Url,
            },
        ],
    },
    LinkCategorySeed {
        title: "Diffusion Models",
        entries: &[
            LinkEntrySeed {
                label: "hunyuanvideo1.5_1080p_sr_distilled_fp16.safetensors",
                target: "https://huggingface.co/Tencent-Hunyuan/HunyuanVideo/resolve/main/hunyuanvideo1.5_1080p_sr_distilled_fp16.safetensors",
                kind: LinkTargetKind::Url,
            },
            LinkEntrySeed {
                label: "hunyuanvideo1.5_720p_t2v_fp16.safetensors",
                target: "https://huggingface.co/Tencent-Hunyuan/HunyuanVideo/resolve/main/hunyuanvideo1.5_720p_t2v_fp16.safetensors",
                kind: LinkTargetKind::Url,
            },
        ],
    },
    LinkCategorySeed {
        title: "VAE",
        entries: &[LinkEntrySeed {
            label: "hunyuanvideo15_vae_fp16.safetensors",
            target: "https://huggingface.co/Tencent-Hunyuan/HunyuanVideo/resolve/main/hunyuanvideo15_vae_fp16.safetensors",
            kind: LinkTargetKind::Url,
        }],
    },
    LinkCategorySeed {
        title: "Model Storage Folders",
        entries: &[
            LinkEntrySeed {
                label: "ComfyUI/models/text_encoders",
                target: "~/ComfyUI/models/text_encoders",
                kind: LinkTargetKind::Path,
            },
            LinkEntrySeed {
                label: "ComfyUI/models/diffusion_models",
                target: "~/ComfyUI/models/diffusion_models",
                kind: LinkTargetKind::Path,
            },
            LinkEntrySeed {
                label: "ComfyUI/models/vae",
                target: "~/ComfyUI/models/vae",
                kind: LinkTargetKind::Path,
            },
        ],
    },
];

struct LinkEntryState {
    label: String,
    target: String,
    kind: LinkTargetKind,
    button_rect: Rect,
}

struct LinkCategoryState {
    title: String,
    entries: Vec<LinkEntryState>,
}

#[derive(Clone, Copy)]
enum EditorContextAction {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Save,
    Close,
}

#[derive(Clone)]
struct EditorContextMenuItem {
    rect: Rect,
    label: &'static str,
    action: EditorContextAction,
}

struct LinkLauncher {
    visible: bool,
    categories: Vec<LinkCategoryState>,
    panel_rect: Rect,
    close_button: Rect,
    hovered: Option<(usize, usize)>,
    status: Option<String>,
}

impl LinkLauncher {
    fn new() -> Self {
        let categories = LINK_LIBRARY
            .iter()
            .map(|seed| LinkCategoryState {
                title: seed.title.to_string(),
                entries: seed
                    .entries
                    .iter()
                    .map(|entry| LinkEntryState {
                        label: entry.label.to_string(),
                        target: entry.target.to_string(),
                        kind: entry.kind,
                        button_rect: Rect::new(0, 0, 0, 0),
                    })
                    .collect(),
            })
            .collect();
        Self {
            visible: false,
            categories,
            panel_rect: Rect::new(0, 0, 0, 0),
            close_button: Rect::new(0, 0, 0, 0),
            hovered: None,
            status: None,
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn show(&mut self) {
        self.visible = true;
        self.status = None;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.hovered = None;
    }

    fn handle_mouse_move(&mut self, pos: Point) {
        if !self.visible {
            return;
        }
        self.hovered = None;
        if !self.panel_rect.contains(pos) {
            return;
        }
        for (cat_idx, cat) in self.categories.iter().enumerate() {
            for (entry_idx, entry) in cat.entries.iter().enumerate() {
                if entry.button_rect.contains(pos) {
                    self.hovered = Some((cat_idx, entry_idx));
                    return;
                }
            }
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool, pos: Point) -> bool {
        if !self.visible || button != MouseButton::Left || !pressed {
            return false;
        }

        if self.close_button.contains(pos) {
            self.hide();
            return true;
        }

        if !self.panel_rect.contains(pos) {
            self.hide();
            return true;
        }

        for (cat_idx, cat) in self.categories.iter().enumerate() {
            for (entry_idx, entry) in cat.entries.iter().enumerate() {
                if entry.button_rect.contains(pos) {
                    self.launch_entry(cat_idx, entry_idx);
                    return true;
                }
            }
        }

        if self.panel_rect.contains(pos) {
            return true;
        }

        false
    }

    fn handle_key(&mut self, key: &Key, pressed: bool) -> bool {
        if !self.visible {
            return false;
        }
        if pressed && matches!(key, Key::Escape) {
            self.hide();
            return true;
        }
        false
    }

    fn draw(&mut self, context: &mut dyn DrawContext, theme: &Theme, window_bounds: Rect) {
        if !self.visible {
            return;
        }

        context.set_color(Color::rgba(5, 8, 15, 200));
        context.fill_rect(window_bounds);

        let panel_width = (window_bounds.width() as f32 * 0.55) as i32;
        let panel_height = (window_bounds.height() as f32 * 0.6) as i32;
        self.panel_rect = Rect::new(
            window_bounds.x() + (window_bounds.width() - panel_width) / 2,
            window_bounds.y() + (window_bounds.height() - panel_height) / 2,
            panel_width,
            panel_height,
        );

        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.panel_rect);
        context.set_color(theme.colors.border);
        context.draw_rect(self.panel_rect);

        let title = "Model links & launchers";
        context.set_color(theme.colors.text);
        context.draw_text(
            title,
            Point::new(self.panel_rect.x() + 18, self.panel_rect.y() + 22),
            theme.typography.font_size_large,
        );

        self.close_button = Rect::new(
            self.panel_rect.right() - 85,
            self.panel_rect.y() + 12,
            70,
            26,
        );
        context.set_color(theme.colors.primary);
        context.fill_rounded_rect(self.close_button, 4);
        context.set_color(theme.colors.surface);
        context.draw_text(
            "Close",
            Point::new(self.close_button.x() + 14, self.close_button.y() + 16),
            theme.typography.font_size_small,
        );

        let mut y = self.panel_rect.y() + 60;
        let label_x = self.panel_rect.x() + 24;
        let button_width = 80;
        for (cat_idx, cat) in self.categories.iter_mut().enumerate() {
            context.set_color(theme.colors.text);
            context.draw_text(
                &cat.title,
                Point::new(label_x, y),
                theme.typography.font_size_base,
            );
            y += theme.typography.font_size_base + 4;

            for (entry_idx, entry) in cat.entries.iter_mut().enumerate() {
                context.set_color(theme.colors.text_secondary);
                context.draw_text(
                    &entry.label,
                    Point::new(label_x + 12, y),
                    theme.typography.font_size_base - 1,
                );

                let button_rect = Rect::new(
                    self.panel_rect.right() - button_width - 24,
                    y - 6,
                    button_width,
                    26,
                );
                entry.button_rect = button_rect;

                let hovered = self.hovered == Some((cat_idx, entry_idx));
                context.set_color(if hovered {
                    theme.colors.primary_active
                } else {
                    theme.colors.primary
                });
                context.fill_rounded_rect(button_rect, 4);
                context.set_color(theme.colors.surface);
                context.draw_text(
                    "Open",
                    Point::new(button_rect.x() + 18, button_rect.y() + 16),
                    theme.typography.font_size_small,
                );
                y += theme.typography.font_size_base + 8;
            }

            y += theme.typography.font_size_base;
        }

        if let Some(status) = &self.status {
            context.set_color(theme.colors.text_secondary);
            context.draw_text(
                status,
                Point::new(label_x, self.panel_rect.bottom() - 20),
                theme.typography.font_size_small,
            );
        }
    }

    fn launch_entry(&mut self, cat_index: usize, entry_index: usize) {
        if let Some(entry) = self
            .categories
            .get(cat_index)
            .and_then(|cat| cat.entries.get(entry_index))
        {
            let mut target = entry.target.clone();
            if entry.kind == LinkTargetKind::Path {
                target = expand_home_path(&target);
                if !Path::new(&target).exists() {
                    self.status = Some(format!("Missing path: {}", target));
                    return;
                }
            }
            match open_link_target(&target) {
                Ok(_) => self.status = Some(format!("Opened {}", entry.label)),
                Err(err) => self.status = Some(format!("Launch failed: {}", err)),
            }
        }
    }
}

const MENU_HEIGHT: i32 = 30;
const PATH_BAR_HEIGHT: i32 = 30;

struct FileManagerDemo {
    // Widget IDs
    main_id: WidgetId,
    toolbar_id: WidgetId,
    path_bar_id: WidgetId,
    content_area_id: WidgetId,
    sidebar_id: WidgetId,
    file_area_id: WidgetId,

    // Widgets
    menu_bar: MenuBar,
    back_button: Button,
    forward_button: Button,
    up_button: Button,
    home_button: Button,
    refresh_button: Button,
    new_folder_button: Button,
    delete_button: Button,
    links_button: Button,

    address_bar: TextInput,
    search_bar: TextInput,
    search_query: String,

    view_combo: ComboBox,
    sort_combo: ComboBox,
    sort_descending: bool,
    last_sort_column: usize,

    sidebar_tree: TreeView,
    status_bar: StatusBar,
    context_menu: ContextMenu,

    // State
    current_path: PathBuf,
    history: Vec<PathBuf>,
    history_index: usize,
    file_entries: Vec<FileEntry>,
    sidebar_paths: HashMap<String, PathBuf>,

    // Layout state
    sidebar_width: i32,
    toolbar_height: i32,
    status_height: i32,
    item_size: i32,
    items_per_row: i32,
    scroll_offset: i32,

    // Interaction state
    dragging_divider: bool,
    last_mouse_pos: Point,
    hovered_index: Option<usize>,
    focused_index: Option<usize>,
    selection_start: Option<usize>,
    ctrl_pressed: bool,
    shift_pressed: bool,

    // Visual settings
    show_hidden: bool,
    view_mode: ViewMode,
    editor: Option<EditorState>,
    link_launcher: LinkLauncher,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Icons,
    List,
    Details,
}

impl FileManagerDemo {
    const DETAILS_HEADER_HEIGHT: i32 = 30;
    const DETAILS_ROW_HEIGHT: i32 = 28;
    const DETAILS_PADDING: i32 = 8;
    const LIST_ROW_HEIGHT: i32 = 24;

    fn new() -> Self {
        // Create menu bar
        let mut menu_bar = MenuBar::new(WidgetId::default());

        // File menu
        menu_bar.add_menu(
            "File",
            vec![
                MenuItem::new("New Folder")
                    .with_shortcut("Ctrl+N")
                    .with_on_click(|| println!("New Folder")),
                MenuItem::new("Open")
                    .with_shortcut("Ctrl+O")
                    .with_on_click(trigger_open_command),
                MenuItem::separator(),
                MenuItem::new("Properties")
                    .with_shortcut("Alt+Enter")
                    .with_on_click(|| println!("Properties")),
                MenuItem::separator(),
                MenuItem::new("Exit")
                    .with_shortcut("Ctrl+Q")
                    .with_on_click(|| std::process::exit(0)),
            ],
        );

        // Edit menu
        menu_bar.add_menu(
            "Edit",
            vec![
                MenuItem::new("Cut")
                    .with_shortcut("Ctrl+X")
                    .with_on_click(|| println!("Cut")),
                MenuItem::new("Copy")
                    .with_shortcut("Ctrl+C")
                    .with_on_click(|| println!("Copy")),
                MenuItem::new("Paste")
                    .with_shortcut("Ctrl+V")
                    .with_on_click(|| println!("Paste")),
                MenuItem::separator(),
                MenuItem::new("Select All")
                    .with_shortcut("Ctrl+A")
                    .with_on_click(|| println!("Select All")),
            ],
        );

        // View menu
        menu_bar.add_menu(
            "View",
            vec![
                MenuItem::new("Icons").with_on_click(set_view_icons),
                MenuItem::new("List").with_on_click(set_view_list),
                MenuItem::new("Details").with_on_click(set_view_details),
                MenuItem::separator(),
                MenuItem::new("Show Hidden Files")
                    .with_shortcut("Ctrl+H")
                    .with_on_click(|| println!("Toggle hidden")),
                MenuItem::separator(),
                MenuItem::new("Refresh")
                    .with_shortcut("F5")
                    .with_on_click(|| println!("Refresh")),
            ],
        );

        // Go menu
        menu_bar.add_menu(
            "Go",
            vec![
                MenuItem::new("Back")
                    .with_shortcut("Alt+Left")
                    .with_on_click(|| println!("Back")),
                MenuItem::new("Forward")
                    .with_shortcut("Alt+Right")
                    .with_on_click(|| println!("Forward")),
                MenuItem::new("Up")
                    .with_shortcut("Alt+Up")
                    .with_on_click(|| println!("Up")),
                MenuItem::separator(),
                MenuItem::new("Home")
                    .with_shortcut("Ctrl+Home")
                    .with_on_click(|| println!("Home")),
                MenuItem::new("Documents").with_on_click(|| println!("Documents")),
                MenuItem::new("Downloads").with_on_click(|| println!("Downloads")),
            ],
        );

        // Create widgets
        let back_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("back.png").unwrap_or(ButtonIcon::Back))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Back"));
        let forward_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("forward.png").unwrap_or(ButtonIcon::Forward))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Forward"));
        let up_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("up.png").unwrap_or(ButtonIcon::Up))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Up"));
        let home_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("home.png").unwrap_or(ButtonIcon::Home))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Home"));
        let refresh_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("refresh.png").unwrap_or(ButtonIcon::Refresh))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Refresh"));
        let new_folder_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("folder_add.png").unwrap_or(ButtonIcon::FolderNew))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("New Folder"));
        let delete_button = Button::new(WidgetId::default(), "")
            .with_icon(load_png_icon("delete.png").unwrap_or(ButtonIcon::Delete))
            .with_style(ButtonStyle::Toolbar)
            .with_on_click(|| println!("Delete"));
        let links_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Custom(draw_link_icon))
            .with_style(ButtonStyle::Toolbar)
            .with_tooltip("Model Links");

        let address_bar = TextInput::new(WidgetId::default()).with_placeholder("Enter path...");
        let search_bar = TextInput::new(WidgetId::default()).with_placeholder("Search...");

        let view_combo = ComboBox::new(WidgetId::default())
            .with_items(vec![
                "Icons".to_string(),
                "List".to_string(),
                "Details".to_string(),
            ])
            .with_selected(0);

        let sort_combo = ComboBox::new(WidgetId::default())
            .with_items(vec![
                "Name".to_string(),
                "Size".to_string(),
                "Type".to_string(),
                "Modified".to_string(),
            ])
            .with_selected(0);

        // Create tree view for sidebar
        let mut sidebar_tree = TreeView::new(WidgetId::default());
        let mut sidebar_paths = HashMap::new();

        // Add common locations
        if let Some(home) = dirs::home_dir() {
            let home_node = TreeNode::new("home".to_string(), "Home".to_string());
            sidebar_paths.insert("home".to_string(), home);
            sidebar_tree.add_root_node(home_node);
        }

        if let Some(docs) = dirs::document_dir() {
            let documents_node = TreeNode::new("documents".to_string(), "Documents".to_string());
            sidebar_paths.insert("documents".to_string(), docs);
            sidebar_tree.add_root_node(documents_node);
        }

        if let Some(downloads) = dirs::download_dir() {
            let downloads_node = TreeNode::new("downloads".to_string(), "Downloads".to_string());
            sidebar_paths.insert("downloads".to_string(), downloads);
            sidebar_tree.add_root_node(downloads_node);
        }

        if let Some(pics) = dirs::picture_dir() {
            let pictures_node = TreeNode::new("pictures".to_string(), "Pictures".to_string());
            sidebar_paths.insert("pictures".to_string(), pics);
            sidebar_tree.add_root_node(pictures_node);
        }

        if let Some(music) = dirs::audio_dir() {
            let music_node = TreeNode::new("music".to_string(), "Music".to_string());
            sidebar_paths.insert("music".to_string(), music);
            sidebar_tree.add_root_node(music_node);
        }

        if let Some(videos) = dirs::video_dir() {
            let videos_node = TreeNode::new("videos".to_string(), "Videos".to_string());
            sidebar_paths.insert("videos".to_string(), videos);
            sidebar_tree.add_root_node(videos_node);
        }

        // Status bar
        let mut status_bar = StatusBar::new(WidgetId::default());
        status_bar.add_panel(StatusPanel::new("Ready"));
        status_bar.add_panel(StatusPanel::new("0 items").with_spring_width());
        status_bar.add_panel(StatusPanel::new("0 bytes"));

        // Context menu
        let context_menu = ContextMenu::new(WidgetId::default()).with_items(vec![
            ContextMenuItem::new("Open").with_on_click(trigger_open_command),
            ContextMenuItem::new("Open in Terminal"),
            ContextMenuItem::separator(),
            ContextMenuItem::new("Cut").with_shortcut("Ctrl+X"),
            ContextMenuItem::new("Copy").with_shortcut("Ctrl+C"),
            ContextMenuItem::new("Paste").with_shortcut("Ctrl+V"),
            ContextMenuItem::separator(),
            ContextMenuItem::new("Rename").with_shortcut("F2"),
            ContextMenuItem::new("Delete").with_shortcut("Del"),
            ContextMenuItem::separator(),
            ContextMenuItem::new("Properties").with_shortcut("Alt+Enter"),
        ]);

        let current_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let history = vec![current_path.clone()];

        let mut demo = Self {
            // IDs
            main_id: WidgetId::default(),
            toolbar_id: WidgetId::default(),
            path_bar_id: WidgetId::default(),
            content_area_id: WidgetId::default(),
            sidebar_id: WidgetId::default(),
            file_area_id: WidgetId::default(),

            // Widgets
            menu_bar,
            back_button,
            forward_button,
            up_button,
            home_button,
            refresh_button,
            new_folder_button,
            delete_button,
            links_button,
            address_bar,
            search_bar,
            search_query: String::new(),
            view_combo,
            sort_combo,
            sort_descending: false,
            last_sort_column: 0,
            sidebar_tree,
            status_bar,
            context_menu,

            // State
            current_path: current_path.clone(),
            history,
            history_index: 0,
            file_entries: Vec::new(),
            sidebar_paths,

            // Layout
            sidebar_width: 200,
            toolbar_height: 40,
            status_height: 24,
            item_size: 80,
            items_per_row: 1,
            scroll_offset: 0,

            // Interaction
            dragging_divider: false,
            last_mouse_pos: Point::ZERO,
            hovered_index: None,
            focused_index: None,
            selection_start: None,
            ctrl_pressed: false,
            shift_pressed: false,

            // Settings
            show_hidden: false,
            view_mode: ViewMode::Icons,
            editor: None,
            link_launcher: LinkLauncher::new(),
        };

        demo.navigate_to(current_path.clone());
        demo
    }

    fn navigate_to(&mut self, path: PathBuf) {
        if path.exists() && path.is_dir() {
            self.current_path = path;

            // Update history
            if self.history_index < self.history.len() - 1 {
                self.history.truncate(self.history_index + 1);
            }
            self.history.push(self.current_path.clone());
            self.history_index = self.history.len() - 1;

            self.address_bar
                .set_text(self.current_path.to_string_lossy().to_string());
            self.refresh_file_list();
            self.update_navigation_buttons();
            self.scroll_offset = 0;
        }
    }

    fn open_editor(&mut self, path: PathBuf) {
        let content = fs::read_to_string(&path).unwrap_or_default();
        self.link_launcher.hide();
        self.editor = Some(EditorState::new(path, content));
    }

    fn close_editor(&mut self) {
        self.editor = None;
    }

    fn editor_active(&self) -> bool {
        self.editor.is_some()
    }

    fn toggle_link_overlay(&mut self) {
        if self.link_launcher.is_visible() {
            self.link_launcher.hide();
        } else {
            self.link_launcher.show();
            self.context_menu.hide();
            self.menu_bar.hide_dropdown();
        }
    }

    fn open_selected_entry(&mut self) {
        if self.file_entries.is_empty() {
            return;
        }
        let index = self
            .focused_index
            .or_else(|| self.file_entries.iter().position(|entry| entry.selected))
            .unwrap_or(0);
        if index >= self.file_entries.len() {
            return;
        }
        let entry_path = self.file_entries[index].path.clone();
        if self.file_entries[index].is_dir {
            self.navigate_to(entry_path);
        } else if Self::is_text_file(&self.file_entries[index]) {
            self.open_editor(entry_path);
        } else {
            // Open other files (images, etc.) with system default application
            let _ = open_link_target(entry_path.to_string_lossy().as_ref());
        }
    }

    fn is_text_file(entry: &FileEntry) -> bool {
        if entry.is_dir {
            return false;
        }
        if let Some(ext) = Path::new(&entry.name).extension().and_then(|s| s.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "txt" | "md" | "rs" | "toml" | "json" | "log" | "cfg"
            )
        } else {
            false
        }
    }

    fn navigate_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.address_bar
                .set_text(self.current_path.to_string_lossy().to_string());
            self.refresh_file_list();
            self.update_navigation_buttons();
        }
    }

    fn navigate_forward(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            self.address_bar
                .set_text(self.current_path.to_string_lossy().to_string());
            self.refresh_file_list();
            self.update_navigation_buttons();
        }
    }

    fn navigate_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
        }
    }

    fn navigate_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.navigate_to(home);
        }
    }

    fn update_navigation_buttons(&mut self) {
        self.back_button.set_enabled(self.history_index > 0);
        self.forward_button
            .set_enabled(self.history_index < self.history.len() - 1);
        self.up_button
            .set_enabled(self.current_path.parent().is_some());
    }

    fn refresh_file_list(&mut self) {
        self.file_entries.clear();

        if let Ok(entries) = fs::read_dir(&self.current_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files if not showing them
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }

                    self.file_entries.push(FileEntry {
                        name,
                        path: entry.path(),
                        is_dir: metadata.is_dir(),
                        size: metadata.len(),
                        modified: metadata.modified().ok(),
                        selected: false,
                    });
                }
            }
        }

        self.apply_search_filter();
        // Sort entries
        self.sort_entries(false);
        self.update_status_bar();
    }

    fn apply_search_filter(&mut self) {
        let query = self.search_query.trim();
        if query.is_empty() {
            return;
        }
        let lower = query.to_lowercase();
        self.file_entries
            .retain(|entry| entry.name.to_lowercase().contains(&lower));
    }
    fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode == mode {
            return;
        }
        self.view_mode = mode;
        self.scroll_offset = 0;
        let idx = match mode {
            ViewMode::Icons => 0,
            ViewMode::List => 1,
            ViewMode::Details => 2,
        };
        self.view_combo.set_selected(Some(idx));
    }

    fn sort_entries(&mut self, toggle_direction: bool) {
        let sort_index = self.sort_combo.selected_index().unwrap_or(0);

        if self.last_sort_column != sort_index {
            self.last_sort_column = sort_index;
            self.sort_descending = false;
        } else if toggle_direction {
            self.sort_descending = !self.sort_descending;
        }

        self.file_entries.sort_by(|a, b| {
            // Directories first
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            match sort_index {
                0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()), // Name
                1 => a.size.cmp(&b.size),                               // Size
                2 => {
                    // Type (extension)
                    let ext_a = Path::new(&a.name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let ext_b = Path::new(&b.name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    ext_a.cmp(ext_b)
                }
                3 => {
                    // Modified
                    match (a.modified, b.modified) {
                        (Some(t1), Some(t2)) => t1.cmp(&t2),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        if self.sort_descending {
            self.file_entries.reverse();
        }
    }

    fn sync_search_query(&mut self) {
        let new_query = self.search_bar.text().trim().to_string();
        if new_query != self.search_query {
            self.search_query = new_query;
            self.refresh_file_list();
        }
    }

    fn update_status_bar(&mut self) {
        let total_items = self.file_entries.len();
        let selected_count = self.file_entries.iter().filter(|e| e.selected).count();
        let total_size: u64 = self
            .file_entries
            .iter()
            .filter(|e| e.selected || selected_count == 0)
            .map(|e| e.size)
            .sum();

        if selected_count > 0 {
            self.status_bar
                .set_panel_text(0, &format!("{} selected", selected_count));
        } else {
            self.status_bar.set_panel_text(0, "Ready");
        }

        self.status_bar
            .set_panel_text(1, &format!("{} items", total_items));
        self.status_bar.set_panel_text(2, &format_size(total_size));
    }

    fn get_file_icon(entry: &FileEntry) -> (Color, &'static str) {
        if entry.is_dir {
            (Color::rgb(80, 150, 255), "DIR")
        } else {
            match Path::new(&entry.name).extension().and_then(|s| s.to_str()) {
                Some("txt") | Some("md") => (Color::rgb(200, 200, 200), "TXT"),
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("bmp") => {
                    (Color::rgb(255, 170, 70), "IMG")
                }
                Some("mp3") | Some("wav") | Some("ogg") | Some("flac") => {
                    (Color::rgb(120, 200, 120), "AUD")
                }
                Some("mp4") | Some("avi") | Some("mkv") | Some("mov") => {
                    (Color::rgb(180, 120, 255), "VID")
                }
                Some("zip") | Some("tar") | Some("gz") | Some("7z") | Some("rar") => {
                    (Color::rgb(255, 120, 120), "ZIP")
                }
                Some("pdf") => (Color::rgb(255, 90, 90), "PDF"),
                Some("doc") | Some("docx") => (Color::rgb(120, 180, 255), "DOC"),
                Some("xls") | Some("xlsx") => (Color::rgb(120, 220, 120), "XLS"),
                Some("rs") | Some("c") | Some("cpp") | Some("h") | Some("py") | Some("js") => {
                    (Color::rgb(255, 200, 90), "SRC")
                }
                _ => (Color::rgb(180, 200, 220), "FILE"),
            }
        }
    }

    fn handle_file_click(&mut self, index: usize, double_click: bool) {
        if index >= self.file_entries.len() {
            return;
        }

        if double_click {
            let entry = &self.file_entries[index];
            if entry.is_dir {
                let path = entry.path.clone();
                self.navigate_to(path);
            } else if Self::is_text_file(entry) {
                self.open_editor(entry.path.clone());
            } else {
                // Open other files (images, etc.) with system default application
                let _ = open_link_target(entry.path.to_string_lossy().as_ref());
            }
            return;
        } else {
            // Handle selection
            if self.ctrl_pressed {
                self.file_entries[index].selected = !self.file_entries[index].selected;
            } else if self.shift_pressed && self.selection_start.is_some() {
                let start = self.selection_start.unwrap();
                let (from, to) = if start < index {
                    (start, index)
                } else {
                    (index, start)
                };
                for i in from..=to {
                    if i < self.file_entries.len() {
                        self.file_entries[i].selected = true;
                    }
                }
            } else {
                // Clear all selections and select only this one
                for entry in &mut self.file_entries {
                    entry.selected = false;
                }
                self.file_entries[index].selected = true;
                self.selection_start = Some(index);
            }

            self.focused_index = Some(index);
            self.update_status_bar();
        }
    }

    fn draw(&mut self, context: &mut dyn DrawContext, theme: &Theme, window_size: Size) {
        let bounds = Rect::new(0, 0, window_size.width, window_size.height);

        // Draw main background
        context.set_color(theme.colors.background);
        context.fill_rect(bounds);

        // Calculate layout areas
        let menu_rect = Rect::new(0, 0, bounds.width(), MENU_HEIGHT);
        let toolbar_rect = Rect::new(0, MENU_HEIGHT, bounds.width(), self.toolbar_height);
        let path_bar_rect = Rect::new(
            0,
            MENU_HEIGHT + self.toolbar_height,
            bounds.width(),
            PATH_BAR_HEIGHT,
        );
        let content_rect = Rect::new(
            0,
            MENU_HEIGHT + self.toolbar_height + PATH_BAR_HEIGHT,
            bounds.width(),
            bounds.height()
                - MENU_HEIGHT
                - self.toolbar_height
                - PATH_BAR_HEIGHT
                - self.status_height,
        );
        let sidebar_rect = Rect::new(
            0,
            content_rect.y(),
            self.sidebar_width,
            content_rect.height(),
        );
        let divider_rect = Rect::new(
            self.sidebar_width,
            content_rect.y(),
            4,
            content_rect.height(),
        );
        let file_area_rect = Rect::new(
            self.sidebar_width + 4,
            content_rect.y(),
            content_rect.width() - self.sidebar_width - 4,
            content_rect.height(),
        );
        let status_rect = Rect::new(
            0,
            bounds.height() - self.status_height,
            bounds.width(),
            self.status_height,
        );

        // Draw menu bar FIRST
        self.menu_bar.set_bounds(menu_rect);
        self.menu_bar.draw(context, theme);

        // Draw separator line between menu and toolbar
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(0, MENU_HEIGHT),
            Point::new(bounds.width(), MENU_HEIGHT),
            1,
        );

        // Draw toolbar
        self.draw_toolbar(context, theme, toolbar_rect);

        // Draw path bar
        self.draw_path_bar(context, theme, path_bar_rect);

        // Draw sidebar
        self.draw_sidebar(context, theme, sidebar_rect);

        // Draw divider
        context.set_color(theme.colors.border);
        context.fill_rect(divider_rect);
        if self.dragging_divider {
            context.set_color(theme.colors.primary);
            context.fill_rect(Rect::new(
                divider_rect.x(),
                divider_rect.y(),
                2,
                divider_rect.height(),
            ));
        }

        // Draw file area
        self.draw_file_area(context, theme, file_area_rect);

        // Draw status bar
        self.status_bar.set_bounds(status_rect);
        self.status_bar.draw(context, theme);

        // Draw context menu if open
        if self.context_menu.is_open() {
            self.context_menu.draw(context, theme);
        }

        if self.link_launcher.is_visible() {
            self.link_launcher.draw(context, theme, bounds);
        }

        if let Some(editor) = &mut self.editor {
            editor.draw(context, theme, bounds);
        }

        // Draw menu dropdown LAST so it appears on top of everything
        self.menu_bar.draw_dropdown_only(context, theme);
    }

    fn place_toolbar_button(
        button: &mut Button,
        x: &mut i32,
        y: i32,
        width: i32,
        height: i32,
        spacing: i32,
        context: &mut dyn DrawContext,
        theme: &Theme,
    ) {
        button.set_bounds(Rect::new(*x, y, width, height));
        button.draw(context, theme);
        *x += width + spacing;
    }

    fn draw_toolbar(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        // Background
        context.set_color(theme.colors.surface);
        context.fill_rect(bounds);
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(bounds.x(), bounds.bottom() - 1),
            Point::new(bounds.right(), bounds.bottom() - 1),
            1,
        );

        let padding = 10;
        let button_size = (bounds.height() - padding * 2).max(32);
        let spacing = 8;
        let mut x = bounds.x() + padding;
        let y = bounds.y() + (bounds.height() - button_size) / 2;

        // Navigation buttons
        Self::place_toolbar_button(
            &mut self.back_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );
        Self::place_toolbar_button(
            &mut self.forward_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );
        Self::place_toolbar_button(
            &mut self.up_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );

        // Separator between navigation and location buttons
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(x - spacing / 2, bounds.y() + 6),
            Point::new(x - spacing / 2, bounds.bottom() - 6),
            1,
        );
        x += spacing;

        Self::place_toolbar_button(
            &mut self.home_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );

        // Separator
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(x - spacing / 2, bounds.y() + 6),
            Point::new(x - spacing / 2, bounds.bottom() - 6),
            1,
        );
        x += spacing;

        // Action buttons
        Self::place_toolbar_button(
            &mut self.refresh_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );
        Self::place_toolbar_button(
            &mut self.new_folder_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );
        Self::place_toolbar_button(
            &mut self.delete_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );
        Self::place_toolbar_button(
            &mut self.links_button,
            &mut x,
            y,
            button_size,
            button_size,
            spacing,
            context,
            theme,
        );

        // Flexible spacer before combos/search
        let search_width = 200;
        let combo_width = 140;
        let mut right_x = bounds.right() - padding;

        // Place combos from right edge
        right_x -= combo_width;
        self.sort_combo
            .set_bounds(Rect::new(right_x, y, combo_width, button_size));
        self.sort_combo.draw(context, theme);
        right_x -= combo_width + spacing;

        self.view_combo
            .set_bounds(Rect::new(right_x, y, combo_width, button_size));
        self.view_combo.draw(context, theme);
        right_x -= search_width + spacing;

        self.search_bar
            .set_bounds(Rect::new(right_x, y, search_width, button_size));
        self.search_bar.draw(context, theme);
    }

    fn draw_path_bar(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        // Background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(bounds);

        // Address bar
        let addr_bounds = Rect::new(bounds.x() + 5, bounds.y() + 5, bounds.width() - 10, 20);
        self.address_bar.set_bounds(addr_bounds);
        self.address_bar.draw(context, theme);
    }

    fn draw_sidebar(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        // Background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(bounds);

        // Tree view
        self.sidebar_tree.set_bounds(Rect::new(
            bounds.x(),
            bounds.y(),
            bounds.width(),
            bounds.height(),
        ));
        self.sidebar_tree.draw(context, theme);
    }

    fn draw_file_area(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        // Background
        context.set_color(theme.colors.background);
        context.fill_rect(bounds);

        // Clip to bounds
        context.push_clip_rect(bounds);

        match self.view_mode {
            ViewMode::Icons => self.draw_icon_view(context, theme, bounds),
            ViewMode::List => self.draw_list_view(context, theme, bounds),
            ViewMode::Details => self.draw_details_view(context, theme, bounds),
        }

        context.pop_clip_rect();

        // Draw scrollbar if needed
        let content_height = self.calculate_content_height(bounds);
        if content_height > bounds.height() {
            self.draw_scrollbar(context, theme, bounds, content_height);
        }
    }

    fn draw_icon_view(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        let icon_size = 64;
        let text_height = 40;
        let item_width = 90;
        let item_height = icon_size + text_height;
        let padding = 10;

        self.items_per_row = (bounds.width() - padding * 2) / item_width;
        if self.items_per_row < 1 {
            self.items_per_row = 1;
        }

        let start_y = bounds.y() + padding - self.scroll_offset;

        for (index, entry) in self.file_entries.iter().enumerate() {
            let row = index as i32 / self.items_per_row;
            let col = index as i32 % self.items_per_row;

            let x = bounds.x() + padding + col * item_width;
            let y = start_y + row * item_height;

            // Skip if outside visible area
            if y + item_height < bounds.y() || y > bounds.bottom() {
                continue;
            }

            let item_rect = Rect::new(x, y, item_width - padding, item_height - padding);

            // Draw selection/hover background
            if entry.selected {
                context.set_color(theme.colors.primary.with_alpha(100));
                context.fill_rounded_rect(item_rect, 5);
            } else if Some(index) == self.hovered_index {
                context.set_color(theme.colors.surface_variant);
                context.fill_rounded_rect(item_rect, 5);
            }

            // Draw focus outline
            if Some(index) == self.focused_index {
                context.set_color(theme.colors.primary);
                context.draw_rounded_rect(item_rect, 5);
            }

            // Draw icon
            let (icon_color, icon_label) = Self::get_file_icon(entry);
            let icon_rect = Rect::new(x + (item_width - padding) / 2 - 18, y + 10, 36, 36);
            context.set_color(icon_color.with_alpha(180));
            context.fill_rounded_rect(icon_rect, 6);
            context.set_color(theme.colors.background);
            context.draw_text(
                icon_label,
                Point::new(icon_rect.x() + 8, icon_rect.y() + 18),
                12,
            );

            // Draw name (truncated if needed)
            let name = &entry.name;
            let _text_width = item_width - padding - 10;
            let truncated = if name.len() > 12 {
                format!("{}...", &name[..9])
            } else {
                name.clone()
            };

            context.set_color(if entry.selected {
                theme.colors.primary
            } else {
                theme.colors.text
            });
            let text_x = x + 5;
            let text_y = y + icon_size + 5;

            // Draw text with wrapping
            let lines: Vec<&str> = truncated.split_whitespace().collect();
            for (i, line) in lines.iter().enumerate() {
                context.draw_text(line, Point::new(text_x, text_y + i as i32 * 12), 11);
            }
        }
    }

    fn draw_list_view(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        let item_height = Self::LIST_ROW_HEIGHT;
        let padding = 5;
        let icon_size = 16;

        let start_y = bounds.y() + padding - self.scroll_offset;

        for (index, entry) in self.file_entries.iter().enumerate() {
            let y = start_y + index as i32 * item_height;

            // Skip if outside visible area
            if y + item_height < bounds.y() || y > bounds.bottom() {
                continue;
            }

            let item_rect = Rect::new(bounds.x(), y, bounds.width(), item_height);

            // Draw selection/hover background
            if entry.selected {
                context.set_color(theme.colors.primary.with_alpha(100));
                context.fill_rect(item_rect);
            } else if Some(index) == self.hovered_index {
                context.set_color(theme.colors.surface_variant);
                context.fill_rect(item_rect);
            }

            // Draw focus outline
            if Some(index) == self.focused_index {
                context.set_color(theme.colors.primary);
                context.draw_rect(item_rect.inset(1));
            }

            // Draw icon
            let (icon_color, icon_label) = Self::get_file_icon(entry);
            let icon_rect = Rect::new(bounds.x() + padding, y + 3, icon_size + 6, icon_size + 6);
            context.set_color(icon_color.with_alpha(180));
            context.fill_rounded_rect(icon_rect, 4);
            context.set_color(theme.colors.background);
            context.draw_text(
                icon_label,
                Point::new(icon_rect.x() + 6, icon_rect.y() + 13),
                10,
            );

            // Draw name
            let label_color = if entry.selected {
                theme.colors.background
            } else {
                theme.colors.text
            };
            context.set_color(label_color);
            let baseline = y + item_height / 2 + theme.typography.font_size_base / 2 - 2;
            context.draw_text(
                &entry.name,
                Point::new(bounds.x() + padding + icon_size + 8, baseline),
                14,
            );
        }
    }

    fn draw_details_view(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        const ICON_SIZE: i32 = 16;

        // Column widths
        let name_width = bounds.width() / 2;
        let size_width = 110;
        let type_width = 110;

        // Draw header
        let header_rect = Rect::new(
            bounds.x(),
            bounds.y(),
            bounds.width(),
            Self::DETAILS_HEADER_HEIGHT,
        );
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(header_rect);

        context.set_color(theme.colors.text);
        let header_baseline = header_rect.y() + Self::DETAILS_HEADER_HEIGHT / 2 + 4;
        context.draw_text(
            "Name",
            Point::new(bounds.x() + Self::DETAILS_PADDING, header_baseline),
            13,
        );
        context.draw_text(
            "Size",
            Point::new(bounds.x() + name_width, header_baseline),
            13,
        );
        context.draw_text(
            "Type",
            Point::new(bounds.x() + name_width + size_width, header_baseline),
            13,
        );
        context.draw_text(
            "Modified",
            Point::new(
                bounds.x() + name_width + size_width + type_width,
                header_baseline,
            ),
            13,
        );

        // Draw separator
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(bounds.x(), bounds.y() + Self::DETAILS_HEADER_HEIGHT),
            Point::new(bounds.right(), bounds.y() + Self::DETAILS_HEADER_HEIGHT),
            1,
        );

        let row_top =
            bounds.y() + Self::DETAILS_HEADER_HEIGHT + Self::DETAILS_PADDING - self.scroll_offset;

        for (index, entry) in self.file_entries.iter().enumerate() {
            let y = row_top + index as i32 * Self::DETAILS_ROW_HEIGHT;

            // Skip if outside visible area
            if y + Self::DETAILS_ROW_HEIGHT < bounds.y() + Self::DETAILS_HEADER_HEIGHT
                || y > bounds.bottom()
            {
                continue;
            }

            let item_rect = Rect::new(bounds.x(), y, bounds.width(), Self::DETAILS_ROW_HEIGHT);

            // Draw selection/hover background
            if entry.selected {
                context.set_color(theme.colors.primary.with_alpha(150));
                context.fill_rect(item_rect);
            } else if Some(index) == self.hovered_index {
                context.set_color(theme.colors.surface_variant.with_alpha(160));
                context.fill_rect(item_rect);
            }

            // Draw focus outline
            if Some(index) == self.focused_index {
                context.set_color(theme.colors.primary);
                context.draw_rect(item_rect.inset(1));
            }

            // Draw icon and name
            let (icon_color, icon_label) = Self::get_file_icon(entry);
            let icon_rect = Rect::new(
                bounds.x() + Self::DETAILS_PADDING,
                y + (Self::DETAILS_ROW_HEIGHT - ICON_SIZE) / 2,
                ICON_SIZE + 6,
                ICON_SIZE + 6,
            );
            context.set_color(icon_color.with_alpha(180));
            context.fill_rounded_rect(icon_rect, 4);
            context.set_color(theme.colors.background);
            context.draw_text(
                icon_label,
                Point::new(icon_rect.x() + 6, icon_rect.y() + 13),
                10,
            );

            let text_color = if entry.selected {
                theme.colors.background
            } else {
                theme.colors.text
            };
            context.set_color(text_color);
            context.draw_text(
                &entry.name,
                Point::new(
                    bounds.x() + Self::DETAILS_PADDING + ICON_SIZE + 10,
                    y + Self::DETAILS_ROW_HEIGHT / 2 + 4,
                ),
                15,
            );

            let secondary_color = if entry.selected {
                theme.colors.background
            } else {
                theme.colors.text_secondary
            };

            // Draw size
            if !entry.is_dir {
                context.set_color(secondary_color);
                context.draw_text(
                    &format_size(entry.size),
                    Point::new(
                        bounds.x() + name_width,
                        y + Self::DETAILS_ROW_HEIGHT / 2 + 4,
                    ),
                    14,
                );
            }

            // Draw type
            let file_type = if entry.is_dir {
                "Folder"
            } else {
                Path::new(&entry.name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("File")
            };
            context.set_color(secondary_color);
            context.draw_text(
                file_type,
                Point::new(
                    bounds.x() + name_width + size_width,
                    y + Self::DETAILS_ROW_HEIGHT / 2 + 4,
                ),
                14,
            );

            // Draw modified time
            if let Some(modified) = entry.modified {
                let time_str = format_time(modified);
                context.set_color(secondary_color);
                context.draw_text(
                    &time_str,
                    Point::new(
                        bounds.x() + name_width + size_width + type_width,
                        y + Self::DETAILS_ROW_HEIGHT / 2 + 4,
                    ),
                    14,
                );
            }
        }
    }

    fn detail_header_hit_test(&self, pos: Point, bounds: Rect) -> Option<usize> {
        if pos.y < bounds.y() || pos.y > bounds.y() + Self::DETAILS_HEADER_HEIGHT {
            return None;
        }

        let name_width = bounds.width() / 2;
        let size_width = 110;
        let type_width = 110;

        let columns = [
            (bounds.x(), name_width),
            (bounds.x() + name_width, size_width),
            (bounds.x() + name_width + size_width, type_width),
            (
                bounds.x() + name_width + size_width + type_width,
                bounds.width() - name_width - size_width - type_width,
            ),
        ];

        for (idx, (start_x, width)) in columns.iter().enumerate() {
            if pos.x >= *start_x && pos.x < *start_x + *width {
                return Some(idx);
            }
        }
        None
    }

    fn draw_scrollbar(
        &self,
        context: &mut dyn DrawContext,
        theme: &Theme,
        bounds: Rect,
        content_height: i32,
    ) {
        let scrollbar_width = 12;
        let scrollbar_rect = Rect::new(
            bounds.right() - scrollbar_width,
            bounds.y(),
            scrollbar_width,
            bounds.height(),
        );

        // Background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(scrollbar_rect);

        // Thumb
        let thumb_height =
            (bounds.height() as f32 * bounds.height() as f32 / content_height as f32) as i32;
        let thumb_height = thumb_height.max(20);
        let max_scroll = content_height - bounds.height();
        let thumb_y = if max_scroll > 0 {
            bounds.y()
                + (self.scroll_offset as f32 * (bounds.height() - thumb_height) as f32
                    / max_scroll as f32) as i32
        } else {
            bounds.y()
        };

        let thumb_rect = Rect::new(
            scrollbar_rect.x() + 2,
            thumb_y,
            scrollbar_width - 4,
            thumb_height,
        );

        context.set_color(theme.colors.primary);
        context.fill_rounded_rect(thumb_rect, 4);
    }

    fn calculate_content_height(&self, _bounds: Rect) -> i32 {
        match self.view_mode {
            ViewMode::Icons => {
                let rows =
                    (self.file_entries.len() as i32 + self.items_per_row - 1) / self.items_per_row;
                rows * (self.item_size + 40) + 20
            }
            ViewMode::List => self.file_entries.len() as i32 * Self::LIST_ROW_HEIGHT + 10,
            ViewMode::Details => {
                Self::DETAILS_HEADER_HEIGHT
                    + Self::DETAILS_PADDING * 2
                    + self.file_entries.len() as i32 * Self::DETAILS_ROW_HEIGHT
            }
        }
    }

    fn content_rect(&self, window_bounds: Rect) -> Rect {
        Rect::new(
            0,
            MENU_HEIGHT + self.toolbar_height + PATH_BAR_HEIGHT,
            window_bounds.width(),
            window_bounds.height()
                - MENU_HEIGHT
                - self.toolbar_height
                - PATH_BAR_HEIGHT
                - self.status_height,
        )
    }

    fn file_area_rect(&self, window_bounds: Rect) -> Rect {
        let content = self.content_rect(window_bounds);
        Rect::new(
            content.x() + self.sidebar_width + 4,
            content.y(),
            content.width() - self.sidebar_width - 4,
            content.height(),
        )
    }

    fn handle_mouse_move(&mut self, pos: Point, bounds: Rect, theme: &Theme) {
        let delta = Point::new(pos.x - self.last_mouse_pos.x, pos.y - self.last_mouse_pos.y);
        self.last_mouse_pos = pos;

        if self.link_launcher.is_visible() {
            self.link_launcher.handle_mouse_move(pos);
            return;
        }

        let move_event = Event::MouseMove(MouseMoveEvent {
            position: pos,
            delta,
            modifiers: Modifiers::empty(),
        });

        if let Some(editor) = &mut self.editor {
            editor.handle_pointer_move(pos);
            editor.forward_event(&move_event, theme);
            return;
        }

        let _ = self.back_button.handle_event(&move_event, theme);
        let _ = self.forward_button.handle_event(&move_event, theme);
        let _ = self.up_button.handle_event(&move_event, theme);
        let _ = self.home_button.handle_event(&move_event, theme);
        let _ = self.refresh_button.handle_event(&move_event, theme);
        let _ = self.new_folder_button.handle_event(&move_event, theme);
        let _ = self.delete_button.handle_event(&move_event, theme);
        let _ = self.links_button.handle_event(&move_event, theme);
        if self.context_menu.is_open() {
            let _ = self.context_menu.handle_event(&move_event, theme);
        }

        // Check if hovering over divider
        let divider_rect = Rect::new(self.sidebar_width, 0, 4, bounds.height());
        if divider_rect.contains(pos) && !self.dragging_divider {
            // Show resize cursor (in a real app)
        }

        let file_area_rect = self.file_area_rect(bounds);
        if file_area_rect.contains(pos) {
            self.hovered_index = self.get_item_at_position(pos, file_area_rect);
        } else {
            self.hovered_index = None;
        }
    }

    fn handle_mouse_down(
        &mut self,
        pos: Point,
        button: MouseButton,
        bounds: Rect,
        theme: &Theme,
    ) -> bool {
        if let Some(editor) = &mut self.editor {
            let action = editor.handle_mouse_button(button, true, pos, theme);
            if matches!(action, EditorAction::Close) {
                self.close_editor();
            }
            return true;
        }
        if self.link_launcher.is_visible()
            && self.link_launcher.handle_mouse_button(button, true, pos)
        {
            return true;
        }
        // If context menu is open and this is a left click outside, hide it
        if button == MouseButton::Left
            && self.context_menu.is_open()
            && !self.context_menu.bounds().contains(pos)
        {
            self.context_menu.hide();
        }

        let press_event = Event::MouseButton(MouseButtonEvent {
            button,
            position: pos,
            pressed: true,
            modifiers: Modifiers::empty(),
        });
        let _ = self.back_button.handle_event(&press_event, theme);
        let _ = self.forward_button.handle_event(&press_event, theme);
        let _ = self.up_button.handle_event(&press_event, theme);
        let _ = self.home_button.handle_event(&press_event, theme);
        let _ = self.refresh_button.handle_event(&press_event, theme);
        let _ = self.new_folder_button.handle_event(&press_event, theme);
        let _ = self.delete_button.handle_event(&press_event, theme);
        let _ = self.links_button.handle_event(&press_event, theme);
        if self.context_menu.is_open() {
            let _ = self.context_menu.handle_event(&press_event, theme);
        }

        // Check divider
        let divider_rect = Rect::new(self.sidebar_width - 2, 0, 8, bounds.height());
        if divider_rect.contains(pos) && button == MouseButton::Left {
            self.dragging_divider = true;
            return true;
        }

        // Check toolbar buttons
        if self.back_button.bounds().contains(pos) {
            self.navigate_back();
            return true;
        }
        if self.forward_button.bounds().contains(pos) {
            self.navigate_forward();
            return true;
        }
        if self.up_button.bounds().contains(pos) {
            self.navigate_up();
            return true;
        }
        if self.home_button.bounds().contains(pos) {
            self.navigate_home();
            return true;
        }
        if self.refresh_button.bounds().contains(pos) {
            self.refresh_file_list();
            return true;
        }
        if self.links_button.bounds().contains(pos) {
            self.toggle_link_overlay();
            return true;
        }

        // View/sort combos
        let combo_event = Event::MouseButton(MouseButtonEvent {
            button,
            position: pos,
            pressed: true,
            modifiers: Modifiers::empty(),
        });

        let old_view = self.view_combo.selected_index();
        let view_result = self.view_combo.handle_event(&combo_event, &Theme::dark());
        if old_view != self.view_combo.selected_index() {
            let mode = match self.view_combo.selected_index() {
                Some(0) => ViewMode::Icons,
                Some(1) => ViewMode::List,
                Some(2) => ViewMode::Details,
                _ => ViewMode::Icons,
            };
            self.set_view_mode(mode);
            return true;
        }

        let old_sort = self.sort_combo.selected_index();
        let sort_result = self.sort_combo.handle_event(&combo_event, &Theme::dark());
        if old_sort != self.sort_combo.selected_index()
            || view_result.is_consumed()
            || sort_result.is_consumed()
        {
            self.sort_entries(false);
            return true;
        }

        if self.view_mode == ViewMode::Details {
            let header_rect = self.file_area_rect(bounds);
            if let Some(column) = self.detail_header_hit_test(pos, header_rect) {
                self.sort_combo.set_selected(Some(column.min(3)));
                self.sort_entries(true);
                return true;
            }
        }

        // Check file area
        let file_area_rect = self.file_area_rect(bounds);

        if file_area_rect.contains(pos) {
            if let Some(index) = self.get_item_at_position(pos, file_area_rect) {
                if button == MouseButton::Left {
                    // Simple double-click detection (in real app, track timing)
                    let double_click = self.focused_index == Some(index);
                    self.handle_file_click(index, double_click);
                    return true;
                } else if button == MouseButton::Right {
                    if !self
                        .file_entries
                        .get(index)
                        .map(|e| e.selected)
                        .unwrap_or(false)
                    {
                        for entry in &mut self.file_entries {
                            entry.selected = false;
                        }
                        if let Some(entry) = self.file_entries.get_mut(index) {
                            entry.selected = true;
                        }
                        self.selection_start = Some(index);
                        self.focused_index = Some(index);
                        self.update_status_bar();
                    }
                    // Close any open menu bar dropdown before showing the context menu
                    self.menu_bar.hide_dropdown();
                    self.context_menu.show_at_bounded(pos, Some(bounds));
                    return true;
                }
            }
        }

        // Check sidebar tree
        if self.sidebar_tree.bounds().contains(pos) {
            let event = Event::MouseButton(MouseButtonEvent {
                button,
                position: pos,
                pressed: true,
                modifiers: Modifiers::empty(),
            });
            if self
                .sidebar_tree
                .handle_event(&event, &Theme::dark())
                .is_consumed()
            {
                if let Some(id) = self.sidebar_tree.selected_id() {
                    if let Some(path) = self.sidebar_paths.get(&id) {
                        self.navigate_to(path.clone());
                    }
                }
                return true;
            }
        }

        false
    }

    fn handle_mouse_up(&mut self, pos: Point, button: MouseButton, theme: &Theme) {
        self.dragging_divider = false;
        if let Some(editor) = &mut self.editor {
            let action = editor.handle_mouse_button(button, false, pos, theme);
            if matches!(action, EditorAction::Close) {
                self.close_editor();
            }
            return;
        }
        if self.link_launcher.is_visible() {
            let _ = self.link_launcher.handle_mouse_button(button, false, pos);
            return;
        }
        let release_event = Event::MouseButton(MouseButtonEvent {
            button,
            position: pos,
            pressed: false,
            modifiers: Modifiers::empty(),
        });
        let _ = self.back_button.handle_event(&release_event, theme);
        let _ = self.forward_button.handle_event(&release_event, theme);
        let _ = self.up_button.handle_event(&release_event, theme);
        let _ = self.home_button.handle_event(&release_event, theme);
        let _ = self.refresh_button.handle_event(&release_event, theme);
        let _ = self.new_folder_button.handle_event(&release_event, theme);
        let _ = self.delete_button.handle_event(&release_event, theme);
        let _ = self.links_button.handle_event(&release_event, theme);
        if self.context_menu.is_open() {
            let _ = self.context_menu.handle_event(&release_event, theme);
        }
    }

    fn handle_scroll(&mut self, delta: i32, window_bounds: Rect, theme: &Theme) {
        if let Some(editor) = &mut self.editor {
            let scroll_delta = Point::new(0, delta * 40);
            editor.handle_mouse_wheel(scroll_delta, self.last_mouse_pos, theme);
            return;
        }
        if self.link_launcher.is_visible() {
            return;
        }
        let content_height = self.calculate_content_height(window_bounds);
        let file_area = self.file_area_rect(window_bounds);
        let max_scroll = (content_height - file_area.height()).max(0);

        self.scroll_offset = (self.scroll_offset - delta * 40).clamp(0, max_scroll);
    }

    fn handle_key(&mut self, key: &Key, pressed: bool) -> bool {
        if self.link_launcher.handle_key(key, pressed) {
            return true;
        }
        match key {
            Key::LeftCtrl | Key::RightCtrl => {
                self.ctrl_pressed = pressed;
                true
            }
            Key::LeftShift | Key::RightShift => {
                self.shift_pressed = pressed;
                true
            }
            Key::F5 if pressed => {
                self.refresh_file_list();
                true
            }
            Key::Delete if pressed => {
                // Delete selected files (with confirmation in real app)
                println!("Delete selected files");
                true
            }
            _ => false,
        }
    }

    fn get_item_at_position(&self, pos: Point, bounds: Rect) -> Option<usize> {
        match self.view_mode {
            ViewMode::Icons => {
                let item_width = 90;
                let item_height = 104;
                let padding = 10;

                let rel_x = pos.x - bounds.x() - padding;
                let rel_y = pos.y - bounds.y() - padding + self.scroll_offset;

                if rel_x < 0 || rel_y < 0 {
                    return None;
                }

                let col = rel_x / item_width;
                let row = rel_y / item_height;

                if col < self.items_per_row {
                    let index = (row * self.items_per_row + col) as usize;
                    if index < self.file_entries.len() {
                        return Some(index);
                    }
                }
            }
            ViewMode::List | ViewMode::Details => {
                let rel_y = pos.y - bounds.y() + self.scroll_offset;

                if self.view_mode == ViewMode::Details {
                    let rel_y = rel_y - Self::DETAILS_HEADER_HEIGHT - Self::DETAILS_PADDING;
                    if rel_y >= 0 {
                        let index = (rel_y / Self::DETAILS_ROW_HEIGHT) as usize;
                        if index < self.file_entries.len() {
                            return Some(index);
                        }
                    }
                } else {
                    let index = (rel_y / Self::LIST_ROW_HEIGHT) as usize;
                    if index < self.file_entries.len() {
                        return Some(index);
                    }
                }
            }
        }
        None
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn expand_home_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn open_link_target(target: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let quoted = format!("\"{}\"", target);
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(quoted)
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(target).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(target).spawn().map(|_| ())
    }
}

fn format_time(time: SystemTime) -> String {
    if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
        let datetime = chrono::DateTime::<chrono::Local>::from(SystemTime::UNIX_EPOCH + duration);
        datetime.format("%Y-%m-%d %H:%M").to_string()
    } else {
        String::new()
    }
}

fn convert_modifiers(state: ModifiersState) -> Modifiers {
    let mut mods = Modifiers::empty();
    if state.shift_key() {
        mods |= Modifiers::SHIFT;
    }
    if state.control_key() {
        mods |= Modifiers::CTRL;
    }
    if state.alt_key() {
        mods |= Modifiers::ALT;
    }
    if state.super_key() {
        mods |= Modifiers::SUPER;
    }
    mods
}

fn map_virtual_keycode(keycode: KeyCode) -> Key {
    match keycode {
        KeyCode::KeyA => Key::Character('a'),
        KeyCode::KeyB => Key::Character('b'),
        KeyCode::KeyC => Key::Character('c'),
        KeyCode::KeyD => Key::Character('d'),
        KeyCode::KeyE => Key::Character('e'),
        KeyCode::KeyF => Key::Character('f'),
        KeyCode::KeyG => Key::Character('g'),
        KeyCode::KeyH => Key::Character('h'),
        KeyCode::KeyI => Key::Character('i'),
        KeyCode::KeyJ => Key::Character('j'),
        KeyCode::KeyK => Key::Character('k'),
        KeyCode::KeyL => Key::Character('l'),
        KeyCode::KeyM => Key::Character('m'),
        KeyCode::KeyN => Key::Character('n'),
        KeyCode::KeyO => Key::Character('o'),
        KeyCode::KeyP => Key::Character('p'),
        KeyCode::KeyQ => Key::Character('q'),
        KeyCode::KeyR => Key::Character('r'),
        KeyCode::KeyS => Key::Character('s'),
        KeyCode::KeyT => Key::Character('t'),
        KeyCode::KeyU => Key::Character('u'),
        KeyCode::KeyV => Key::Character('v'),
        KeyCode::KeyW => Key::Character('w'),
        KeyCode::KeyX => Key::Character('x'),
        KeyCode::KeyY => Key::Character('y'),
        KeyCode::KeyZ => Key::Character('z'),
        KeyCode::Digit0 => Key::Character('0'),
        KeyCode::Digit1 => Key::Character('1'),
        KeyCode::Digit2 => Key::Character('2'),
        KeyCode::Digit3 => Key::Character('3'),
        KeyCode::Digit4 => Key::Character('4'),
        KeyCode::Digit5 => Key::Character('5'),
        KeyCode::Digit6 => Key::Character('6'),
        KeyCode::Digit7 => Key::Character('7'),
        KeyCode::Digit8 => Key::Character('8'),
        KeyCode::Digit9 => Key::Character('9'),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Space => Key::Space,
        KeyCode::Delete => Key::Delete,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        // Original code mapped F5 -> F1 (kept for behavioral parity).
        KeyCode::F5 => Key::F1,
        KeyCode::ControlLeft => Key::LeftCtrl,
        KeyCode::ControlRight => Key::RightCtrl,
        KeyCode::ShiftLeft => Key::LeftShift,
        KeyCode::ShiftRight => Key::RightShift,
        _ => Key::Unknown(0),
    }
}

/// Helper: extract a `KeyCode` from a `KeyEvent`'s physical_key, returning
/// `None` for unidentified keys. Used to replace `KeyEvent.virtual_keycode`
/// (which existed as `KeyboardInput.virtual_keycode` in winit 0.28).
fn keyevent_keycode(event: &winit::event::KeyEvent) -> Option<KeyCode> {
    match event.physical_key {
        PhysicalKey::Code(code) => Some(code),
        PhysicalKey::Unidentified(_) => None,
    }
}

fn map_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(1) => MouseButton::Extra1,
        winit::event::MouseButton::Other(2) => MouseButton::Extra2,
        _ => MouseButton::Left,
    }
}

fn apply_file_cmd(demo: &mut FileManagerDemo) -> bool {
    match FILE_CMD.swap(-1, Ordering::SeqCst) {
        FILE_CMD_OPEN => {
            demo.open_selected_entry();
            true
        }
        _ => false,
    }
}

fn main() -> erigui_core::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::new(&event_loop, 1280, 860, "EriGui File Manager")?;

    let mut theme = Theme::dark();
    theme.colors.primary = Color::rgb(90, 160, 255);
    theme.colors.surface = Color::rgb(32, 36, 48);
    theme.colors.surface_variant = Color::rgb(44, 48, 60);
    theme.colors.border = Color::rgb(70, 80, 100);
    theme.colors.text = Color::rgb(230, 235, 245);
    theme.colors.text_secondary = Color::rgb(170, 180, 195);
    let mut demo = FileManagerDemo::new();
    let mut last_cursor_pos = PhysicalPosition::new(0.0, 0.0);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        // Apply view-mode commands coming from the menu
        let apply_view_cmd = |demo: &mut FileManagerDemo, renderer: &mut Renderer| {
            let cmd = VIEW_MODE_CMD.swap(-1, Ordering::SeqCst);
            if cmd >= 0 {
                let mode = match cmd {
                    0 => ViewMode::Icons,
                    1 => ViewMode::List,
                    2 => ViewMode::Details,
                    _ => ViewMode::Icons,
                };
                demo.set_view_mode(mode);
                demo.menu_bar.hide_dropdown();
                renderer.window().request_redraw();
                true
            } else {
                false
            }
        };

        let _cmd_applied = apply_view_cmd(&mut demo, &mut renderer);
        let _ = apply_file_cmd(&mut demo);

        match event {
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size.width, physical_size.height);
                    renderer.window().request_redraw();
                }

                WindowEvent::CursorMoved { position, .. } => {
                    last_cursor_pos = position;
                    let pos = Point::new(position.x as i32, position.y as i32);
                    let size = renderer.viewport_size();
                    let bounds = Rect::new(0, 0, size.width, size.height);

                    if demo.editor_active() {
                        demo.handle_mouse_move(pos, bounds, &theme);
                    } else {
                        // Always handle menu bar mouse move (for dropdowns)
                        let event = Event::MouseMove(MouseMoveEvent {
                            position: pos,
                            delta: Point::new(
                                pos.x - demo.last_mouse_pos.x,
                                pos.y - demo.last_mouse_pos.y,
                            ),
                            modifiers: Modifiers::empty(),
                        });
                        demo.menu_bar.handle_event(&event, &theme);

                        demo.handle_mouse_move(pos, bounds, &theme);

                        if demo.dragging_divider {
                            demo.sidebar_width = (pos.x - 2).clamp(150, 400);
                        }
                    }

                    renderer.window().request_redraw();
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    let pos = Point::new(last_cursor_pos.x as i32, last_cursor_pos.y as i32);
                    let size = renderer.viewport_size();
                    let bounds = Rect::new(0, 0, size.width, size.height);

                    let mapped_button = map_mouse_button(button);

                    if demo.editor_active() {
                        match state {
                            winit::event::ElementState::Pressed => {
                                demo.handle_mouse_down(pos, mapped_button, bounds, &theme);
                            }
                            winit::event::ElementState::Released => {
                                demo.handle_mouse_up(pos, mapped_button, &theme);
                            }
                        }
                    } else {
                        if demo.context_menu.is_open() && !demo.context_menu.bounds().contains(pos)
                        {
                            demo.context_menu.hide();
                        }

                        match state {
                            winit::event::ElementState::Pressed => {
                                let dropdown_was_visible = demo.menu_bar.is_dropdown_visible();
                                let menu_event = Event::MouseButton(MouseButtonEvent {
                                    button: mapped_button,
                                    position: pos,
                                    pressed: true,
                                    modifiers: Modifiers::empty(),
                                });
                                let menu_result = demo.menu_bar.handle_event(&menu_event, &theme);
                                let menu_changed =
                                    dropdown_was_visible != demo.menu_bar.is_dropdown_visible();
                                let menu_consumed = menu_result.is_consumed() || menu_changed;
                                if menu_consumed {
                                    let _ = apply_view_cmd(&mut demo, &mut renderer);
                                }

                                if !menu_result.is_consumed() {
                                    demo.handle_mouse_down(pos, mapped_button, bounds, &theme);
                                }
                            }
                            winit::event::ElementState::Released => {
                                let dropdown_was_visible = demo.menu_bar.is_dropdown_visible();
                                let event = Event::MouseButton(MouseButtonEvent {
                                    button: mapped_button,
                                    position: pos,
                                    pressed: false,
                                    modifiers: Modifiers::empty(),
                                });
                                let menu_result = demo.menu_bar.handle_event(&event, &theme);
                                let menu_changed =
                                    dropdown_was_visible != demo.menu_bar.is_dropdown_visible();
                                if menu_result.is_consumed() || menu_changed {
                                    let _ = apply_view_cmd(&mut demo, &mut renderer);
                                }

                                demo.handle_mouse_up(pos, mapped_button, &theme);

                                if demo.context_menu.is_open()
                                    && !demo.context_menu.bounds().contains(pos)
                                {
                                    demo.context_menu.hide();
                                }
                            }
                        }
                    }

                    renderer.window().request_redraw();
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll_amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => -y as i32,
                        MouseScrollDelta::PixelDelta(pos) => -(pos.y as i32),
                    };

                    let size = renderer.viewport_size();
                    let bounds = Rect::new(0, 0, size.width, size.height);
                    demo.handle_scroll(scroll_amount, bounds, &theme);

                    renderer.window().request_redraw();
                }

                WindowEvent::KeyboardInput { event: ref input, .. } => {
                    if demo.editor_active() {
                        let pressed = input.state == winit::event::ElementState::Pressed;
                        let modifiers = Modifiers::empty() /* TODO: thread last-known modifiers through, winit 0.29 removed input.modifiers */;
                        let mut close_editor_now = false;
                        let mut escape_handled = false;

                        if pressed && keyevent_keycode(input) == Some(KeyCode::Escape) {
                            escape_handled = true;
                            if let Some(editor) = demo.editor.as_mut() {
                                if let EditorAction::Close = editor.handle_close_request() {
                                    close_editor_now = true;
                                }
                            }
                        }

                        if !escape_handled {
                            if let Some(editor) = demo.editor.as_mut() {
                                let mut consumed = false;
                                let mut shortcut_close = false;
                                if pressed {
                                    if let Some(keycode) = keyevent_keycode(input) {
                                        let key = map_virtual_keycode(keycode);
                                        let (handled, should_close) =
                                            editor.handle_shortcut_key(&key, modifiers);
                                        consumed = handled;
                                        shortcut_close = should_close;
                                    }
                                }

                                if shortcut_close {
                                    close_editor_now = true;
                                } else if !consumed {
                                    if let Some(keycode) = keyevent_keycode(input) {
                                        let key = map_virtual_keycode(keycode);
                                        if !matches!(key, Key::Unknown(_)) {
                                            let event = if pressed {
                                                Event::KeyPress(KeyPressEvent {
                                                    key: key.clone(),
                                                    modifiers,
                                                    repeat: false,
                                                })
                                            } else {
                                                Event::KeyRelease(KeyReleaseEvent {
                                                    key: key.clone(),
                                                    modifiers,
                                                })
                                            };
                                            editor.forward_event(&event, &theme);
                                        }
                                    }
                                }
                            }
                        }

                        if close_editor_now {
                            demo.close_editor();
                        }

                        renderer.window().request_redraw();
                    } else if demo.link_launcher.is_visible() {
                        if let Some(keycode) = keyevent_keycode(input) {
                            let key = map_virtual_keycode(keycode);
                            let pressed = input.state == winit::event::ElementState::Pressed;
                            if demo.link_launcher.handle_key(&key, pressed) {
                                renderer.window().request_redraw();
                            }
                        }
                    } else if let Some(keycode) = keyevent_keycode(input) {
                        let key = map_virtual_keycode(keycode);

                        if !matches!(key, Key::Unknown(_)) {
                            let pressed = input.state == winit::event::ElementState::Pressed;
                            let modifiers = Modifiers::empty() /* TODO: thread last-known modifiers through, winit 0.29 removed input.modifiers */;

                            // Handle special keys
                            if matches!(key, Key::F1) && pressed {
                                // F5 refresh
                                demo.refresh_file_list();
                            } else {
                                demo.handle_key(&key, pressed);
                            }

                            // Handle widget events
                            let event = if pressed {
                                Event::KeyPress(KeyPressEvent {
                                    key: key.clone(),
                                    modifiers,
                                    repeat: false,
                                })
                            } else {
                                Event::KeyRelease(KeyReleaseEvent {
                                    key: key.clone(),
                                    modifiers,
                                })
                            };

                            // Send to focused widget
                            if demo.address_bar.is_focused() {
                                demo.address_bar.handle_event(&event, &theme);
                                if matches!(key, Key::Enter) && pressed {
                                    let path = PathBuf::from(demo.address_bar.text());
                                    demo.navigate_to(path);
                                }
                            } else if demo.search_bar.is_focused() {
                                demo.search_bar.handle_event(&event, &theme);
                                demo.sync_search_query();
                            }

                            // Handle view mode change
                            let old_view = demo.view_combo.selected_index();
                            demo.view_combo.handle_event(&event, &theme);
                            if old_view != demo.view_combo.selected_index() {
                                let mode = match demo.view_combo.selected_index() {
                                    Some(0) => ViewMode::Icons,
                                    Some(1) => ViewMode::List,
                                    Some(2) => ViewMode::Details,
                                    _ => ViewMode::Icons,
                                };
                                demo.set_view_mode(mode);
                            }

                            // Handle sort change
                            let old_sort = demo.sort_combo.selected_index();
                            demo.sort_combo.handle_event(&event, &theme);
                            if old_sort != demo.sort_combo.selected_index() {
                                demo.sort_entries(false);
                            }

                            renderer.window().request_redraw();
                        }
                    }
                }

                // (winit 0.29) WindowEvent::ReceivedCharacter arm removed; EventTranslator now derives TextInput from KeyEvent.text

                _ => {}
            },

            WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                let size = renderer.viewport_size();

                renderer.begin_frame(theme.colors.background);
                demo.draw(&mut renderer, &theme, size);
                renderer.end_frame();
            }

            _ => {}
        }

        // Apply any pending view-mode command that may have been set during handling
        let _ = apply_view_cmd(&mut demo, &mut renderer);
        let _ = apply_file_cmd(&mut demo);
    }).unwrap();
    Ok(())
}

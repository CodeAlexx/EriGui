use crate::{Breadcrumb, BreadcrumbItem, Button, ListItem, ListView, TextInput};
use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::fs;
use std::path::{Path, PathBuf};

/// Callback fired when the user selects a file. Argument: selected path.
type FileSelectedCallback = Box<dyn FnMut(&Path)>;
/// Callback fired when the file dialog is cancelled.
type CancelCallback = Box<dyn FnMut()>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileDialogMode {
    Open,
    Save,
    SelectFolder,
}

#[derive(Clone)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: Vec<&str>) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn matches(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }

        // The "*" sentinel matches any path, including extensionless
        // ones like "Makefile" or "README". Mirrors native picker
        // behavior across macOS / Windows / GTK.
        if self.extensions.iter().any(|e| e == "*") {
            return true;
        }

        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self
                    .extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext_str));
            }
        }

        false
    }
}

#[derive(Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

pub struct FileDialog {
    state: WidgetState,
    mode: FileDialogMode,
    current_path: PathBuf,
    selected_file: Option<PathBuf>,

    // UI Components
    breadcrumb: Breadcrumb,
    file_list: ListView,
    filename_input: TextInput,
    filter_dropdown: Button,
    ok_button: Button,
    cancel_button: Button,
    new_folder_button: Button,

    // State
    filters: Vec<FileFilter>,
    selected_filter: usize,
    show_hidden: bool,
    entries: Vec<FileEntry>,

    // Layout rects
    header_rect: Rect,
    content_rect: Rect,
    footer_rect: Rect,

    // Callbacks
    on_file_selected: Option<FileSelectedCallback>,
    on_cancel: Option<CancelCallback>,
}

impl FileDialog {
    pub fn new(id: WidgetId, mode: FileDialogMode) -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let ok_text = match mode {
            FileDialogMode::Open => "Open",
            FileDialogMode::Save => "Save",
            FileDialogMode::SelectFolder => "Select",
        };

        let mut dialog = Self {
            state: WidgetState::new(id),
            mode,
            current_path: home_dir.clone(),
            selected_file: None,
            breadcrumb: Breadcrumb::new(WidgetId::default()),
            file_list: ListView::new(WidgetId::default()),
            filename_input: TextInput::new(WidgetId::default()).with_placeholder("Filename..."),
            filter_dropdown: Button::new(WidgetId::default(), "All Files (*.*)"),
            ok_button: Button::new(WidgetId::default(), ok_text),
            cancel_button: Button::new(WidgetId::default(), "Cancel"),
            new_folder_button: Button::new(WidgetId::default(), "New Folder"),
            filters: vec![FileFilter::new("All Files", vec!["*"])],
            selected_filter: 0,
            show_hidden: false,
            entries: Vec::new(),
            header_rect: Rect::default(),
            content_rect: Rect::default(),
            footer_rect: Rect::default(),
            on_file_selected: None,
            on_cancel: None,
        };

        dialog.navigate_to_path(&home_dir);
        dialog
    }

    pub fn with_filters(mut self, filters: Vec<FileFilter>) -> Self {
        self.filters = filters;
        if !self.filters.is_empty() {
            self.update_filter_button();
        }
        self
    }

    pub fn with_initial_path(mut self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if path.exists() {
            if path.is_dir() {
                self.navigate_to_path(path);
            } else if let Some(parent) = path.parent() {
                self.navigate_to_path(parent);
                self.selected_file = Some(path.to_path_buf());
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        self.filename_input.set_text(name_str);
                    }
                }
            }
        }
        self
    }

    pub fn with_on_file_selected<F: FnMut(&Path) + 'static>(mut self, f: F) -> Self {
        self.on_file_selected = Some(Box::new(f));
        self
    }

    pub fn with_on_cancel<F: FnMut() + 'static>(mut self, f: F) -> Self {
        self.on_cancel = Some(Box::new(f));
        self
    }

    pub fn get_selected_path(&self) -> Option<&Path> {
        self.selected_file.as_deref()
    }

    fn navigate_to_path(&mut self, path: &Path) {
        if !path.exists() || !path.is_dir() {
            return;
        }

        self.current_path = path.to_path_buf();
        self.update_breadcrumb();
        self.refresh_file_list();

        if self.mode != FileDialogMode::Save {
            self.filename_input.set_text("");
        }
    }

    fn update_breadcrumb(&mut self) {
        let mut items = Vec::new();
        let mut current = self.current_path.as_path();
        let mut paths = Vec::new();

        // Build path components
        while let Some(parent) = current.parent() {
            if let Some(name) = current.file_name() {
                if let Some(name_str) = name.to_str() {
                    paths.push((name_str.to_string(), current.to_path_buf()));
                }
            }
            current = parent;
        }

        // Add root
        #[cfg(unix)]
        {
            paths.push(("/".to_string(), PathBuf::from("/")));
        }
        #[cfg(windows)]
        {
            if let Some(prefix) = self.current_path.components().next() {
                if let Some(drive) = prefix.as_os_str().to_str() {
                    paths.push((drive.to_string(), PathBuf::from(drive)));
                }
            }
        }

        // Reverse to get correct order
        paths.reverse();

        // Create breadcrumb items
        for (name, path) in paths {
            items.push(BreadcrumbItem::new(name, path.to_string_lossy()));
        }

        self.breadcrumb.set_path(items);
    }

    fn refresh_file_list(&mut self) {
        self.entries.clear();

        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            for entry in read_dir.filter_map(Result::ok) {
                let path = entry.path();

                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        // Skip hidden files unless show_hidden is true
                        if !self.show_hidden && name_str.starts_with('.') {
                            continue;
                        }

                        let is_dir = path.is_dir();

                        // Apply filter to files
                        if !is_dir && self.mode != FileDialogMode::SelectFolder {
                            let filter = &self.filters[self.selected_filter];
                            if !filter.matches(&path) {
                                continue;
                            }
                        }

                        let size = if is_dir {
                            0
                        } else {
                            entry.metadata().map(|m| m.len()).unwrap_or(0)
                        };

                        self.entries.push(FileEntry {
                            name: name_str.to_string(),
                            path: path.clone(),
                            is_dir,
                            size,
                        });
                    }
                }
            }
        }

        // Sort: directories first, then alphabetically
        self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        // Update list view
        let mut items = Vec::new();
        for entry in &self.entries {
            let icon = if entry.is_dir { "📁" } else { "📄" };
            let size_str = if entry.is_dir {
                String::new()
            } else {
                format_file_size(entry.size)
            };

            let text = if size_str.is_empty() {
                format!("{} {}", icon, entry.name)
            } else {
                format!("{} {} ({})", icon, entry.name, size_str)
            };

            items.push(ListItem {
                id: entry.path.to_string_lossy().to_string(),
                text,
                icon: None,
                selected: false,
            });
        }

        self.file_list = ListView::new(self.file_list.id()).with_items(items);
    }

    fn update_filter_button(&mut self) {
        if self.selected_filter < self.filters.len() {
            let filter = &self.filters[self.selected_filter];
            let text = format!("{} ({})", filter.name, filter.extensions.join(", "));
            self.filter_dropdown = Button::new(self.filter_dropdown.id(), text);
        }
    }

    fn handle_ok(&mut self) {
        match self.mode {
            FileDialogMode::Open => {
                if let Some(selected) = &self.selected_file {
                    if selected.exists() && selected.is_file() {
                        if let Some(callback) = &mut self.on_file_selected {
                            callback(selected);
                        }
                    }
                }
            }
            FileDialogMode::Save => {
                let filename = self.filename_input.get_text();
                // Validate filename for security before proceeding
                if validate_filename(&filename).is_ok() {
                    let mut path = self.current_path.join(&filename);

                    // Add extension if needed
                    if path.extension().is_none() && self.selected_filter < self.filters.len() {
                        let filter = &self.filters[self.selected_filter];
                        if !filter.extensions.is_empty() && filter.extensions[0] != "*" {
                            path.set_extension(&filter.extensions[0]);
                        }
                    }

                    // Additional safety: ensure the final path is still within current_path
                    // This catches edge cases where the filename might be crafted to escape
                    if let (Ok(canonical_base), Ok(canonical_path)) = (
                        self.current_path.canonicalize(),
                        path.parent().and_then(|p| p.canonicalize().ok()).ok_or(()),
                    ) {
                        if canonical_path.starts_with(&canonical_base) {
                            if let Some(callback) = &mut self.on_file_selected {
                                callback(&path);
                            }
                        }
                    } else {
                        // If we can't canonicalize (file doesn't exist yet), allow if
                        // the parent exists and is the current path
                        if path.parent() == Some(self.current_path.as_path()) {
                            if let Some(callback) = &mut self.on_file_selected {
                                callback(&path);
                            }
                        }
                    }
                }
                // Silently ignore invalid filenames - UI should provide feedback
            }
            FileDialogMode::SelectFolder => {
                if let Some(callback) = &mut self.on_file_selected {
                    callback(&self.current_path);
                }
            }
        }
    }
}

/// Validates a filename to prevent path traversal and other security issues.
/// Returns Ok(()) if the filename is safe, or Err with a description of the issue.
fn validate_filename(filename: &str) -> Result<(), &'static str> {
    // Check for empty filename
    if filename.is_empty() {
        return Err("Filename cannot be empty");
    }

    // Check for path traversal attempts
    if filename.contains("..") {
        return Err("Filename cannot contain path traversal sequences (..)");
    }

    // Check for path separators (should be a filename, not a path)
    if filename.contains('/') || filename.contains('\\') {
        return Err("Filename cannot contain path separators");
    }

    // Check for null bytes (could be used for injection attacks)
    if filename.contains('\0') {
        return Err("Filename cannot contain null bytes");
    }

    // Check for leading/trailing whitespace or dots (problematic on some systems)
    let trimmed = filename.trim();
    if trimmed != filename {
        return Err("Filename cannot have leading or trailing whitespace");
    }

    if filename.starts_with('.') && filename.len() == 1 {
        return Err("Filename cannot be just a dot");
    }

    // Check for reserved characters on Windows
    const RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
    for ch in RESERVED_CHARS {
        if filename.contains(*ch) {
            return Err("Filename contains reserved characters");
        }
    }

    // Check for reserved names on Windows (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    let upper = filename.to_uppercase();
    let base_name = upper.split('.').next().unwrap_or(&upper);
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED_NAMES.contains(&base_name) {
        return Err("Filename uses a reserved system name");
    }

    Ok(())
}

fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

impl Widget for FileDialog {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(700, 500)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;

        let padding = 10;
        let header_height = 80;
        let footer_height = 50;

        // Layout regions
        self.header_rect = Rect::new(rect.x(), rect.y(), rect.width(), header_height);

        self.footer_rect = Rect::new(
            rect.x(),
            rect.bottom() - footer_height,
            rect.width(),
            footer_height,
        );

        self.content_rect = Rect::new(
            rect.x(),
            self.header_rect.bottom(),
            rect.width(),
            rect.height() - header_height - footer_height,
        );

        // Layout header components
        let breadcrumb_height = 30;
        self.breadcrumb.layout(
            Rect::new(
                self.header_rect.x() + padding,
                self.header_rect.y() + padding,
                self.header_rect.width() - padding * 2,
                breadcrumb_height,
            ),
            theme,
        );

        // Layout toolbar buttons
        let button_size = Size::new(100, 30);
        let toolbar_y = self.header_rect.y() + padding + breadcrumb_height + 5;

        self.new_folder_button.layout(
            Rect::new(
                self.header_rect.right() - padding - button_size.width,
                toolbar_y,
                button_size.width,
                button_size.height,
            ),
            theme,
        );

        // Layout content (file list)
        self.file_list.layout(
            Rect::new(
                self.content_rect.x() + padding,
                self.content_rect.y() + padding,
                self.content_rect.width() - padding * 2,
                self.content_rect.height() - padding * 2,
            ),
            theme,
        );

        // Layout footer components
        let input_width = 300;
        let button_width = 80;
        let filter_width = 150;

        // Filename input (for save mode)
        if self.mode == FileDialogMode::Save {
            self.filename_input.layout(
                Rect::new(
                    self.footer_rect.x() + padding,
                    self.footer_rect.y() + (footer_height - 30) / 2,
                    input_width,
                    30,
                ),
                theme,
            );
        }

        // Filter dropdown
        self.filter_dropdown.layout(
            Rect::new(
                self.footer_rect.x()
                    + padding
                    + (if self.mode == FileDialogMode::Save {
                        input_width + 10
                    } else {
                        0
                    }),
                self.footer_rect.y() + (footer_height - 30) / 2,
                filter_width,
                30,
            ),
            theme,
        );

        // Cancel button
        self.cancel_button.layout(
            Rect::new(
                self.footer_rect.right() - padding - button_width,
                self.footer_rect.y() + (footer_height - 30) / 2,
                button_width,
                30,
            ),
            theme,
        );

        // OK button
        self.ok_button.layout(
            Rect::new(
                self.footer_rect.right() - padding - button_width * 2 - 10,
                self.footer_rect.y() + (footer_height - 30) / 2,
                button_width,
                30,
            ),
            theme,
        );
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);

        // Draw header background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.header_rect);

        // Draw separator
        context.set_color(theme.colors.border);
        context.fill_rect(Rect::new(
            self.header_rect.x(),
            self.header_rect.bottom() - 1,
            self.header_rect.width(),
            1,
        ));

        // Draw footer background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.footer_rect);

        // Draw separator
        context.set_color(theme.colors.border);
        context.fill_rect(Rect::new(
            self.footer_rect.x(),
            self.footer_rect.y(),
            self.footer_rect.width(),
            1,
        ));

        // Draw components
        self.breadcrumb.draw(context, theme);
        self.new_folder_button.draw(context, theme);
        self.file_list.draw(context, theme);

        if self.mode == FileDialogMode::Save {
            self.filename_input.draw(context, theme);
        }

        self.filter_dropdown.draw(context, theme);
        self.ok_button.draw(context, theme);
        self.cancel_button.draw(context, theme);
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Handle component events
        let breadcrumb_result = self.breadcrumb.handle_event(event, theme);
        let file_list_result = self.file_list.handle_event(event, theme);
        let filename_result = if self.mode == FileDialogMode::Save {
            self.filename_input.handle_event(event, theme)
        } else {
            EventResult::Ignored
        };
        let new_folder_result = self.new_folder_button.handle_event(event, theme);
        let ok_result = self.ok_button.handle_event(event, theme);
        let cancel_result = self.cancel_button.handle_event(event, theme);

        // Handle file selection
        if let Some(item) = self.file_list.selected_item() {
            let selected_index = self
                .entries
                .iter()
                .position(|e| e.path.to_string_lossy() == item.id)
                .unwrap_or(0);
            if selected_index < self.entries.len() {
                let entry = &self.entries[selected_index];

                if !entry.is_dir {
                    self.selected_file = Some(entry.path.clone());
                    if let Some(name) = entry.path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            self.filename_input.set_text(name_str);
                        }
                    }
                }
            }
        }

        // Handle double-click on directories
        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if *pressed {
                    // Check for double-click on file list
                    if self.file_list.bounds().contains(*position) {
                        if let Some(item) = self.file_list.selected_item() {
                            let selected_index = self
                                .entries
                                .iter()
                                .position(|e| e.path.to_string_lossy() == item.id)
                                .unwrap_or(0);
                            if selected_index < self.entries.len() {
                                let is_dir = self.entries[selected_index].is_dir;
                                let path = self.entries[selected_index].path.clone();

                                if is_dir {
                                    self.navigate_to_path(&path);
                                    return EventResult::Consumed;
                                } else if self.mode == FileDialogMode::Open {
                                    self.selected_file = Some(path);
                                    self.handle_ok();
                                    return EventResult::Consumed;
                                }
                            }
                        }
                    }
                } else {
                    // Handle button clicks on release
                    if self.ok_button.bounds().contains(*position) {
                        self.handle_ok();
                        return EventResult::Consumed;
                    }

                    if self.cancel_button.bounds().contains(*position) {
                        if let Some(callback) = &mut self.on_cancel {
                            callback();
                        }
                        return EventResult::Consumed;
                    }

                    if self.new_folder_button.bounds().contains(*position) {
                        // TODO: Implement new folder creation
                        return EventResult::Consumed;
                    }
                }
            }
            Event::KeyPress(KeyPressEvent { key, .. }) => match key {
                Key::Enter => {
                    if self.filename_input.is_focused() || self.file_list.is_focused() {
                        self.handle_ok();
                        return EventResult::Consumed;
                    }
                }
                Key::Escape => {
                    if let Some(callback) = &mut self.on_cancel {
                        callback();
                    }
                    return EventResult::Consumed;
                }
                _ => {}
            },
            _ => {}
        }

        // Handle breadcrumb navigation
        if breadcrumb_result == EventResult::Consumed {
            // Get clicked path from breadcrumb
            let path_items = self.breadcrumb.get_path();
            if !path_items.is_empty() {
                // Reconstruct path from breadcrumb items
                let mut path = PathBuf::new();
                for item in path_items {
                    path.push(item);
                }
                self.navigate_to_path(&path);
            }
            return EventResult::Consumed;
        }

        if file_list_result == EventResult::Consumed
            || filename_result == EventResult::Consumed
            || new_folder_result == EventResult::Consumed
            || ok_result == EventResult::Consumed
            || cancel_result == EventResult::Consumed
        {
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }

    fn bounds(&self) -> Rect {
        self.state.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
    }

    fn is_visible(&self) -> bool {
        self.state.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.state.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
    }

    fn is_focused(&self) -> bool {
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
    }

    fn can_focus(&self) -> bool {
        self.state.enabled && self.state.visible
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

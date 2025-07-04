use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent, VirtualKeyCode, MouseScrollDelta},
    event_loop::{ControlFlow, EventLoop},
    dpi::PhysicalPosition,
};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use std::collections::HashMap;

struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
    selected: bool,
}

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
    
    address_bar: TextInput,
    search_bar: TextInput,
    
    view_combo: ComboBox,
    sort_combo: ComboBox,
    
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Icons,
    List,
    Details,
}

impl FileManagerDemo {
    fn new() -> Self {
        // Create menu bar
        let mut menu_bar = MenuBar::new(WidgetId::default());
        
        // File menu
        menu_bar.add_menu("File", vec![
            MenuItem::new("New Folder").with_shortcut("Ctrl+N").with_on_click(|| println!("New Folder")),
            MenuItem::new("Open").with_shortcut("Ctrl+O").with_on_click(|| println!("Open")),
            MenuItem::separator(),
            MenuItem::new("Properties").with_shortcut("Alt+Enter").with_on_click(|| println!("Properties")),
            MenuItem::separator(),
            MenuItem::new("Exit").with_shortcut("Ctrl+Q").with_on_click(|| std::process::exit(0)),
        ]);
        
        // Edit menu
        menu_bar.add_menu("Edit", vec![
            MenuItem::new("Cut").with_shortcut("Ctrl+X").with_on_click(|| println!("Cut")),
            MenuItem::new("Copy").with_shortcut("Ctrl+C").with_on_click(|| println!("Copy")),
            MenuItem::new("Paste").with_shortcut("Ctrl+V").with_on_click(|| println!("Paste")),
            MenuItem::separator(),
            MenuItem::new("Select All").with_shortcut("Ctrl+A").with_on_click(|| println!("Select All")),
        ]);
        
        // View menu
        menu_bar.add_menu("View", vec![
            MenuItem::new("Icons").with_on_click(|| println!("Icons view")),
            MenuItem::new("List").with_on_click(|| println!("List view")),
            MenuItem::new("Details").with_on_click(|| println!("Details view")),
            MenuItem::separator(),
            MenuItem::new("Show Hidden Files").with_shortcut("Ctrl+H").with_on_click(|| println!("Toggle hidden")),
            MenuItem::separator(),
            MenuItem::new("Refresh").with_shortcut("F5").with_on_click(|| println!("Refresh")),
        ]);
        
        // Go menu
        menu_bar.add_menu("Go", vec![
            MenuItem::new("Back").with_shortcut("Alt+Left").with_on_click(|| println!("Back")),
            MenuItem::new("Forward").with_shortcut("Alt+Right").with_on_click(|| println!("Forward")),
            MenuItem::new("Up").with_shortcut("Alt+Up").with_on_click(|| println!("Up")),
            MenuItem::separator(),
            MenuItem::new("Home").with_shortcut("Ctrl+Home").with_on_click(|| println!("Home")),
            MenuItem::new("Documents").with_on_click(|| println!("Documents")),
            MenuItem::new("Downloads").with_on_click(|| println!("Downloads")),
        ]);
        
        // Create widgets
        let back_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Back)
            .with_on_click(|| println!("Back"));
        let forward_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Forward)
            .with_on_click(|| println!("Forward"));
        let up_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Up)
            .with_on_click(|| println!("Up"));
        let home_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Home)
            .with_on_click(|| println!("Home"));
        let refresh_button = Button::new(WidgetId::default(), "")
            .with_icon(ButtonIcon::Refresh)
            .with_on_click(|| println!("Refresh"));
        let new_folder_button = Button::new(WidgetId::default(), "New Folder")
            .with_icon(ButtonIcon::FolderNew)
            .with_on_click(|| println!("New Folder"));
        let delete_button = Button::new(WidgetId::default(), "Delete")
            .with_icon(ButtonIcon::Delete)
            .with_on_click(|| println!("Delete"));
        
        let address_bar = TextInput::new(WidgetId::default())
            .with_placeholder("Enter path...");
        let search_bar = TextInput::new(WidgetId::default())
            .with_placeholder("Search...");
        
        let view_combo = ComboBox::new(WidgetId::default())
            .with_items(vec![
                "🖼 Icons".to_string(),
                "📋 List".to_string(),
                "📊 Details".to_string(),
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
            let home_node = TreeNode::new("home".to_string(), "🏠 Home".to_string());
            sidebar_paths.insert("home".to_string(), home);
            sidebar_tree.add_root_node(home_node);
        }
        
        if let Some(docs) = dirs::document_dir() {
            let documents_node = TreeNode::new("documents".to_string(), "📄 Documents".to_string());
            sidebar_paths.insert("documents".to_string(), docs);
            sidebar_tree.add_root_node(documents_node);
        }
        
        if let Some(downloads) = dirs::download_dir() {
            let downloads_node = TreeNode::new("downloads".to_string(), "📥 Downloads".to_string());
            sidebar_paths.insert("downloads".to_string(), downloads);
            sidebar_tree.add_root_node(downloads_node);
        }
        
        if let Some(pics) = dirs::picture_dir() {
            let pictures_node = TreeNode::new("pictures".to_string(), "🖼 Pictures".to_string());
            sidebar_paths.insert("pictures".to_string(), pics);
            sidebar_tree.add_root_node(pictures_node);
        }
        
        if let Some(music) = dirs::audio_dir() {
            let music_node = TreeNode::new("music".to_string(), "🎵 Music".to_string());
            sidebar_paths.insert("music".to_string(), music);
            sidebar_tree.add_root_node(music_node);
        }
        
        if let Some(videos) = dirs::video_dir() {
            let videos_node = TreeNode::new("videos".to_string(), "🎬 Videos".to_string());
            sidebar_paths.insert("videos".to_string(), videos);
            sidebar_tree.add_root_node(videos_node);
        }
        
        // Status bar
        let mut status_bar = StatusBar::new(WidgetId::default());
        status_bar.add_panel(StatusPanel::new("Ready"));
        status_bar.add_panel(StatusPanel::new("0 items").with_spring_width());
        status_bar.add_panel(StatusPanel::new("0 bytes"));
        
        // Context menu
        let context_menu = ContextMenu::new(WidgetId::default())
            .with_items(vec![
                ContextMenuItem::new("Open"),
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
            address_bar,
            search_bar,
            view_combo,
            sort_combo,
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
            
            self.address_bar.set_text(self.current_path.to_string_lossy().to_string());
            self.refresh_file_list();
            self.update_navigation_buttons();
            self.scroll_offset = 0;
        }
    }
    
    fn navigate_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.address_bar.set_text(self.current_path.to_string_lossy().to_string());
            self.refresh_file_list();
            self.update_navigation_buttons();
        }
    }
    
    fn navigate_forward(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            self.address_bar.set_text(self.current_path.to_string_lossy().to_string());
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
        self.forward_button.set_enabled(self.history_index < self.history.len() - 1);
        self.up_button.set_enabled(self.current_path.parent().is_some());
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
        
        // Sort entries
        self.sort_entries();
        self.update_status_bar();
    }
    
    fn sort_entries(&mut self) {
        let sort_index = self.sort_combo.selected_index().unwrap_or(0);
        
        self.file_entries.sort_by(|a, b| {
            // Directories first
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            match sort_index {
                0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()), // Name
                1 => a.size.cmp(&b.size), // Size
                2 => { // Type (extension)
                    let ext_a = Path::new(&a.name).extension().and_then(|s| s.to_str()).unwrap_or("");
                    let ext_b = Path::new(&b.name).extension().and_then(|s| s.to_str()).unwrap_or("");
                    ext_a.cmp(ext_b)
                }
                3 => { // Modified
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
    }
    
    fn update_status_bar(&mut self) {
        let total_items = self.file_entries.len();
        let selected_count = self.file_entries.iter().filter(|e| e.selected).count();
        let total_size: u64 = self.file_entries.iter()
            .filter(|e| e.selected || selected_count == 0)
            .map(|e| e.size)
            .sum();
        
        if selected_count > 0 {
            self.status_bar.set_panel_text(0, &format!("{} selected", selected_count));
        } else {
            self.status_bar.set_panel_text(0, "Ready");
        }
        
        self.status_bar.set_panel_text(1, &format!("{} items", total_items));
        self.status_bar.set_panel_text(2, &format_size(total_size));
    }
    
    fn get_file_icon(entry: &FileEntry) -> &'static str {
        if entry.is_dir {
            "📁"
        } else {
            match Path::new(&entry.name).extension().and_then(|s| s.to_str()) {
                Some("txt") | Some("md") => "📄",
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("bmp") => "🖼",
                Some("mp3") | Some("wav") | Some("ogg") | Some("flac") => "🎵",
                Some("mp4") | Some("avi") | Some("mkv") | Some("mov") => "🎬",
                Some("zip") | Some("tar") | Some("gz") | Some("7z") | Some("rar") => "📦",
                Some("pdf") => "📕",
                Some("doc") | Some("docx") => "📘",
                Some("xls") | Some("xlsx") => "📗",
                Some("rs") | Some("c") | Some("cpp") | Some("h") | Some("py") | Some("js") => "💻",
                _ => "📄",
            }
        }
    }
    
    fn handle_file_click(&mut self, index: usize, double_click: bool) {
        if index >= self.file_entries.len() {
            return;
        }
        
        if double_click && self.file_entries[index].is_dir {
            let path = self.file_entries[index].path.clone();
            self.navigate_to(path);
        } else {
            // Handle selection
            if self.ctrl_pressed {
                self.file_entries[index].selected = !self.file_entries[index].selected;
            } else if self.shift_pressed && self.selection_start.is_some() {
                let start = self.selection_start.unwrap();
                let (from, to) = if start < index { (start, index) } else { (index, start) };
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
        let menu_height = 30;
        let menu_rect = Rect::new(0, 0, bounds.width(), menu_height);
        let toolbar_rect = Rect::new(0, menu_height, bounds.width(), self.toolbar_height);
        let path_bar_rect = Rect::new(0, menu_height + self.toolbar_height, bounds.width(), 30);
        let content_rect = Rect::new(
            0,
            menu_height + self.toolbar_height + 30,
            bounds.width(),
            bounds.height() - menu_height - self.toolbar_height - 30 - self.status_height
        );
        let sidebar_rect = Rect::new(0, content_rect.y(), self.sidebar_width, content_rect.height());
        let divider_rect = Rect::new(self.sidebar_width, content_rect.y(), 4, content_rect.height());
        let file_area_rect = Rect::new(
            self.sidebar_width + 4,
            content_rect.y(),
            content_rect.width() - self.sidebar_width - 4,
            content_rect.height()
        );
        let status_rect = Rect::new(0, bounds.height() - self.status_height, bounds.width(), self.status_height);
        
        // Draw menu bar FIRST
        self.menu_bar.set_bounds(menu_rect);
        self.menu_bar.draw(context, theme);
        
        // Draw separator line between menu and toolbar
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(0, menu_height),
            Point::new(bounds.width(), menu_height),
            1
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
            context.fill_rect(Rect::new(divider_rect.x(), divider_rect.y(), 2, divider_rect.height()));
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
        
        // Draw menu dropdown LAST so it appears on top of everything
        self.menu_bar.draw_dropdown_only(context, theme);
    }
    
    fn draw_toolbar(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        // Background
        context.set_color(theme.colors.surface);
        context.fill_rect(bounds);
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(bounds.x(), bounds.bottom() - 1),
            Point::new(bounds.right(), bounds.bottom() - 1),
            1
        );
        
        // Layout buttons
        let mut x = 5;
        let button_size = 30;
        let spacing = 5;
        
        // Navigation buttons
        self.back_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.back_button.draw(context, theme);
        x += button_size + spacing;
        
        self.forward_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.forward_button.draw(context, theme);
        x += button_size + spacing;
        
        self.up_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.up_button.draw(context, theme);
        x += button_size + spacing + 10;
        
        // Home button
        self.home_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.home_button.draw(context, theme);
        x += button_size + spacing + 10;
        
        // Action buttons
        self.refresh_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.refresh_button.draw(context, theme);
        x += button_size + spacing;
        
        self.new_folder_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size + 10, button_size));
        self.new_folder_button.draw(context, theme);
        x += button_size + 10 + spacing;
        
        self.delete_button.set_bounds(Rect::new(x, bounds.y() + 5, button_size, button_size));
        self.delete_button.draw(context, theme);
        
        // View and sort combos on the right
        let combo_width = 120;
        x = bounds.right() - combo_width - 5;
        
        self.sort_combo.set_bounds(Rect::new(x, bounds.y() + 5, combo_width, button_size));
        self.sort_combo.draw(context, theme);
        x -= combo_width + spacing;
        
        self.view_combo.set_bounds(Rect::new(x, bounds.y() + 5, combo_width, button_size));
        self.view_combo.draw(context, theme);
        x -= 150 + spacing;
        
        // Search bar
        self.search_bar.set_bounds(Rect::new(x, bounds.y() + 5, 150, button_size));
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
        self.sidebar_tree.set_bounds(Rect::new(bounds.x(), bounds.y(), bounds.width(), bounds.height()));
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
            let icon = Self::get_file_icon(entry);
            context.set_color(theme.colors.text);
            context.draw_text(
                icon,
                Point::new(x + (item_width - padding) / 2 - 16, y + 16),
                32
            );
            
            // Draw name (truncated if needed)
            let name = &entry.name;
            let _text_width = item_width - padding - 10;
            let truncated = if name.len() > 12 {
                format!("{}...", &name[..9])
            } else {
                name.clone()
            };
            
            context.set_color(if entry.selected { theme.colors.primary } else { theme.colors.text });
            let text_x = x + 5;
            let text_y = y + icon_size + 5;
            
            // Draw text with wrapping
            let lines: Vec<&str> = truncated.split_whitespace().collect();
            for (i, line) in lines.iter().enumerate() {
                context.draw_text(
                    line,
                    Point::new(text_x, text_y + i as i32 * 12),
                    11
                );
            }
        }
    }
    
    fn draw_list_view(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        let item_height = 24;
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
            let icon = Self::get_file_icon(entry);
            context.set_color(theme.colors.text);
            context.draw_text(
                icon,
                Point::new(bounds.x() + padding, y + 4),
                icon_size
            );
            
            // Draw name
            context.set_color(if entry.selected { theme.colors.primary } else { theme.colors.text });
            context.draw_text(
                &entry.name,
                Point::new(bounds.x() + padding + icon_size + 5, y + 4),
                14
            );
        }
    }
    
    fn draw_details_view(&mut self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect) {
        let item_height = 24;
        let padding = 5;
        let icon_size = 16;
        
        // Column widths
        let name_width = bounds.width() / 2;
        let size_width = 100;
        let type_width = 100;
        let _modified_width = 150;
        
        // Draw header
        let header_rect = Rect::new(bounds.x(), bounds.y(), bounds.width(), item_height);
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(header_rect);
        
        context.set_color(theme.colors.text_secondary);
        context.draw_text("Name", Point::new(bounds.x() + padding, bounds.y() + 4), 12);
        context.draw_text("Size", Point::new(bounds.x() + name_width, bounds.y() + 4), 12);
        context.draw_text("Type", Point::new(bounds.x() + name_width + size_width, bounds.y() + 4), 12);
        context.draw_text("Modified", Point::new(bounds.x() + name_width + size_width + type_width, bounds.y() + 4), 12);
        
        // Draw separator
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(bounds.x(), bounds.y() + item_height),
            Point::new(bounds.right(), bounds.y() + item_height),
            1
        );
        
        let start_y = bounds.y() + item_height + padding - self.scroll_offset;
        
        for (index, entry) in self.file_entries.iter().enumerate() {
            let y = start_y + index as i32 * item_height;
            
            // Skip if outside visible area
            if y + item_height < bounds.y() + item_height || y > bounds.bottom() {
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
            
            // Draw icon and name
            let icon = Self::get_file_icon(entry);
            context.set_color(theme.colors.text);
            context.draw_text(icon, Point::new(bounds.x() + padding, y + 4), icon_size);
            
            context.set_color(if entry.selected { theme.colors.primary } else { theme.colors.text });
            context.draw_text(
                &entry.name,
                Point::new(bounds.x() + padding + icon_size + 5, y + 4),
                14
            );
            
            // Draw size
            if !entry.is_dir {
                context.draw_text(
                    &format_size(entry.size),
                    Point::new(bounds.x() + name_width, y + 4),
                    14
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
            context.draw_text(
                file_type,
                Point::new(bounds.x() + name_width + size_width, y + 4),
                14
            );
            
            // Draw modified time
            if let Some(modified) = entry.modified {
                let time_str = format_time(modified);
                context.draw_text(
                    &time_str,
                    Point::new(bounds.x() + name_width + size_width + type_width, y + 4),
                    14
                );
            }
        }
    }
    
    fn draw_scrollbar(&self, context: &mut dyn DrawContext, theme: &Theme, bounds: Rect, content_height: i32) {
        let scrollbar_width = 12;
        let scrollbar_rect = Rect::new(
            bounds.right() - scrollbar_width,
            bounds.y(),
            scrollbar_width,
            bounds.height()
        );
        
        // Background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(scrollbar_rect);
        
        // Thumb
        let thumb_height = (bounds.height() as f32 * bounds.height() as f32 / content_height as f32) as i32;
        let thumb_height = thumb_height.max(20);
        let max_scroll = content_height - bounds.height();
        let thumb_y = if max_scroll > 0 {
            bounds.y() + (self.scroll_offset as f32 * (bounds.height() - thumb_height) as f32 / max_scroll as f32) as i32
        } else {
            bounds.y()
        };
        
        let thumb_rect = Rect::new(
            scrollbar_rect.x() + 2,
            thumb_y,
            scrollbar_width - 4,
            thumb_height
        );
        
        context.set_color(theme.colors.primary);
        context.fill_rounded_rect(thumb_rect, 4);
    }
    
    fn calculate_content_height(&self, _bounds: Rect) -> i32 {
        match self.view_mode {
            ViewMode::Icons => {
                let rows = (self.file_entries.len() as i32 + self.items_per_row - 1) / self.items_per_row;
                rows * (self.item_size + 40) + 20
            }
            ViewMode::List => {
                self.file_entries.len() as i32 * 24 + 10
            }
            ViewMode::Details => {
                (self.file_entries.len() + 1) as i32 * 24 + 10
            }
        }
    }
    
    fn handle_mouse_move(&mut self, pos: Point, bounds: Rect) {
        self.last_mouse_pos = pos;
        
        // Check if hovering over divider
        let divider_rect = Rect::new(self.sidebar_width, 0, 4, bounds.height());
        if divider_rect.contains(pos) && !self.dragging_divider {
            // Show resize cursor (in a real app)
        }
        
        // Update hovered file
        let file_area_rect = Rect::new(
            self.sidebar_width + 4,
            self.toolbar_height + 30,
            bounds.width() - self.sidebar_width - 4,
            bounds.height() - self.toolbar_height - 30 - self.status_height
        );
        
        if file_area_rect.contains(pos) {
            self.hovered_index = self.get_item_at_position(pos, file_area_rect);
        } else {
            self.hovered_index = None;
        }
    }
    
    fn handle_mouse_down(&mut self, pos: Point, button: MouseButton, bounds: Rect) -> bool {
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
        
        // Check file area
        let file_area_rect = Rect::new(
            self.sidebar_width + 4,
            self.toolbar_height + 30,
            bounds.width() - self.sidebar_width - 4,
            bounds.height() - self.toolbar_height - 30 - self.status_height
        );
        
        if file_area_rect.contains(pos) {
            if let Some(index) = self.get_item_at_position(pos, file_area_rect) {
                if button == MouseButton::Left {
                    // Simple double-click detection (in real app, track timing)
                    let double_click = self.focused_index == Some(index);
                    self.handle_file_click(index, double_click);
                    return true;
                } else if button == MouseButton::Right {
                    self.context_menu.show_at(pos);
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
            if self.sidebar_tree.handle_event(&event, &Theme::dark()).is_consumed() {
                // TODO: Handle tree view selection
                // TreeView doesn't have selected_node method yet
                return true;
            }
        }
        
        false
    }
    
    fn handle_mouse_up(&mut self, _pos: Point, _button: MouseButton) {
        self.dragging_divider = false;
    }
    
    fn handle_scroll(&mut self, delta: i32, bounds: Rect) {
        let content_height = self.calculate_content_height(bounds);
        let max_scroll = (content_height - bounds.height()).max(0);
        
        self.scroll_offset = (self.scroll_offset - delta * 40).clamp(0, max_scroll);
    }
    
    fn handle_key(&mut self, key: &Key, pressed: bool) -> bool {
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
            _ => false
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
                let item_height = 24;
                let rel_y = pos.y - bounds.y() + self.scroll_offset;
                
                if self.view_mode == ViewMode::Details {
                    // Account for header
                    let rel_y = rel_y - item_height;
                    if rel_y >= 0 {
                        let index = (rel_y / item_height) as usize;
                        if index < self.file_entries.len() {
                            return Some(index);
                        }
                    }
                } else {
                    let index = (rel_y / item_height) as usize;
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

fn format_time(time: SystemTime) -> String {
    if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
        let datetime = chrono::DateTime::<chrono::Local>::from(SystemTime::UNIX_EPOCH + duration);
        datetime.format("%Y-%m-%d %H:%M").to_string()
    } else {
        String::new()
    }
}

fn main() -> erigui_core::Result<()> {
    env_logger::init();
    
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 1200, 800, "EriGui File Manager")?;
    
    let theme = Theme::dark();
    let mut demo = FileManagerDemo::new();
    let mut last_cursor_pos = PhysicalPosition::new(0.0, 0.0);
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size.width, physical_size.height);
                    renderer.window().request_redraw();
                }
                
                WindowEvent::CursorMoved { position, .. } => {
                    last_cursor_pos = position;
                    let pos = Point::new(position.x as i32, position.y as i32);
                    let size = renderer.viewport_size();
                    let bounds = Rect::new(0, 0, size.width, size.height);
                    
                    // Always handle menu bar mouse move (for dropdowns)
                    let event = Event::MouseMove(MouseMoveEvent {
                        position: pos,
                        delta: Point::new(pos.x - demo.last_mouse_pos.x, pos.y - demo.last_mouse_pos.y),
                        modifiers: Modifiers::empty(),
                    });
                    demo.menu_bar.handle_event(&event, &theme);
                    
                    demo.handle_mouse_move(pos, bounds);
                    
                    if demo.dragging_divider {
                        demo.sidebar_width = (pos.x - 2).clamp(150, 400);
                    }
                    
                    renderer.window().request_redraw();
                }
                
                WindowEvent::MouseInput { state, button, .. } => {
                    let pos = Point::new(last_cursor_pos.x as i32, last_cursor_pos.y as i32);
                    let size = renderer.viewport_size();
                    let bounds = Rect::new(0, 0, size.width, size.height);
                    
                    let button = match button {
                        winit::event::MouseButton::Left => MouseButton::Left,
                        winit::event::MouseButton::Right => MouseButton::Right,
                        winit::event::MouseButton::Middle => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };
                    
                    match state {
                        winit::event::ElementState::Pressed => {
                            // Handle menu bar first
                            let menu_height = 30;  // Must match the actual menu height!
                            let menu_rect = Rect::new(0, 0, bounds.width(), menu_height);
                            if menu_rect.contains(pos) {
                                let event = Event::MouseButton(MouseButtonEvent {
                                    button,
                                    position: pos,
                                    pressed: true,
                                    modifiers: Modifiers::empty(),
                                });
                                if demo.menu_bar.handle_event(&event, &theme).is_consumed() {
                                    renderer.window().request_redraw();
                                    return;
                                }
                            }
                            
                            demo.handle_mouse_down(pos, button, bounds);
                        }
                        winit::event::ElementState::Released => {
                            // Handle menu bar release
                            let event = Event::MouseButton(MouseButtonEvent {
                                button,
                                position: pos,
                                pressed: false,
                                modifiers: Modifiers::empty(),
                            });
                            demo.menu_bar.handle_event(&event, &theme);
                            
                            demo.handle_mouse_up(pos, button);
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
                    demo.handle_scroll(scroll_amount, bounds);
                    
                    renderer.window().request_redraw();
                }
                
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        let key = match keycode {
                            VirtualKeyCode::Back => Key::Backspace,
                            VirtualKeyCode::Return => Key::Enter,
                            VirtualKeyCode::Escape => Key::Escape,
                            VirtualKeyCode::Tab => Key::Tab,
                            VirtualKeyCode::Space => Key::Space,
                            VirtualKeyCode::Delete => Key::Delete,
                            VirtualKeyCode::Up => Key::Up,
                            VirtualKeyCode::Down => Key::Down,
                            VirtualKeyCode::Left => Key::Left,
                            VirtualKeyCode::Right => Key::Right,
                            VirtualKeyCode::F5 => Key::F1, // Using F1 as we don't have F5
                            VirtualKeyCode::LControl => Key::LeftCtrl,
                            VirtualKeyCode::RControl => Key::RightCtrl,
                            VirtualKeyCode::LShift => Key::LeftShift,
                            VirtualKeyCode::RShift => Key::RightShift,
                            _ => Key::Unknown(0),
                        };
                        
                        if !matches!(key, Key::Unknown(_)) {
                            let pressed = input.state == winit::event::ElementState::Pressed;
                            
                            // Handle special keys
                            if matches!(key, Key::F1) && pressed { // F5 refresh
                                demo.refresh_file_list();
                            } else {
                                demo.handle_key(&key, pressed);
                            }
                            
                            // Handle widget events
                            let event = if pressed {
                                Event::KeyPress(KeyPressEvent {
                                    key: key.clone(),
                                    modifiers: Modifiers::empty(),
                                    repeat: false,
                                })
                            } else {
                                Event::KeyRelease(KeyReleaseEvent {
                                    key: key.clone(),
                                    modifiers: Modifiers::empty(),
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
                            }
                            
                            // Handle view mode change
                            let old_view = demo.view_combo.selected_index();
                            demo.view_combo.handle_event(&event, &theme);
                            if old_view != demo.view_combo.selected_index() {
                                demo.view_mode = match demo.view_combo.selected_index() {
                                    Some(0) => ViewMode::Icons,
                                    Some(1) => ViewMode::List,
                                    Some(2) => ViewMode::Details,
                                    _ => ViewMode::Icons,
                                };
                            }
                            
                            // Handle sort change
                            let old_sort = demo.sort_combo.selected_index();
                            demo.sort_combo.handle_event(&event, &theme);
                            if old_sort != demo.sort_combo.selected_index() {
                                demo.sort_entries();
                            }
                            
                            renderer.window().request_redraw();
                        }
                    }
                }
                
                WindowEvent::ReceivedCharacter(ch) => {
                    let event = Event::KeyPress(KeyPressEvent {
                        key: Key::Character(ch),
                        modifiers: Modifiers::empty(),
                        repeat: false,
                    });
                    
                    if demo.address_bar.is_focused() {
                        demo.address_bar.handle_event(&event, &theme);
                    } else if demo.search_bar.is_focused() {
                        demo.search_bar.handle_event(&event, &theme);
                    }
                    
                    renderer.window().request_redraw();
                }
                
                _ => {}
            },
            
            WinitEvent::RedrawRequested(_) => {
                let size = renderer.viewport_size();
                
                renderer.begin_frame(theme.colors.background);
                demo.draw(&mut renderer, &theme, size);
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}
# EriGui Widget Guide

This guide provides comprehensive documentation and examples for all widgets available in the EriGui library.

## Table of Contents

1. [Basic Widgets](#basic-widgets)
   - [Button](#button)
   - [Label](#label)
   - [TextInput](#textinput)
   - [CheckBox](#checkbox)
   - [RadioButton](#radiobutton)
   - [Slider](#slider)
   - [ProgressBar](#progressbar)
   - [SpinBox](#spinbox)
2. [Container Widgets](#container-widgets)
   - [Container](#container)
   - [ScrollView](#scrollview)
   - [TabControl](#tabcontrol)
   - [Accordion](#accordion)
3. [Selection Widgets](#selection-widgets)
   - [ComboBox](#combobox)
   - [ListView](#listview)
   - [TreeView](#treeview)
   - [ColorPicker](#colorpicker)
4. [Input Widgets](#input-widgets)
   - [TextArea](#textarea)
   - [SearchBox](#searchbox)
   - [DateTimePicker](#datetimepicker)
5. [Navigation Widgets](#navigation-widgets)
   - [Menu](#menu)
   - [ContextMenu](#contextmenu)
   - [Toolbar](#toolbar)
   - [Breadcrumb](#breadcrumb)
   - [StatusBar](#statusbar)
6. [Dialog Widgets](#dialog-widgets)
   - [Dialog](#dialog)
   - [FileDialog](#filedialog)
   - [Notification](#notification)
7. [Advanced Widgets](#advanced-widgets)
   - [DockPanel](#dockpanel)
   - [FileManager](#filemanager)
8. [Enhancement Systems](#enhancement-systems)
   - [Drag & Drop](#drag--drop)
   - [Keyboard Navigation](#keyboard-navigation)
   - [Accessibility](#accessibility)
   - [Tooltip System](#tooltip-system)

---

## Basic Widgets

### Button

A clickable button with optional icon support.

```rust
use erigui_widgets::*;

// Basic button
let button = Button::new(WidgetId::default(), "Click Me");

// Button with icon
let icon_button = Button::new(WidgetId::default(), "Save")
    .with_icon("save");

// Button with click handler
let action_button = Button::new(WidgetId::default(), "Submit")
    .with_on_click(|| {
        println!("Button clicked!");
    });

// Disabled button
let mut disabled_button = Button::new(WidgetId::default(), "Disabled");
disabled_button.set_enabled(false);
```

### Label

A simple text display widget.

```rust
// Basic label
let label = Label::new(WidgetId::default(), "Hello, World!");

// Multi-line label
let multi_label = Label::new(WidgetId::default(), "Line 1\nLine 2\nLine 3");

// Dynamic label
let mut status_label = Label::new(WidgetId::default(), "Status: Ready");
// Update text later
status_label.set_text("Status: Processing...");
```

### TextInput

Single-line text input field.

```rust
// Basic text input
let text_input = TextInput::new(WidgetId::default());

// With placeholder
let email_input = TextInput::new(WidgetId::default())
    .with_placeholder("Enter your email...");

// Password input
let password_input = TextInput::new(WidgetId::default())
    .with_password(true);

// With change handler
let search_input = TextInput::new(WidgetId::default())
    .with_on_change(|text| {
        println!("Text changed: {}", text);
    });

// Get/set text
let mut name_input = TextInput::new(WidgetId::default());
name_input.set_text("John Doe");
let name = name_input.get_text();
```

### CheckBox

A toggle box for boolean options.

```rust
// Basic checkbox
let checkbox = CheckBox::new(WidgetId::default(), "Enable notifications");

// Pre-checked
let agree_checkbox = CheckBox::new(WidgetId::default(), "I agree to terms")
    .with_checked(true);

// With change handler
let mut dark_mode = CheckBox::new(WidgetId::default(), "Dark Mode")
    .with_on_change(|checked| {
        println!("Dark mode: {}", checked);
    });

// Check state
if dark_mode.is_checked() {
    // Apply dark theme
}
```

### RadioButton

Radio buttons for mutually exclusive options.

```rust
// Create radio group
let group = RadioButtonGroup::new("theme");

// Create radio buttons
let light_radio = RadioButton::new(WidgetId::default(), "Light Theme", group.clone());
let dark_radio = RadioButton::new(WidgetId::default(), "Dark Theme", group.clone());
let auto_radio = RadioButton::new(WidgetId::default(), "Auto", group.clone())
    .with_checked(true); // Default selection

// With change handler
let size_group = RadioButtonGroup::new("size")
    .with_on_change(|value| {
        println!("Selected size: {:?}", value);
    });

let small = RadioButton::new(WidgetId::default(), "Small", size_group.clone())
    .with_value("S");
let medium = RadioButton::new(WidgetId::default(), "Medium", size_group.clone())
    .with_value("M");
let large = RadioButton::new(WidgetId::default(), "Large", size_group.clone())
    .with_value("L");
```

### Slider

A draggable slider for numeric input.

```rust
// Basic slider (0-100)
let volume_slider = Slider::new(WidgetId::default())
    .with_range(0.0, 100.0)
    .with_value(50.0);

// With step size
let opacity_slider = Slider::new(WidgetId::default())
    .with_range(0.0, 1.0)
    .with_value(1.0)
    .with_step(0.1);

// With change handler
let mut brightness = Slider::new(WidgetId::default())
    .with_range(0.0, 255.0)
    .with_on_change(|value| {
        println!("Brightness: {}", value);
    });

// Show value label
let labeled_slider = Slider::new(WidgetId::default())
    .with_show_value(true)
    .with_value_format("{:.1}%");
```

### ProgressBar

Shows progress of an operation.

```rust
// Basic progress bar
let progress = ProgressBar::new(WidgetId::default())
    .with_value(0.5); // 50%

// With custom range
let download_progress = ProgressBar::new(WidgetId::default())
    .with_range(0.0, 1024.0) // MB
    .with_value(512.0);

// Indeterminate progress
let loading = ProgressBar::new(WidgetId::default())
    .with_indeterminate(true);

// With label
let task_progress = ProgressBar::new(WidgetId::default())
    .with_value(0.75)
    .with_show_label(true)
    .with_label_format("Processing: {:.0}%");
```

### SpinBox

Numeric input with increment/decrement buttons.

```rust
// Integer spin box
let quantity = SpinBox::new(WidgetId::default())
    .with_range(1.0, 100.0)
    .with_value(1.0)
    .with_step(1.0);

// Decimal spin box
let price = SpinBox::new(WidgetId::default())
    .with_range(0.0, 9999.99)
    .with_value(19.99)
    .with_step(0.01)
    .with_decimals(2);

// With prefix/suffix
let temperature = SpinBox::new(WidgetId::default())
    .with_range(-50.0, 50.0)
    .with_suffix("°C");

// With change handler
let mut font_size = SpinBox::new(WidgetId::default())
    .with_range(8.0, 72.0)
    .with_value(14.0)
    .with_on_change(|value| {
        println!("Font size: {}", value);
    });
```

---

## Container Widgets

### Container

A basic container for grouping widgets.

```rust
// Simple container
let mut container = Container::new(WidgetId::default());

// Add children
let label_id = container.add_child(Box::new(
    Label::new(WidgetId::default(), "Title")
));
let button_id = container.add_child(Box::new(
    Button::new(WidgetId::default(), "OK")
));

// With padding
let padded_container = Container::new(WidgetId::default())
    .with_padding(20);

// With background
let styled_container = Container::new(WidgetId::default())
    .with_background_color(Color::rgba(50, 50, 50, 255));
```

### ScrollView

Scrollable container for content that exceeds visible area.

```rust
// Basic scroll view
let mut scroll_view = ScrollView::new(WidgetId::default());

// Add scrollable content
let mut content = Container::new(WidgetId::default());
// Add many widgets to content...
scroll_view.set_content(Box::new(content));

// Vertical only scrolling
let v_scroll = ScrollView::new(WidgetId::default())
    .with_horizontal_scroll(false);

// With scroll bars always visible
let always_scroll = ScrollView::new(WidgetId::default())
    .with_scroll_bar_policy(ScrollBarPolicy::AlwaysVisible);

// Programmatic scrolling
scroll_view.scroll_to_top();
scroll_view.scroll_to_bottom();
scroll_view.scroll_to(Point::new(0, 100));
```

### TabControl

Tabbed interface for organizing content.

```rust
// Create tab control
let mut tabs = TabControl::new(WidgetId::default());

// Add tabs
let mut home_tab = TabPage::new("Home");
home_tab.add_widget(label_widget.id());
tabs.add_tab(home_tab);

let mut settings_tab = TabPage::new("Settings");
settings_tab.add_widget(settings_widget.id());
tabs.add_tab(settings_tab);

// Closeable tabs
let doc_tab = TabPage::new("Document.txt")
    .with_closeable(true);
tabs.add_tab(doc_tab);

// With tab change handler
let mut nav_tabs = TabControl::new(WidgetId::default())
    .with_on_tab_changed(|index| {
        println!("Switched to tab {}", index);
    })
    .with_on_tab_closed(|index| {
        println!("Closed tab {}", index);
    });

// Set active tab
tabs.set_active_tab(1);
```

### Accordion

Collapsible panels for organizing content.

```rust
// Create accordion
let mut accordion = Accordion::new(WidgetId::default());

// Add panels
let general_panel = AccordionPanel::new(
    WidgetId::default(),
    "General Settings",
    Box::new(general_settings_widget)
).with_expanded(true); // Start expanded

let advanced_panel = AccordionPanel::new(
    WidgetId::default(),
    "Advanced Settings",
    Box::new(advanced_settings_widget)
).with_icon("settings");

accordion.add_panel(general_panel);
accordion.add_panel(advanced_panel);

// Allow multiple panels open
let multi_accordion = Accordion::new(WidgetId::default())
    .with_multiple_open(true);

// With animation
let animated = Accordion::new(WidgetId::default())
    .with_animation_duration(300); // ms
```

---

## Selection Widgets

### ComboBox

Dropdown selection list.

```rust
// Basic combo box
let combo = ComboBox::new(WidgetId::default())
    .with_items(vec![
        "Option 1".to_string(),
        "Option 2".to_string(),
        "Option 3".to_string(),
    ])
    .with_selected(0);

// With placeholder
let country_combo = ComboBox::new(WidgetId::default())
    .with_placeholder("Select a country...")
    .with_items(vec!["USA".to_string(), "Canada".to_string()]);

// With change handler
let mut size_combo = ComboBox::new(WidgetId::default())
    .with_items(vec!["Small".to_string(), "Medium".to_string(), "Large".to_string()])
    .with_on_selection_changed(|index, text| {
        println!("Selected: {} - {}", index.unwrap_or(0), text.unwrap_or(""));
    });

// Editable combo box (with filtering)
let editable_combo = ComboBox::new(WidgetId::default())
    .with_editable(true)
    .with_items(vec!["Apple".to_string(), "Banana".to_string()]);
```

### ListView

Scrollable list of selectable items.

```rust
// Create list view
let list_view = ListView::new(WidgetId::default())
    .with_items(vec![
        ListItem::new("Item 1", "item1"),
        ListItem::new("Item 2", "item2"),
        ListItem::new("Item 3", "item3"),
    ]);

// With icons
let file_list = ListView::new(WidgetId::default())
    .with_items(vec![
        ListItem::new("Document.pdf", "doc1").with_icon("file"),
        ListItem::new("Image.png", "img1").with_icon("image"),
    ]);

// Multi-selection
let multi_list = ListView::new(WidgetId::default())
    .with_multi_select(true);

// With selection handler
let mut task_list = ListView::new(WidgetId::default())
    .with_on_selection_changed(|selected_items| {
        println!("Selected: {:?}", selected_items);
    });

// Add/remove items dynamically
task_list.add_item(ListItem::new("New Task", "task4"));
task_list.remove_item("task1");
```

### TreeView

Hierarchical tree structure display.

```rust
// Create tree view
let mut tree = TreeView::new(WidgetId::default());

// Create tree structure
let mut root = TreeNode::new("root", "Project");
let mut src = TreeNode::new("src", "src/");
src.add_child(TreeNode::new("main", "main.rs"));
src.add_child(TreeNode::new("lib", "lib.rs"));
root.add_child(src);

let mut docs = TreeNode::new("docs", "docs/");
docs.add_child(TreeNode::new("readme", "README.md"));
root.add_child(docs);

tree.set_root(root);

// With selection handler
let file_tree = TreeView::new(WidgetId::default())
    .with_on_selection_changed(|node_id| {
        println!("Selected node: {}", node_id);
    })
    .with_on_node_expanded(|node_id| {
        println!("Expanded: {}", node_id);
    });

// Show checkboxes
let check_tree = TreeView::new(WidgetId::default())
    .with_checkboxes(true)
    .with_on_checked_changed(|node_id, checked| {
        println!("Node {} checked: {}", node_id, checked);
    });
```

### ColorPicker

Color selection widget with various modes.

```rust
// Basic color picker
let color_picker = ColorPicker::new(WidgetId::default())
    .with_color(Color::rgb(255, 0, 0)); // Start with red

// With alpha channel
let rgba_picker = ColorPicker::new(WidgetId::default())
    .with_alpha(true)
    .with_color(Color::rgba(0, 128, 255, 200));

// Compact mode (button that opens popup)
let compact_picker = ColorPicker::new(WidgetId::default())
    .with_mode(ColorPickerMode::Compact);

// With change handler
let mut theme_color = ColorPicker::new(WidgetId::default())
    .with_on_color_changed(|color| {
        println!("Color changed: {:?}", color);
    });

// With presets
let preset_picker = ColorPicker::new(WidgetId::default())
    .with_presets(vec![
        Color::rgb(255, 0, 0),    // Red
        Color::rgb(0, 255, 0),    // Green
        Color::rgb(0, 0, 255),    // Blue
        Color::rgb(255, 255, 0),  // Yellow
    ]);
```

---

## Input Widgets

### TextArea

Multi-line text editor with rich features.

```rust
// Basic text area
let text_area = TextArea::new(WidgetId::default())
    .with_text("Initial content");

// With line numbers
let code_editor = TextArea::new(WidgetId::default())
    .with_line_numbers(true)
    .with_font_family("monospace");

// Read-only
let log_viewer = TextArea::new(WidgetId::default())
    .with_readonly(true);

// With syntax highlighting (if implemented)
let script_editor = TextArea::new(WidgetId::default())
    .with_syntax_highlighting("rust");

// Undo/redo support
let mut editor = TextArea::new(WidgetId::default());
editor.undo();
editor.redo();

// Text manipulation
editor.insert_text("Hello, ");
editor.append_line("World!");
let selected = editor.get_selected_text();
```

### SearchBox

Search input with suggestions and history.

```rust
// Basic search box
let search = SearchBox::new(WidgetId::default())
    .with_placeholder("Search...");

// With suggestions
let search_with_suggestions = SearchBox::new(WidgetId::default())
    .with_suggestions(vec![
        "apple".to_string(),
        "application".to_string(),
        "apply".to_string(),
    ]);

// With search handler
let mut product_search = SearchBox::new(WidgetId::default())
    .with_on_search(|query| {
        println!("Searching for: {}", query);
    })
    .with_debounce_ms(300); // Debounce input

// With history
let history_search = SearchBox::new(WidgetId::default())
    .with_history_enabled(true)
    .with_max_history(10);

// Advanced options
let advanced_search = SearchBox::new(WidgetId::default())
    .with_case_sensitive(false)
    .with_regex_enabled(true)
    .with_whole_word(false);
```

### DateTimePicker

Date and time selection widget.

```rust
use chrono::{Local, NaiveDate};

// Date picker only
let date_picker = DateTimePicker::new(WidgetId::default())
    .with_mode(DateTimePickerMode::Date)
    .with_date(Local::now().naive_local());

// Time picker only
let time_picker = DateTimePicker::new(WidgetId::default())
    .with_mode(DateTimePickerMode::Time);

// Date and time
let datetime_picker = DateTimePicker::new(WidgetId::default())
    .with_mode(DateTimePickerMode::DateTime);

// With format
let custom_format = DateTimePicker::new(WidgetId::default())
    .with_format("%d/%m/%Y %H:%M");

// With limits
let limited_picker = DateTimePicker::new(WidgetId::default())
    .with_min_date(NaiveDate::from_ymd(2024, 1, 1).and_hms(0, 0, 0))
    .with_max_date(NaiveDate::from_ymd(2024, 12, 31).and_hms(23, 59, 59));

// With change handler
let mut appointment = DateTimePicker::new(WidgetId::default())
    .with_on_change(|datetime| {
        println!("Selected: {}", datetime.format("%Y-%m-%d %H:%M"));
    });
```

---

## Navigation Widgets

### Menu

Application menu bar and menu items.

```rust
// Create menu bar
let mut menu_bar = MenuBar::new(WidgetId::default());

// File menu
let mut file_menu = Menu::new("File");
file_menu.add_item(MenuItem::new("New", "new").with_shortcut("Ctrl+N"));
file_menu.add_item(MenuItem::new("Open", "open").with_shortcut("Ctrl+O"));
file_menu.add_separator();
file_menu.add_item(MenuItem::new("Exit", "exit"));

// Edit menu with submenus
let mut edit_menu = Menu::new("Edit");
edit_menu.add_item(MenuItem::new("Cut", "cut").with_icon("cut"));
edit_menu.add_item(MenuItem::new("Copy", "copy").with_icon("copy"));

let mut paste_submenu = Menu::new("Paste Special");
paste_submenu.add_item(MenuItem::new("Paste as Text", "paste_text"));
paste_submenu.add_item(MenuItem::new("Paste as HTML", "paste_html"));
edit_menu.add_submenu(paste_submenu);

menu_bar.add_menu(file_menu);
menu_bar.add_menu(edit_menu);

// With action handler
menu_bar.set_on_item_clicked(|item_id| {
    match item_id {
        "new" => println!("New file"),
        "open" => println!("Open file"),
        _ => {}
    }
});
```

### ContextMenu

Right-click context menus.

```rust
// Create context menu
let mut context_menu = ContextMenu::new(WidgetId::default());

// Add items
context_menu.add_item(MenuItem::new("Cut", "cut").with_icon("cut"));
context_menu.add_item(MenuItem::new("Copy", "copy").with_icon("copy"));
context_menu.add_item(MenuItem::new("Paste", "paste").with_icon("paste"));
context_menu.add_separator();
context_menu.add_item(MenuItem::new("Delete", "delete").with_icon("delete"));

// Attach to widget
let mut label = Label::new(WidgetId::default(), "Right-click me");
label.set_context_menu(Some(context_menu));

// Dynamic context menu
let dynamic_menu = ContextMenu::new(WidgetId::default())
    .with_on_show(|menu| {
        // Dynamically populate menu based on context
        menu.clear();
        menu.add_item(MenuItem::new("Context Action", "action"));
    });
```

### Toolbar

Toolbar with buttons and other controls.

```rust
// Create toolbar
let mut toolbar = Toolbar::new(WidgetId::default());

// Add buttons
toolbar.add_button(ToolbarButton::new("new", "New").with_icon("file-new"));
toolbar.add_button(ToolbarButton::new("open", "Open").with_icon("file-open"));
toolbar.add_button(ToolbarButton::new("save", "Save").with_icon("file-save"));
toolbar.add_separator();

// Add toggle button
toolbar.add_toggle(ToolbarButton::new("bold", "Bold")
    .with_icon("format-bold")
    .with_tooltip("Bold text"));

// Add dropdown
let view_menu = Menu::new("View");
view_menu.add_item(MenuItem::new("Zoom In", "zoom_in"));
view_menu.add_item(MenuItem::new("Zoom Out", "zoom_out"));
toolbar.add_dropdown(ToolbarButton::new("view", "View"), view_menu);

// With overflow handling
let overflow_toolbar = Toolbar::new(WidgetId::default())
    .with_overflow_behavior(OverflowBehavior::Menu);

// Action handler
toolbar.set_on_action(|action_id| {
    println!("Toolbar action: {}", action_id);
});
```

### Breadcrumb

Navigation breadcrumb for hierarchical structures.

```rust
// Create breadcrumb
let breadcrumb = Breadcrumb::new(WidgetId::default())
    .with_separator(" > ");

// Set path
let mut file_breadcrumb = Breadcrumb::new(WidgetId::default());
file_breadcrumb.set_path(vec![
    BreadcrumbItem::new("Home", "/home"),
    BreadcrumbItem::new("Documents", "/home/documents"),
    BreadcrumbItem::new("Projects", "/home/documents/projects"),
]);

// With click handler
let nav_breadcrumb = Breadcrumb::new(WidgetId::default())
    .with_on_item_clicked(|item_id| {
        println!("Navigate to: {}", item_id);
    });

// Collapsible for long paths
let collapsible = Breadcrumb::new(WidgetId::default())
    .with_max_items(3)
    .with_ellipsis("...");
```

### StatusBar

Application status bar with panels.

```rust
// Create status bar
let mut status_bar = StatusBar::new(WidgetId::default());

// Add panels
status_bar.add_panel(StatusPanel::new("status")
    .with_text("Ready")
    .with_flex(1.0)); // Take remaining space

status_bar.add_panel(StatusPanel::new("line_col")
    .with_text("Line 1, Col 1")
    .with_width(150));

status_bar.add_panel(StatusPanel::new("mode")
    .with_text("INSERT")
    .with_width(80));

// Update panel text
status_bar.set_panel_text("status", "Processing...");

// With progress bar
let progress_panel = StatusPanel::new("progress")
    .with_progress_bar(0.5);
status_bar.add_panel(progress_panel);
```

---

## Dialog Widgets

### Dialog

Modal dialog windows.

```rust
// Basic dialog
let dialog = Dialog::new(WidgetId::default())
    .with_title("Confirm Action")
    .with_message("Are you sure you want to continue?")
    .with_buttons(vec![
        DialogButton::Ok,
        DialogButton::Cancel,
    ]);

// Custom buttons
let save_dialog = Dialog::new(WidgetId::default())
    .with_title("Save Changes?")
    .with_message("You have unsaved changes.")
    .with_buttons(vec![
        DialogButton::Custom("Save", "save"),
        DialogButton::Custom("Don't Save", "dont_save"),
        DialogButton::Cancel,
    ]);

// With result handler
let confirm_dialog = Dialog::new(WidgetId::default())
    .with_title("Delete File")
    .with_on_result(|button_id| {
        match button_id {
            "ok" => println!("Deleting file..."),
            _ => println!("Cancelled"),
        }
    });

// Non-modal dialog
let info_dialog = Dialog::new(WidgetId::default())
    .with_modal(false)
    .with_title("Information")
    .with_message("This is a non-modal dialog.");
```

### FileDialog

File open/save dialogs.

```rust
// Open file dialog
let open_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Open)
    .with_title("Open File")
    .with_filters(vec![
        FileFilter::new("Rust Files", vec!["rs"]),
        FileFilter::new("All Files", vec!["*"]),
    ])
    .with_initial_path("/home/user/projects");

// Save file dialog
let save_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Save)
    .with_title("Save As")
    .with_default_name("untitled.txt");

// Select folder
let folder_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::SelectFolder)
    .with_title("Choose Directory");

// With selection handler
let mut image_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Open)
    .with_filters(vec![
        FileFilter::new("Images", vec!["png", "jpg", "jpeg", "gif"]),
    ])
    .with_on_file_selected(|path| {
        println!("Selected file: {:?}", path);
    });

// Multi-selection
let multi_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Open)
    .with_multi_select(true);
```

### Notification

Toast notifications for user feedback.

```rust
// Basic notification
let mut notif_manager = NotificationManager::new(WidgetId::default());

notif_manager.show(Notification::new(
    "notif1",
    "Success",
    "Operation completed successfully"
).with_type(NotificationType::Success));

// Error notification with longer duration
notif_manager.show(Notification::new(
    "error1",
    "Error",
    "Failed to save file"
)
.with_type(NotificationType::Error)
.with_duration(Duration::from_secs(10)));

// With action button
notif_manager.show(Notification::new(
    "update1",
    "Update Available",
    "A new version is available"
)
.with_type(NotificationType::Info)
.with_action("Download")
.with_icon("download"));

// Progress notification
notif_manager.show(Notification::new(
    "download1",
    "Downloading",
    "file.zip - 65%"
)
.with_progress(0.65)
.with_can_close(false));

// Position and styling
let styled_notifs = NotificationManager::new(WidgetId::default())
    .with_position(NotificationPosition::BottomRight)
    .with_max_visible(3)
    .with_size(400, 100);
```

---

## Advanced Widgets

### DockPanel

Dockable panel system for complex layouts.

```rust
// Create dock panel system
let mut dock_panel = DockPanel::new(WidgetId::default());

// Add dockable panels
let properties = DockablePanel::new(
    "properties",
    "Properties",
    Box::new(properties_widget)
)
.with_icon("settings")
.with_can_close(true)
.with_can_float(true);

dock_panel.add_panel(properties, DockPosition::Right);

// Add to different positions
dock_panel.add_panel(explorer_panel, DockPosition::Left);
dock_panel.add_panel(output_panel, DockPosition::Bottom);
dock_panel.add_panel(editor_panel, DockPosition::Center);

// Floating panel
dock_panel.add_panel(tools_panel, DockPosition::Floating);

// Event handlers
let dock_system = DockPanel::new(WidgetId::default())
    .with_on_panel_close(|panel_id| {
        println!("Panel closed: {}", panel_id);
    })
    .with_on_layout_change(|| {
        println!("Dock layout changed");
    });

// Save/load layout
let layout = dock_panel.get_layout();
dock_panel.set_layout(layout);
```

### FileManager

File browser widget.

```rust
// Create file manager
let file_manager = FileManager::new(WidgetId::default())
    .with_initial_path("/home/user");

// View modes
let icon_view = FileManager::new(WidgetId::default())
    .with_view_mode(FileManagerViewMode::Icons);

let detail_view = FileManager::new(WidgetId::default())
    .with_view_mode(FileManagerViewMode::Details);

// With filters
let image_browser = FileManager::new(WidgetId::default())
    .with_filter(FileFilter::new("Images", vec!["png", "jpg", "jpeg"]));

// Event handlers
let mut explorer = FileManager::new(WidgetId::default())
    .with_on_file_selected(|path| {
        println!("Selected: {:?}", path);
    })
    .with_on_file_double_clicked(|path| {
        println!("Open: {:?}", path);
    });

// Show hidden files
explorer.set_show_hidden(true);
```

---

## Enhancement Systems

### Drag & Drop

System for drag and drop operations.

```rust
use erigui_widgets::{drag_drop_manager, DragData};

// Register drop target
let drop_manager = drag_drop_manager();
drop_manager.register_drop_target(
    target_widget.id(),
    vec!["text/plain".to_string(), "file".to_string()]
);

// Start drag operation
if let Event::MouseButton(e) = event {
    if e.button == MouseButton::Left && e.pressed {
        drop_manager.start_drag(
            source_widget.id(),
            "text/plain".to_string(),
            Box::new("Dragged text".to_string()),
            Point::new(10, 10) // Offset from cursor
        );
    }
}

// Handle drop
if drop_manager.check_drop_target(widget.id(), widget.bounds()) {
    if let Some(drag_data) = drop_manager.complete_drop() {
        if let Ok(text) = drag_data.data.downcast::<String>() {
            println!("Dropped: {}", text);
        }
    }
}

// Visual feedback
if drop_manager.is_dragging() {
    let pos = drop_manager.get_drag_visual_position();
    // Draw drag preview at pos
}
```

### Keyboard Navigation

Keyboard navigation support.

```rust
use erigui_widgets::{keyboard_nav_manager, FocusableWidget};

// Register focusable widget
let nav_manager = keyboard_nav_manager();
nav_manager.register_widget(FocusableWidget {
    id: widget.id(),
    bounds: widget.bounds(),
    tab_index: Some(1), // Explicit tab order
    focusable: true,
    group_id: None, // Or Some("form1") for grouping
});

// Handle keyboard navigation
if let Event::KeyPress(e) = event {
    if let Some(next_widget) = nav_manager.handle_key(e.key) {
        // Update focus
        current_widget.set_focused(false);
        next_widget.set_focused(true);
    }
}

// Focus trap (e.g., for dialogs)
nav_manager.set_focus_trap(Some("dialog_group"));

// Programmatic focus
nav_manager.set_focus(Some(widget.id()));
```

### Accessibility

Screen reader and accessibility support.

```rust
use erigui_widgets::{accessibility_manager, AccessibilityNode, AccessibilityRole};

// Register accessible widget
let a11y_manager = accessibility_manager();
a11y_manager.register_node(AccessibilityNode {
    id: button.id(),
    role: AccessibilityRole::Button,
    label: "Save Document".to_string(),
    description: Some("Saves the current document to disk".to_string()),
    bounds: button.bounds(),
    state: AccessibilityState {
        disabled: Some(false),
        focused: Some(true),
        ..Default::default()
    },
    children: vec![],
    parent: None,
    actions: vec![AccessibilityAction::Click],
});

// Update state
a11y_manager.update_node_state(checkbox.id(), AccessibilityState {
    checked: Some(checkbox.is_checked()),
    ..Default::default()
});

// Announce changes
a11y_manager.announce("File saved successfully".to_string());

// Get announcements for screen reader
let announcements = a11y_manager.get_announcements();
```

### Tooltip System

Contextual tooltips for widgets.

```rust
// Simple tooltip
button.set_tooltip("Click to save the document");

// Rich tooltip
let rich_tooltip = Tooltip::new(WidgetId::default())
    .with_title("Keyboard Shortcuts")
    .with_content("Ctrl+S - Save\nCtrl+O - Open\nCtrl+N - New")
    .with_delay(500); // ms

widget.set_tooltip_widget(rich_tooltip);

// Global tooltip manager
let tooltip_manager = tooltip_manager();
tooltip_manager.show_tooltip(
    widget.id(),
    "Custom tooltip text",
    Point::new(100, 100)
);

// Tooltip positioning
button.set_tooltip_position(TooltipPosition::Right);
```

---

## Complete Example Application

Here's a complete example showing how to create a simple text editor:

```rust
use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct TextEditor {
    menu_bar: MenuBar,
    toolbar: Toolbar,
    text_area: TextArea,
    status_bar: StatusBar,
    file_path: Option<String>,
}

impl TextEditor {
    fn new() -> Self {
        // Menu bar
        let mut menu_bar = MenuBar::new(WidgetId::default());
        
        let mut file_menu = Menu::new("File");
        file_menu.add_item(MenuItem::new("New", "new").with_shortcut("Ctrl+N"));
        file_menu.add_item(MenuItem::new("Open", "open").with_shortcut("Ctrl+O"));
        file_menu.add_item(MenuItem::new("Save", "save").with_shortcut("Ctrl+S"));
        file_menu.add_separator();
        file_menu.add_item(MenuItem::new("Exit", "exit"));
        menu_bar.add_menu(file_menu);
        
        // Toolbar
        let mut toolbar = Toolbar::new(WidgetId::default());
        toolbar.add_button(ToolbarButton::new("new", "New").with_icon("file-new"));
        toolbar.add_button(ToolbarButton::new("open", "Open").with_icon("file-open"));
        toolbar.add_button(ToolbarButton::new("save", "Save").with_icon("file-save"));
        
        // Text area
        let text_area = TextArea::new(WidgetId::default())
            .with_line_numbers(true);
        
        // Status bar
        let mut status_bar = StatusBar::new(WidgetId::default());
        status_bar.add_panel(StatusPanel::new("file").with_text("Untitled"));
        status_bar.add_panel(StatusPanel::new("position").with_text("Line 1, Col 1"));
        
        Self {
            menu_bar,
            toolbar,
            text_area,
            status_bar,
            file_path: None,
        }
    }
    
    fn handle_menu_action(&mut self, action: &str) {
        match action {
            "new" => {
                self.text_area.clear();
                self.file_path = None;
                self.status_bar.set_panel_text("file", "Untitled");
            }
            "open" => {
                // Show file dialog
                println!("Open file dialog");
            }
            "save" => {
                if let Some(path) = &self.file_path {
                    // Save to file
                    println!("Saving to {}", path);
                } else {
                    // Show save dialog
                    println!("Save as dialog");
                }
            }
            "exit" => {
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 800, 600, "Text Editor")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    let mut editor = TextEditor::new();
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        // Handle events and draw UI...
    });
}
```

---

## Tips and Best Practices

1. **Widget IDs**: Always use unique `WidgetId::default()` for each widget
2. **Event Handling**: Check `EventResult::Consumed` to avoid handling events multiple times
3. **Layout**: Call `layout()` before `draw()` and when window resizes
4. **Memory**: Use `Box<dyn Widget>` for dynamic widget storage
5. **Themes**: Pass `&Theme` to layout, draw, and event handling
6. **Focus**: Manage focus explicitly for keyboard navigation
7. **Accessibility**: Register widgets with accessibility manager for screen reader support

---

## Widget Feature Matrix

| Widget | Keyboard | Mouse | Touch | Drag & Drop | Accessibility | Themes |
|--------|----------|-------|-------|-------------|---------------|---------|
| Button | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| TextInput | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| CheckBox | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| RadioButton | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| Slider | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| ComboBox | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| ListView | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| TreeView | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| TextArea | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ColorPicker | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| DateTimePicker | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| DockPanel | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

---

This guide covers all widgets available in EriGui. For more detailed API documentation, refer to the generated Rust docs using `cargo doc`.
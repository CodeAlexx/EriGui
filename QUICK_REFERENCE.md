# EriGui Quick Reference

## Common Imports
```rust
use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
```

## Widget Creation Pattern
```rust
// Basic pattern
let widget = WidgetType::new(WidgetId::default(), /* params */)
    .with_property(value)
    .with_handler(|args| { /* ... */ });

// Mutable operations
let mut widget = WidgetType::new(WidgetId::default());
widget.set_property(value);
```

## Event Handling
```rust
// In event loop
if widget.handle_event(&event, &theme) == EventResult::Consumed {
    // Event was handled
    renderer.window().request_redraw();
}
```

## Layout Pattern
```rust
// Calculate size
let size = widget.measure(&constraints, &theme);

// Position widget
widget.layout(Rect::new(x, y, width, height), &theme);

// Draw widget
widget.draw(&mut renderer as &mut dyn DrawContext, &theme);
```

## Common Widget Patterns

### Form Layout
```rust
let mut y = 10;
let spacing = 40;
let label_width = 100;
let input_width = 200;

// Label + Input pairs
label.layout(Rect::new(10, y, label_width, 30), &theme);
input.layout(Rect::new(120, y, input_width, 30), &theme);
y += spacing;
```

### Button Row
```rust
let button_width = 80;
let button_spacing = 10;
let mut x = 10;

for button in &mut buttons {
    button.layout(Rect::new(x, y, button_width, 36), &theme);
    x += button_width + button_spacing;
}
```

### Centered Content
```rust
let content_size = widget.measure(&constraints, &theme);
let x = (window_width - content_size.width) / 2;
let y = (window_height - content_size.height) / 2;
widget.layout(Rect::new(x, y, content_size.width, content_size.height), &theme);
```

## Event Handlers

### Click Handler
```rust
Button::new(id, "Click Me")
    .with_on_click(|| println!("Clicked!"))
```

### Change Handler
```rust
TextInput::new(id)
    .with_on_change(|text| println!("Text: {}", text))
```

### Selection Handler
```rust
ListView::new(id)
    .with_on_selection_changed(|items| {
        for item in items {
            println!("Selected: {}", item.text);
        }
    })
```

## State Management

### Toggle State
```rust
if checkbox.is_checked() {
    // Handle checked state
}

button.set_enabled(false);
widget.set_visible(false);
widget.set_focused(true);
```

### Dynamic Content
```rust
// Update text
label.set_text("New text");
text_input.set_text("Initial value");

// Update list
list_view.add_item(ListItem::new("New Item", "id"));
list_view.remove_item("id");
list_view.clear();

// Update selection
combo_box.set_selected(Some(2));
radio_button.set_checked(true);
```

## Container Management

### Adding Children
```rust
let mut container = Container::new(id);
let child_id = container.add_child(Box::new(widget));
container.remove_child(child_id);
```

### Scroll View
```rust
let mut scroll = ScrollView::new(id);
scroll.set_content(Box::new(large_content));
scroll.scroll_to(Point::new(0, 100));
```

### Tab Control
```rust
let mut tabs = TabControl::new(id);
let mut tab = TabPage::new("Title");
tab.add_widget(content.id());
tabs.add_tab(tab);
tabs.set_active_tab(0);
```

## Dialog Patterns

### Confirmation Dialog
```rust
let dialog = Dialog::new(id)
    .with_title("Confirm")
    .with_message("Are you sure?")
    .with_buttons(vec![DialogButton::Yes, DialogButton::No])
    .with_on_result(|result| {
        if result == "yes" {
            // Confirmed
        }
    });
```

### File Dialog
```rust
let file_dialog = FileDialog::new(id, FileDialogMode::Open)
    .with_filters(vec![
        FileFilter::new("Images", vec!["png", "jpg"]),
    ])
    .with_on_file_selected(|path| {
        println!("Selected: {:?}", path);
    });
```

### Notification
```rust
notification_manager.show(
    Notification::new("id", "Title", "Message")
        .with_type(NotificationType::Success)
        .with_duration(Duration::from_secs(5))
);
```

## Keyboard Shortcuts

### Register Shortcuts
```rust
menu_item.with_shortcut("Ctrl+S")
```

### Handle Key Events
```rust
if let Event::KeyPress(e) = event {
    match e.key {
        Key::Enter => { /* ... */ }
        Key::Escape => { /* ... */ }
        Key::Tab => { /* ... */ }
        _ => {}
    }
}
```

## Theme Customization

### Access Theme Colors
```rust
context.set_color(theme.colors.primary);
context.set_color(theme.colors.text);
context.set_color(theme.colors.background);
context.set_color(theme.colors.surface);
context.set_color(theme.colors.border);
```

### Font Sizes
```rust
theme.typography.font_size_small  // 12
theme.typography.font_size_base   // 14
theme.typography.font_size_large  // 18
theme.typography.font_size_xlarge // 24
```

## Performance Tips

1. **Batch Updates**: Update multiple properties before calling redraw
2. **Conditional Rendering**: Only redraw when state changes
3. **Event Filtering**: Return `EventResult::Ignored` for unhandled events
4. **Lazy Loading**: Load data on demand for large lists/trees
5. **Reuse Widgets**: Update existing widgets instead of recreating

## Common Gotchas

1. **Widget IDs**: Must be unique - use `WidgetId::default()` for each
2. **Layout Order**: Always measure → layout → draw
3. **Event Consumption**: Check if event was consumed before handling
4. **Mutable Access**: Some operations require `mut` widget reference
5. **Focus Management**: Explicitly manage focus for keyboard navigation

## Debug Helpers

```rust
// Print widget bounds
println!("Widget bounds: {:?}", widget.bounds());

// Check widget state
println!("Visible: {}, Enabled: {}, Focused: {}", 
    widget.is_visible(), 
    widget.is_enabled(), 
    widget.is_focused()
);

// Track events
if let Event::MouseMove(e) = event {
    println!("Mouse at: {:?}", e.position);
}
```
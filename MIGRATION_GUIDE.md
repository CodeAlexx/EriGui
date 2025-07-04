# Migration Guide to EriGui

This guide helps developers familiar with other GUI frameworks transition to EriGui.

## Coming from GTK

### Widget Creation
```rust
// GTK
let button = gtk::Button::with_label("Click me");

// EriGui
let button = Button::new(WidgetId::default(), "Click me");
```

### Signal Handling
```rust
// GTK
button.connect_clicked(|_| {
    println!("Button clicked");
});

// EriGui
let button = Button::new(WidgetId::default(), "Click me")
    .with_on_click(|| {
        println!("Button clicked");
    });
```

### Container Packing
```rust
// GTK
let box = gtk::Box::new(gtk::Orientation::Vertical, 5);
box.pack_start(&label, false, false, 0);
box.pack_start(&button, false, false, 0);

// EriGui
let mut container = Container::new(WidgetId::default());
container.add_child(Box::new(label));
container.add_child(Box::new(button));
// Layout handled in layout() call
```

## Coming from Qt

### Widget Hierarchy
```rust
// Qt (C++)
QPushButton *button = new QPushButton("Click me", parent);

// EriGui
let button = Button::new(WidgetId::default(), "Click me");
// Parent-child relationships managed through containers
```

### Layouts
```rust
// Qt (C++)
QVBoxLayout *layout = new QVBoxLayout;
layout->addWidget(label);
layout->addWidget(button);

// EriGui
// Manual layout in layout() method
let mut y = 10;
label.layout(Rect::new(10, y, 200, 30), &theme);
y += 40;
button.layout(Rect::new(10, y, 200, 36), &theme);
```

### Slots and Signals
```rust
// Qt
connect(button, &QPushButton::clicked, this, &MyClass::onButtonClick);

// EriGui
let button = Button::new(id, "Click")
    .with_on_click(|| {
        // Handle click
    });
```

## Coming from React/Web

### Component Structure
```rust
// React
function MyComponent() {
    const [count, setCount] = useState(0);
    return <button onClick={() => setCount(count + 1)}>{count}</button>;
}

// EriGui
struct MyWidget {
    button: Button,
    count: i32,
}

impl MyWidget {
    fn new() -> Self {
        Self {
            button: Button::new(WidgetId::default(), "0"),
            count: 0,
        }
    }
    
    fn handle_click(&mut self) {
        self.count += 1;
        self.button.set_text(&self.count.to_string());
    }
}
```

### Event Handling
```rust
// React
<input onChange={(e) => setText(e.target.value)} />

// EriGui
let input = TextInput::new(WidgetId::default())
    .with_on_change(|text| {
        println!("Text changed: {}", text);
    });
```

### Conditional Rendering
```rust
// React
{showDialog && <Dialog />}

// EriGui
if show_dialog {
    dialog.set_visible(true);
    dialog.draw(context, theme);
} else {
    dialog.set_visible(false);
}
```

## Coming from Dear ImGui

### Immediate vs Retained
```rust
// Dear ImGui (Immediate mode)
if (ImGui::Button("Click me")) {
    // Handle click immediately
}

// EriGui (Retained mode)
// Create widget once
let button = Button::new(id, "Click me")
    .with_on_click(|| { /* handler */ });

// In render loop
if button.handle_event(&event, &theme) == EventResult::Consumed {
    // Event was handled
}
button.draw(context, theme);
```

### Window Management
```rust
// Dear ImGui
ImGui::Begin("My Window");
ImGui::Text("Hello");
ImGui::End();

// EriGui
let dialog = Dialog::new(id)
    .with_title("My Window")
    .with_content(Box::new(Label::new(id, "Hello")));
```

## Coming from SwiftUI/Flutter

### Declarative Style
```rust
// SwiftUI
VStack {
    Text("Hello")
    Button("Click me") { print("Clicked") }
}

// EriGui (Imperative style)
let mut container = Container::new(id);
container.add_child(Box::new(Label::new(id, "Hello")));
container.add_child(Box::new(
    Button::new(id, "Click me").with_on_click(|| println!("Clicked"))
));
```

### State Management
```rust
// SwiftUI
@State private var isOn = false

// EriGui
struct AppState {
    is_on: bool,
}

// Update state and trigger redraw
state.is_on = !state.is_on;
window.request_redraw();
```

## Key Differences in EriGui

### 1. Explicit Layout
Unlike auto-layout systems, EriGui requires explicit positioning:
```rust
widget.layout(Rect::new(x, y, width, height), &theme);
```

### 2. Event-Driven Updates
```rust
// Check if event was handled
if widget.handle_event(&event, &theme) == EventResult::Consumed {
    window.request_redraw();
}
```

### 3. Widget IDs
Every widget needs a unique ID:
```rust
let button = Button::new(WidgetId::default(), "Click");
```

### 4. Theme Parameter
Theme is passed to most methods:
```rust
widget.measure(&constraints, &theme);
widget.layout(rect, &theme);
widget.draw(context, &theme);
```

### 5. Manual Memory Management
Widgets are owned, use `Box<dyn Widget>` for dynamic storage:
```rust
container.add_child(Box::new(button));
```

## Common Patterns Comparison

### Forms
```rust
// Other frameworks often have form helpers
// EriGui: Manual layout
let mut y = 10;
for (label, input) in form_fields {
    label.layout(Rect::new(10, y, 100, 30), &theme);
    input.layout(Rect::new(120, y, 200, 30), &theme);
    y += 40;
}
```

### Lists
```rust
// Other frameworks: Data binding
// EriGui: Manual updates
list_view.clear();
for item in data {
    list_view.add_item(ListItem::new(&item.name, &item.id));
}
```

### Dialogs
```rust
// Other frameworks: Often modal by default
// EriGui: Explicit modal setting
let dialog = Dialog::new(id)
    .with_modal(true)
    .with_title("Confirm")
    .with_on_result(|result| { /* ... */ });
```

## Performance Considerations

1. **Retained Mode**: Widgets persist between frames
2. **Manual Layout**: More control but more responsibility
3. **Event Batching**: Handle multiple events before redraw
4. **Integer Coordinates**: Optimized for pixel-perfect rendering

## Best Practices

1. **Create widgets once**, update properties as needed
2. **Use builders** for cleaner initialization
3. **Handle events** before drawing
4. **Batch updates** to minimize redraws
5. **Manage focus** explicitly for keyboard navigation
6. **Use proper widget IDs** for state management

## Getting Started Checklist

- [ ] Understand retained vs immediate mode
- [ ] Learn the measure → layout → draw cycle
- [ ] Master event handling and `EventResult`
- [ ] Practice manual layout calculations
- [ ] Understand widget ownership with `Box<dyn Widget>`
- [ ] Learn theme system usage
- [ ] Implement proper focus management
- [ ] Use builder pattern for widget creation
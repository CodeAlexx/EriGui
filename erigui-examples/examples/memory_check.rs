use erigui_core::{LayoutConstraints, Rect, Theme, WidgetId};
use erigui_widgets::*;

fn main() {
    println!("Starting memory check...");

    // Create and destroy many widgets to check for memory leaks
    for i in 0..1000 {
        let id = WidgetId::default();

        // Create various widgets
        let _ = Button::new(id, format!("Button {}", i));
        let _ = Checkbox::new(id, format!("Checkbox {}", i));
        let _ = Slider::new(id, 0.0, 100.0, 50.0);
        let _ = ProgressBar::new(id);
        let _ = ComboBox::new(id).with_items(vec!["Item 1".to_string(), "Item 2".to_string()]);
        let _ = TabControl::new(id);
        let _ = Dialog::new(id, "Title", "Message");
        let _ = StatusBar::new(id);
        let _ = ContextMenu::new(id);
        let _ = SpinBox::new(id, 0.0, 100.0, 50.0);

        // Create with callbacks
        let _ = Button::new(id, "Test").with_on_click(|| println!("Clicked"));
        let _ = Checkbox::new(id, "Test").with_on_toggle(|_| println!("Toggled"));
        let _ = Slider::new(id, 0.0, 100.0, 50.0).with_on_value_changed(|_| println!("Changed"));

        if i % 100 == 0 {
            println!("Created {} sets of widgets", i);
        }
    }

    println!("Memory check complete. If running under valgrind, check for leaks.");
}

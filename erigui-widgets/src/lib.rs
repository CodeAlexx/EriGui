pub mod accessibility;
pub mod accordion;
pub mod breadcrumb;
pub mod button;
pub mod checkbox;
pub mod clipboard;
pub mod color_picker;
pub mod combo_box;
pub mod container;
pub mod context_menu;
pub mod date_time_picker;
pub mod dialog;
pub mod dock_panel;
pub mod drag_drop;
pub mod file_dialog;
pub mod file_manager;
pub mod icon;
pub mod keyboard_nav;
pub mod label;
pub mod list_view;
pub mod menu;
pub mod node_graph;
pub mod notification;
pub mod progress_bar;
pub mod radio_button;
pub mod scroll_view;
pub mod search_box;
pub mod slider;
pub mod spin_box;
pub mod status_bar;
pub mod tab_control;
pub mod text_area;
pub mod text_input;
pub mod toolbar;
pub mod tooltip;
pub mod tree_view;

pub use accessibility::*;
pub use accordion::*;
pub use breadcrumb::*;
pub use button::*;
pub use checkbox::*;
pub use clipboard::{clear_clipboard, get_clipboard, set_clipboard};
pub use color_picker::*;
pub use combo_box::*;
pub use container::*;
pub use context_menu::*;
pub use date_time_picker::*;
pub use dialog::*;
pub use dock_panel::*;
pub use drag_drop::*;
pub use file_dialog::*;
pub use file_manager::*;
pub use icon::*;
pub use keyboard_nav::*;
pub use label::*;
pub use list_view::*;
pub use menu::*;
pub use node_graph::*;
pub use notification::*;
pub use progress_bar::*;
pub use radio_button::*;
pub use scroll_view::*;
pub use search_box::*;
pub use slider::*;
pub use spin_box::*;
pub use status_bar::*;
pub use tab_control::*;
pub use text_area::*;
pub use text_input::*;
pub use toolbar::*;
pub use tooltip::*;
pub use tree_view::*;

// Re-export commonly used types from core
pub use erigui_core::WidgetId;

use erigui_core::Widget;
use slotmap::SlotMap;

pub struct WidgetManager {
    widgets: SlotMap<WidgetId, Box<dyn Widget>>,
}

impl Default for WidgetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetManager {
    pub fn new() -> Self {
        Self {
            widgets: SlotMap::new(),
        }
    }

    pub fn add_widget(&mut self, widget: Box<dyn Widget>) -> WidgetId {
        self.widgets.insert(widget)
    }

    pub fn get(&self, id: WidgetId) -> Option<&dyn Widget> {
        self.widgets.get(id).map(|w| w.as_ref())
    }

    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        self.widgets.get_mut(id).map(|w| w.as_mut())
    }

    pub fn remove(&mut self, id: WidgetId) -> Option<Box<dyn Widget>> {
        self.widgets.remove(id)
    }

    pub fn get_typed<T: Widget + 'static>(&self, id: WidgetId) -> Option<&T> {
        self.widgets
            .get(id)
            .and_then(|w| w.as_any().downcast_ref::<T>())
    }

    pub fn get_typed_mut<T: Widget + 'static>(&mut self, id: WidgetId) -> Option<&mut T> {
        self.widgets
            .get_mut(id)
            .and_then(|w| w.as_any_mut().downcast_mut::<T>())
    }
}

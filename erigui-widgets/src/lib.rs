pub mod button;
pub mod label;
pub mod text_input;
pub mod container;
pub mod scroll_view;
pub mod list_view;
pub mod tree_view;
pub mod file_manager;
pub mod menu;
pub mod checkbox;
pub mod slider;
pub mod progress_bar;
pub mod combo_box;
pub mod tab_control;
pub mod dialog;
pub mod status_bar;
pub mod context_menu;
pub mod spin_box;
pub mod icon;
pub mod radio_button;
pub mod color_picker;
pub mod tooltip;
pub mod toolbar;
pub mod text_area;
pub mod search_box;
pub mod accordion;
pub mod breadcrumb;
pub mod date_time_picker;
pub mod file_dialog;
pub mod dock_panel;
pub mod notification;
pub mod drag_drop;
pub mod keyboard_nav;
pub mod accessibility;

pub use button::*;
pub use label::*;
pub use text_input::*;
pub use container::*;
pub use scroll_view::*;
pub use list_view::*;
pub use tree_view::*;
pub use file_manager::*;
pub use menu::*;
pub use checkbox::*;
pub use slider::*;
pub use progress_bar::*;
pub use combo_box::*;
pub use tab_control::*;
pub use dialog::*;
pub use status_bar::*;
pub use context_menu::*;
pub use spin_box::*;
pub use icon::*;
pub use radio_button::*;
pub use color_picker::*;
pub use tooltip::*;
pub use toolbar::*;
pub use text_area::*;
pub use search_box::*;
pub use accordion::*;
pub use breadcrumb::*;
pub use date_time_picker::*;
pub use file_dialog::*;
pub use dock_panel::*;
pub use notification::*;
pub use drag_drop::*;
pub use keyboard_nav::*;
pub use accessibility::*;

// Re-export commonly used types from core
pub use erigui_core::WidgetId;

use erigui_core::Widget;
use slotmap::SlotMap;

pub struct WidgetManager {
    widgets: SlotMap<WidgetId, Box<dyn Widget>>,
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
        self.widgets.get(id)
            .and_then(|w| w.as_any().downcast_ref::<T>())
    }
    
    pub fn get_typed_mut<T: Widget + 'static>(&mut self, id: WidgetId) -> Option<&mut T> {
        self.widgets.get_mut(id)
            .and_then(|w| w.as_any_mut().downcast_mut::<T>())
    }
}
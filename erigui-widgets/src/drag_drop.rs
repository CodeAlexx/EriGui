use erigui_core::{Point, Rect, WidgetId};
use std::any::Any;
use std::collections::HashMap;

#[derive(Debug)]
pub struct DragData {
    pub source_widget: WidgetId,
    pub data_type: String,
    pub data: Box<dyn Any + Send + Sync>,
    pub visual_rect: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragDropEffect {
    None,
    Copy,
    Move,
    Link,
}

pub struct DragDropManager {
    active_drag: Option<DragData>,
    drag_position: Point,
    drag_offset: Point,
    drop_targets: HashMap<WidgetId, Vec<String>>, // Widget ID -> accepted data types
    current_drop_target: Option<WidgetId>,
    drag_effect: DragDropEffect,
}

impl DragDropManager {
    pub fn new() -> Self {
        Self {
            active_drag: None,
            drag_position: Point::ZERO,
            drag_offset: Point::ZERO,
            drop_targets: HashMap::new(),
            current_drop_target: None,
            drag_effect: DragDropEffect::None,
        }
    }
    
    pub fn start_drag(&mut self, source: WidgetId, data_type: String, data: Box<dyn Any + Send + Sync>, offset: Point) {
        self.active_drag = Some(DragData {
            source_widget: source,
            data_type,
            data,
            visual_rect: None,
        });
        self.drag_offset = offset;
        self.drag_effect = DragDropEffect::Move;
    }
    
    pub fn update_drag_position(&mut self, position: Point) {
        self.drag_position = position;
    }
    
    pub fn register_drop_target(&mut self, widget: WidgetId, accepted_types: Vec<String>) {
        self.drop_targets.insert(widget, accepted_types);
    }
    
    pub fn unregister_drop_target(&mut self, widget: WidgetId) {
        self.drop_targets.remove(&widget);
    }
    
    pub fn is_dragging(&self) -> bool {
        self.active_drag.is_some()
    }
    
    pub fn get_drag_data(&self) -> Option<&DragData> {
        self.active_drag.as_ref()
    }
    
    pub fn get_drag_position(&self) -> Point {
        self.drag_position
    }
    
    pub fn get_drag_visual_position(&self) -> Point {
        Point::new(
            self.drag_position.x - self.drag_offset.x,
            self.drag_position.y - self.drag_offset.y
        )
    }
    
    pub fn check_drop_target(&mut self, widget: WidgetId, bounds: Rect) -> bool {
        if !self.is_dragging() {
            return false;
        }
        
        if !bounds.contains(self.drag_position) {
            if self.current_drop_target == Some(widget) {
                self.current_drop_target = None;
            }
            return false;
        }
        
        if let Some(drag_data) = &self.active_drag {
            if let Some(accepted_types) = self.drop_targets.get(&widget) {
                if accepted_types.contains(&drag_data.data_type) {
                    self.current_drop_target = Some(widget);
                    return true;
                }
            }
        }
        
        false
    }
    
    pub fn get_current_drop_target(&self) -> Option<WidgetId> {
        self.current_drop_target
    }
    
    pub fn get_drag_effect(&self) -> DragDropEffect {
        self.drag_effect
    }
    
    pub fn set_drag_effect(&mut self, effect: DragDropEffect) {
        self.drag_effect = effect;
    }
    
    pub fn complete_drop(&mut self) -> Option<DragData> {
        self.current_drop_target = None;
        self.active_drag.take()
    }
    
    pub fn cancel_drag(&mut self) {
        self.active_drag = None;
        self.current_drop_target = None;
        self.drag_effect = DragDropEffect::None;
    }
}

// Global drag drop manager instance
use std::sync::{Mutex, OnceLock};

static DRAG_DROP_MANAGER: OnceLock<Mutex<DragDropManager>> = OnceLock::new();

pub fn drag_drop_manager() -> std::sync::MutexGuard<'static, DragDropManager> {
    DRAG_DROP_MANAGER.get_or_init(|| {
        Mutex::new(DragDropManager::new())
    }).lock().unwrap()
}

// Trait for widgets that support drag and drop
pub trait Draggable {
    fn can_drag(&self) -> bool;
    fn get_drag_data(&self) -> Option<(String, Box<dyn Any + Send + Sync>)>;
    fn on_drag_start(&mut self);
    fn on_drag_end(&mut self, completed: bool);
}

pub trait Droppable {
    fn get_accepted_types(&self) -> Vec<String>;
    fn on_drag_enter(&mut self, data_type: &str);
    fn on_drag_over(&mut self, data_type: &str, position: Point) -> DragDropEffect;
    fn on_drag_leave(&mut self);
    fn on_drop(&mut self, data: DragData) -> bool;
}
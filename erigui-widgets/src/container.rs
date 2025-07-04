use erigui_core::{
    DrawContext, Event, EventResult, FlexChild, LayoutConfig, LayoutConstraints, LayoutMode,
    Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use smallvec::SmallVec;
use std::any::Any;

pub struct Container {
    state: WidgetState,
    children: SmallVec<[WidgetId; 8]>,
    flex_children: SmallVec<[FlexChild; 8]>,
    layout: LayoutConfig,
}

impl Container {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            children: SmallVec::new(),
            flex_children: SmallVec::new(),
            layout: LayoutConfig::default(),
        }
    }
    
    pub fn with_layout(mut self, layout: LayoutConfig) -> Self {
        self.layout = layout;
        self
    }
    
    pub fn add_child(&mut self, child_id: WidgetId) {
        self.children.push(child_id);
        self.flex_children.push(FlexChild::default());
    }
    
    pub fn add_flex_child(&mut self, child_id: WidgetId, flex: f32) {
        self.children.push(child_id);
        self.flex_children.push(FlexChild::new(flex));
    }
    
    pub fn remove_child(&mut self, child_id: WidgetId) -> bool {
        if let Some(index) = self.children.iter().position(|&id| id == child_id) {
            self.children.remove(index);
            self.flex_children.remove(index);
            true
        } else {
            false
        }
    }
    
    pub fn clear_children(&mut self) {
        self.children.clear();
        self.flex_children.clear();
    }
    
    pub fn set_layout_mode(&mut self, mode: LayoutMode) {
        self.layout.mode = mode;
    }
    
    pub fn get_layout_info(&self) -> LayoutConfig {
        self.layout.clone()
    }
    
    fn layout_children<'a>(&self, widgets: &'a mut dyn FnMut(WidgetId) -> Option<&'a mut dyn Widget>, theme: &Theme) {
        let content_rect = self.state.bounds.inset(self.layout.padding.left);
        let spacing = self.layout.spacing;
        
        match self.layout.mode {
            LayoutMode::None => {
                // Manual positioning - widgets keep their own positions
            }
            
            LayoutMode::Vertical => {
                let mut y = content_rect.y();
                let total_flex: f32 = self.flex_children.iter().map(|f| f.flex).sum();
                let child_count = self.children.len() as i32;
                let total_spacing = spacing * (child_count - 1).max(0);
                
                // First pass: measure fixed children
                let mut fixed_height = 0;
                for (i, &child_id) in self.children.iter().enumerate() {
                    if self.flex_children[i].flex == 0.0 {
                        if let Some(child) = widgets(child_id) {
                            let constraints = LayoutConstraints::bounded(content_rect.width(), i32::MAX);
                            let size = child.measure(&constraints, theme);
                            fixed_height += size.height;
                        }
                    }
                }
                
                let flex_height = (content_rect.height() - fixed_height - total_spacing).max(0);
                
                // Second pass: layout all children
                for (i, &child_id) in self.children.iter().enumerate() {
                    if let Some(child) = widgets(child_id) {
                        let flex = &self.flex_children[i];
                        
                        let height = if flex.flex > 0.0 {
                            ((flex_height as f32) * flex.flex / total_flex) as i32
                        } else {
                            let constraints = LayoutConstraints::bounded(content_rect.width(), i32::MAX);
                            child.measure(&constraints, theme).height
                        };
                        
                        child.layout(Rect::new(content_rect.x(), y, content_rect.width(), height), theme);
                        y += height + spacing;
                    }
                }
            }
            
            LayoutMode::Horizontal => {
                let mut x = content_rect.x();
                let total_flex: f32 = self.flex_children.iter().map(|f| f.flex).sum();
                let child_count = self.children.len() as i32;
                let total_spacing = spacing * (child_count - 1).max(0);
                
                // First pass: measure fixed children
                let mut fixed_width = 0;
                for (i, &child_id) in self.children.iter().enumerate() {
                    if self.flex_children[i].flex == 0.0 {
                        if let Some(child) = widgets(child_id) {
                            let constraints = LayoutConstraints::bounded(i32::MAX, content_rect.height());
                            let size = child.measure(&constraints, theme);
                            fixed_width += size.width;
                        }
                    }
                }
                
                let flex_width = (content_rect.width() - fixed_width - total_spacing).max(0);
                
                // Second pass: layout all children
                for (i, &child_id) in self.children.iter().enumerate() {
                    if let Some(child) = widgets(child_id) {
                        let flex = &self.flex_children[i];
                        
                        let width = if flex.flex > 0.0 {
                            ((flex_width as f32) * flex.flex / total_flex) as i32
                        } else {
                            let constraints = LayoutConstraints::bounded(i32::MAX, content_rect.height());
                            child.measure(&constraints, theme).width
                        };
                        
                        child.layout(Rect::new(x, content_rect.y(), width, content_rect.height()), theme);
                        x += width + spacing;
                    }
                }
            }
            
            LayoutMode::Grid { columns } => {
                let cell_width = (content_rect.width() - spacing * (columns - 1)) / columns;
                let mut row = 0;
                let mut col = 0;
                
                for &child_id in &self.children {
                    if let Some(child) = widgets(child_id) {
                        let x = content_rect.x() + col * (cell_width + spacing);
                        let y = content_rect.y() + row * (cell_width + spacing); // Assuming square cells
                        
                        child.layout(Rect::new(x, y, cell_width, cell_width), theme);
                        
                        col += 1;
                        if col >= columns {
                            col = 0;
                            row += 1;
                        }
                    }
                }
            }
        }
    }
}

impl Widget for Container {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        let padding = self.layout.padding;
        let content_constraints = constraints.deflate(padding);
        
        let content_size = match self.layout.mode {
            LayoutMode::None => Size::ZERO,
            LayoutMode::Vertical => {
                let width = content_constraints.max_width.unwrap_or(0);
                let height = content_constraints.max_height.unwrap_or(200);
                Size::new(width, height)
            }
            LayoutMode::Horizontal => {
                let width = content_constraints.max_width.unwrap_or(200);
                let height = content_constraints.max_height.unwrap_or(0);
                Size::new(width, height)
            }
            LayoutMode::Grid { columns } => {
                let width = content_constraints.max_width.unwrap_or(200);
                let rows = (self.children.len() as i32 + columns - 1) / columns;
                let height = rows * (width / columns);
                Size::new(width, height)
            }
        };
        
        Size::new(
            content_size.width + padding.horizontal(),
            content_size.height + padding.vertical(),
        )
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
        // Layout children needs to be called here, but we need access to the widget manager
        // For now, we'll rely on the external layout system to handle child layout
    }
    
    fn draw(&self, _context: &mut dyn DrawContext, _theme: &Theme) {
        if !self.state.visible {
            return;
        }
        
        // Containers typically don't draw themselves, just their children
        // But we could draw a background if needed
    }
    
    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
        // Container doesn't handle events directly
        EventResult::Ignored
    }
    
    fn children(&self) -> &[WidgetId] {
        &self.children
    }
    
    fn children_mut(&mut self) -> &mut [WidgetId] {
        &mut self.children
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
        false
    }
    
    fn set_focused(&mut self, _focused: bool) {}
    
    fn can_focus(&self) -> bool {
        false
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
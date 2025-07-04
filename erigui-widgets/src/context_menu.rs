use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Clone)]
pub struct ContextMenuItem {
    pub text: String,
    pub shortcut: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub submenu: Option<Vec<ContextMenuItem>>,
    pub on_click: Option<fn()>,
}

impl ContextMenuItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shortcut: String::new(),
            enabled: true,
            is_separator: false,
            submenu: None,
            on_click: None,
        }
    }
    
    pub fn separator() -> Self {
        Self {
            text: String::new(),
            shortcut: String::new(),
            enabled: true,
            is_separator: true,
            submenu: None,
            on_click: None,
        }
    }
    
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self
    }
    
    pub fn with_on_click(mut self, handler: fn()) -> Self {
        self.on_click = Some(handler);
        self
    }
    
    pub fn with_submenu(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }
    
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

pub struct ContextMenu {
    state: WidgetState,
    items: Vec<ContextMenuItem>,
    is_open: bool,
    hover_index: Option<usize>,
    position: Point,
    item_height: i32,
    min_width: i32,
    submenu_open: Option<usize>,
    submenu: Option<Box<ContextMenu>>,
}

impl ContextMenu {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            items: Vec::new(),
            is_open: false,
            hover_index: None,
            position: Point::ZERO,
            item_height: 25,
            min_width: 150,
            submenu_open: None,
            submenu: None,
        }
    }
    
    pub fn with_items(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.items = items;
        self
    }
    
    pub fn add_item(&mut self, item: ContextMenuItem) {
        self.items.push(item);
    }
    
    pub fn show_at(&mut self, position: Point) {
        self.position = position;
        self.is_open = true;
        self.hover_index = None;
        self.submenu_open = None;
        self.submenu = None;
        
        // Calculate bounds
        let width = self.calculate_width();
        let height = self.calculate_height();
        self.state.bounds = Rect::new(position.x, position.y, width, height);
    }
    
    pub fn hide(&mut self) {
        self.is_open = false;
        self.hover_index = None;
        self.submenu_open = None;
        self.submenu = None;
    }
    
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    fn calculate_width(&self) -> i32 {
        let mut max_width = self.min_width;
        
        for item in &self.items {
            if !item.is_separator {
                let text_width = item.text.len() as i32 * 7; // Rough estimate
                let shortcut_width = if item.shortcut.is_empty() { 0 } else { item.shortcut.len() as i32 * 7 + 20 };
                let submenu_arrow = if item.submenu.is_some() { 20 } else { 0 };
                let total_width = 40 + text_width + shortcut_width + submenu_arrow;
                max_width = max_width.max(total_width);
            }
        }
        
        max_width
    }
    
    fn calculate_height(&self) -> i32 {
        let mut height = 0;
        for item in &self.items {
            height += if item.is_separator { 9 } else { self.item_height };
        }
        height + 4 // padding
    }
    
    fn get_item_rect(&self, index: usize) -> Rect {
        let mut y = self.state.bounds.y() + 2;
        
        for i in 0..index {
            y += if self.items[i].is_separator { 9 } else { self.item_height };
        }
        
        let height = if self.items[index].is_separator { 9 } else { self.item_height };
        Rect::new(self.state.bounds.x(), y, self.state.bounds.width(), height)
    }
    
    fn item_from_point(&self, point: Point) -> Option<usize> {
        if !self.state.bounds.contains(point) {
            return None;
        }
        
        let mut y = self.state.bounds.y() + 2;
        for (i, item) in self.items.iter().enumerate() {
            let height = if item.is_separator { 9 } else { self.item_height };
            if point.y >= y && point.y < y + height {
                return if item.is_separator { None } else { Some(i) };
            }
            y += height;
        }
        None
    }
}

impl Widget for ContextMenu {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(self.calculate_width(), self.calculate_height())
    }
    
    fn layout(&mut self, _rect: Rect, _theme: &Theme) {
        // Context menu controls its own position
    }
    
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible || !self.is_open {
            return;
        }
        
        // Draw shadow
        let shadow_offset = 2;
        let shadow_rect = Rect::new(
            self.state.bounds.x() + shadow_offset,
            self.state.bounds.y() + shadow_offset,
            self.state.bounds.width(),
            self.state.bounds.height()
        );
        context.set_color(theme.colors.shadow);
        context.fill_rect(shadow_rect);
        
        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.state.bounds);
        
        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.state.bounds);
        
        // Draw items
        for (i, item) in self.items.iter().enumerate() {
            let item_rect = self.get_item_rect(i);
            
            if item.is_separator {
                // Draw separator line
                context.set_color(theme.colors.border);
                let y = item_rect.center().y;
                context.draw_line(
                    Point::new(item_rect.x() + 5, y),
                    Point::new(item_rect.right() - 5, y),
                    1
                );
            } else {
                // Draw item background if hovered
                if Some(i) == self.hover_index && item.enabled {
                    context.set_color(theme.colors.primary_hover);
                    context.fill_rect(item_rect);
                }
                
                // Draw item text
                let text_color = if item.enabled {
                    theme.colors.text
                } else {
                    theme.colors.text_disabled
                };
                context.set_color(text_color);
                
                let text_y = item_rect.center().y - theme.typography.font_size_base / 2;
                context.draw_text(
                    &item.text,
                    Point::new(item_rect.x() + 10, text_y),
                    theme.typography.font_size_base
                );
                
                // Draw shortcut
                if !item.shortcut.is_empty() {
                    context.set_color(theme.colors.text_secondary);
                    let shortcut_width = item.shortcut.len() as i32 * 7;
                    let shortcut_x = item_rect.right() - shortcut_width - 10;
                    context.draw_text(
                        &item.shortcut,
                        Point::new(shortcut_x, text_y),
                        theme.typography.font_size_base
                    );
                }
                
                // Draw submenu arrow
                if item.submenu.is_some() {
                    context.set_color(text_color);
                    let arrow_x = item_rect.right() - 15;
                    let arrow_y = item_rect.center().y;
                    
                    // Draw right-pointing triangle
                    context.draw_line(
                        Point::new(arrow_x, arrow_y - 4),
                        Point::new(arrow_x + 4, arrow_y),
                        1
                    );
                    context.draw_line(
                        Point::new(arrow_x + 4, arrow_y),
                        Point::new(arrow_x, arrow_y + 4),
                        1
                    );
                }
            }
        }
        
        // Draw submenu if open
        if let Some(submenu) = &self.submenu {
            submenu.draw(context, theme);
        }
    }
    
    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.visible || !self.state.enabled || !self.is_open {
            return EventResult::Ignored;
        }
        
        // Handle submenu events first
        if let Some(submenu) = &mut self.submenu {
            if submenu.handle_event(event, _theme).is_consumed() {
                return EventResult::Consumed;
            }
        }
        
        match event {
            Event::MouseMove(mouse_event) => {
                let old_hover = self.hover_index;
                self.hover_index = self.item_from_point(mouse_event.position);
                
                // Handle submenu opening/closing
                if let Some(hover_idx) = self.hover_index {
                    if self.items[hover_idx].submenu.is_some() {
                        if self.submenu_open != Some(hover_idx) {
                            // Open new submenu
                            self.submenu_open = Some(hover_idx);
                            let item_rect = self.get_item_rect(hover_idx);
                            let submenu_pos = Point::new(item_rect.right(), item_rect.y());
                            
                            let mut submenu = ContextMenu::new(WidgetId::default());
                            submenu.items = self.items[hover_idx].submenu.as_ref().unwrap().clone();
                            submenu.show_at(submenu_pos);
                            self.submenu = Some(Box::new(submenu));
                        }
                    } else if self.submenu_open.is_some() {
                        // Close submenu if hovering over non-submenu item
                        self.submenu_open = None;
                        self.submenu = None;
                    }
                } else if self.submenu_open.is_some() && !self.state.bounds.contains(mouse_event.position) {
                    // Keep submenu open if mouse is outside but submenu is open
                    if let Some(submenu) = &self.submenu {
                        if !submenu.state.bounds.contains(mouse_event.position) {
                            self.submenu_open = None;
                            self.submenu = None;
                        }
                    }
                }
                
                if old_hover != self.hover_index {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            
            Event::MouseButton(mouse_event) => {
                if mouse_event.button == MouseButton::Left && mouse_event.pressed {
                    if let Some(index) = self.item_from_point(mouse_event.position) {
                        let item = &self.items[index];
                        if item.enabled && !item.is_separator && item.submenu.is_none() {
                            if let Some(handler) = item.on_click {
                                handler();
                            }
                            self.hide();
                            return EventResult::Consumed;
                        }
                    } else if !self.state.bounds.contains(mouse_event.position) {
                        // Click outside - check if in submenu
                        if let Some(submenu) = &self.submenu {
                            if !submenu.state.bounds.contains(mouse_event.position) {
                                self.hide();
                                return EventResult::Consumed;
                            }
                        } else {
                            self.hide();
                            return EventResult::Consumed;
                        }
                    }
                } else if mouse_event.button == MouseButton::Right {
                    // Right click outside closes menu
                    if !self.state.bounds.contains(mouse_event.position) {
                        self.hide();
                        return EventResult::Consumed;
                    }
                }
                EventResult::Ignored
            }
            
            _ => EventResult::Ignored
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
        self.is_open
    }
    
    fn set_focused(&mut self, focused: bool) {
        if !focused {
            self.hide();
        }
    }
    
    fn can_focus(&self) -> bool {
        true
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
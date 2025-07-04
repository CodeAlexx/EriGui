use erigui_core::{
    Color, DrawContext, Point, Rect, Size, Theme, WidgetId,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
    Auto,
}

pub struct Tooltip {
    id: WidgetId,
    text: String,
    target_bounds: Rect,
    position: TooltipPosition,
    delay_ms: u32,
    visible: bool,
    hover_start: Option<Instant>,
    offset: i32,
    padding: i32,
    max_width: i32,
}

impl Tooltip {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
            target_bounds: Rect::default(),
            position: TooltipPosition::Auto,
            delay_ms: 500,
            visible: false,
            hover_start: None,
            offset: 4,
            padding: 8,
            max_width: 200,
        }
    }
    
    pub fn with_position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }
    
    pub fn with_delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }
    
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
    
    pub fn set_target_bounds(&mut self, bounds: Rect) {
        self.target_bounds = bounds;
    }
    
    fn calculate_position(&self, tooltip_size: Size, viewport_size: Size) -> (Point, TooltipPosition) {
        let mut final_position = self.position;
        
        if final_position == TooltipPosition::Auto {
            // Try above first
            if self.target_bounds.y() - tooltip_size.height - self.offset >= 0 {
                final_position = TooltipPosition::Above;
            }
            // Try below
            else if self.target_bounds.bottom() + tooltip_size.height + self.offset <= viewport_size.height {
                final_position = TooltipPosition::Below;
            }
            // Try right
            else if self.target_bounds.right() + tooltip_size.width + self.offset <= viewport_size.width {
                final_position = TooltipPosition::Right;
            }
            // Try left
            else if self.target_bounds.x() - tooltip_size.width - self.offset >= 0 {
                final_position = TooltipPosition::Left;
            }
            // Default to above
            else {
                final_position = TooltipPosition::Above;
            }
        }
        
        let pos = match final_position {
            TooltipPosition::Above | TooltipPosition::Auto => Point::new(
                self.target_bounds.x() + (self.target_bounds.width() - tooltip_size.width) / 2,
                self.target_bounds.y() - tooltip_size.height - self.offset
            ),
            TooltipPosition::Below => Point::new(
                self.target_bounds.x() + (self.target_bounds.width() - tooltip_size.width) / 2,
                self.target_bounds.bottom() + self.offset
            ),
            TooltipPosition::Left => Point::new(
                self.target_bounds.x() - tooltip_size.width - self.offset,
                self.target_bounds.y() + (self.target_bounds.height() - tooltip_size.height) / 2
            ),
            TooltipPosition::Right => Point::new(
                self.target_bounds.right() + self.offset,
                self.target_bounds.y() + (self.target_bounds.height() - tooltip_size.height) / 2
            ),
        };
        
        // Clamp to viewport
        let clamped_pos = Point::new(
            pos.x.max(0).min(viewport_size.width - tooltip_size.width),
            pos.y.max(0).min(viewport_size.height - tooltip_size.height)
        );
        
        (clamped_pos, final_position)
    }
    
    pub fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.visible || self.text.is_empty() {
            return;
        }
        
        // Calculate text size
        let text_width = self.text.len() as i32 * theme.typography.font_size_base * 3 / 5;
        let text_height = theme.typography.font_size_base;
        
        // Calculate tooltip size
        let tooltip_width = (text_width + self.padding * 2).min(self.max_width);
        let tooltip_height = text_height + self.padding * 2;
        let tooltip_size = Size::new(tooltip_width, tooltip_height);
        
        let viewport_size = context.viewport_size();
        let (position, _) = self.calculate_position(tooltip_size, viewport_size);
        
        let tooltip_rect = Rect::new(
            position.x,
            position.y,
            tooltip_width,
            tooltip_height
        );
        
        // Draw shadow
        context.set_color(Color::rgba(0, 0, 0, 64));
        context.fill_rect(Rect::new(
            tooltip_rect.x() + 2,
            tooltip_rect.y() + 2,
            tooltip_rect.width(),
            tooltip_rect.height()
        ));
        
        // Draw background
        context.set_color(Color::rgba(48, 48, 48, 240));
        context.fill_rect(tooltip_rect);
        
        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(tooltip_rect);
        
        // Draw text
        context.set_color(Color::WHITE);
        context.draw_text(
            &self.text,
            Point::new(
                tooltip_rect.x() + self.padding,
                tooltip_rect.center().y + theme.typography.font_size_base / 2 - 2
            ),
            theme.typography.font_size_base
        );
    }
}

// Global tooltip manager
pub struct TooltipManager {
    tooltips: HashMap<WidgetId, Tooltip>,
    active_widget: Option<WidgetId>,
    mouse_position: Point,
    last_update: Instant,
}

impl TooltipManager {
    pub fn new() -> Self {
        Self {
            tooltips: HashMap::new(),
            active_widget: None,
            mouse_position: Point::ZERO,
            last_update: Instant::now(),
        }
    }
    
    pub fn register_tooltip(&mut self, widget_id: WidgetId, tooltip: Tooltip) {
        self.tooltips.insert(widget_id, tooltip);
    }
    
    pub fn unregister_tooltip(&mut self, widget_id: WidgetId) {
        self.tooltips.remove(&widget_id);
        if self.active_widget == Some(widget_id) {
            self.active_widget = None;
        }
    }
    
    pub fn set_tooltip_text(&mut self, widget_id: WidgetId, text: impl Into<String>) {
        if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
            tooltip.set_text(text);
        }
    }
    
    pub fn update(&mut self, mouse_position: Point, hovered_widget: Option<WidgetId>, widget_bounds: Option<Rect>) {
        self.mouse_position = mouse_position;
        let now = Instant::now();
        
        // Check if we're hovering over a new widget
        if hovered_widget != self.active_widget {
            // Hide current tooltip
            if let Some(current_id) = self.active_widget {
                if let Some(tooltip) = self.tooltips.get_mut(&current_id) {
                    tooltip.visible = false;
                    tooltip.hover_start = None;
                }
            }
            
            // Start tracking new widget
            self.active_widget = hovered_widget;
            if let Some(widget_id) = hovered_widget {
                if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
                    tooltip.hover_start = Some(now);
                    if let Some(bounds) = widget_bounds {
                        tooltip.set_target_bounds(bounds);
                    }
                }
            }
        }
        
        // Update visibility based on hover duration
        if let Some(widget_id) = self.active_widget {
            if let Some(tooltip) = self.tooltips.get_mut(&widget_id) {
                if let Some(hover_start) = tooltip.hover_start {
                    let elapsed = now.duration_since(hover_start);
                    if elapsed >= Duration::from_millis(tooltip.delay_ms as u64) {
                        tooltip.visible = true;
                    }
                }
            }
        }
        
        self.last_update = now;
    }
    
    pub fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if let Some(widget_id) = self.active_widget {
            if let Some(tooltip) = self.tooltips.get(&widget_id) {
                tooltip.draw(context, theme);
            }
        }
    }
    
    pub fn hide_all(&mut self) {
        for tooltip in self.tooltips.values_mut() {
            tooltip.visible = false;
            tooltip.hover_start = None;
        }
        self.active_widget = None;
    }
}

// Singleton instance
use std::sync::{Mutex, OnceLock};

static TOOLTIP_MANAGER: OnceLock<Mutex<TooltipManager>> = OnceLock::new();

pub fn tooltip_manager() -> std::sync::MutexGuard<'static, TooltipManager> {
    TOOLTIP_MANAGER.get_or_init(|| {
        Mutex::new(TooltipManager::new())
    }).lock().unwrap()
}

// Helper trait to add tooltip support to widgets
pub trait TooltipExt {
    fn set_tooltip(&mut self, text: impl Into<String>);
    fn clear_tooltip(&mut self);
}

// Widget extension to support tooltips
pub struct TooltipWidget {
    pub widget_id: WidgetId,
    pub tooltip_text: Option<String>,
}

impl TooltipWidget {
    pub fn new(widget_id: WidgetId) -> Self {
        Self {
            widget_id,
            tooltip_text: None,
        }
    }
    
    pub fn set_tooltip(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.tooltip_text = Some(text.clone());
        let tooltip = Tooltip::new(self.widget_id, text);
        tooltip_manager().register_tooltip(self.widget_id, tooltip);
    }
    
    pub fn clear_tooltip(&mut self) {
        self.tooltip_text = None;
        tooltip_manager().unregister_tooltip(self.widget_id);
    }
}
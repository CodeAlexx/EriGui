use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone)]
pub struct StatusPanel {
    pub text: String,
    pub width: StatusPanelWidth,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone)]
pub enum StatusPanelWidth {
    Fixed(i32),
    Spring,
    Content,
}

#[derive(Debug, Clone, Copy)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl StatusPanel {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            width: StatusPanelWidth::Spring,
            alignment: TextAlignment::Left,
        }
    }
    
    pub fn with_fixed_width(mut self, width: i32) -> Self {
        self.width = StatusPanelWidth::Fixed(width);
        self
    }
    
    pub fn with_spring_width(mut self) -> Self {
        self.width = StatusPanelWidth::Spring;
        self
    }
    
    pub fn with_content_width(mut self) -> Self {
        self.width = StatusPanelWidth::Content;
        self
    }
    
    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

pub struct StatusBar {
    state: WidgetState,
    panels: Vec<StatusPanel>,
    height: i32,
    separator_style: SeparatorStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum SeparatorStyle {
    None,
    Line,
    Raised,
    Sunken,
}

impl StatusBar {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            panels: Vec::new(),
            height: 24,
            separator_style: SeparatorStyle::Sunken,
        }
    }
    
    pub fn with_separator_style(mut self, style: SeparatorStyle) -> Self {
        self.separator_style = style;
        self
    }
    
    pub fn add_panel(&mut self, panel: StatusPanel) {
        self.panels.push(panel);
    }
    
    pub fn set_panel_text(&mut self, index: usize, text: impl Into<String>) {
        if let Some(panel) = self.panels.get_mut(index) {
            panel.text = text.into();
        }
    }
    
    pub fn clear_panels(&mut self) {
        self.panels.clear();
    }
    
    fn calculate_panel_rects(&self) -> Vec<Rect> {
        let mut rects = Vec::new();
        let total_width = self.state.bounds.width();
        let panel_count = self.panels.len();
        
        if panel_count == 0 {
            return rects;
        }
        
        // Calculate widths
        let mut fixed_width = 0;
        let mut content_widths = Vec::new();
        let mut spring_count = 0;
        
        for panel in &self.panels {
            match &panel.width {
                StatusPanelWidth::Fixed(w) => {
                    fixed_width += w;
                    content_widths.push(*w);
                }
                StatusPanelWidth::Content => {
                    let width = panel.text.len() as i32 * 7 + 16; // Rough estimate
                    fixed_width += width;
                    content_widths.push(width);
                }
                StatusPanelWidth::Spring => {
                    spring_count += 1;
                    content_widths.push(0);
                }
            }
        }
        
        // Calculate spring widths
        let available_for_springs = (total_width - fixed_width - (panel_count as i32 - 1) * 2).max(0);
        let spring_width = if spring_count > 0 {
            available_for_springs / spring_count as i32
        } else {
            0
        };
        
        // Build rectangles
        let mut x = self.state.bounds.x();
        for (i, panel) in self.panels.iter().enumerate() {
            let width = match panel.width {
                StatusPanelWidth::Spring => spring_width,
                _ => content_widths[i],
            };
            
            rects.push(Rect::new(x, self.state.bounds.y(), width, self.height));
            x += width + 2; // 2px separator
        }
        
        rects
    }
    
    fn draw_separator(&self, context: &mut dyn DrawContext, x: i32, theme: &Theme) {
        match self.separator_style {
            SeparatorStyle::None => {}
            SeparatorStyle::Line => {
                context.set_color(theme.colors.border);
                context.draw_line(
                    Point::new(x, self.state.bounds.y() + 2),
                    Point::new(x, self.state.bounds.bottom() - 2),
                    1
                );
            }
            SeparatorStyle::Raised => {
                // Light line on left
                context.set_color(theme.colors.surface);
                context.draw_line(
                    Point::new(x, self.state.bounds.y() + 2),
                    Point::new(x, self.state.bounds.bottom() - 2),
                    1
                );
                // Dark line on right
                context.set_color(theme.colors.border);
                context.draw_line(
                    Point::new(x + 1, self.state.bounds.y() + 2),
                    Point::new(x + 1, self.state.bounds.bottom() - 2),
                    1
                );
            }
            SeparatorStyle::Sunken => {
                // Dark line on left
                context.set_color(theme.colors.border);
                context.draw_line(
                    Point::new(x, self.state.bounds.y() + 2),
                    Point::new(x, self.state.bounds.bottom() - 2),
                    1
                );
                // Light line on right
                context.set_color(theme.colors.surface);
                context.draw_line(
                    Point::new(x + 1, self.state.bounds.y() + 2),
                    Point::new(x + 1, self.state.bounds.bottom() - 2),
                    1
                );
            }
        }
    }
}

impl Widget for StatusBar {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(100, self.height)
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }
    
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }
        
        // Draw background
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(self.state.bounds);
        
        // Draw top border
        context.set_color(theme.colors.border);
        context.draw_line(
            Point::new(self.state.bounds.x(), self.state.bounds.y()),
            Point::new(self.state.bounds.right(), self.state.bounds.y()),
            1
        );
        
        // Draw panels
        let panel_rects = self.calculate_panel_rects();
        for (i, (panel, rect)) in self.panels.iter().zip(panel_rects.iter()).enumerate() {
            // Draw panel text
            context.set_color(theme.colors.text_secondary);
            
            let text_y = rect.center().y - theme.typography.font_size_small / 2;
            let text_x = match panel.alignment {
                TextAlignment::Left => rect.x() + 8,
                TextAlignment::Center => rect.center().x - (panel.text.len() as i32 * theme.typography.font_size_small / 4),
                TextAlignment::Right => rect.right() - 8 - (panel.text.len() as i32 * theme.typography.font_size_small / 2),
            };
            
            context.draw_text(
                &panel.text,
                Point::new(text_x, text_y),
                theme.typography.font_size_small
            );
            
            // Draw separator (except after last panel)
            if i < self.panels.len() - 1 {
                self.draw_separator(context, rect.right(), theme);
            }
        }
    }
    
    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
        EventResult::Ignored
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
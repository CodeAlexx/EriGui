use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints, MouseButtonEvent, MouseButton,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState, MouseMoveEvent, Key, KeyPressEvent,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerStyle {
    Compact,
    Wheel,
    Square,
    Sliders,
}

pub struct ColorPicker {
    state: WidgetState,
    color: Color,
    hsv: (f32, f32, f32), // Hue (0-360), Saturation (0-1), Value (0-1)
    style: ColorPickerStyle,
    show_alpha: bool,
    popup_visible: bool,
    preview_size: i32,
    
    // Interaction state
    dragging_hue: bool,
    dragging_sv: bool,
    dragging_alpha: bool,
    hover_preview: bool,
    
    // Hex input
    hex_input: String,
    hex_focused: bool,
    hex_cursor: usize,
    
    on_change: Option<Box<dyn FnMut(Color)>>,
}

impl ColorPicker {
    pub fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            color: Color::from_hex(0xFF0000),
            hsv: (0.0, 1.0, 1.0),
            style: ColorPickerStyle::Square,
            show_alpha: true,
            popup_visible: false,
            preview_size: 32,
            dragging_hue: false,
            dragging_sv: false,
            dragging_alpha: false,
            hover_preview: false,
            hex_input: String::from("FF0000"),
            hex_focused: false,
            hex_cursor: 6,
            on_change: None,
        }
    }
    
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self.hsv = Self::rgb_to_hsv(color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0);
        self.hex_input = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
        self
    }
    
    pub fn with_style(mut self, style: ColorPickerStyle) -> Self {
        self.style = style;
        self
    }
    
    pub fn with_alpha(mut self, show_alpha: bool) -> Self {
        self.show_alpha = show_alpha;
        self
    }
    
    pub fn with_on_change<F: FnMut(Color) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
    
    pub fn get_color(&self) -> Color {
        self.color
    }
    
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
        self.hsv = Self::rgb_to_hsv(color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0);
        self.hex_input = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
        if let Some(callback) = &mut self.on_change {
            callback(color);
        }
    }
    
    // Color conversion functions
    fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        
        let value = max;
        let saturation = if max > 0.0 { delta / max } else { 0.0 };
        
        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        
        let hue = if hue < 0.0 { hue + 360.0 } else { hue };
        
        (hue, saturation, value)
    }
    
    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        
        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        
        (r_prime + m, g_prime + m, b_prime + m)
    }
    
    fn update_color_from_hsv(&mut self) {
        let (r, g, b) = Self::hsv_to_rgb(self.hsv.0, self.hsv.1, self.hsv.2);
        self.color = Color::rgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            self.color.a
        );
        self.hex_input = format!("{:02X}{:02X}{:02X}", self.color.r, self.color.g, self.color.b);
        if let Some(callback) = &mut self.on_change {
            callback(self.color);
        }
    }
    
    fn update_color_from_hex(&mut self) {
        if let Ok(value) = u32::from_str_radix(&self.hex_input, 16) {
            self.color = Color::from_hex(value);
            self.hsv = Self::rgb_to_hsv(
                self.color.r as f32 / 255.0,
                self.color.g as f32 / 255.0,
                self.color.b as f32 / 255.0
            );
            if let Some(callback) = &mut self.on_change {
                callback(self.color);
            }
        }
    }
    
    fn draw_compact(&self, context: &mut dyn DrawContext, theme: &Theme) {
        // Draw preview square
        let preview_rect = Rect::new(
            self.state.bounds.x(),
            self.state.bounds.y(),
            self.preview_size,
            self.preview_size
        );
        
        // Draw checkerboard pattern for alpha
        if self.show_alpha {
            let checker_size = 8;
            for y in 0..(self.preview_size / checker_size) {
                for x in 0..(self.preview_size / checker_size) {
                    if (x + y) % 2 == 0 {
                        context.set_color(Color::from_hex(0xCCCCCC));
                    } else {
                        context.set_color(Color::from_hex(0xFFFFFF));
                    }
                    context.fill_rect(Rect::new(
                        preview_rect.x() + x * checker_size,
                        preview_rect.y() + y * checker_size,
                        checker_size,
                        checker_size
                    ));
                }
            }
        }
        
        // Draw color
        context.set_color(self.color);
        context.fill_rect(preview_rect);
        
        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(preview_rect);
        
        // Draw dropdown arrow
        let arrow_x = preview_rect.right() + 4;
        let arrow_y = preview_rect.center().y;
        context.set_color(theme.colors.text);
        context.fill_rect(Rect::new(arrow_x, arrow_y - 2, 6, 1));
        context.fill_rect(Rect::new(arrow_x + 1, arrow_y - 1, 4, 1));
        context.fill_rect(Rect::new(arrow_x + 2, arrow_y, 2, 1));
    }
    
    fn draw_popup(&self, context: &mut dyn DrawContext, theme: &Theme) {
        let popup_rect = Rect::new(
            self.state.bounds.x(),
            self.state.bounds.bottom() + 4,
            250,
            300
        );
        
        // Draw popup background
        context.set_color(theme.colors.surface);
        context.fill_rect(popup_rect);
        context.set_color(theme.colors.border);
        context.draw_rect(popup_rect);
        
        let padding = 10;
        let content_x = popup_rect.x() + padding;
        let mut y = popup_rect.y() + padding;
        
        // Draw saturation/value square
        let sv_size = 200;
        let sv_rect = Rect::new(content_x, y, sv_size, sv_size);
        
        // Draw SV gradient
        for py in 0..sv_size {
            for px in 0..sv_size {
                let s = px as f32 / sv_size as f32;
                let v = 1.0 - (py as f32 / sv_size as f32);
                let (r, g, b) = Self::hsv_to_rgb(self.hsv.0, s, v);
                context.set_color(Color::rgb(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8
                ));
                context.fill_rect(Rect::new(
                    sv_rect.x() + px,
                    sv_rect.y() + py,
                    1,
                    1
                ));
            }
        }
        
        // Draw SV cursor
        let cursor_x = sv_rect.x() + (self.hsv.1 * sv_size as f32) as i32;
        let cursor_y = sv_rect.y() + ((1.0 - self.hsv.2) * sv_size as f32) as i32;
        context.set_color(Color::WHITE);
        context.draw_circle(Point::new(cursor_x, cursor_y), 5, 16);
        context.set_color(Color::BLACK);
        context.draw_circle(Point::new(cursor_x, cursor_y), 4, 16);
        
        y += sv_size + padding;
        
        // Draw hue slider
        let hue_height = 20;
        let hue_rect = Rect::new(content_x, y, sv_size, hue_height);
        
        // Draw hue gradient
        for x in 0..sv_size {
            let hue = (x as f32 / sv_size as f32) * 360.0;
            let (r, g, b) = Self::hsv_to_rgb(hue, 1.0, 1.0);
            context.set_color(Color::rgb(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8
            ));
            context.fill_rect(Rect::new(hue_rect.x() + x, hue_rect.y(), 1, hue_height));
        }
        
        // Draw hue cursor
        let hue_cursor_x = hue_rect.x() + (self.hsv.0 / 360.0 * sv_size as f32) as i32;
        context.set_color(Color::WHITE);
        context.fill_rect(Rect::new(hue_cursor_x - 2, hue_rect.y() - 2, 4, hue_height + 4));
        context.set_color(Color::BLACK);
        context.draw_rect(Rect::new(hue_cursor_x - 2, hue_rect.y() - 2, 4, hue_height + 4));
        
        y += hue_height + padding;
        
        // Draw alpha slider if enabled
        if self.show_alpha {
            let alpha_rect = Rect::new(content_x, y, sv_size, hue_height);
            
            // Draw checkerboard
            let checker_size = 5;
            for cy in 0..(hue_height / checker_size) {
                for cx in 0..(sv_size / checker_size) {
                    if (cx + cy) % 2 == 0 {
                        context.set_color(Color::from_hex(0xCCCCCC));
                    } else {
                        context.set_color(Color::from_hex(0xFFFFFF));
                    }
                    context.fill_rect(Rect::new(
                        alpha_rect.x() + cx * checker_size,
                        alpha_rect.y() + cy * checker_size,
                        checker_size,
                        checker_size
                    ));
                }
            }
            
            // Draw alpha gradient
            for x in 0..sv_size {
                let alpha = (x as f32 / sv_size as f32 * 255.0) as u8;
                context.set_color(Color::rgba(self.color.r, self.color.g, self.color.b, alpha));
                context.fill_rect(Rect::new(alpha_rect.x() + x, alpha_rect.y(), 1, hue_height));
            }
            
            // Draw alpha cursor
            let alpha_cursor_x = alpha_rect.x() + (self.color.a as f32 / 255.0 * sv_size as f32) as i32;
            context.set_color(Color::WHITE);
            context.fill_rect(Rect::new(alpha_cursor_x - 2, alpha_rect.y() - 2, 4, hue_height + 4));
            context.set_color(Color::BLACK);
            context.draw_rect(Rect::new(alpha_cursor_x - 2, alpha_rect.y() - 2, 4, hue_height + 4));
            
            y += hue_height + padding;
        }
        
        // Draw hex input
        let hex_rect = Rect::new(content_x, y, 80, 24);
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(hex_rect);
        context.set_color(if self.hex_focused { theme.colors.primary } else { theme.colors.border });
        context.draw_rect(hex_rect);
        
        // Draw # symbol
        context.set_color(theme.colors.text_secondary);
        context.draw_text("#", Point::new(hex_rect.x() + 4, hex_rect.center().y + 6), theme.typography.font_size_base);
        
        // Draw hex value
        context.set_color(theme.colors.text);
        context.draw_text(&self.hex_input, Point::new(hex_rect.x() + 16, hex_rect.center().y + 6), theme.typography.font_size_base);
        
        // Draw cursor if focused
        if self.hex_focused {
            let cursor_x = hex_rect.x() + 16 + (self.hex_cursor as i32 * 8);
            context.set_color(theme.colors.text);
            context.fill_rect(Rect::new(cursor_x, hex_rect.y() + 4, 1, hex_rect.height() - 8));
        }
    }
}

impl Widget for ColorPicker {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        match self.style {
            ColorPickerStyle::Compact => Size::new(self.preview_size + 12, self.preview_size),
            _ => Size::new(250, 300),
        }
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }
    
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }
        
        match self.style {
            ColorPickerStyle::Compact => {
                self.draw_compact(context, theme);
                if self.popup_visible {
                    self.draw_popup(context, theme);
                }
            }
            _ => {
                // For now, other styles just draw the full picker
                self.draw_popup(context, theme);
            }
        }
    }
    
    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }
        
        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if self.style == ColorPickerStyle::Compact {
                    let preview_rect = Rect::new(
                        self.state.bounds.x(),
                        self.state.bounds.y(),
                        self.state.bounds.width(),
                        self.preview_size
                    );
                    
                    if preview_rect.contains(*position) && *pressed {
                        self.popup_visible = !self.popup_visible;
                        return EventResult::Consumed;
                    }
                }
                
                if self.popup_visible {
                    let popup_rect = Rect::new(
                        self.state.bounds.x(),
                        self.state.bounds.bottom() + 4,
                        250,
                        300
                    );
                    
                    if popup_rect.contains(*position) {
                        let padding = 10;
                        let content_x = popup_rect.x() + padding;
                        let mut y = popup_rect.y() + padding;
                        
                        // Check SV square
                        let sv_size = 200;
                        let sv_rect = Rect::new(content_x, y, sv_size, sv_size);
                        if sv_rect.contains(*position) {
                            if *pressed {
                                self.dragging_sv = true;
                                let s = ((position.x - sv_rect.x()) as f32 / sv_size as f32).max(0.0).min(1.0);
                                let v = 1.0 - ((position.y - sv_rect.y()) as f32 / sv_size as f32).max(0.0).min(1.0);
                                self.hsv.1 = s;
                                self.hsv.2 = v;
                                self.update_color_from_hsv();
                            }
                            return EventResult::Consumed;
                        }
                        
                        y += sv_size + padding;
                        
                        // Check hue slider
                        let hue_height = 20;
                        let hue_rect = Rect::new(content_x, y, sv_size, hue_height);
                        if hue_rect.contains(*position) || (position.y >= hue_rect.y() - 5 && position.y <= hue_rect.bottom() + 5 && position.x >= hue_rect.x() && position.x <= hue_rect.right()) {
                            if *pressed {
                                self.dragging_hue = true;
                                let hue = ((position.x - hue_rect.x()) as f32 / sv_size as f32 * 360.0).max(0.0).min(360.0);
                                self.hsv.0 = hue;
                                self.update_color_from_hsv();
                            }
                            return EventResult::Consumed;
                        }
                        
                        y += hue_height + padding;
                        
                        // Check alpha slider
                        if self.show_alpha {
                            let alpha_rect = Rect::new(content_x, y, sv_size, hue_height);
                            if alpha_rect.contains(*position) || (position.y >= alpha_rect.y() - 5 && position.y <= alpha_rect.bottom() + 5 && position.x >= alpha_rect.x() && position.x <= alpha_rect.right()) {
                                if *pressed {
                                    self.dragging_alpha = true;
                                    let alpha = ((position.x - alpha_rect.x()) as f32 / sv_size as f32 * 255.0) as u8;
                                    self.color.a = alpha;
                                    if let Some(callback) = &mut self.on_change {
                                        callback(self.color);
                                    }
                                }
                                return EventResult::Consumed;
                            }
                            
                            y += hue_height + padding;
                        }
                        
                        // Check hex input
                        let hex_rect = Rect::new(content_x, y, 80, 24);
                        if hex_rect.contains(*position) && *pressed {
                            self.hex_focused = true;
                            return EventResult::Consumed;
                        } else {
                            self.hex_focused = false;
                        }
                    } else if *pressed {
                        // Click outside popup - close it
                        self.popup_visible = false;
                        return EventResult::Consumed;
                    }
                }
                
                if !*pressed {
                    self.dragging_hue = false;
                    self.dragging_sv = false;
                    self.dragging_alpha = false;
                }
            }
            Event::MouseMove(MouseMoveEvent { position, .. }) => {
                if self.popup_visible {
                    let popup_rect = Rect::new(
                        self.state.bounds.x(),
                        self.state.bounds.bottom() + 4,
                        250,
                        300
                    );
                    
                    let padding = 10;
                    let content_x = popup_rect.x() + padding;
                    let sv_size = 200;
                    
                    if self.dragging_sv {
                        let sv_rect = Rect::new(content_x, popup_rect.y() + padding, sv_size, sv_size);
                        let s = ((position.x - sv_rect.x()) as f32 / sv_size as f32).max(0.0).min(1.0);
                        let v = 1.0 - ((position.y - sv_rect.y()) as f32 / sv_size as f32).max(0.0).min(1.0);
                        self.hsv.1 = s;
                        self.hsv.2 = v;
                        self.update_color_from_hsv();
                        return EventResult::Consumed;
                    }
                    
                    if self.dragging_hue {
                        let hue_rect = Rect::new(content_x, popup_rect.y() + padding + sv_size + padding, sv_size, 20);
                        let hue = ((position.x - hue_rect.x()) as f32 / sv_size as f32 * 360.0).max(0.0).min(360.0);
                        self.hsv.0 = hue;
                        self.update_color_from_hsv();
                        return EventResult::Consumed;
                    }
                    
                    if self.dragging_alpha && self.show_alpha {
                        let alpha_rect = Rect::new(content_x, popup_rect.y() + padding + sv_size + padding + 20 + padding, sv_size, 20);
                        let alpha = ((position.x - alpha_rect.x()) as f32 / sv_size as f32 * 255.0) as u8;
                        self.color.a = alpha;
                        if let Some(callback) = &mut self.on_change {
                            callback(self.color);
                        }
                        return EventResult::Consumed;
                    }
                }
                
                if self.style == ColorPickerStyle::Compact {
                    let preview_rect = Rect::new(
                        self.state.bounds.x(),
                        self.state.bounds.y(),
                        self.state.bounds.width(),
                        self.preview_size
                    );
                    self.hover_preview = preview_rect.contains(*position);
                }
            }
            Event::KeyPress(KeyPressEvent { key, .. }) => {
                if self.hex_focused {
                    match key {
                        Key::Backspace => {
                            if self.hex_cursor > 0 {
                                self.hex_input.remove(self.hex_cursor - 1);
                                self.hex_cursor -= 1;
                                self.update_color_from_hex();
                            }
                            return EventResult::Consumed;
                        }
                        Key::Delete => {
                            if self.hex_cursor < self.hex_input.len() {
                                self.hex_input.remove(self.hex_cursor);
                                self.update_color_from_hex();
                            }
                            return EventResult::Consumed;
                        }
                        Key::Left => {
                            if self.hex_cursor > 0 {
                                self.hex_cursor -= 1;
                            }
                            return EventResult::Consumed;
                        }
                        Key::Right => {
                            if self.hex_cursor < self.hex_input.len() {
                                self.hex_cursor += 1;
                            }
                            return EventResult::Consumed;
                        }
                        Key::Character(ch) => {
                            if self.hex_input.len() < 6 && ch.is_ascii_hexdigit() {
                                self.hex_input.insert(self.hex_cursor, ch.to_ascii_uppercase());
                                self.hex_cursor += 1;
                                self.update_color_from_hex();
                            }
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        
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
        self.state.focused
    }
    
    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
    }
    
    fn can_focus(&self) -> bool {
        self.state.enabled && self.state.visible
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
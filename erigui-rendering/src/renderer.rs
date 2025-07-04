use erigui_core::{Color, DrawContext, Point, Rect, Size};
use crate::{GlContext, FontRenderer};
use winit::event_loop::EventLoop;
use std::cell::RefCell;

pub struct Renderer {
    gl: GlContext,
    font_renderer: RefCell<FontRenderer>,
    clip_stack: Vec<Rect>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, width: i32, height: i32, title: &str) -> erigui_core::Result<Self> {
        let gl = GlContext::new(event_loop, width, height, title)?;
        let font_renderer = RefCell::new(FontRenderer::new()?);
        
        Ok(Self {
            gl,
            font_renderer,
            clip_stack: Vec::new(),
        })
    }
    
    pub fn window(&self) -> &winit::window::Window {
        self.gl.window()
    }
    
    pub fn begin_frame(&mut self, clear_color: Color) {
        self.gl.clear(clear_color);
        self.clip_stack.clear();
    }
    
    pub fn end_frame(&mut self) {
        self.gl.swap_buffers();
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.gl.resize(width, height);
    }
    
    pub fn viewport_size(&self) -> Size {
        self.gl.viewport_size()
    }
}

impl DrawContext for Renderer {
    fn set_color(&mut self, color: Color) {
        self.gl.set_color(color);
    }
    
    fn draw_rect(&mut self, rect: Rect) {
        self.gl.draw_rect(rect);
    }
    
    fn fill_rect(&mut self, rect: Rect) {
        self.gl.fill_rect(rect);
    }
    
    fn draw_circle(&mut self, center: Point, radius: i32, segments: i32) {
        self.gl.draw_circle(center, radius, segments);
    }
    
    fn fill_circle(&mut self, center: Point, radius: i32, segments: i32) {
        self.gl.fill_circle(center, radius, segments);
    }
    
    fn draw_line(&mut self, start: Point, end: Point, thickness: i32) {
        self.gl.draw_line(start, end, thickness);
    }
    
    fn draw_text(&mut self, text: &str, position: Point, size: i32) {
        self.font_renderer.borrow_mut().draw_text(
            &self.gl,
            text,
            position,
            size,
            self.gl.viewport_size()
        );
    }
    
    fn measure_text(&self, text: &str, size: i32) -> Size {
        self.font_renderer.borrow().measure_text(text, size)
    }
    
    fn push_clip_rect(&mut self, rect: Rect) {
        let clipped = if let Some(current) = self.clip_stack.last() {
            rect.intersection(current).unwrap_or(Rect::new(0, 0, 0, 0))
        } else {
            rect
        };
        
        self.clip_stack.push(clipped);
        self.gl.enable_scissor(clipped);
    }
    
    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        
        if let Some(rect) = self.clip_stack.last() {
            self.gl.enable_scissor(*rect);
        } else {
            self.gl.disable_scissor();
        }
    }
    
    fn viewport_size(&self) -> Size {
        self.gl.viewport_size()
    }
    
    fn draw_rounded_rect(&mut self, rect: Rect, corner_radius: i32) {
        self.gl.draw_rounded_rect(rect, corner_radius);
    }
    
    fn fill_rounded_rect(&mut self, rect: Rect, corner_radius: i32) {
        self.gl.fill_rounded_rect(rect, corner_radius);
    }
    
    fn draw_gradient_rect(&mut self, rect: Rect, top_color: Color, bottom_color: Color, horizontal: bool) {
        self.gl.draw_gradient_rect(rect, top_color, bottom_color, horizontal);
    }
    
    fn draw_shadow(&mut self, rect: Rect, shadow_color: Color, blur_radius: i32, offset: Point) {
        self.gl.draw_shadow(rect, shadow_color, blur_radius, offset);
    }
    
    fn draw_ellipse(&mut self, center: Point, radius_x: i32, radius_y: i32, segments: i32) {
        self.gl.draw_ellipse(center, radius_x, radius_y, segments);
    }
    
    fn fill_ellipse(&mut self, center: Point, radius_x: i32, radius_y: i32, segments: i32) {
        self.gl.fill_ellipse(center, radius_x, radius_y, segments);
    }
    
    fn draw_polygon(&mut self, points: &[Point]) {
        self.gl.draw_polygon(points);
    }
    
    fn fill_polygon(&mut self, points: &[Point]) {
        self.gl.fill_polygon(points);
    }
    
    fn set_line_width(&mut self, width: i32) {
        self.gl.set_line_width(width);
    }
}
use erigui_core::{Color, Point, Rect, Size, Result};
use glutin::context::{ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface, SwapInterval};
use glutin::config::ConfigTemplateBuilder;
use glutin::prelude::*;
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use std::num::NonZeroU32;
use std::ffi::CString;

use crate::gl;

pub struct GlContext {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    viewport_size: Size,
}

impl GlContext {
    pub fn new(event_loop: &EventLoop<()>, width: i32, height: i32, title: &str) -> Result<Self> {
        // Force Wayland backend only
        std::env::set_var("WINIT_UNIX_BACKEND", "wayland");
        
        let window_builder = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width as f64, height as f64));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .prefer_hardware_accelerated(Some(true));

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs.max_by_key(|c| c.num_samples()).unwrap()
            })
            .map_err(|e| erigui_core::EriGuiError::OpenGLInit(format!("Failed to create display: {}", e)))?;

        let window = window.ok_or_else(|| {
            erigui_core::EriGuiError::WindowCreation("Failed to create window".into())
        })?;

        let raw_window_handle = window.raw_window_handle();

        let context_attributes = ContextAttributesBuilder::new()
            .with_profile(glutin::context::GlProfile::Compatibility)
            .build(Some(raw_window_handle));

        let gl_display = gl_config.display();

        let not_current_gl_context = unsafe {
            gl_display.create_context(&gl_config, &context_attributes)
                .map_err(|e| erigui_core::EriGuiError::OpenGLInit(format!("Failed to create context: {}", e)))?
        };

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(width as u32).unwrap(),
            NonZeroU32::new(height as u32).unwrap(),
        );

        let gl_surface = unsafe {
            gl_display.create_window_surface(&gl_config, &surface_attributes)
                .map_err(|e| erigui_core::EriGuiError::OpenGLInit(format!("Failed to create surface: {}", e)))?
        };

        let gl_context = not_current_gl_context
            .make_current(&gl_surface)
            .map_err(|e| erigui_core::EriGuiError::OpenGLInit(format!("Failed to make context current: {}", e)))?;

        // Set vsync
        gl_surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())).ok();

        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()) as *const _
        });

        unsafe {
            let version = gl::GetString(gl::VERSION);
            if !version.is_null() {
                let version_str = std::ffi::CStr::from_ptr(version as *const i8).to_string_lossy();
                println!("OpenGL Version: {}", version_str);
            }
            
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            
            gl::Viewport(0, 0, width, height);
            
            // Set up 2D orthographic projection
            let err = gl::GetError();
            if err != gl::NO_ERROR {
                println!("GL Error before matrix setup: 0x{:x}", err);
            }
            
            gl::MatrixMode(gl::PROJECTION);
            gl::LoadIdentity();
            gl::Ortho(0.0, width as f64, height as f64, 0.0, -1.0, 1.0);
            gl::MatrixMode(gl::MODELVIEW);
            gl::LoadIdentity();
            
            let err = gl::GetError();
            if err != gl::NO_ERROR {
                println!("GL Error after matrix setup: 0x{:x}", err);
            }
        }

        Ok(Self {
            window,
            gl_context,
            gl_surface,
            viewport_size: Size::new(width, height),
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn swap_buffers(&self) {
        unsafe {
            let err = gl::GetError();
            if err != gl::NO_ERROR {
                println!("GL Error before swap: 0x{:x}", err);
            }
        }
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }

    pub fn clear(&self, color: Color) {
        unsafe {
            let [r, g, b, a] = color.to_gl_color();
            gl::ClearColor(r, g, b, a);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn set_viewport(&mut self, size: Size) {
        self.viewport_size = size;
        unsafe {
            gl::Viewport(0, 0, size.width, size.height);
            gl::MatrixMode(gl::PROJECTION);
            gl::LoadIdentity();
            gl::Ortho(0.0, size.width as f64, size.height as f64, 0.0, -1.0, 1.0);
            gl::MatrixMode(gl::MODELVIEW);
        }
    }

    pub fn viewport_size(&self) -> Size {
        self.viewport_size
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.gl_surface.resize(&self.gl_context, width, height);
            self.set_viewport(Size::new(width.get() as i32, height.get() as i32));
        }
    }

    pub fn set_color(&self, color: Color) {
        unsafe {
            let [r, g, b, a] = color.to_gl_color();
            gl::Color4f(r, g, b, a);
        }
    }

    pub fn draw_rect(&self, rect: Rect) {
        unsafe {
            gl::Begin(gl::LINE_LOOP);
            gl::Vertex2i(rect.x(), rect.y());
            gl::Vertex2i(rect.right(), rect.y());
            gl::Vertex2i(rect.right(), rect.bottom());
            gl::Vertex2i(rect.x(), rect.bottom());
            gl::End();
        }
    }

    pub fn fill_rect(&self, rect: Rect) {
        unsafe {
            gl::Begin(gl::QUADS);
            gl::Vertex2i(rect.x(), rect.y());
            gl::Vertex2i(rect.right(), rect.y());
            gl::Vertex2i(rect.right(), rect.bottom());
            gl::Vertex2i(rect.x(), rect.bottom());
            gl::End();
        }
    }

    pub fn draw_line(&self, start: Point, end: Point, thickness: i32) {
        unsafe {
            gl::LineWidth(thickness as f32);
            gl::Begin(gl::LINES);
            gl::Vertex2i(start.x, start.y);
            gl::Vertex2i(end.x, end.y);
            gl::End();
            gl::LineWidth(1.0);
        }
    }

    pub fn draw_circle(&self, center: Point, radius: i32, segments: i32) {
        use std::f32::consts::PI;
        
        unsafe {
            gl::Begin(gl::LINE_LOOP);
            for i in 0..segments {
                let angle = 2.0 * PI * (i as f32) / (segments as f32);
                let x = center.x + (radius as f32 * angle.cos()) as i32;
                let y = center.y + (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
        }
    }

    pub fn fill_circle(&self, center: Point, radius: i32, segments: i32) {
        use std::f32::consts::PI;
        
        unsafe {
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(center.x, center.y);
            for i in 0..=segments {
                let angle = 2.0 * PI * (i as f32) / (segments as f32);
                let x = center.x + (radius as f32 * angle.cos()) as i32;
                let y = center.y + (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
        }
    }

    pub fn enable_scissor(&self, rect: Rect) {
        unsafe {
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(
                rect.x(),
                self.viewport_size.height - rect.bottom(),
                rect.width(),
                rect.height(),
            );
        }
    }

    pub fn disable_scissor(&self) {
        unsafe {
            gl::Disable(gl::SCISSOR_TEST);
        }
    }
    
    pub fn draw_rounded_rect(&self, rect: Rect, corner_radius: i32) {
        use std::f32::consts::PI;
        
        let radius = corner_radius.min(rect.width() / 2).min(rect.height() / 2);
        if radius <= 0 {
            self.draw_rect(rect);
            return;
        }
        
        unsafe {
            gl::Begin(gl::LINE_LOOP);
            
            // Top-right corner
            for i in 0..=10 {
                let angle = (i as f32 / 10.0) * PI / 2.0;
                let x = rect.right() - radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.y() + radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            
            // Top-left corner
            for i in 0..=10 {
                let angle = PI / 2.0 + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.x() + radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.y() + radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            
            // Bottom-left corner
            for i in 0..=10 {
                let angle = PI + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.x() + radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.bottom() - radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            
            // Bottom-right corner
            for i in 0..=10 {
                let angle = 3.0 * PI / 2.0 + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.right() - radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.bottom() - radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            
            gl::End();
        }
    }
    
    pub fn fill_rounded_rect(&self, rect: Rect, corner_radius: i32) {
        use std::f32::consts::PI;
        
        let radius = corner_radius.min(rect.width() / 2).min(rect.height() / 2);
        if radius <= 0 {
            self.fill_rect(rect);
            return;
        }
        
        unsafe {
            // Draw center rectangle
            gl::Begin(gl::QUADS);
            gl::Vertex2i(rect.x() + radius, rect.y());
            gl::Vertex2i(rect.right() - radius, rect.y());
            gl::Vertex2i(rect.right() - radius, rect.bottom());
            gl::Vertex2i(rect.x() + radius, rect.bottom());
            gl::End();
            
            // Draw left rectangle
            gl::Begin(gl::QUADS);
            gl::Vertex2i(rect.x(), rect.y() + radius);
            gl::Vertex2i(rect.x() + radius, rect.y() + radius);
            gl::Vertex2i(rect.x() + radius, rect.bottom() - radius);
            gl::Vertex2i(rect.x(), rect.bottom() - radius);
            gl::End();
            
            // Draw right rectangle
            gl::Begin(gl::QUADS);
            gl::Vertex2i(rect.right() - radius, rect.y() + radius);
            gl::Vertex2i(rect.right(), rect.y() + radius);
            gl::Vertex2i(rect.right(), rect.bottom() - radius);
            gl::Vertex2i(rect.right() - radius, rect.bottom() - radius);
            gl::End();
            
            // Draw corners
            // Top-right
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(rect.right() - radius, rect.y() + radius);
            for i in 0..=10 {
                let angle = (i as f32 / 10.0) * PI / 2.0;
                let x = rect.right() - radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.y() + radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
            
            // Top-left
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(rect.x() + radius, rect.y() + radius);
            for i in 0..=10 {
                let angle = PI / 2.0 + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.x() + radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.y() + radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
            
            // Bottom-left
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(rect.x() + radius, rect.bottom() - radius);
            for i in 0..=10 {
                let angle = PI + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.x() + radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.bottom() - radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
            
            // Bottom-right
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(rect.right() - radius, rect.bottom() - radius);
            for i in 0..=10 {
                let angle = 3.0 * PI / 2.0 + (i as f32 / 10.0) * PI / 2.0;
                let x = rect.right() - radius + (radius as f32 * angle.cos()) as i32;
                let y = rect.bottom() - radius - (radius as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
        }
    }
    
    pub fn draw_gradient_rect(&self, rect: Rect, top_color: Color, bottom_color: Color, horizontal: bool) {
        unsafe {
            gl::Begin(gl::QUADS);
            
            if horizontal {
                // Left to right gradient
                let [r1, g1, b1, a1] = top_color.to_gl_color();
                let [r2, g2, b2, a2] = bottom_color.to_gl_color();
                
                gl::Color4f(r1, g1, b1, a1);
                gl::Vertex2i(rect.x(), rect.y());
                gl::Vertex2i(rect.x(), rect.bottom());
                
                gl::Color4f(r2, g2, b2, a2);
                gl::Vertex2i(rect.right(), rect.bottom());
                gl::Vertex2i(rect.right(), rect.y());
            } else {
                // Top to bottom gradient
                let [r1, g1, b1, a1] = top_color.to_gl_color();
                let [r2, g2, b2, a2] = bottom_color.to_gl_color();
                
                gl::Color4f(r1, g1, b1, a1);
                gl::Vertex2i(rect.x(), rect.y());
                gl::Vertex2i(rect.right(), rect.y());
                
                gl::Color4f(r2, g2, b2, a2);
                gl::Vertex2i(rect.right(), rect.bottom());
                gl::Vertex2i(rect.x(), rect.bottom());
            }
            
            gl::End();
        }
    }
    
    pub fn draw_shadow(&self, rect: Rect, shadow_color: Color, blur_radius: i32, offset: Point) {
        // Simple shadow implementation using multiple transparent rectangles
        let steps = (blur_radius as f32).sqrt() as i32 + 1;
        let base_alpha = shadow_color.a;
        
        for i in (0..steps).rev() {
            let alpha = (base_alpha as f32 * (1.0 - (i as f32 / steps as f32).powi(2))) as u8;
            let color = shadow_color.with_alpha(alpha);
            self.set_color(color);
            
            let expansion = i * 2;
            let shadow_rect = Rect::new(
                rect.x() + offset.x - expansion,
                rect.y() + offset.y - expansion,
                rect.width() + expansion * 2,
                rect.height() + expansion * 2
            );
            
            self.fill_rect(shadow_rect);
        }
    }
    
    pub fn draw_ellipse(&self, center: Point, radius_x: i32, radius_y: i32, segments: i32) {
        use std::f32::consts::PI;
        
        unsafe {
            gl::Begin(gl::LINE_LOOP);
            for i in 0..segments {
                let angle = 2.0 * PI * (i as f32) / (segments as f32);
                let x = center.x + (radius_x as f32 * angle.cos()) as i32;
                let y = center.y + (radius_y as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
        }
    }
    
    pub fn fill_ellipse(&self, center: Point, radius_x: i32, radius_y: i32, segments: i32) {
        use std::f32::consts::PI;
        
        unsafe {
            gl::Begin(gl::TRIANGLE_FAN);
            gl::Vertex2i(center.x, center.y);
            for i in 0..=segments {
                let angle = 2.0 * PI * (i as f32) / (segments as f32);
                let x = center.x + (radius_x as f32 * angle.cos()) as i32;
                let y = center.y + (radius_y as f32 * angle.sin()) as i32;
                gl::Vertex2i(x, y);
            }
            gl::End();
        }
    }
    
    pub fn draw_polygon(&self, points: &[Point]) {
        if points.len() < 2 {
            return;
        }
        
        unsafe {
            gl::Begin(gl::LINE_LOOP);
            for point in points {
                gl::Vertex2i(point.x, point.y);
            }
            gl::End();
        }
    }
    
    pub fn fill_polygon(&self, points: &[Point]) {
        if points.len() < 3 {
            return;
        }
        
        unsafe {
            gl::Begin(gl::POLYGON);
            for point in points {
                gl::Vertex2i(point.x, point.y);
            }
            gl::End();
        }
    }
    
    pub fn set_line_width(&self, width: i32) {
        unsafe {
            gl::LineWidth(width as f32);
        }
    }
}
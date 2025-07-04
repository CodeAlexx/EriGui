use erigui_core::{Point, Size, Result, EriGuiError};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use crate::GlContext;

use crate::gl;

const DEFAULT_FONT_DATA: &[u8] = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");

struct GlyphInfo {
    texture_id: u32,
    width: i32,
    height: i32,
    advance: i32,
    bearing_x: i32,
    bearing_y: i32,
}

pub struct FontRenderer {
    font: Font,
    glyph_cache: HashMap<(char, u32), GlyphInfo>,
}

impl FontRenderer {
    pub fn new() -> Result<Self> {
        let font = Font::from_bytes(DEFAULT_FONT_DATA, FontSettings::default())
            .map_err(|e| EriGuiError::FontLoading(e.to_string()))?;
            
        Ok(Self {
            font,
            glyph_cache: HashMap::new(),
        })
    }
    
    pub fn load_font(&mut self, font_data: &[u8]) -> Result<()> {
        self.font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| EriGuiError::FontLoading(e.to_string()))?;
        self.glyph_cache.clear();
        Ok(())
    }
    
    pub fn measure_text(&self, text: &str, size: i32) -> Size {
        let scale = size as f32;
        let mut width = 0.0;
        let line_height = self.font.horizontal_line_metrics(scale)
            .map(|m| m.new_line_size)
            .unwrap_or(scale);
        
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, scale);
            width += metrics.advance_width;
        }
        
        Size::new(width as i32, line_height as i32)
    }
    
    pub fn draw_text(
        &mut self,
        gl: &GlContext,
        text: &str,
        position: Point,
        size: i32,
        viewport_size: Size,
    ) {
        let scale = size as f32;
        let mut cursor_x = position.x as f32;
        let cursor_y = position.y as f32;
        
        unsafe {
            gl::Enable(gl::TEXTURE_2D);
            // Set texture environment to use red channel as alpha
            gl::TexEnvi(gl::TEXTURE_ENV, gl::TEXTURE_ENV_MODE, gl::MODULATE as i32);
        }
        
        for ch in text.chars() {
            let key = (ch, size as u32);
            
            // Get or create glyph
            if !self.glyph_cache.contains_key(&key) {
                let scale = size as f32;
                let (metrics, bitmap) = self.font.rasterize(ch, scale);
                
                if !bitmap.is_empty() {
                    let texture_id = self.create_texture(&bitmap, metrics.width, metrics.height);
                    
                    let glyph_info = GlyphInfo {
                        texture_id,
                        width: metrics.width as i32,
                        height: metrics.height as i32,
                        advance: metrics.advance_width as i32,
                        bearing_x: metrics.xmin as i32,
                        bearing_y: metrics.ymin as i32,
                    };
                    
                    self.glyph_cache.insert(key, glyph_info);
                }
            }
            
            // Draw glyph if it exists
            if let Some(glyph_info) = self.glyph_cache.get(&key) {
                let glyph_pos = Point::new(
                    (cursor_x + glyph_info.bearing_x as f32) as i32,
                    (cursor_y - glyph_info.bearing_y as f32 + scale) as i32,
                );
                
                self.draw_glyph(gl, glyph_info, glyph_pos, viewport_size);
                cursor_x += glyph_info.advance as f32;
            }
        }
        
        unsafe {
            gl::Disable(gl::TEXTURE_2D);
        }
    }
    
    
    fn create_texture(&self, bitmap: &[u8], width: usize, height: usize) -> u32 {
        let mut texture_id = 0;
        
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            
            // Fontdue returns a grayscale bitmap - convert to RGBA
            let mut rgba_bitmap = Vec::with_capacity(bitmap.len() * 4);
            for &alpha in bitmap.iter() {
                rgba_bitmap.push(255); // R
                rgba_bitmap.push(255); // G
                rgba_bitmap.push(255); // B
                rgba_bitmap.push(alpha); // A
            }
            
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                rgba_bitmap.as_ptr() as *const _,
            );
        }
        
        texture_id
    }
    
    fn draw_glyph(
        &self,
        _gl: &GlContext,
        glyph: &GlyphInfo,
        position: Point,
        _viewport_size: Size,
    ) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, glyph.texture_id);
            
            let x = position.x;
            let y = position.y;
            let w = glyph.width;
            let h = glyph.height;
            
            gl::Begin(gl::QUADS);
            gl::TexCoord2f(0.0, 0.0);
            gl::Vertex2i(x, y);
            
            gl::TexCoord2f(1.0, 0.0);
            gl::Vertex2i(x + w, y);
            
            gl::TexCoord2f(1.0, 1.0);
            gl::Vertex2i(x + w, y + h);
            
            gl::TexCoord2f(0.0, 1.0);
            gl::Vertex2i(x, y + h);
            gl::End();
        }
    }
}

impl Drop for FontRenderer {
    fn drop(&mut self) {
        unsafe {
            for glyph in self.glyph_cache.values() {
                gl::DeleteTextures(1, &glyph.texture_id);
            }
        }
    }
}
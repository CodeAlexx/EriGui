use erigui_core::{Point, Size, Result, EriGuiError};
use freetype::{Library, Face};
use freetype::face::{DEFAULT, RENDER};
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
    library: Library,
    font_data: Box<[u8]>,
    face_index: isize,
    glyph_cache: HashMap<(char, u32), GlyphInfo>,
}

impl FontRenderer {
    pub fn new() -> Result<Self> {
        let library = Library::init()
            .map_err(|e| EriGuiError::FontLoading(format!("Failed to init FreeType: {:?}", e)))?;
            
        let font_data = DEFAULT_FONT_DATA.to_vec().into_boxed_slice();
            
        Ok(Self {
            library,
            font_data,
            face_index: 0,
            glyph_cache: HashMap::new(),
        })
    }
    
    pub fn load_font(&mut self, font_data: &[u8]) -> Result<()> {
        self.font_data = font_data.to_vec().into_boxed_slice();
        self.glyph_cache.clear();
        Ok(())
    }
    
    fn get_face(&self) -> Result<Face> {
        self.library.new_memory_face(&self.font_data, self.face_index)
            .map_err(|e| EriGuiError::FontLoading(format!("Failed to load font: {:?}", e)))
    }
    
    pub fn measure_text(&self, text: &str, size: i32) -> Size {
        let face = match self.get_face() {
            Ok(f) => f,
            Err(_) => return Size::new(0, 0),
        };
        
        // Set pixel size
        face.set_pixel_sizes(0, size as u32).ok();
        
        let mut width = 0;
        let mut max_height = 0;
        
        for ch in text.chars() {
            if let Ok(_) = face.load_char(ch as usize, DEFAULT) {
                let glyph = face.glyph();
                width += (glyph.advance().x >> 6) as i32;
                
                let metrics = glyph.metrics();
                let height = (metrics.height >> 6) as i32;
                max_height = max_height.max(height);
            }
        }
        
        Size::new(width, max_height.max(size))
    }
    
    pub fn draw_text(
        &mut self,
        gl: &GlContext,
        text: &str,
        position: Point,
        size: i32,
        viewport_size: Size,
    ) {
        let mut cursor_x = position.x as f32;
        let baseline_y = position.y as f32;
        
        unsafe {
            gl::Enable(gl::TEXTURE_2D);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        
        for ch in text.chars() {
            let key = (ch, size as u32);
            
            // Get or create glyph
            if !self.glyph_cache.contains_key(&key) {
                // Create glyph info in a separate scope to avoid borrow issues
                let glyph_info_opt = {
                    let face = match self.get_face() {
                        Ok(f) => f,
                        Err(_) => continue,
                    };
                    
                    // Set pixel size
                    face.set_pixel_sizes(0, size as u32).ok();
                    
                    if let Ok(_) = face.load_char(ch as usize, RENDER) {
                        let glyph = face.glyph();
                        let bitmap = glyph.bitmap();
                        
                        if bitmap.width() > 0 && bitmap.rows() > 0 {
                            let texture_id = self.create_texture(
                                bitmap.buffer(),
                                bitmap.width() as usize,
                                bitmap.rows() as usize,
                                bitmap.pitch()
                            );
                            
                            let _metrics = glyph.metrics();
                            Some(GlyphInfo {
                                texture_id,
                                width: bitmap.width(),
                                height: bitmap.rows(),
                                advance: (glyph.advance().x >> 6) as i32,
                                bearing_x: glyph.bitmap_left(),
                                bearing_y: glyph.bitmap_top(),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                
                if let Some(glyph_info) = glyph_info_opt {
                    self.glyph_cache.insert(key, glyph_info);
                }
            }
            
            // Draw glyph if it exists
            if let Some(glyph_info) = self.glyph_cache.get(&key) {
                let glyph_x = cursor_x + glyph_info.bearing_x as f32;
                let glyph_y = baseline_y - glyph_info.bearing_y as f32;
                
                self.draw_glyph(
                    gl,
                    glyph_info,
                    Point::new(glyph_x as i32, glyph_y as i32),
                    viewport_size
                );
                
                cursor_x += glyph_info.advance as f32;
            }
        }
        
        unsafe {
            gl::Disable(gl::TEXTURE_2D);
        }
    }
    
    fn create_texture(&self, buffer: &[u8], width: usize, height: usize, pitch: i32) -> u32 {
        let mut texture_id = 0;
        
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            
            // FreeType gives us a grayscale bitmap
            // We need to convert it to RGBA for proper rendering
            let mut rgba_buffer = Vec::with_capacity(width * height * 4);
            
            for y in 0..height {
                for x in 0..width {
                    let pixel = buffer[y * pitch.abs() as usize + x];
                    rgba_buffer.push(255);   // R
                    rgba_buffer.push(255);   // G
                    rgba_buffer.push(255);   // B
                    rgba_buffer.push(pixel); // A
                }
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
                rgba_buffer.as_ptr() as *const _,
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
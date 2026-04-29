use crate::gl;
use crate::GlContext;
use erigui_core::{EriGuiError, Point, Result, Size};
use freetype::face::LoadFlag;
use freetype::{Face, Library};
use std::collections::HashMap;
use std::rc::Rc;

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
    font_data: Rc<Vec<u8>>,
    face_index: isize,
    glyph_cache: HashMap<(char, u32), GlyphInfo>,
}

impl FontRenderer {
    pub fn new() -> Result<Self> {
        let library = Library::init()
            .map_err(|e| EriGuiError::FontLoading(format!("Failed to init FreeType: {:?}", e)))?;

        let font_data = Rc::new(DEFAULT_FONT_DATA.to_vec());

        Ok(Self {
            library,
            font_data,
            face_index: 0,
            glyph_cache: HashMap::new(),
        })
    }

    pub fn load_font(&mut self, font_data: &[u8]) -> Result<()> {
        self.font_data = Rc::new(font_data.to_vec());
        self.glyph_cache.clear();
        Ok(())
    }

    fn get_face(&self) -> Result<Face> {
        self.library
            .new_memory_face(self.font_data.clone(), self.face_index)
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
            if face.load_char(ch as usize, LoadFlag::DEFAULT).is_ok() {
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

                    if face.load_char(ch as usize, LoadFlag::RENDER).is_ok() {
                        let glyph = face.glyph();
                        let bitmap = glyph.bitmap();
                        let advance = (glyph.advance().x >> 6) as i32;

                        if bitmap.width() > 0 && bitmap.rows() > 0 {
                            let texture_id = self.create_texture(
                                bitmap.buffer(),
                                bitmap.width() as usize,
                                bitmap.rows() as usize,
                                bitmap.pitch(),
                            );

                            Some(GlyphInfo {
                                texture_id,
                                width: bitmap.width(),
                                height: bitmap.rows(),
                                advance,
                                bearing_x: glyph.bitmap_left(),
                                bearing_y: glyph.bitmap_top(),
                            })
                        } else {
                            // Zero-bitmap glyphs (' ', '\t', etc.) have no
                            // visual but DO have an advance — cache them
                            // with texture_id=0 so the cursor still moves.
                            // Without this, every space in rendered text
                            // collapses to zero width and words run
                            // together ("Basic Inputs" -> "BasicInputs").
                            Some(GlyphInfo {
                                texture_id: 0,
                                width: 0,
                                height: 0,
                                advance,
                                bearing_x: 0,
                                bearing_y: 0,
                            })
                        }
                    } else {
                        None
                    }
                };

                if let Some(glyph_info) = glyph_info_opt {
                    self.glyph_cache.insert(key, glyph_info);
                }
            }

            // Draw glyph if it exists. Zero-texture entries (e.g., ' ')
            // have no bitmap to draw but still advance the cursor.
            if let Some(glyph_info) = self.glyph_cache.get(&key) {
                if glyph_info.texture_id != 0 {
                    let glyph_x = cursor_x + glyph_info.bearing_x as f32;
                    let glyph_y = baseline_y - glyph_info.bearing_y as f32;

                    self.draw_glyph(
                        gl,
                        glyph_info,
                        Point::new(glyph_x as i32, glyph_y as i32),
                        viewport_size,
                    );
                }
                cursor_x += glyph_info.advance as f32;
            }
        }

        unsafe {
            gl::Disable(gl::TEXTURE_2D);
        }
    }

    fn create_texture(&self, buffer: &[u8], width: usize, height: usize, pitch: i32) -> u32 {
        let mut texture_id = 0;

        // SAFETY: OpenGL texture creation and upload. texture_id is initialized by
        // GenTextures. The rgba_buffer is constructed with exact required size
        // (width * height * 4 bytes) and its pointer is valid for the TexImage2D call.
        // The buffer slice bounds are checked by the loop indices.
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
                    let pixel = buffer[y * pitch.unsigned_abs() as usize + x];
                    rgba_buffer.push(255); // R
                    rgba_buffer.push(255); // G
                    rgba_buffer.push(255); // B
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
        // SAFETY: Cleaning up OpenGL textures. Each texture_id was created by
        // GenTextures in create_texture() and is valid. We own these resources
        // and are responsible for deleting them.
        unsafe {
            for glyph in self.glyph_cache.values() {
                gl::DeleteTextures(1, &glyph.texture_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! These tests exercise CPU-only paths through FreeType (`FontRenderer::new`,
    //! `measure_text`, `load_font`). They do NOT call into OpenGL, so they run
    //! safely on a headless host. The `Drop` impl will iterate an empty
    //! glyph_cache (no GL calls).
    use super::*;

    #[test]
    fn new_succeeds_with_default_font() {
        let r = FontRenderer::new();
        assert!(r.is_ok(), "FontRenderer::new should succeed with embedded font");
    }

    #[test]
    fn measure_empty_string_has_zero_width() {
        let r = FontRenderer::new().expect("init");
        let s = r.measure_text("", 16);
        assert_eq!(s.width, 0);
        // Height: when no glyphs are loaded, max_height stays 0 and is bumped
        // to `size`, so the line height matches the requested size.
        assert_eq!(s.height, 16);
    }

    #[test]
    fn measure_single_char_has_positive_width() {
        let r = FontRenderer::new().expect("init");
        let s = r.measure_text("M", 16);
        assert!(s.width > 0, "expected positive width, got {}", s.width);
        assert!(s.height > 0);
    }

    #[test]
    fn measure_longer_string_is_wider_than_single_char() {
        let r = FontRenderer::new().expect("init");
        let one = r.measure_text("M", 16);
        let many = r.measure_text("MMMMM", 16);
        assert!(
            many.width > one.width,
            "5 chars ({}) should be wider than 1 char ({})",
            many.width,
            one.width
        );
    }

    #[test]
    fn measure_height_at_least_size() {
        // measure_text returns max(measured_glyph_height, size).
        let r = FontRenderer::new().expect("init");
        let s = r.measure_text("Hello", 32);
        assert!(s.height >= 32, "height {} should be >= size 32", s.height);
    }

    #[test]
    fn measure_larger_size_is_wider_than_smaller() {
        let r = FontRenderer::new().expect("init");
        let small = r.measure_text("Hello", 12);
        let large = r.measure_text("Hello", 48);
        assert!(
            large.width > small.width,
            "48px ({}) should be wider than 12px ({})",
            large.width,
            small.width
        );
        assert!(large.height >= small.height);
    }

    #[test]
    fn load_font_with_embedded_data_succeeds() {
        let mut r = FontRenderer::new().expect("init");
        let result = r.load_font(DEFAULT_FONT_DATA);
        assert!(result.is_ok());
    }

    #[test]
    fn load_font_clears_glyph_cache_state() {
        // Indirect: after a load_font call we should still be able to measure text.
        let mut r = FontRenderer::new().expect("init");
        let before = r.measure_text("Hi", 14);
        r.load_font(DEFAULT_FONT_DATA).expect("reload");
        let after = r.measure_text("Hi", 14);
        // Reloaded the same font ⇒ measurement should be identical.
        assert_eq!(before, after);
    }

    #[test]
    fn measure_handles_unicode_without_panicking() {
        let r = FontRenderer::new().expect("init");
        // Many of these may not be in the JetBrains Mono coverage; the function
        // should still return a non-negative size and not panic.
        let s = r.measure_text("héllo·世界", 16);
        assert!(s.width >= 0);
        assert!(s.height > 0);
    }

    #[test]
    fn measure_with_size_one_does_not_panic() {
        let r = FontRenderer::new().expect("init");
        let s = r.measure_text("a", 1);
        // Even at the smallest pixel size, the function should return cleanly.
        assert!(s.height >= 1);
    }
}

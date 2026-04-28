//! Wave 3 (C2): inline image preview for nodes that emit `NodeValue::Image`.
//!
//! When the executor reports that a node finished and one of its outputs is
//! a `[B, 3, H, W]` image tensor, the host converts it to an RGB byte buffer
//! and calls [`NodeGraph::set_node_image`] with the bytes. This module:
//!
//!   - Accepts a CPU-side `u8` RGB buffer (the conversion lives in the host
//!     so this crate stays clear of `flame_core` / cuDNN).
//!   - Downsamples to a max of 256×256 so in-canvas rendering stays cheap.
//!   - Allocates a GL texture (best-effort — when no GL context is current
//!     we record `texture_id = 0` and rely on the existing
//!     `DrawContext::draw_image_rgba` path, which already uploads CPU bytes
//!     per-frame, identical to the widget's other image previews).
//!
//! ## Decoupling note
//!
//! `erigui-widgets` deliberately stays out of the `erigui-nodes` /
//! `erigui-runtime` cycle and out of the GPU stack. The host (erigui-app)
//! does the tensor → RGB conversion and hands us the bytes.
//!
//! ## What `draw_node_image_preview` does NOT do
//!
//! It does NOT bind the stored GL texture itself — `DrawContext` doesn't
//! expose a `draw_texture(id)` primitive, and adding one would force every
//! `DrawContext` impl (incl. test fakes) to grow GL knowledge. Instead we
//! reuse the same `draw_image_rgba(rect, w, h, &bytes)` path the widget
//! already uses for file previews. The `texture_id` is held for future
//! shader-side compositing (wave 4) and freed in `Drop`.

use erigui_core::{Color, DrawContext, Point, Rect, Size, Theme};

use super::NodeGraph;

/// Cap on the longest edge of any in-canvas preview. Keeps `draw_image_rgba`
/// uploads cheap (a 4K image would be ~70MB per frame otherwise).
const MAX_PREVIEW_EDGE: u32 = 256;

/// Per-node image preview state.
///
/// Holds both a GL texture id (best-effort, may be `0` if no GL context was
/// current at upload time) and a CPU-side RGB buffer. The CPU buffer is the
/// authoritative source for rendering through `DrawContext::draw_image_rgba`;
/// the GL id is reserved for a future shader-side composite path.
pub struct NodeImagePreview {
    /// GL texture handle, or `0` when no GL context was current at upload.
    /// `gl::types::GLuint` aliases `u32`.
    pub texture_id: gl::types::GLuint,
    /// Width of the downsampled preview in pixels (≤ `MAX_PREVIEW_EDGE`).
    pub width: u32,
    /// Height of the downsampled preview in pixels (≤ `MAX_PREVIEW_EDGE`).
    pub height: u32,
    /// CPU-side RGB pixel buffer: tightly packed `width * height * 3` bytes.
    /// Used by `draw_image_rgba` for actual rendering.
    pub rgb_bytes: Vec<u8>,
}

impl Drop for NodeImagePreview {
    fn drop(&mut self) {
        // Only free the GL texture if we actually allocated one AND the GL
        // function pointers are still loaded. `0` is the documented
        // "no texture" sentinel; the `is_loaded()` guard keeps the call out
        // of tests / shutdown paths where the context has been torn down.
        if self.texture_id != 0 && gl::DeleteTextures::is_loaded() {
            unsafe {
                gl::DeleteTextures(1, &self.texture_id);
            }
        }
    }
}

/// Errors that can arise while building a `NodeImagePreview` from RGB bytes.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("preview: width or height is zero ({width}x{height})")]
    BadSize { width: u32, height: u32 },
    #[error(
        "preview: expected width*height*3 = {expected} RGB bytes, got {actual}"
    )]
    BadByteLen { expected: usize, actual: usize },
}

/// Build a downsampled CPU RGB preview (+ best-effort GL texture) from a
/// caller-supplied tightly-packed RGB byte buffer.
///
/// The host (erigui-app) owns the tensor → RGB conversion (the standard
/// `(x + 1) * 127.5` clamp-and-scale used by every flame-diffusion VAE
/// decoder; see `core/save_image` / `klein_lora_infer.rs`). Keeping that
/// conversion out of the widget crate is what lets `erigui-widgets` build
/// and test without cuDNN.
pub fn upload_image_to_texture(
    rgb_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<NodeImagePreview, PreviewError> {
    if width == 0 || height == 0 {
        return Err(PreviewError::BadSize { width, height });
    }
    let expected = (width as usize) * (height as usize) * 3;
    if rgb_bytes.len() != expected {
        return Err(PreviewError::BadByteLen {
            expected,
            actual: rgb_bytes.len(),
        });
    }

    // Downsample to ≤ MAX_PREVIEW_EDGE × MAX_PREVIEW_EDGE. We use a simple
    // box average — adequate for an in-canvas thumbnail and ~10× faster than
    // `image::resize` with Lanczos for sizes this small.
    let (out_w, out_h, out_bytes) = downsample_rgb(rgb_bytes, width, height, MAX_PREVIEW_EDGE);

    // Best-effort GL upload. If no GL context is current (test runs, headless
    // tools), `gl::GenTextures` returns 0 and we keep the CPU bytes only.
    let texture_id = upload_gl_texture(&out_bytes, out_w, out_h);

    Ok(NodeImagePreview {
        texture_id,
        width: out_w,
        height: out_h,
        rgb_bytes: out_bytes,
    })
}

/// Box-average downsample. Returns `(out_w, out_h, out_bytes_rgb)`.
fn downsample_rgb(src: &[u8], src_w: u32, src_h: u32, max_edge: u32) -> (u32, u32, Vec<u8>) {
    if src_w <= max_edge && src_h <= max_edge {
        return (src_w, src_h, src.to_vec());
    }
    let scale = f32::min(
        max_edge as f32 / src_w as f32,
        max_edge as f32 / src_h as f32,
    );
    let dst_w = ((src_w as f32 * scale).round() as u32).max(1);
    let dst_h = ((src_h as f32 * scale).round() as u32).max(1);
    let mut out = vec![0u8; (dst_w * dst_h * 3) as usize];
    let inv_scale_x = src_w as f32 / dst_w as f32;
    let inv_scale_y = src_h as f32 / dst_h as f32;
    for dy in 0..dst_h {
        let y0 = (dy as f32 * inv_scale_y) as u32;
        let y1 = (((dy + 1) as f32 * inv_scale_y).ceil() as u32).min(src_h);
        for dx in 0..dst_w {
            let x0 = (dx as f32 * inv_scale_x) as u32;
            let x1 = (((dx + 1) as f32 * inv_scale_x).ceil() as u32).min(src_w);
            let mut acc = [0u32; 3];
            let mut n = 0u32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * src_w + x) * 3) as usize;
                    acc[0] += src[i] as u32;
                    acc[1] += src[i + 1] as u32;
                    acc[2] += src[i + 2] as u32;
                    n += 1;
                }
            }
            let n = n.max(1);
            let oi = ((dy * dst_w + dx) * 3) as usize;
            out[oi] = (acc[0] / n) as u8;
            out[oi + 1] = (acc[1] / n) as u8;
            out[oi + 2] = (acc[2] / n) as u8;
        }
    }
    (dst_w, dst_h, out)
}

/// Best-effort GL texture upload. Returns `0` if no GL context is current —
/// callers must be ready to fall back to `draw_image_rgba` for actual
/// rendering. We DO NOT panic in this path because the widget tests run
/// without a GL context and the host application uploads each frame anyway.
fn upload_gl_texture(rgb: &[u8], width: u32, height: u32) -> gl::types::GLuint {
    // Cheap presence check: if `gl::GenTextures` is unloaded
    // (no GL context bound), calling it segfaults. We guard with the
    // function-pointer load check that the `gl` crate exposes.
    if !gl::GenTextures::is_loaded() {
        return 0;
    }
    unsafe {
        let mut id: gl::types::GLuint = 0;
        gl::GenTextures(1, &mut id);
        if id == 0 {
            return 0;
        }
        gl::BindTexture(gl::TEXTURE_2D, id);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            width as i32,
            height as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            rgb.as_ptr() as *const _,
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);
        id
    }
}

// ---------------------------------------------------------------------------
// NodeGraph integration
// ---------------------------------------------------------------------------

impl NodeGraph {
    /// Upload an RGB image (tightly-packed `width*height*3` bytes) and store
    /// the resulting preview keyed by `node_id`. Replaces (and frees) any
    /// prior preview for the same node.
    ///
    /// The host is expected to perform the tensor → RGB conversion before
    /// calling this — see the host's `NodePreview` / `NodeDone` handlers.
    pub fn set_node_image(
        &mut self,
        node_id: usize,
        rgb_bytes: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<(), PreviewError> {
        let preview = upload_image_to_texture(&rgb_bytes, width, height)?;
        // Inserting drops the old value, which runs `Drop for NodeImagePreview`
        // and frees the prior GL texture (if any).
        self.node_image_textures.insert(node_id, preview);
        Ok(())
    }

    /// Drop the preview entry for `node_id`. Frees the GL texture as a side
    /// effect of the `NodeImagePreview` drop.
    pub fn clear_node_image(&mut self, node_id: usize) {
        self.node_image_textures.remove(&node_id);
    }

    /// Read-only accessor used by tests and by hosts that want to inspect
    /// preview metadata (size, texture id).
    pub fn node_image_for(&self, node_id: usize) -> Option<&NodeImagePreview> {
        self.node_image_textures.get(&node_id)
    }
}

/// Render the inline image preview for one node.
///
/// `node_world_rect` is the node's full bounding rect in **world** space;
/// `last_field_bottom_world` is the y-coordinate (world space) just below
/// the last field row, returned by the field-stack draw loop. The preview
/// is laid out between that y and the bottom of the node card.
///
/// The renderer DOES NOT auto-grow the node — the host is expected to size
/// the node so there's room for the preview (or the caller can clamp; we
/// cope with zero or negative remaining height by returning early).
pub(crate) fn draw_node_image_preview(
    ctx: &mut dyn DrawContext,
    theme: &Theme,
    preview: &NodeImagePreview,
    node_world_rect: Rect,
    last_field_bottom_world: i32,
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
) {
    // World-space placement: 8px gap above the preview, 8px gap to the
    // bottom of the node card, 8px horizontal padding either side. Same
    // pattern as the file-preview block in `draw_nodes`.
    let pad_world: i32 = 8;
    let avail_w_world = (node_world_rect.size.width - pad_world * 2).max(1);
    let preview_top_world = last_field_bottom_world + pad_world;
    let avail_h_world = (node_world_rect.bottom() - preview_top_world - pad_world).max(0);
    if avail_h_world <= 0 || avail_w_world <= 0 {
        return;
    }

    let preview_world = Rect::new(
        node_world_rect.x() + pad_world,
        preview_top_world,
        avail_w_world,
        avail_h_world,
    );

    // World → screen. Rounded by hand to match the rest of the file.
    let to_screen = |p: Point| {
        Point::new(
            ((p.x as f32) * zoom + pan_x).round() as i32,
            ((p.y as f32) * zoom + pan_y).round() as i32,
        )
    };
    let scale_size = |s: Size| {
        Size::new(
            (s.width as f32 * zoom).round() as i32,
            (s.height as f32 * zoom).round() as i32,
        )
    };
    let preview_screen = Rect::from_origin_size(
        to_screen(preview_world.origin),
        scale_size(preview_world.size),
    );

    // Background card + border (matches the file-preview block).
    let radius = ((8.0 * zoom).round() as i32).clamp(4, 12);
    ctx.set_color(theme.colors.surface);
    ctx.fill_rounded_rect(preview_screen, radius);
    ctx.set_color(theme.colors.border);
    ctx.draw_rounded_rect(preview_screen, radius);

    // Aspect-fit the preview bytes inside the card with a small inner pad.
    let inner_pad = ((6.0 * zoom).round() as i32).clamp(2, 12);
    let avail_w_screen = (preview_screen.width() - inner_pad * 2).max(1);
    let avail_h_screen = (preview_screen.height() - inner_pad * 2).max(1);
    let img_w = preview.width.max(1) as f32;
    let img_h = preview.height.max(1) as f32;
    let fit_scale = f32::min(
        avail_w_screen as f32 / img_w,
        avail_h_screen as f32 / img_h,
    );
    let draw_w = (img_w * fit_scale).round().max(1.0) as i32;
    let draw_h = (img_h * fit_scale).round().max(1.0) as i32;
    let dx = preview_screen.x() + inner_pad + (avail_w_screen - draw_w) / 2;
    let dy = preview_screen.y() + inner_pad + (avail_h_screen - draw_h) / 2;
    let dest = Rect::new(dx, dy, draw_w, draw_h);

    // The widget's existing image rendering goes through `draw_image_rgba`
    // which expects 4 bytes per pixel. Expand RGB → RGBA on the fly.
    let mut rgba = Vec::with_capacity((preview.width * preview.height * 4) as usize);
    for chunk in preview.rgb_bytes.chunks_exact(3) {
        rgba.extend_from_slice(chunk);
        rgba.push(255);
    }

    ctx.set_color(Color::WHITE);
    ctx.draw_image_rgba(
        dest,
        preview.width as i32,
        preview.height as i32,
        &rgba,
    );
}

// ---------------------------------------------------------------------------
// Tests — bookkeeping only (HashMap insert / replace / drop). The GL upload
// path requires a current GL context and is gated `#[ignore]` below.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::{Graph, NodeGraph};
    use erigui_core::WidgetId;

    /// Build a `NodeImagePreview` directly without invoking GL or flame_core.
    /// Lets the bookkeeping tests stand on their own without a CUDA device.
    fn fake_preview(width: u32, height: u32, fill: u8) -> NodeImagePreview {
        let rgb_bytes = vec![fill; (width * height * 3) as usize];
        NodeImagePreview {
            texture_id: 0, // No GL upload in tests.
            width,
            height,
            rgb_bytes,
        }
    }

    #[test]
    fn image_output_creates_texture() {
        let mut g = NodeGraph::new(WidgetId::default(), Graph::default());
        let p = fake_preview(8, 8, 64);
        g.node_image_textures.insert(0, p);
        assert!(g.node_image_for(0).is_some());
        assert_eq!(g.node_image_for(0).unwrap().width, 8);
        assert_eq!(g.node_image_for(0).unwrap().height, 8);
    }

    #[test]
    fn repeated_run_replaces_texture() {
        let mut g = NodeGraph::new(WidgetId::default(), Graph::default());
        let first = fake_preview(8, 8, 10);
        let second = fake_preview(16, 16, 200);
        g.node_image_textures.insert(0, first);
        g.node_image_textures.insert(0, second);
        // Map size unchanged at exactly one entry.
        assert_eq!(g.node_image_textures.len(), 1);
        // The replacement is the second one (drop ran on the first).
        let cur = g.node_image_for(0).expect("preview present");
        assert_eq!(cur.width, 16);
        assert_eq!(cur.height, 16);
        assert_eq!(cur.rgb_bytes[0], 200);
    }

    #[test]
    fn clear_node_image_removes_entry() {
        let mut g = NodeGraph::new(WidgetId::default(), Graph::default());
        g.node_image_textures.insert(7, fake_preview(4, 4, 0));
        assert!(g.node_image_for(7).is_some());
        g.clear_node_image(7);
        assert!(g.node_image_for(7).is_none());
        assert_eq!(g.node_image_textures.len(), 0);
    }

    #[test]
    fn downsample_passes_through_when_under_cap() {
        let src = vec![123u8; 32 * 32 * 3];
        let (w, h, out) = downsample_rgb(&src, 32, 32, 256);
        assert_eq!((w, h), (32, 32));
        assert_eq!(out.len(), 32 * 32 * 3);
        assert_eq!(out[0], 123);
    }

    #[test]
    fn downsample_caps_long_edge() {
        // 1024×512 → must fit inside 256 on the long edge.
        let src = vec![200u8; (1024 * 512 * 3) as usize];
        let (w, h, out) = downsample_rgb(&src, 1024, 512, 256);
        assert!(w <= 256 && h <= 256);
        assert_eq!(w, 256);
        assert_eq!(h, 128); // aspect ratio preserved
        assert_eq!(out.len() as u32, w * h * 3);
        // Box-average of an all-200 input is exactly 200.
        assert_eq!(out[0], 200);
    }

    #[test]
    #[ignore = "requires a current GL context (Wayland/X11 window) — runs only in integration tests"]
    fn upload_gl_texture_round_trip() {
        // Real GL upload is verified manually by running the example app.
        // This test exists as documentation of the contract; flip the
        // ignore off if you have a context-bound test harness.
        let bytes = vec![64u8; 4 * 4 * 3];
        let id = upload_gl_texture(&bytes, 4, 4);
        assert_ne!(id, 0);
        unsafe { gl::DeleteTextures(1, &id) };
    }
}

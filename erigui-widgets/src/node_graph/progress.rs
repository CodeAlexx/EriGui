//! Wave 3 (C3): live progress bars for nodes that are mid-execute.
//!
//! When a node calls `progress.step(current, total, label)` from inside
//! `NodeType::execute()`, the application's main loop is expected to drain
//! `Executor::poll_progress()` each frame and translate `ProgressEvent::NodeStep`
//! events into `NodeGraph::set_node_progress(...)` calls.
//!
//! This submodule is purely additive:
//!   - exposes a small `NodeProgress` value type,
//!   - defines an `impl NodeGraph` block with the setter / clearer / error API,
//!   - provides `draw_node_progress_bar(...)` which the main `draw_nodes()` loop
//!     invokes once per node when `node_progress` has an entry.
//!
//! This module does NOT depend on `erigui-runtime` or `erigui-nodes` (avoiding a
//! circular dep). The application wires the runtime side; this module only
//! exposes the setter API and the renderer.

use erigui_core::{Color, DrawContext, Point, Rect, Size, Theme};

use super::NodeGraph;

/// Live execution-progress state for a single node, mirrored from
/// `ProgressEvent::NodeStep`. `error` is `Some(_)` if the node finished
/// with `ProgressEvent::NodeError` and the application chose to keep the
/// indicator on screen until the user dismisses it.
#[derive(Clone, Debug, Default)]
pub struct NodeProgress {
    pub current: u32,
    pub total: u32,
    pub label: String,
    pub error: Option<String>,
}

impl NodeProgress {
    /// 0.0 .. 1.0 fill ratio. Always saturated to that range and treats
    /// `total == 0` as "unknown duration" (returns 0.0 so the bar shows
    /// only the label).
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }

    /// Has the node entered an error state?
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

impl NodeGraph {
    /// Update or create the progress entry for `node_id`. Called by the UI
    /// thread when it observes a `ProgressEvent::NodeStep` from the executor.
    ///
    /// If the node was previously in an error state, calling this clears the
    /// error (a fresh run is now in flight).
    pub fn set_node_progress(
        &mut self,
        node_id: usize,
        current: u32,
        total: u32,
        label: impl Into<String>,
    ) {
        let entry = self.node_progress.entry(node_id).or_default();
        entry.current = current;
        entry.total = total;
        entry.label = label.into();
        entry.error = None;
    }

    /// Drop the progress entry. Called on `ProgressEvent::NodeDone` (or any
    /// time the application wants to retire the indicator).
    pub fn clear_node_progress(&mut self, node_id: usize) {
        self.node_progress.remove(&node_id);
    }

    /// Mark the node as having errored out. Keeps the entry alive (so the
    /// red bar stays on-screen) but flips the error flag.
    pub fn set_node_error(&mut self, node_id: usize, msg: impl Into<String>) {
        let entry = self.node_progress.entry(node_id).or_default();
        entry.error = Some(msg.into());
    }

    /// Read-only accessor used by tests and by the host application that
    /// wants to inspect current progress (e.g. for a side-panel mirror).
    pub fn node_progress_for(&self, node_id: usize) -> Option<&NodeProgress> {
        self.node_progress.get(&node_id)
    }
}

/// Render the progress bar for a single node.
///
/// `node_top_screen` is the screen-space position of the node's top-left
/// corner; `header_h_screen` is the header height already laid out by the
/// caller; `node_width_screen` is the full width of the node card.
///
/// The bar is drawn directly under the header and above the field stack.
/// Bar height is `~16px` in widget space, scaled to current zoom (clamped
/// so it stays readable at extreme zoom levels).
pub(crate) fn draw_node_progress_bar(
    ctx: &mut dyn DrawContext,
    theme: &Theme,
    progress: &NodeProgress,
    node_top_screen: Point,
    header_h_screen: i32,
    node_width_screen: i32,
    zoom: f32,
) {
    // Bar height: ~16 widget-px, scales with zoom but clamped.
    let bar_h = ((16.0 * zoom).round() as i32).clamp(8, 32);
    let pad_x = ((6.0 * zoom).round() as i32).clamp(2, 12);
    let pad_y = ((2.0 * zoom).round() as i32).clamp(1, 6);
    let radius = ((4.0 * zoom).round() as i32).clamp(2, 8);

    let bar_rect = Rect::from_origin_size(
        Point::new(
            node_top_screen.x + pad_x,
            node_top_screen.y + header_h_screen + pad_y,
        ),
        Size::new((node_width_screen - pad_x * 2).max(1), bar_h),
    );

    // Background track: dark gray with subtle border.
    let track_bg = Color::rgba(28, 32, 40, 220);
    ctx.set_color(track_bg);
    ctx.fill_rounded_rect(bar_rect, radius);

    // Fill: green (running) or red (error).
    let frac = progress.fraction();
    let fill_w = ((bar_rect.size.width as f32) * frac).round() as i32;
    if fill_w > 0 {
        let fill_rect = Rect::from_origin_size(
            bar_rect.origin,
            Size::new(fill_w.max(1), bar_rect.size.height),
        );
        let fill_color = if progress.is_error() {
            Color::rgba(220, 60, 60, 230)
        } else {
            Color::rgba(60, 180, 90, 230)
        };
        ctx.set_color(fill_color);
        ctx.fill_rounded_rect(fill_rect, radius);
    } else if progress.is_error() {
        // Error with zero progress: still paint a thin red strip so the
        // user sees *something* indicating the failure.
        let fill_rect = Rect::from_origin_size(
            bar_rect.origin,
            Size::new((bar_rect.size.width / 8).max(2), bar_rect.size.height),
        );
        ctx.set_color(Color::rgba(220, 60, 60, 230));
        ctx.fill_rounded_rect(fill_rect, radius);
    }

    // Border on top.
    ctx.set_color(theme.colors.border);
    ctx.draw_rounded_rect(bar_rect, radius);

    // Overlay label text. If the node is errored and the error message
    // exists, prefer that over the "step" label so the user sees the cause.
    let label_text: String = if let Some(err) = progress.error.as_ref() {
        // Truncate long error messages for the bar overlay.
        if err.len() > 64 {
            format!("error: {}…", &err[..60])
        } else {
            format!("error: {}", err)
        }
    } else if progress.total > 0 {
        format!("{} {}/{}", progress.label, progress.current, progress.total)
    } else {
        progress.label.clone()
    };

    let font_size = ((theme.typography.font_size_small as f32 * zoom).round() as i32).clamp(9, 16);
    if !label_text.is_empty() && bar_rect.size.height >= font_size {
        ctx.set_color(theme.colors.text);
        let text_x = bar_rect.x() + ((4.0 * zoom).round() as i32).clamp(2, 8);
        let text_y = bar_rect.y() + (bar_rect.size.height + font_size) / 2 - 2;
        ctx.draw_text(&label_text, Point::new(text_x, text_y), font_size);
    }
}

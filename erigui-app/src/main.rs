//! erigui-app: top-level binary that wires the wave-1+2+3 components into
//! a runnable program.
//!
//! Architecture:
//!   - The right side of the window is a single `NodeGraph` widget. The
//!     widget owns its own pan/zoom, marquee selection, edge wiring, and
//!     right-click "add node" menu (C4) — we just hand it events.
//!   - A thin top toolbar exposes Queue / Cancel / Save / Load buttons.
//!   - Each frame the main loop drains `Executor::poll_progress()` and
//!     translates `ProgressEvent`s into widget calls (`set_node_progress`,
//!     `set_node_image`, `clear_node_progress`, `set_node_error`). The
//!     widget itself does not pull from the executor.
//!   - File save/load uses the existing `FileDialog` widget, opened as a
//!     modal overlay. We don't block the event loop — the dialog renders
//!     each frame like any other widget; selecting a path fires our
//!     callback and we close the dialog.
//!
//! See `graph_translate.rs` for the `Graph` <-> `Workflow` translation
//! and `registry_handle.rs` for the `NodeRegistry` -> `NodeRegistryHandle`
//! adapter that lets the widget's add-menu enumerate node types.

mod graph_translate;
mod registry_handle;

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use erigui_core::{Color, DrawContext, Event, Point, Rect, Size, Theme, Widget};
use erigui_nodes::NodeRegistry;
use erigui_rendering::{EventTranslator, Renderer};
use erigui_runtime::{Executor, NodeId, ProgressEvent};
use erigui_widgets::file_dialog::{FileDialog, FileDialogMode, FileFilter};
use erigui_widgets::node_graph::{Graph, NodeGraph};
use erigui_widgets::{Button, Label, WidgetId};
use erigui_workflow::Workflow;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::graph_translate::{graph_to_workflow, workflow_to_graph};
use crate::registry_handle::RegistryHandle;

const TOOLBAR_HEIGHT: i32 = 56;
const STATUS_HEIGHT: i32 = 28;
const FILE_DIALOG_W: i32 = 720;
const FILE_DIALOG_H: i32 = 520;

/// Toolbar action requested from a button callback. Buttons can't borrow
/// `App` mutably (FnMut + 'static), so they push intent into a shared
/// `Vec` that the event loop drains once per frame.
#[derive(Clone, Debug)]
enum Action {
    Queue,
    Cancel,
    SaveDialog,
    LoadDialog,
}

/// Which file-dialog flow is active. Determines whether a chosen path is
/// passed to `Workflow::save_to_file` or `Workflow::load_from_file`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialogPurpose {
    Save,
    Load,
}

struct App {
    theme: Theme,
    /// The single NodeGraph widget. We keep it inline (rather than via
    /// WidgetManager) so the C2/C3/C4 typed methods (`set_node_image`,
    /// `set_node_progress`, `set_node_registry`) are direct field accesses
    /// instead of `get_typed_mut::<NodeGraph>` round-trips.
    graph: NodeGraph,
    registry: Arc<NodeRegistry>,
    executor: Executor,

    // Toolbar widgets.
    queue_btn: Button,
    cancel_btn: Button,
    save_btn: Button,
    load_btn: Button,
    status_label: Label,

    // File dialog state. `Some(_)` while the modal is open.
    file_dialog: Option<FileDialog>,
    dialog_purpose: Option<DialogPurpose>,
    pending_path: Rc<RefCell<Option<PathBuf>>>,
    dialog_cancelled: Rc<RefCell<bool>>,

    // Layout caches.
    toolbar_rect: Rect,
    canvas_rect: Rect,
    status_rect: Rect,

    /// Action queue that toolbar callbacks push into and the event loop
    /// drains once per frame.
    actions: Rc<RefCell<Vec<Action>>>,
}

impl App {
    fn new(theme: Theme, viewport: Size) -> Self {
        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = Executor::new(Arc::clone(&registry));

        let mut graph = NodeGraph::new(WidgetId::default(), Graph::default());
        let handle: Arc<dyn erigui_widgets::node_graph::add_menu::NodeRegistryHandle> =
            Arc::new(RegistryHandle::new(Arc::clone(&registry)));
        graph.set_node_registry(handle);

        let actions: Rc<RefCell<Vec<Action>>> = Rc::new(RefCell::new(Vec::new()));

        let queue_btn = {
            let a = Rc::clone(&actions);
            Button::new(WidgetId::default(), "Queue").with_on_click(move || {
                a.borrow_mut().push(Action::Queue);
            })
        };
        let cancel_btn = {
            let a = Rc::clone(&actions);
            Button::new(WidgetId::default(), "Cancel").with_on_click(move || {
                a.borrow_mut().push(Action::Cancel);
            })
        };
        let save_btn = {
            let a = Rc::clone(&actions);
            Button::new(WidgetId::default(), "Save").with_on_click(move || {
                a.borrow_mut().push(Action::SaveDialog);
            })
        };
        let load_btn = {
            let a = Rc::clone(&actions);
            Button::new(WidgetId::default(), "Load").with_on_click(move || {
                a.borrow_mut().push(Action::LoadDialog);
            })
        };

        let status_label = Label::new(WidgetId::default(), "Idle. Right-click canvas to add nodes.");

        let mut app = Self {
            theme,
            graph,
            registry,
            executor,
            queue_btn,
            cancel_btn,
            save_btn,
            load_btn,
            status_label,
            file_dialog: None,
            dialog_purpose: None,
            pending_path: Rc::new(RefCell::new(None)),
            dialog_cancelled: Rc::new(RefCell::new(false)),
            toolbar_rect: Rect::default(),
            canvas_rect: Rect::default(),
            status_rect: Rect::default(),
            actions,
        };
        app.layout(viewport);
        app
    }

    fn layout(&mut self, viewport: Size) {
        self.toolbar_rect = Rect::new(0, 0, viewport.width, TOOLBAR_HEIGHT);
        self.status_rect = Rect::new(
            0,
            (viewport.height - STATUS_HEIGHT).max(0),
            viewport.width,
            STATUS_HEIGHT,
        );
        let canvas_h = (viewport.height - TOOLBAR_HEIGHT - STATUS_HEIGHT).max(0);
        self.canvas_rect = Rect::new(0, TOOLBAR_HEIGHT, viewport.width, canvas_h);

        // Canvas (NodeGraph widget).
        self.graph.layout(self.canvas_rect, &self.theme);

        // Toolbar buttons: laid out left-to-right.
        let pad = 8;
        let btn_w = 96;
        let btn_h = TOOLBAR_HEIGHT - pad * 2;
        let y = self.toolbar_rect.y() + pad;
        let mut x = self.toolbar_rect.x() + pad;

        for btn in [
            &mut self.queue_btn,
            &mut self.cancel_btn,
            &mut self.save_btn,
            &mut self.load_btn,
        ] {
            btn.layout(Rect::new(x, y, btn_w, btn_h), &self.theme);
            x += btn_w + pad;
        }

        // Status label spans the bottom strip.
        self.status_label.layout(
            Rect::new(
                self.status_rect.x() + pad,
                self.status_rect.y() + 4,
                self.status_rect.width() - pad * 2,
                STATUS_HEIGHT - 8,
            ),
            &self.theme,
        );

        // File dialog: centered over the canvas.
        if let Some(dialog) = &mut self.file_dialog {
            let dx = self.canvas_rect.x() + (self.canvas_rect.width() - FILE_DIALOG_W) / 2;
            let dy = self.canvas_rect.y() + (self.canvas_rect.height() - FILE_DIALOG_H) / 2;
            dialog.layout(
                Rect::new(dx.max(0), dy.max(0), FILE_DIALOG_W, FILE_DIALOG_H),
                &self.theme,
            );
        }
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status_label.set_text(text.into());
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        // Modal file dialog takes priority when open.
        if let Some(dialog) = &mut self.file_dialog {
            let r = dialog.handle_event(event, &self.theme);
            // The dialog's selection / cancel callbacks fire while we're
            // borrowing it; check the shared cells AFTER we release.
            let consumed = r.is_consumed();
            // Drain the cells.
            let chosen = self.pending_path.borrow_mut().take();
            let cancelled = std::mem::replace(&mut *self.dialog_cancelled.borrow_mut(), false);
            if let Some(path) = chosen {
                self.complete_dialog(path);
            } else if cancelled {
                self.file_dialog = None;
                self.dialog_purpose = None;
                self.set_status("Dialog cancelled.");
            }
            return consumed;
        }

        // Toolbar — only react to events whose position lands in the strip.
        // (`Button` doesn't hit-test event positions itself; we route by
        // bounds so canvas events still flow to the NodeGraph.)
        let pos = event_position(event);
        if let Some(p) = pos {
            if self.toolbar_rect.contains(p) {
                let mut consumed = false;
                for btn in [
                    &mut self.queue_btn,
                    &mut self.cancel_btn,
                    &mut self.save_btn,
                    &mut self.load_btn,
                ] {
                    if btn.handle_event(event, &self.theme).is_consumed() {
                        consumed = true;
                    }
                }
                return consumed;
            }
        }

        // Otherwise hand to the canvas.
        self.graph.handle_event(event, &self.theme).is_consumed()
    }

    /// Drain queued toolbar actions. Called once per frame from the event
    /// loop so we never react to stale clicks across redraws.
    fn drain_actions(&mut self) {
        let actions: Vec<Action> = self.actions.borrow_mut().drain(..).collect();
        for action in actions {
            match action {
                Action::Queue => self.queue_run(),
                Action::Cancel => self.cancel_run(),
                Action::SaveDialog => self.open_save_dialog(),
                Action::LoadDialog => self.open_load_dialog(),
            }
        }
    }

    fn queue_run(&mut self) {
        // Real per-node dirty tracking: the widget records every field edit
        // and edge add/remove in `dirty_set()`. We enqueue only those nodes
        // (the executor's downstream-propagation does the rest) and clear
        // the set so subsequent edits start a fresh frontier. If nothing is
        // dirty, the run is a no-op — the cache already has every output.
        let dirty: HashSet<NodeId> = self
            .graph
            .dirty_set()
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .collect();
        let dirty_count = dirty.len();
        match self.executor.enqueue(self.graph.graph.clone(), dirty) {
            Ok(()) => {
                self.graph.clear_dirty();
                self.set_status(format!(
                    "Queued run ({} dirty of {} nodes).",
                    dirty_count,
                    self.graph.graph.nodes.len()
                ));
            }
            Err(e) => self.set_status(format!("Enqueue failed: {e}")),
        }
    }

    fn cancel_run(&mut self) {
        self.executor.cancel();
        self.set_status("Cancel requested (effective at next inter-node boundary).");
    }

    fn open_save_dialog(&mut self) {
        let pending = Rc::clone(&self.pending_path);
        let cancelled = Rc::clone(&self.dialog_cancelled);
        let dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Save)
            .with_filters(vec![
                FileFilter::new("Workflow JSON", vec!["json"]),
                FileFilter::new("All Files", vec!["*"]),
            ])
            .with_on_file_selected(move |p| {
                *pending.borrow_mut() = Some(p.to_path_buf());
            })
            .with_on_cancel(move || {
                *cancelled.borrow_mut() = true;
            });
        self.file_dialog = Some(dialog);
        self.dialog_purpose = Some(DialogPurpose::Save);
        let viewport = self.canvas_rect.size;
        self.layout_dialog_only(viewport);
        self.set_status("Save: choose a workflow JSON path.");
    }

    fn open_load_dialog(&mut self) {
        let pending = Rc::clone(&self.pending_path);
        let cancelled = Rc::clone(&self.dialog_cancelled);
        let dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Open)
            .with_filters(vec![
                FileFilter::new("Workflow JSON", vec!["json"]),
                FileFilter::new("All Files", vec!["*"]),
            ])
            .with_on_file_selected(move |p| {
                *pending.borrow_mut() = Some(p.to_path_buf());
            })
            .with_on_cancel(move || {
                *cancelled.borrow_mut() = true;
            });
        self.file_dialog = Some(dialog);
        self.dialog_purpose = Some(DialogPurpose::Load);
        let viewport = self.canvas_rect.size;
        self.layout_dialog_only(viewport);
        self.set_status("Load: choose a workflow JSON file.");
    }

    fn layout_dialog_only(&mut self, _hint: Size) {
        if let Some(dialog) = &mut self.file_dialog {
            let dx = self.canvas_rect.x() + (self.canvas_rect.width() - FILE_DIALOG_W) / 2;
            let dy = self.canvas_rect.y() + (self.canvas_rect.height() - FILE_DIALOG_H) / 2;
            dialog.layout(
                Rect::new(dx.max(0), dy.max(0), FILE_DIALOG_W, FILE_DIALOG_H),
                &self.theme,
            );
        }
    }

    fn complete_dialog(&mut self, path: PathBuf) {
        let purpose = self.dialog_purpose.take();
        self.file_dialog = None;
        match purpose {
            Some(DialogPurpose::Save) => {
                let wf = graph_to_workflow(&self.graph.graph);
                match wf.save_to_file(&path) {
                    Ok(()) => self.set_status(format!("Saved workflow to {}", path.display())),
                    Err(e) => self.set_status(format!("Save failed: {e}")),
                }
            }
            Some(DialogPurpose::Load) => match Workflow::load_from_file(&path) {
                Ok(wf) => {
                    let report = wf.resolve(&self.registry);
                    if !report.unresolved.is_empty() {
                        log::warn!(
                            "workflow has {} unresolved nodes (will be dropped): {:?}",
                            report.unresolved.len(),
                            report.unresolved
                        );
                    }
                    let new_graph = workflow_to_graph(&wf, &self.registry);
                    let n = new_graph.nodes.len();
                    self.graph.graph = new_graph;
                    self.graph.layout(self.canvas_rect, &self.theme);
                    if report.unresolved.is_empty() {
                        self.set_status(format!("Loaded {n} nodes from {}", path.display()));
                    } else {
                        self.set_status(format!(
                            "Loaded {n} nodes ({} unresolved dropped) from {}",
                            report.unresolved.len(),
                            path.display()
                        ));
                    }
                }
                Err(e) => self.set_status(format!("Load failed: {e}")),
            },
            None => {}
        }
    }

    /// Drain executor progress events into widget state.
    fn drain_progress(&mut self) {
        for ev in self.executor.poll_progress() {
            match ev {
                ProgressEvent::NodeStart { node_id } => {
                    self.graph.set_node_progress(node_id as usize, 0, 1, "starting");
                }
                ProgressEvent::NodeStep {
                    node_id,
                    current,
                    total,
                    label,
                } => {
                    self.graph
                        .set_node_progress(node_id as usize, current, total, label);
                }
                ProgressEvent::NodePreview { node_id, image } => {
                    match tensor_to_rgb_bytes(&image) {
                        Ok((bytes, w, h)) => {
                            if let Err(e) =
                                self.graph.set_node_image(node_id as usize, bytes, w, h)
                            {
                                log::warn!("preview upload failed for node {node_id}: {e}");
                            }
                        }
                        Err(e) => log::warn!("preview convert failed for node {node_id}: {e}"),
                    }
                }
                ProgressEvent::NodeDone { node_id, outputs } => {
                    self.graph.clear_node_progress(node_id as usize);
                    for (port_name, value) in outputs {
                        if let erigui_nodes::NodeValue::Image(t) = value {
                            // Convert once.
                            let bytes_wh = match tensor_to_rgb_bytes(&t) {
                                Ok(b) => Some(b),
                                Err(e) => {
                                    log::warn!(
                                        "output image convert failed for node {node_id}: {e}"
                                    );
                                    None
                                }
                            };
                            if let Some((bytes, w, h)) = bytes_wh {
                                // Set on the producer.
                                if let Err(e) =
                                    self.graph.set_node_image(node_id as usize, bytes.clone(), w, h)
                                {
                                    log::warn!(
                                        "output image upload failed for node {node_id}: {e}"
                                    );
                                }
                                // Also set on every downstream node that
                                // consumes this Image output. SaveImage is
                                // the typical case — user expects the
                                // generated picture to show on the "save"
                                // node, not the upstream VAE decode.
                                // Find downstream consumers of this output port
                                // by matching on the producer node + port index
                                // (port label == NodeValueType name in our schemas).
                                let producer_outputs: Vec<String> = self
                                    .graph
                                    .graph
                                    .nodes
                                    .iter()
                                    .find(|n| n.id == node_id as usize)
                                    .map(|n| n.outputs.iter().map(|p| p.label.clone()).collect())
                                    .unwrap_or_default();
                                let downstream: Vec<usize> = self
                                    .graph
                                    .graph
                                    .edges
                                    .iter()
                                    .filter(|e| {
                                        e.from_node == node_id as usize
                                            && producer_outputs
                                                .get(e.from_port)
                                                .map(|s| s.as_str() == port_name.as_str())
                                                .unwrap_or(false)
                                    })
                                    .map(|e| e.to_node)
                                    .collect();
                                for to_id in downstream {
                                    if let Err(e) =
                                        self.graph.set_node_image(to_id, bytes.clone(), w, h)
                                    {
                                        log::warn!(
                                            "downstream image upload failed for node {to_id}: {e}"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                ProgressEvent::NodeError { node_id, error } => {
                    let msg = format!("{:?}", error);
                    self.graph.set_node_error(node_id as usize, msg.clone());
                    self.set_status(format!("Node {node_id} failed: {msg}"));
                }
                ProgressEvent::QueueIdle => {
                    self.set_status("Queue idle.");
                }
            }
        }
    }

    fn draw(&self, renderer: &mut Renderer) {
        // Toolbar background.
        renderer.set_color(self.theme.colors.surface_variant);
        renderer.fill_rect(self.toolbar_rect);
        renderer.set_color(self.theme.colors.border);
        renderer.fill_rect(Rect::new(
            self.toolbar_rect.x(),
            self.toolbar_rect.bottom() - 1,
            self.toolbar_rect.width(),
            1,
        ));

        // Canvas (NodeGraph already paints its own background).
        self.graph.draw(renderer, &self.theme);

        // Status bar.
        renderer.set_color(self.theme.colors.surface_variant);
        renderer.fill_rect(self.status_rect);
        renderer.set_color(self.theme.colors.border);
        renderer.fill_rect(Rect::new(
            self.status_rect.x(),
            self.status_rect.y(),
            self.status_rect.width(),
            1,
        ));
        self.status_label.draw(renderer, &self.theme);

        // Toolbar buttons (drawn after the surface so they sit on top).
        for btn in [&self.queue_btn, &self.cancel_btn, &self.save_btn, &self.load_btn] {
            btn.draw(renderer, &self.theme);
        }

        // Modal file dialog (drawn last, with a translucent backdrop).
        if let Some(dialog) = &self.file_dialog {
            renderer.set_color(Color::rgba(0, 0, 0, 160));
            renderer.fill_rect(self.canvas_rect);
            dialog.draw(renderer, &self.theme);
        }
    }
}

fn event_position(event: &Event) -> Option<Point> {
    match event {
        Event::MouseMove(e) => Some(e.position),
        Event::MouseButton(e) => Some(e.position),
        Event::MouseWheel(e) => Some(e.position),
        _ => None,
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    // Detect primary monitor and use 85% of its size in logical pixels.
    // Falls back to 2400x1600 if no monitor info (was 1400x900 — too small
    // on 4K screens when primary_monitor returned None).
    let (win_w, win_h, ui_scale) = match event_loop.primary_monitor() {
        Some(m) => {
            let scale = m.scale_factor().max(1.0);
            let phys = m.size();
            let w = ((phys.width as f64) * 0.85 / scale) as i32;
            let h = ((phys.height as f64) * 0.85 / scale) as i32;
            log::info!(
                "monitor: {}x{} phys, scale {:.2} -> window {}x{} logical",
                phys.width,
                phys.height,
                scale,
                w,
                h
            );
            (w.max(1024), h.max(720), scale as f32)
        }
        None => {
            log::warn!("no primary monitor detected; falling back to 2400x1600 logical");
            (2400, 1600, 1.0_f32)
        }
    };
    let mut renderer = Renderer::new(&event_loop, win_w, win_h, "EriGui — Node Graph App")?;
    log::info!("UI scale = {:.2} (HiDPI font/spacing/border multiplier)", ui_scale);
    let theme = Theme::alex_jammin().with_scale(ui_scale);
    let viewport = renderer.viewport_size();
    let mut app = App::new(theme.clone(), viewport);

    // Auto-load default workflow if it exists. Bypasses the file dialog
    // for the common "open the app, get the Klein graph" path.
    let default_workflow =
        std::path::PathBuf::from("/home/alex/EriGui/rust-gui/workflows/klein_basic.json");
    if default_workflow.exists() {
        match Workflow::load_from_file(&default_workflow) {
            Ok(wf) => {
                let report = wf.resolve(&app.registry);
                if !report.unresolved.is_empty() {
                    log::warn!(
                        "default workflow has {} unresolved nodes (will be dropped): {:?}",
                        report.unresolved.len(),
                        report.unresolved
                    );
                }
                let new_graph = workflow_to_graph(&wf, &app.registry);
                app.graph.graph = new_graph;
                app.set_status(format!("Loaded {}", default_workflow.display()));
            }
            Err(e) => {
                log::error!("Failed to load default workflow: {e}");
                app.set_status(format!("Load failed: {e}"));
            }
        }
    }

    let mut translator = EventTranslator::new(viewport);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match event {
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(physical) => {
                    let new_size = Size::new(physical.width as i32, physical.height as i32);
                    renderer.resize(physical.width, physical.height);
                    translator.set_window_size(new_size);
                    app.layout(new_size);
                    renderer.window().request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    redraw(&mut app, &mut renderer, &theme);
                }
                other => {
                    if let Some(ev) = translator.translate(&other) {
                        if app.handle_event(&ev) {
                            renderer.window().request_redraw();
                        }
                    }
                }
            },
            WinitEvent::AboutToWait => {
                redraw(&mut app, &mut renderer, &theme);
            }
            _ => {}
        }
    })?;
    Ok(())
}

/// Per-frame: drain executor progress, run any pending toolbar action, then
/// render. Pulled out so both `AboutToWait` (driven each idle tick when the
/// control flow is `Poll`) and explicit `RedrawRequested` paint identically.
fn redraw(app: &mut App, renderer: &mut Renderer, theme: &Theme) {
    app.drain_progress();
    app.drain_actions();

    let size = renderer.viewport_size();
    app.layout(size);

    renderer.begin_frame(theme.colors.background);
    app.draw(renderer);
    renderer.end_frame();
}

/// Convert a `[B, 3, H, W]` image tensor whose values land in `[-1, 1]` into
/// a tightly-packed `(rgb_bytes, width, height)` triple suitable for
/// `NodeGraph::set_node_image`. Mirrors the
/// `core/save_image` / `klein_lora_infer.rs:381` `(x + 1) * 127.5` remap;
/// only batch index 0 is rendered. This conversion lives here (not in
/// erigui-widgets) so the widget crate can build/test without flame-core.
fn tensor_to_rgb_bytes(image: &flame_core::Tensor) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let dims = image.dims();
    if dims.len() != 4 {
        anyhow::bail!(
            "preview: expected 4-D image tensor [B, 3, H, W], got rank {}",
            dims.len()
        );
    }
    let (b, c, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    if c != 3 {
        anyhow::bail!("preview: expected 3 channels, got {c}");
    }
    if b == 0 {
        anyhow::bail!("preview: empty batch");
    }
    let rgb_f32 = image.to_dtype(flame_core::DType::F32)?;
    let data = rgb_f32.to_vec()?;
    let plane = h * w;
    let mut out = vec![0u8; h * w * 3];
    for y in 0..h {
        for x in 0..w {
            for ch in 0..3 {
                let src = ch * plane + y * w + x;
                let dst = (y * w + x) * 3 + ch;
                out[dst] = (127.5 * (data[src].clamp(-1.0, 1.0) + 1.0)) as u8;
            }
        }
    }
    Ok((out, w as u32, h as u32))
}

#[cfg(test)]
mod klein_basic_test {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn klein_basic_workflow_loads_with_all_fields() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("workflows")
            .join("klein_basic.json");

        let wf = Workflow::load_from_file(&path)
            .unwrap_or_else(|e| panic!("klein_basic.json failed to parse: {e}"));
        assert_eq!(wf.version, 1);
        assert_eq!(wf.nodes.len(), 7);
        assert_eq!(wf.edges.len(), 9);

        let registry = Arc::new(NodeRegistry::with_builtins());
        let report = wf.resolve(&registry);
        assert!(
            report.unresolved.is_empty(),
            "unresolved type_ids: {:?}",
            report.unresolved
        );

        let graph = workflow_to_graph(&wf, &registry);
        assert_eq!(graph.nodes.len(), 7, "graph dropped nodes");
        assert_eq!(graph.edges.len(), 9, "graph dropped edges");

        let by_type = |t: &str| -> Vec<&erigui_widgets::node_graph::Node> {
            graph
                .nodes
                .iter()
                .filter(|n| n.component_type.as_deref() == Some(t))
                .collect()
        };

        let lc = by_type("core/load_checkpoint");
        assert_eq!(lc.len(), 1);
        assert_eq!(lc[0].fields.len(), 1, "load_checkpoint should have 1 field (path)");

        let el = by_type("core/empty_latent");
        assert_eq!(el.len(), 1);
        assert_eq!(el[0].fields.len(), 3, "empty_latent should have 3 fields");

        let ep = by_type("core/encode_prompt");
        assert_eq!(ep.len(), 2);
        for n in &ep {
            assert_eq!(n.fields.len(), 2, "encode_prompt should have 2 fields");
        }

        let ks = by_type("core/k_sampler");
        assert_eq!(ks.len(), 1);
        assert_eq!(ks[0].fields.len(), 4, "k_sampler should have 4 fields");

        let vd = by_type("core/vae_decode");
        assert_eq!(vd.len(), 1);
        assert_eq!(vd[0].fields.len(), 0);

        let si = by_type("core/save_image");
        assert_eq!(si.len(), 1);
        assert_eq!(si[0].fields.len(), 1, "save_image should have 1 field");
    }
}

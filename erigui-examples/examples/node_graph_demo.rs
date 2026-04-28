use anyhow::{self, Context as AnyhowContext};
use erigui_core::{DrawContext, Event, Key, Point, Rect, Size, Theme, Widget};
use erigui_examples::node_graph_handler::{Command, GraphHandler, PaletteNodeKind};
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::{Button, Label, TextInput, WidgetId, WidgetManager};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

const PALETTE_WIDTH: i32 = 260;
const SIDEBAR_WIDTH: i32 = 320;
const BACKEND_BASE_URL: &str = "http://127.0.0.1:8000";

#[derive(Debug, Clone, Deserialize)]
struct ApiPortInfo {
    name: String,
    #[serde(rename = "type")]
    port_type: String,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiPropertyDefinition {
    name: String,
    display_name: String,
    #[serde(rename = "type")]
    prop_type: String,
    #[serde(default)]
    default_value: Option<serde_json::Value>,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiComponentInfo {
    id: String,
    #[serde(rename = "type")]
    comp_type: String,
    display_name: String,
    category: String,
    icon: String,
    input_ports: Vec<ApiPortInfo>,
    output_ports: Vec<ApiPortInfo>,
    property_definitions: Vec<ApiPropertyDefinition>,
}

struct App {
    theme: Theme,
    widgets: WidgetManager,
    graph: GraphHandler,
    sidebar_rect: Rect,
    palette_rect: Rect,
    inspector_rect: Rect,
    backend_label_id: WidgetId,
    label_id: WidgetId,
    title_input_id: WidgetId,
    add_in_id: WidgetId,
    add_out_id: WidgetId,
    delete_id: WidgetId,
    fit_id: WidgetId,
    reset_id: WidgetId,
    add_image_id: WidgetId,
    last_selected: Option<usize>,
    commands: Rc<RefCell<Vec<Command>>>,
    backend_components: Vec<ApiComponentInfo>,
    palette_buttons: Vec<PaletteButton>,
    active_drag: Option<PaletteNodeKind>,
    graph_area: Rect,
    inspector_fields: Vec<(String, String)>,
}

#[derive(Debug)]
enum BackendMessage {
    ComponentsLoaded(Vec<ApiComponentInfo>),
    BackendError(String),
}

#[derive(Clone)]
struct PaletteButton {
    id: WidgetId,
    kind: PaletteNodeKind,
    bounds: Rect,
    label: &'static str,
}

impl App {
    fn new(theme: Theme) -> Self {
        let mut widgets = WidgetManager::new();
        let commands: Rc<RefCell<Vec<Command>>> = Rc::new(RefCell::new(Vec::new()));

        let graph = GraphHandler::attach(&mut widgets);
        graph.add_default_groups(&mut widgets, &theme);

        let backend_label_id = widgets.add_widget(Box::new(Label::new(
            WidgetId::default(),
            "Backend: checking...",
        )));

        let label_id = widgets.add_widget(Box::new(Label::new(
            WidgetId::default(),
            "Selected: (none)",
        )));

        let title_input_id = widgets.add_widget(Box::new(
            TextInput::new(WidgetId::default()).with_placeholder("Node title"),
        ));

        let add_in_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Add Input").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::AddInput)
            }),
        ));

        let add_out_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Add Output").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::AddOutput)
            }),
        ));

        let delete_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Delete Selected").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::DeleteSelected)
            }),
        ));

        let fit_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Fit View").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::FitView)
            }),
        ));

        let reset_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Reset View").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::ResetView)
            }),
        ));

        let add_image_id = widgets.add_widget(Box::new(
            Button::new(WidgetId::default(), "Add Image Node").with_on_click({
                let cmds = commands.clone();
                move || cmds.borrow_mut().push(Command::AddImageNode)
            }),
        ));

        let model_button = widgets.add_widget(Box::new(Button::new(
            WidgetId::default(),
            "Model Node (drag)",
        )));
        let params_button = widgets.add_widget(Box::new(Button::new(
            WidgetId::default(),
            "Params Node (drag)",
        )));
        let vae_button = widgets.add_widget(Box::new(Button::new(
            WidgetId::default(),
            "VAE Node (drag)",
        )));
        let gen_button = widgets.add_widget(Box::new(Button::new(
            WidgetId::default(),
            "Generator Node (drag)",
        )));

        let palette_buttons = vec![
            PaletteButton {
                id: model_button,
                kind: PaletteNodeKind::Model,
                bounds: Rect::new(0, 0, 0, 0),
                label: "Model",
            },
            PaletteButton {
                id: params_button,
                kind: PaletteNodeKind::Params,
                bounds: Rect::new(0, 0, 0, 0),
                label: "Params",
            },
            PaletteButton {
                id: vae_button,
                kind: PaletteNodeKind::Vae,
                bounds: Rect::new(0, 0, 0, 0),
                label: "VAE",
            },
            PaletteButton {
                id: gen_button,
                kind: PaletteNodeKind::Generator,
                bounds: Rect::new(0, 0, 0, 0),
                label: "Generator",
            },
        ];

        Self {
            theme,
            widgets,
            graph,
            sidebar_rect: Rect::new(0, 0, SIDEBAR_WIDTH, 0),
            palette_rect: Rect::new(0, 0, PALETTE_WIDTH, 0),
            inspector_rect: Rect::new(0, 0, SIDEBAR_WIDTH, 0),
            backend_label_id,
            label_id,
            title_input_id,
            add_in_id,
            add_out_id,
            delete_id,
            fit_id,
            reset_id,
            add_image_id,
            last_selected: None,
            commands,
            backend_components: Vec::new(),
            palette_buttons,
            active_drag: None,
            graph_area: Rect::new(0, 0, 0, 0),
            inspector_fields: Vec::new(),
        }
    }

    fn layout(&mut self, viewport: Size) {
        self.palette_rect = Rect::new(0, 0, PALETTE_WIDTH, viewport.height);

        let available_graph_width = (viewport.width - PALETTE_WIDTH - SIDEBAR_WIDTH).max(240);
        let graph_x = self.palette_rect.right();
        let graph_rect = Rect::new(graph_x, 0, available_graph_width, viewport.height);
        self.graph_area = graph_rect;

        let inspector_x = graph_rect.right();
        self.inspector_rect = Rect::new(inspector_x, 0, SIDEBAR_WIDTH, viewport.height);
        self.sidebar_rect = self.inspector_rect;

        if let Some(graph) = self.widgets.get_mut(self.graph.graph_id()) {
            graph.layout(graph_rect, &self.theme);
        }

        let mut y = self.inspector_rect.y() + 16;
        let pad = 16;
        let full_w = self.inspector_rect.width() - pad * 2;
        let inspector_x = inspector_x + pad;

        self.set_bounds(self.backend_label_id, inspector_x, y, full_w, 24);
        y += 28;
        self.set_bounds(self.label_id, inspector_x, y, full_w, 24);
        y += 32;

        self.set_bounds(self.title_input_id, inspector_x, y, full_w, 36);
        y += 44;

        self.set_bounds(self.add_in_id, inspector_x, y, full_w, 32);
        y += 40;
        self.set_bounds(self.add_out_id, inspector_x, y, full_w, 32);
        y += 40;
        self.set_bounds(self.add_image_id, inspector_x, y, full_w, 32);
        y += 40;
        self.set_bounds(self.delete_id, inspector_x, y, full_w, 32);
        y += 44;
        self.set_bounds(self.fit_id, inspector_x, y, full_w, 32);
        y += 40;
        self.set_bounds(self.reset_id, inspector_x, y, full_w, 32);

        let palette_pad = 16;
        let palette_x = self.palette_rect.x() + palette_pad;
        let palette_width = self.palette_rect.width() - palette_pad * 2;
        let mut palette_y = self.palette_rect.y() + 24;
        for id in self.palette_button_ids() {
            self.set_bounds(id, palette_x, palette_y, palette_width, 44);
            palette_y += 56;
        }
    }

    fn set_bounds(&mut self, id: WidgetId, x: i32, y: i32, w: i32, h: i32) {
        if let Some(wgt) = self.widgets.get_mut(id) {
            wgt.layout(Rect::new(x, y, w, h), &self.theme);
        }
        if let Some(entry) = self
            .palette_buttons
            .iter_mut()
            .find(|button| button.id == id)
        {
            entry.bounds = Rect::new(x, y, w, h);
        }
    }

    fn dispatch_widget(&mut self, id: WidgetId, event: &Event) -> bool {
        if let Some(wgt) = self.widgets.get_mut(id) {
            return wgt.handle_event(event, &self.theme).is_consumed();
        }
        false
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::MouseMove(ev) => {
                if self.palette_rect.contains(ev.position) {
                    let mut consumed = false;
                    for id in self.palette_button_ids() {
                        consumed |= self.dispatch_widget(id, event);
                    }
                    return consumed;
                }
                let in_sidebar = self.sidebar_rect.contains(ev.position);
                if in_sidebar {
                    let mut consumed = false;
                    for id in self.sidebar_widget_ids() {
                        consumed |= self.dispatch_widget(id, event);
                    }
                    return consumed;
                } else {
                    return self.dispatch_widget(self.graph.graph_id(), event);
                }
            }
            Event::MouseButton(ev) => {
                if self.palette_rect.contains(ev.position) {
                    let mut consumed = false;
                    for id in self.palette_button_ids() {
                        consumed |= self.dispatch_widget(id, event);
                    }
                    if ev.pressed {
                        if let Some(kind) = self.palette_hit(ev.position) {
                            self.active_drag = Some(kind);
                            return true;
                        }
                    } else if let Some(kind) = self.active_drag.take() {
                        if self.graph_area.contains(ev.position) {
                            let local_x = (ev.position.x - self.graph_area.x())
                                .clamp(0, self.graph_area.width());
                            let local_y = (ev.position.y - self.graph_area.y())
                                .clamp(0, self.graph_area.height());
                            let local = Point::new(local_x, local_y);
                            self.graph.add_palette_node(&mut self.widgets, kind, local);
                            return true;
                        }
                    }
                    return consumed;
                }

                if !ev.pressed {
                    if let Some(kind) = self.active_drag.take() {
                        if self.graph_area.contains(ev.position) {
                            let local_x = (ev.position.x - self.graph_area.x())
                                .clamp(0, self.graph_area.width());
                            let local_y = (ev.position.y - self.graph_area.y())
                                .clamp(0, self.graph_area.height());
                            let local = Point::new(local_x, local_y);
                            self.graph.add_palette_node(&mut self.widgets, kind, local);
                            return true;
                        }
                    }
                }

                let in_sidebar = self.sidebar_rect.contains(ev.position);
                if in_sidebar {
                    if ev.pressed {
                        if let Some(input) =
                            self.widgets.get_typed_mut::<TextInput>(self.title_input_id)
                        {
                            input.set_focused(true);
                        }
                    }
                    let mut consumed = false;
                    for id in self.sidebar_widget_ids() {
                        consumed |= self.dispatch_widget(id, event);
                    }
                    return consumed;
                }

                if ev.pressed {
                    if let Some(input) =
                        self.widgets.get_typed_mut::<TextInput>(self.title_input_id)
                    {
                        input.set_focused(false);
                    }
                }
                self.dispatch_widget(self.graph.graph_id(), event)
            }
            Event::MouseWheel(ev) => {
                if self.palette_rect.contains(ev.position) {
                    let mut consumed = false;
                    for id in self.palette_button_ids() {
                        consumed |= self.dispatch_widget(id, event);
                    }
                    return consumed;
                }
                let in_sidebar = self.sidebar_rect.contains(ev.position);
                if in_sidebar {
                    let mut consumed = false;
                    consumed |= self.dispatch_widget(self.title_input_id, event);
                    return consumed;
                } else {
                    return self.dispatch_widget(self.graph.graph_id(), event);
                }
            }
            Event::KeyPress(key_ev) => {
                let mut consumed = false;
                if let Some(input) = self.widgets.get_typed::<TextInput>(self.title_input_id) {
                    if input.is_focused() {
                        consumed |= self.dispatch_widget(self.title_input_id, event);
                    }
                }
                if !consumed {
                    consumed |= self.dispatch_widget(self.graph.graph_id(), event);
                }
                if !consumed {
                    match key_ev.key {
                        Key::S => {
                            let _ = self
                                .graph
                                .save_to_path(&self.widgets, "/tmp/node_graph_state.json");
                            consumed = true;
                        }
                        Key::T => {
                            self.commands.borrow_mut().push(Command::ToggleGrid);
                            consumed = true;
                        }
                        Key::I => {
                            self.commands.borrow_mut().push(Command::AddImageNode);
                            consumed = true;
                        }
                        Key::L => {
                            let _ = self
                                .graph
                                .load_from_path(&mut self.widgets, "/tmp/node_graph_state.json");
                            self.last_selected = None;
                            consumed = true;
                        }
                        Key::F2 => {
                            consumed = true;
                        }
                        _ => {}
                    }
                }
                consumed
            }
            Event::KeyRelease(_) | Event::TextInput(_) => {
                let mut consumed = false;
                if let Some(input) = self.widgets.get_typed::<TextInput>(self.title_input_id) {
                    if input.is_focused() {
                        consumed |= self.dispatch_widget(self.title_input_id, event);
                    }
                }
                if !consumed {
                    consumed |= self.dispatch_widget(self.graph.graph_id(), event);
                }
                consumed
            }
            Event::DragDrop(_) => self.dispatch_widget(self.graph.graph_id(), event),
            _ => false,
        }
    }

    fn apply_commands(&mut self) {
        let cmds: Vec<Command> = self.commands.borrow_mut().drain(..).collect();
        if !cmds.is_empty() {
            self.graph.apply_commands(&mut self.widgets, cmds);
        }
    }

    fn handle_backend_message(&mut self, msg: BackendMessage) {
        match msg {
            BackendMessage::ComponentsLoaded(list) => {
                self.backend_components = list;
                self.update_backend_label(format!(
                    "Backend OK ({} components)",
                    self.backend_components.len()
                ));
            }
            BackendMessage::BackendError(err) => {
                self.backend_components.clear();
                self.update_backend_label(format!("Backend error: {}", err));
            }
        }
    }

    fn update_backend_label(&mut self, text: impl Into<String>) {
        if let Some(label) = self.widgets.get_typed_mut::<Label>(self.backend_label_id) {
            label.set_text(text.into());
        }
    }

    fn palette_hit(&self, position: Point) -> Option<PaletteNodeKind> {
        self.palette_buttons
            .iter()
            .find(|entry| entry.bounds.contains(position))
            .map(|entry| entry.kind)
    }

    fn sync_inspector(&mut self) {
        self.apply_commands();

        let selected = self.graph.selected_node(&self.widgets);

        if let Some(label) = self.widgets.get_typed_mut::<Label>(self.label_id) {
            if let Some(node) = &selected {
                label.set_text(format!("Selected: {} (id {})", node.title, node.id));
            } else {
                label.set_text("Selected: (none)");
            }
        }

        if let Some(input) = self.widgets.get_typed_mut::<TextInput>(self.title_input_id) {
            if selected.as_ref().map(|n| n.id) != self.last_selected {
                if let Some(node) = &selected {
                    input.set_text(node.title.clone());
                } else {
                    input.set_text("");
                }
                self.last_selected = selected.as_ref().map(|n| n.id);
            } else if let Some(node) = &selected {
                let text = input.text().to_string();
                if text != node.title {
                    self.graph.set_selected_title(&mut self.widgets, text);
                }
            }
        }
    }

    fn draw(&self, renderer: &mut Renderer) {
        renderer.set_color(self.theme.colors.surface);
        renderer.fill_rect(self.palette_rect);
        renderer.set_color(self.theme.colors.surface_variant);
        renderer.fill_rect(self.sidebar_rect);

        if let Some(graph) = self.widgets.get(self.graph.graph_id()) {
            graph.draw(renderer, &self.theme);
        }

        for id in self.palette_button_ids() {
            if let Some(wgt) = self.widgets.get(id) {
                wgt.draw(renderer, &self.theme);
            }
        }

        for id in self.sidebar_widget_ids_with_backend() {
            if let Some(wgt) = self.widgets.get(id) {
                wgt.draw(renderer, &self.theme);
            }
        }
    }

    fn palette_button_ids(&self) -> Vec<WidgetId> {
        self.palette_buttons.iter().map(|b| b.id).collect()
    }

    fn sidebar_widget_ids(&self) -> [WidgetId; 7] {
        [
            self.title_input_id,
            self.add_in_id,
            self.add_out_id,
            self.delete_id,
            self.fit_id,
            self.reset_id,
            self.add_image_id,
        ]
    }

    fn sidebar_widget_ids_with_backend(&self) -> [WidgetId; 9] {
        [
            self.backend_label_id,
            self.label_id,
            self.title_input_id,
            self.add_in_id,
            self.add_out_id,
            self.add_image_id,
            self.delete_id,
            self.fit_id,
            self.reset_id,
        ]
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(all(unix, not(target_os = "macos")))]
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let (backend_tx, backend_rx) = mpsc::channel();
    request_backend_components(backend_tx.clone());

    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::new(&event_loop, 1400, 900, "EriGui - Node Graph Demo")?;
    let theme = Theme::alex_jammin();
    let mut app = App::new(theme.clone());
    let mut translator = EventTranslator::new(renderer.viewport_size());

    let viewport = renderer.viewport_size();
    app.layout(Size::new(viewport.width, viewport.height));

    let backend_sender = backend_tx.clone();
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        while let Ok(msg) = backend_rx.try_recv() {
            app.handle_backend_message(msg);
        }
        match event {
            WinitEvent::WindowEvent { event, .. } => {
                if let WindowEvent::CloseRequested = event {
                    elwt.exit();
                    return;
                }
                if let WindowEvent::KeyboardInput { event: ref input, .. } = event {
                    if input.state == winit::event::ElementState::Pressed
                        && input.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F2)
                    {
                        request_backend_components(backend_sender.clone());
                    }
                }
                if let WindowEvent::Resized(physical) = event {
                    let new_size = Size::new(physical.width as i32, physical.height as i32);
                    renderer.resize(physical.width, physical.height);
                    app.layout(new_size);
                    translator.set_window_size(new_size);
                }
                if let Some(ev) = translator.translate(&event) {
                    if app.handle_event(&ev) {
                        renderer.window().request_redraw();
                    }
                }
            }
            WinitEvent::AboutToWait | WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                let size = renderer.viewport_size();
                app.layout(Size::new(size.width, size.height));
                app.sync_inspector();

                renderer.begin_frame(theme.colors.background);
                app.draw(&mut renderer);
                renderer.end_frame();
            }
            _ => {}
        }
    }).unwrap();
    Ok(())
}

fn fetch_backend_components() -> anyhow::Result<Vec<ApiComponentInfo>> {
    let url = format!("{}/api/components", BACKEND_BASE_URL);
    let response = reqwest::blocking::get(&url).with_context(|| format!("requesting {}", url))?;
    if !response.status().is_success() {
        anyhow::bail!("backend status {}", response.status());
    }
    let items = response
        .json::<Vec<ApiComponentInfo>>()
        .context("parsing component payload")?;
    Ok(items)
}

fn request_backend_components(tx: Sender<BackendMessage>) {
    thread::spawn(move || {
        let message = match fetch_backend_components() {
            Ok(list) => BackendMessage::ComponentsLoaded(list),
            Err(err) => BackendMessage::BackendError(err.to_string()),
        };
        let _ = tx.send(message);
    });
}

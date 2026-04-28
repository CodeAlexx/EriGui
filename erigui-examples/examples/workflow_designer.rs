use anyhow::{self, Context as AnyhowContext};
use erigui_core::{Color, DrawContext, Event, Point, Rect, Size, Theme};
use erigui_examples::node_graph_handler::{self, Command, GraphHandler};
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::{FieldValue, Graph, Node, WidgetManager};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use winit::event::{Event as WinitEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

const TOP_BAR_HEIGHT: i32 = 72;
const RIGHT_PANEL_WIDTH: i32 = 320;
const LEFT_PANEL_WIDTH: i32 = 200;
const BOTTOM_PANEL_HEIGHT: i32 = 200;
const CARD_HEIGHT: i32 = 48;
const CARD_RADIUS: i32 = 12;
const BACKEND_BASE_URL: &str = "http://127.0.0.1:8000";
const APP_TITLE: &str = "EriDiffusion";

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ApiComponentInfo {
    id: String,
    #[serde(rename = "type", default)]
    comp_type: Option<String>,
    display_name: String,
    category: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    inputs: Vec<ApiPortInfo>,
    #[serde(default)]
    outputs: Vec<ApiPortInfo>,
    #[serde(default = "Vec::new")]
    properties: Vec<ApiPropertyDefinition>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ApiPortInfo {
    name: String,
    #[serde(rename = "type")]
    port_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ApiPropertyDefinition {
    name: String,
    #[serde(rename = "type")]
    prop_type: String,
    #[serde(default)]
    default: Option<serde_json::Value>,
}

#[derive(Debug)]
enum BackendMessage {
    Connected(usize),
    Error(String),
    ExecutionFinished(Result<String, String>),
    ExecutionFiles(Vec<String>),
}

#[derive(Clone, Copy)]
enum ToolbarCommand {
    Generate,
    Clear,
    Save,
    Load,
    ResetView,
    CenterView,
    FitContent,
}

struct ToolbarButton {
    label: &'static str,
    width: i32,
    command: ToolbarCommand,
    rect: Rect,
}

impl ToolbarButton {
    fn new(label: &'static str, width: i32, command: ToolbarCommand) -> Self {
        Self {
            label,
            width,
            command,
            rect: Rect::new(0, 0, width, 40),
        }
    }
}

#[derive(Clone)]
struct ComponentCard {
    label: &'static str,
    icon: &'static str,
    component_type: &'static str,
    rect: Rect,
}

struct ComponentGroup {
    name: &'static str,
    cards: Vec<ComponentCard>,
    header_rect: Rect,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DesignerMode {
    Trainer,
    Inference,
    Samples,
}

impl DesignerMode {
    fn label(&self) -> &'static str {
        match self {
            DesignerMode::Trainer => "Trainer",
            DesignerMode::Inference => "Inference",
            DesignerMode::Samples => "Samples",
        }
    }
}

#[derive(Serialize)]
struct WorkflowPayload {
    workflow: WorkflowDataPayload,
}

#[derive(Serialize)]
struct WorkflowDataPayload {
    components: Vec<ComponentPayload>,
    connections: Vec<ConnectionPayload>,
}

#[derive(Serialize)]
struct ComponentPayload {
    id: String,
    #[serde(rename = "type")]
    comp_type: String,
    properties: JsonMap<String, JsonValue>,
}

#[derive(Serialize)]
struct ConnectionPayload {
    id: String,
    from_component: String,
    from_port: String,
    to_component: String,
    to_port: String,
}

#[derive(Clone)]
struct PropertyLine {
    label: String,
    value: String,
}

struct App {
    theme: Theme,
    widgets: WidgetManager,
    graph: GraphHandler,
    commands: Rc<RefCell<Vec<Command>>>,
    backend_tx: Sender<BackendMessage>,

    toolbar_buttons: Vec<ToolbarButton>,
    component_groups: Vec<ComponentGroup>,
    mode: DesignerMode,
    mode_toggle_rects: [Rect; 3],
    training_control_rects: [Rect; 3],

    top_bar_rect: Rect,
    graph_rect: Rect,
    right_panel_rect: Rect,
    bottom_panel_rect: Rect,
    left_panel_rect: Rect,
    add_component_rect: Rect,

    hovered_toolbar: Option<usize>,
    pressed_toolbar: Option<usize>,
    hovered_component: Option<(usize, usize)>,

    active_drag: Option<ComponentCard>,
    cursor_position: Point,

    backend_status: String,
    backend_connected: bool,

    property_lines: Vec<PropertyLine>,
}

impl App {
    fn new(theme: Theme, backend_tx: Sender<BackendMessage>) -> Self {
        let mut widgets = WidgetManager::new();
        let commands: Rc<RefCell<Vec<Command>>> = Rc::new(RefCell::new(Vec::new()));
        let graph = GraphHandler::attach(&mut widgets);
        graph.replace_graph(&mut widgets, node_graph_handler::trainer_graph());

        let toolbar_buttons = vec![
            ToolbarButton::new("Generate", 120, ToolbarCommand::Generate),
            ToolbarButton::new("Clear", 90, ToolbarCommand::Clear),
            ToolbarButton::new("Save", 80, ToolbarCommand::Save),
            ToolbarButton::new("Load", 80, ToolbarCommand::Load),
            ToolbarButton::new("Reset", 90, ToolbarCommand::ResetView),
            ToolbarButton::new("Center View", 140, ToolbarCommand::CenterView),
            ToolbarButton::new("Fit Content", 140, ToolbarCommand::FitContent),
        ];

        let component_groups = trainer_component_groups();

        Self {
            theme,
            widgets,
            graph,
            commands,
            backend_tx,
            toolbar_buttons,
            component_groups,
            mode: DesignerMode::Trainer,
            mode_toggle_rects: [Rect::new(0, 0, 0, 0); 3],
            training_control_rects: [Rect::new(0, 0, 0, 0); 3],
            top_bar_rect: Rect::new(0, 0, 0, 0),
            graph_rect: Rect::new(0, 0, 0, 0),
            right_panel_rect: Rect::new(0, 0, 0, 0),
            bottom_panel_rect: Rect::new(0, 0, 0, 0),
            left_panel_rect: Rect::new(0, 0, 0, 0),
            add_component_rect: Rect::new(0, 0, 0, 0),
            hovered_toolbar: None,
            pressed_toolbar: None,
            hovered_component: None,
            active_drag: None,
            cursor_position: Point::ZERO,
            backend_status: "Checking backend...".to_string(),
            backend_connected: false,
            property_lines: Vec::new(),
        }
    }

    fn layout(&mut self, viewport: Size) {
        self.top_bar_rect = Rect::new(0, 0, viewport.width, TOP_BAR_HEIGHT);

        let graph_width = (viewport.width - RIGHT_PANEL_WIDTH - LEFT_PANEL_WIDTH).max(400);
        let graph_height = (viewport.height - TOP_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT).max(300);
        self.graph_rect = Rect::new(LEFT_PANEL_WIDTH, TOP_BAR_HEIGHT, graph_width, graph_height);
        self.right_panel_rect = Rect::new(
            self.graph_rect.right(),
            TOP_BAR_HEIGHT,
            RIGHT_PANEL_WIDTH,
            viewport.height - TOP_BAR_HEIGHT,
        );
        self.left_panel_rect = Rect::new(
            0,
            TOP_BAR_HEIGHT,
            LEFT_PANEL_WIDTH,
            viewport.height - TOP_BAR_HEIGHT,
        );
        self.bottom_panel_rect = Rect::new(
            self.graph_rect.x(),
            self.graph_rect.bottom(),
            self.graph_rect.width(),
            BOTTOM_PANEL_HEIGHT,
        );

        let mut sidebar_y = self.left_panel_rect.y() + 24;
        for rect in self.mode_toggle_rects.iter_mut() {
            *rect = Rect::new(
                self.left_panel_rect.x() + 16,
                sidebar_y,
                self.left_panel_rect.width() - 32,
                36,
            );
            sidebar_y += 48;
        }
        sidebar_y += 16;
        for (idx, rect) in self.training_control_rects.iter_mut().enumerate() {
            *rect = Rect::new(
                self.left_panel_rect.x() + 16,
                sidebar_y + idx as i32 * 44,
                self.left_panel_rect.width() - 32,
                32,
            );
        }

        if let Some(widget) = self.widgets.get_mut(self.graph.graph_id()) {
            widget.layout(self.graph_rect, &self.theme);
        }

        let mut x = self.left_panel_rect.right() + 24;
        for button in &mut self.toolbar_buttons {
            button.rect = Rect::new(x, self.top_bar_rect.y() + 18, button.width, 36);
            x += button.width + 12;
        }

        let panel_padding = 20;
        self.add_component_rect = Rect::new(
            self.right_panel_rect.x() + panel_padding,
            self.right_panel_rect.y() + 12,
            self.right_panel_rect.width() - panel_padding * 2,
            38,
        );

        let mut y = self.add_component_rect.bottom() + 24;
        for group in &mut self.component_groups {
            group.header_rect = Rect::new(
                self.right_panel_rect.x() + panel_padding,
                y,
                self.right_panel_rect.width() - panel_padding * 2,
                22,
            );
            y += 28;
            for card in &mut group.cards {
                card.rect = Rect::new(
                    self.right_panel_rect.x() + panel_padding,
                    y,
                    self.right_panel_rect.width() - panel_padding * 2,
                    CARD_HEIGHT,
                );
                y += CARD_HEIGHT + 12;
            }
            y += 16;
        }
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::MouseMove(ev) => {
                self.cursor_position = ev.position;
                self.hovered_toolbar = self.toolbar_hit(ev.position);
                self.hovered_component = self.component_hit(ev.position);
                if self.active_drag.is_some() {
                    return true;
                }
                if self.graph_rect.contains(ev.position) {
                    return self.dispatch_graph(event);
                }
                false
            }
            Event::MouseButton(ev) => {
                if ev.pressed && self.left_panel_rect.contains(ev.position) {
                    if self.handle_left_panel_click(ev.position) {
                        return true;
                    }
                    return true;
                }
                if ev.pressed {
                    if let Some(idx) = self.toolbar_hit(ev.position) {
                        self.pressed_toolbar = Some(idx);
                        return true;
                    }
                    if let Some((group_idx, card_idx)) = self.component_hit(ev.position) {
                        if let Some(card) = self
                            .component_groups
                            .get(group_idx)
                            .and_then(|g| g.cards.get(card_idx))
                        {
                            self.active_drag = Some(card.clone());
                            return true;
                        }
                    }
                    if self.add_component_rect.contains(ev.position) {
                        return true;
                    }
                } else {
                    if let Some(idx) = self.pressed_toolbar.take() {
                        if self.toolbar_buttons[idx].rect.contains(ev.position) {
                            self.activate_toolbar(self.toolbar_buttons[idx].command);
                            return true;
                        }
                    }
                    if let Some(card) = self.active_drag.take() {
                        if self.graph_rect.contains(ev.position) {
                            let local = Point::new(
                                (ev.position.x - self.graph_rect.x())
                                    .clamp(0, self.graph_rect.width()),
                                (ev.position.y - self.graph_rect.y())
                                    .clamp(0, self.graph_rect.height()),
                            );
                            self.graph.add_component_node(
                                &mut self.widgets,
                                card.component_type,
                                card.label,
                                local,
                            );
                            return true;
                        }
                    }
                }

                if self.graph_rect.contains(ev.position) {
                    return self.dispatch_graph(event);
                }
                false
            }
            Event::MouseWheel(ev) => {
                if self.graph_rect.contains(ev.position) {
                    return self.dispatch_graph(event);
                }
                false
            }
            Event::KeyPress(_)
            | Event::KeyRelease(_)
            | Event::TextInput(_)
            | Event::DragDrop(_) => self.dispatch_graph(event),
            _ => false,
        }
    }

    fn dispatch_graph(&mut self, event: &Event) -> bool {
        if let Some(widget) = self.widgets.get_mut(self.graph.graph_id()) {
            return widget.handle_event(event, &self.theme).is_consumed();
        }
        false
    }

    fn toolbar_hit(&self, point: Point) -> Option<usize> {
        self.toolbar_buttons
            .iter()
            .enumerate()
            .find(|(_, btn)| btn.rect.contains(point))
            .map(|(idx, _)| idx)
    }

    fn component_hit(&self, point: Point) -> Option<(usize, usize)> {
        for (group_idx, group) in self.component_groups.iter().enumerate() {
            for (card_idx, card) in group.cards.iter().enumerate() {
                if card.rect.contains(point) {
                    return Some((group_idx, card_idx));
                }
            }
        }
        None
    }

    fn activate_toolbar(&mut self, command: ToolbarCommand) {
        match command {
            ToolbarCommand::Generate => match self.build_workflow_payload() {
                Ok(payload) => {
                    self.backend_status = "Sending workflow...".to_string();
                    request_workflow_execution(self.backend_tx.clone(), payload);
                }
                Err(err) => {
                    self.backend_status = format!("Workflow error: {err}");
                }
            },
            ToolbarCommand::Clear => {
                self.commands.borrow_mut().push(Command::ClearGraph);
            }
            ToolbarCommand::Save => {
                let _ = self
                    .graph
                    .save_to_path(&self.widgets, "/tmp/erigui_workflow.json");
                self.backend_status = "Saved to /tmp/erigui_workflow.json".to_string();
            }
            ToolbarCommand::Load => {
                if self
                    .graph
                    .load_from_path(&mut self.widgets, "/tmp/erigui_workflow.json")
                    .is_ok()
                {
                    self.backend_status = "Loaded /tmp/erigui_workflow.json".to_string();
                }
            }
            ToolbarCommand::ResetView => {
                self.commands.borrow_mut().push(Command::ResetView);
            }
            ToolbarCommand::CenterView => {
                self.commands.borrow_mut().push(Command::FitView);
            }
            ToolbarCommand::FitContent => {
                self.commands.borrow_mut().push(Command::FitView);
            }
        }
    }

    fn apply_commands(&mut self) {
        let cmds: Vec<Command> = self.commands.borrow_mut().drain(..).collect();
        if !cmds.is_empty() {
            self.graph.apply_commands(&mut self.widgets, cmds);
        }
    }

    fn sync_inspector(&mut self) {
        self.apply_commands();
        self.property_lines.clear();
        if let Some(node) = self.graph.selected_node(&self.widgets) {
            self.property_lines.push(PropertyLine {
                label: "Node".into(),
                value: node.title.clone(),
            });
            for field in node.fields {
                let value = match field.value {
                    erigui_widgets::FieldValue::Text(text) => text,
                    erigui_widgets::FieldValue::Number(num) => format!("{num}"),
                    erigui_widgets::FieldValue::Select(choice) => choice,
                };
                self.property_lines.push(PropertyLine {
                    label: field.label,
                    value,
                });
            }
        }
    }

    fn set_mode(&mut self, mode: DesignerMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.component_groups = match mode {
            DesignerMode::Trainer => trainer_component_groups(),
            DesignerMode::Inference => inference_component_groups(),
            DesignerMode::Samples => samples_component_groups(),
        };
        let graph = match mode {
            DesignerMode::Trainer => node_graph_handler::trainer_graph(),
            DesignerMode::Inference => node_graph_handler::default_graph(),
            DesignerMode::Samples => node_graph_handler::samples_graph(),
        };
        self.graph
            .replace_graph(&mut self.widgets, graph);
        self.backend_status = format!("Switched to {}", mode.label());
    }

    fn handle_left_panel_click(&mut self, point: Point) -> bool {
        for (idx, rect) in self.mode_toggle_rects.iter().enumerate() {
            if rect.contains(point) {
                let mode = match idx {
                    0 => DesignerMode::Trainer,
                    1 => DesignerMode::Inference,
                    _ => DesignerMode::Samples,
                };
                self.set_mode(mode);
                return true;
            }
        }
        if matches!(self.mode, DesignerMode::Trainer) {
            let status = ["Training started", "Training stopped", "Training resumed"];
            for (idx, rect) in self.training_control_rects.iter().enumerate() {
                if rect.contains(point) {
                    self.backend_status = status[idx].to_string();
                    return true;
                }
            }
        }
        false
    }

    fn build_workflow_payload(&self) -> Result<WorkflowPayload, String> {
        let state = self
            .graph
            .graph_state(&self.widgets)
            .ok_or("Graph data unavailable")?;

        let mut components = Vec::new();
        for node in &state.graph.nodes {
            let comp_type = match node.component_type.as_deref() {
                Some(t) => t.to_string(),
                None => continue,
            };
            if comp_type.starts_with("trainer_")
                || comp_type.starts_with("image_display")
                || comp_type.starts_with("media_display")
                || comp_type.starts_with("samples_view")
            {
                continue;
            }
            let properties = self.node_properties(node)?;
            components.push(ComponentPayload {
                id: format!("node_{}", node.id),
                comp_type,
                properties,
            });
        }

        if components.is_empty() {
            return Err("Add nodes from the palette before generating.".into());
        }

        let mut connections = Vec::new();
        for (idx, edge) in state.graph.edges.iter().enumerate() {
            let from_port = Self::port_label(&state.graph, edge.from_node, edge.from_port, false);
            let to_port = Self::port_label(&state.graph, edge.to_node, edge.to_port, true);
            if let (Some(from_label), Some(to_label)) = (from_port, to_port) {
                connections.push(ConnectionPayload {
                    id: format!("conn_{idx}"),
                    from_component: format!("node_{}", edge.from_node),
                    from_port: from_label,
                    to_component: format!("node_{}", edge.to_node),
                    to_port: to_label,
                });
            }
        }

        Ok(WorkflowPayload {
            workflow: WorkflowDataPayload {
                components,
                connections,
            },
        })
    }

    fn node_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        match node.component_type.as_deref() {
            Some("sdxl_model") => self.model_properties(node),
            Some("flux") | Some("hidream_i1") | Some("omnigen") | Some("sd35") | Some("lumina") => {
                self.model_properties(node)
            }
            Some("sampler_v2") => self.sampler_properties(node),
            Some("sampler") => self.sampler_properties(node),
            Some("vae_model") => self.vae_properties(node),
            Some("output") => self.output_properties(node),
            Some(kind) if kind.starts_with("trainer_") => self.trainer_properties(node),
            Some(other) => Err(format!(
                "Unsupported component type {other} in this backend build"
            )),
            None => Err("Node missing component metadata".into()),
        }
    }

    fn model_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        let mut map = JsonMap::new();
        let checkpoint = self
            .field_text(node, "Model Path")
            .unwrap_or_else(|| "model.safetensors".into());
        let vae_path = self
            .field_text(node, "VAE Path")
            .unwrap_or_else(|| "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors".into());
        map.insert("checkpoint_path".into(), JsonValue::String(checkpoint));
        map.insert("vae_path".into(), JsonValue::String(vae_path));
        map.insert("vae_type".into(), JsonValue::String("auto".into()));
        map.insert("enable_cpu_offload".into(), JsonValue::Bool(true));
        if let Some(ct) = node.component_type.as_deref() {
            map.insert("model_type".into(), JsonValue::String(ct.to_string()));
        }
        let lora_paths = self.field_text(node, "LoRA Paths").unwrap_or_default();
        let lora_scale = self.field_number(node, "LoRA Scale").unwrap_or(1.0);
        map.insert("lora_paths".into(), JsonValue::String(lora_paths));
        map.insert("lora_scale".into(), json!(lora_scale));
        Ok(map)
    }

    fn trainer_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        let mut map = JsonMap::new();
        for field in &node.fields {
            let key = field.label.to_lowercase().replace(' ', "_");
            let value = match &field.value {
                FieldValue::Text(t) => JsonValue::String(t.clone()),
                FieldValue::Number(n) => json!(*n),
                FieldValue::Select(s) => JsonValue::String(s.clone()),
            };
            map.insert(key, value);
        }
        Ok(map)
    }

    fn sampler_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        let mut map = JsonMap::new();
        map.insert(
            "prompt".into(),
            JsonValue::String(self.field_text(node, "Prompt").unwrap_or_default()),
        );
        map.insert(
            "negative_prompt".into(),
            JsonValue::String(
                self.field_text(node, "Negative Prompt")
                    .unwrap_or_else(|| "blurry, low quality".into()),
            ),
        );
        let width = self.field_number(node, "Width").unwrap_or(1024.0) as i32;
        let height = self.field_number(node, "Height").unwrap_or(1024.0) as i32;
        let steps = self.field_number(node, "Steps").unwrap_or(20.0) as i32;
        let cfg = self.field_number(node, "CFG Scale").unwrap_or(7.0);
        let seed = self.field_number(node, "Seed").unwrap_or(-1.0) as i64;
        let batch = self.field_number(node, "Batch Size").unwrap_or(1.0) as i32;
        let sampler = self
            .field_text(node, "Sampler")
            .unwrap_or_else(|| "euler".into());

        map.insert("width".into(), json!(width));
        map.insert("height".into(), json!(height));
        map.insert("steps".into(), json!(steps));
        map.insert("cfg_scale".into(), json!(cfg));
        map.insert("seed".into(), json!(seed));
        map.insert("batch_size".into(), json!(batch));
        map.insert("sampler_name".into(), JsonValue::String(sampler));
        map.insert("scheduler".into(), JsonValue::String("normal".into()));
        Ok(map)
    }

    fn vae_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        let mut map = JsonMap::new();
        map.insert(
            "vae_path".into(),
            JsonValue::String(
                self.field_text(node, "VAE Path")
                    .unwrap_or_else(|| "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors".into()),
            ),
        );
        map.insert(
            "vae_type".into(),
            JsonValue::String(
                self.field_text(node, "VAE Type")
                    .unwrap_or_else(|| "standard".into()),
            ),
        );
        map.insert("enable_tiling".into(), JsonValue::Bool(true));
        map.insert("enable_slicing".into(), JsonValue::Bool(false));
        Ok(map)
    }

    fn output_properties(&self, node: &Node) -> Result<JsonMap<String, JsonValue>, String> {
        let mut map = JsonMap::new();
        map.insert(
            "path".into(),
            JsonValue::String(
                self.field_text(node, "Output Directory")
                    .unwrap_or_else(|| "/home/alex/Eri/output".into()),
            ),
        );
        map.insert(
            "filename_pattern".into(),
            JsonValue::String(
                self.field_text(node, "Filename Pattern")
                    .unwrap_or_else(|| "img_{seed}_{timestamp}".into()),
            ),
        );
        let format = self
            .field_text(node, "Format")
            .unwrap_or_else(|| "PNG".into());
        map.insert("format".into(), JsonValue::String(format.to_uppercase()));
        map.insert("quality".into(), json!(95));
        Ok(map)
    }

    fn field_text(&self, node: &Node, label: &str) -> Option<String> {
        node.fields
            .iter()
            .find(|f| f.label == label)
            .and_then(|field| match &field.value {
                FieldValue::Text(val) => Some(val.clone()),
                FieldValue::Select(val) => Some(val.clone()),
                FieldValue::Number(val) => Some(val.to_string()),
            })
            .filter(|s| !s.is_empty())
    }

    fn field_number(&self, node: &Node, label: &str) -> Option<f32> {
        node.fields
            .iter()
            .find(|f| f.label == label)
            .and_then(|field| match &field.value {
                FieldValue::Number(val) => Some(*val),
                FieldValue::Text(val) => val.parse::<f32>().ok(),
                FieldValue::Select(_) => None,
            })
    }

    fn port_label(graph: &Graph, node_id: usize, port_id: usize, is_input: bool) -> Option<String> {
        graph
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .and_then(|node| {
                let ports = if is_input {
                    &node.inputs
                } else {
                    &node.outputs
                };
                ports
                    .iter()
                    .find(|p| p.id == port_id)
                    .map(|port| port.label.clone())
            })
    }

    fn handle_backend_message(&mut self, msg: BackendMessage) {
        match msg {
            BackendMessage::Connected(count) => {
                self.backend_connected = true;
                self.backend_status = format!("Connected · {count} components");
            }
            BackendMessage::Error(err) => {
                self.backend_connected = false;
                self.backend_status = format!("Backend error: {err}");
            }
            BackendMessage::ExecutionFinished(Ok(message)) => {
                self.backend_status = format!("Generation complete: {message}");
            }
            BackendMessage::ExecutionFinished(Err(err)) => {
                self.backend_status = format!("Generation failed: {err}");
            }
            BackendMessage::ExecutionFiles(paths) => {
                if let Some(first) = paths.first() {
                    self.backend_status = format!("Saved {}", first);
                    if let Some(display_id) = self
                        .graph
                        .graph_state(&self.widgets)
                        .and_then(|s| {
                            s.graph
                                .nodes
                                .iter()
                                .find(|n| n.component_type.as_deref() == Some("image_display"))
                                .map(|n| n.id)
                        })
                    {
                        self.graph
                            .set_image_preview(&mut self.widgets, display_id, first);
                    }
                }
            }
        }
    }

    fn draw(&self, renderer: &mut Renderer) {
        self.draw_top_bar(renderer);
        self.draw_left_panel(renderer);
        self.draw_right_panel(renderer);
        self.draw_bottom_panel(renderer);

        if let Some(graph) = self.widgets.get(self.graph.graph_id()) {
            graph.draw(renderer, &self.theme);
        }

        if let Some(card) = &self.active_drag {
            self.draw_drag_preview(renderer, card);
        }
    }

    fn draw_top_bar(&self, renderer: &mut Renderer) {
        renderer.draw_gradient_rect(
            self.top_bar_rect,
            Color::rgba(28, 24, 44, 230),
            Color::rgba(20, 18, 30, 220),
            false,
        );
        self.render_top_header(
            renderer,
            Point::new(self.top_bar_rect.x() + 24, self.top_bar_rect.y() + 14),
            26,
            16,
            self.mode.label(),
        );
        for (idx, button) in self.toolbar_buttons.iter().enumerate() {
            let mut color = Color::rgba(60, 44, 120, 200);
            if self.hovered_toolbar == Some(idx) {
                color = Color::rgba(90, 70, 160, 220);
            }
            renderer.set_color(color);
            renderer.fill_rounded_rect(button.rect, 18);
            renderer.set_color(Color::WHITE);
            let text_pos = Point::new(button.rect.x() + 16, button.rect.y() + 24);
            renderer.draw_text(button.label, text_pos, 18);
        }

        let status_rect = Rect::new(
            self.top_bar_rect.right() - 160,
            self.top_bar_rect.y() + 18,
            140,
            36,
        );
        let badge_color = if self.backend_connected {
            Color::rgba(70, 200, 120, 230)
        } else {
            Color::rgba(220, 80, 80, 230)
        };
        renderer.set_color(badge_color);
        renderer.fill_rounded_rect(status_rect, 18);
        renderer.set_color(Color::BLACK);
        renderer.draw_text(
            if self.backend_connected {
                "Connected"
            } else {
                "Offline"
            },
            Point::new(status_rect.x() + 16, status_rect.y() + 24),
            18,
        );

        renderer.set_color(Color::WHITE);
        renderer.draw_text(
            &self.backend_status,
            Point::new(self.top_bar_rect.x() + 24, self.top_bar_rect.bottom() - 12),
            16,
        );
    }


    fn render_top_header(
        &self,
        renderer: &mut Renderer,
        origin: Point,
        title_size: i32,
        subtitle_size: i32,
        subtitle: &str,
    ) {
        renderer.set_color(Color::WHITE);
        renderer.draw_text(APP_TITLE, origin, title_size);
        renderer.set_color(Color::rgba(200, 200, 220, 220));
        renderer.draw_text(
            subtitle,
            Point::new(origin.x + 4, origin.y + title_size + 4),
            subtitle_size,
        );
    }

    fn draw_right_panel(&self, renderer: &mut Renderer) {
        renderer.set_color(Color::rgba(18, 18, 32, 230));
        renderer.fill_rect(self.right_panel_rect);

        renderer.set_color(Color::rgba(80, 70, 160, 220));
        renderer.fill_rounded_rect(self.add_component_rect, 16);
        renderer.set_color(Color::WHITE);
        renderer.draw_text(
            "+ Add Component",
            Point::new(
                self.add_component_rect.x() + 16,
                self.add_component_rect.y() + 26,
            ),
            18,
        );

        for (g_idx, group) in self.component_groups.iter().enumerate() {
            renderer.set_color(Color::rgba(150, 150, 180, 240));
            renderer.draw_text(
                group.name,
                Point::new(group.header_rect.x(), group.header_rect.y() + 18),
                18,
            );
            for (c_idx, card) in group.cards.iter().enumerate() {
                let mut base = Color::rgba(34, 32, 52, 255);
                if Some((g_idx, c_idx)) == self.hovered_component {
                    base = Color::rgba(62, 56, 102, 255);
                }
                renderer.set_color(base);
                renderer.fill_rounded_rect(card.rect, CARD_RADIUS);

                let icon_rect = Rect::new(card.rect.x() + 12, card.rect.y() + 8, 36, 32);
                renderer.set_color(Color::rgba(80, 70, 130, 255));
                renderer.fill_rounded_rect(icon_rect, 12);
                renderer.set_color(Color::WHITE);
                renderer.draw_text(
                    card.icon,
                    Point::new(icon_rect.x() + 10, icon_rect.y() + 22),
                    18,
                );

                renderer.draw_text(
                    card.label,
                    Point::new(icon_rect.right() + 14, card.rect.y() + 28),
                    18,
                );
            }
        }
    }

    fn draw_left_panel(&self, renderer: &mut Renderer) {
        renderer.set_color(Color::rgba(18, 18, 32, 230));
        renderer.fill_rect(self.left_panel_rect);
        let mode_labels = ["Trainer", "Inference", "Samples"];
        for (idx, rect) in self.mode_toggle_rects.iter().enumerate() {
            let active = matches!(
                (idx, self.mode),
                (0, DesignerMode::Trainer)
                    | (1, DesignerMode::Inference)
                    | (2, DesignerMode::Samples)
            );
            let color = if active {
                Color::rgba(120, 90, 200, 230)
            } else {
                Color::rgba(60, 50, 90, 200)
            };
            renderer.set_color(color);
            renderer.fill_rounded_rect(*rect, 16);
            renderer.set_color(Color::WHITE);
            renderer.draw_text(
                mode_labels[idx],
                Point::new(rect.x() + 12, rect.y() + 22),
                18,
            );
        }
        if matches!(self.mode, DesignerMode::Trainer) {
            let labels = ["Start Training", "Stop Training", "Resume"];
            for (idx, rect) in self.training_control_rects.iter().enumerate() {
                renderer.set_color(Color::rgba(90, 70, 140, 220));
                renderer.fill_rounded_rect(*rect, 14);
                renderer.set_color(Color::WHITE);
                renderer.draw_text(
                    labels[idx],
                    Point::new(rect.x() + 10, rect.y() + 20),
                    16,
                );
            }
        }
    }

    fn draw_bottom_panel(&self, renderer: &mut Renderer) {
        renderer.set_color(Color::rgba(18, 18, 28, 240));
        renderer.fill_rect(self.bottom_panel_rect);
        renderer.set_color(Color::rgba(150, 150, 180, 240));
        renderer.draw_text(
            "Properties",
            Point::new(
                self.bottom_panel_rect.x() + 24,
                self.bottom_panel_rect.y() + 28,
            ),
            20,
        );

        if self.property_lines.is_empty() {
            renderer.set_color(Color::rgba(120, 120, 150, 220));
            renderer.draw_text(
                "Select a node to view properties",
                Point::new(
                    self.bottom_panel_rect.x() + 24,
                    self.bottom_panel_rect.y() + 64,
                ),
                18,
            );
        } else {
            let mut y = self.bottom_panel_rect.y() + 64;
            for line in &self.property_lines {
                renderer.set_color(Color::rgba(130, 130, 160, 240));
                renderer.draw_text(
                    &line.label,
                    Point::new(self.bottom_panel_rect.x() + 24, y),
                    16,
                );
                renderer.set_color(Color::WHITE);
                renderer.draw_text(
                    &line.value,
                    Point::new(self.bottom_panel_rect.x() + 180, y),
                    16,
                );
                y += 28;
            }
        }
    }

    fn draw_drag_preview(&self, renderer: &mut Renderer, card: &ComponentCard) {
        let rect = Rect::new(
            self.cursor_position.x - 60,
            self.cursor_position.y - 20,
            140,
            40,
        );
        renderer.set_color(Color::rgba(80, 80, 140, 220));
        renderer.fill_rounded_rect(rect, 18);
        renderer.set_color(Color::WHITE);
        renderer.draw_text(
            card.label,
            Point::new(rect.x() + 16, rect.y() + 26),
            18,
        );
    }
}

fn trainer_component_groups() -> Vec<ComponentGroup> {
    vec![
        ComponentGroup {
            name: "Trainer",
            header_rect: Rect::new(0, 0, 0, 0),
            cards: vec![
                ComponentCard {
                    label: "Model Specs",
                    icon: "MD",
                    component_type: "trainer_model",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Data Specs",
                    icon: "DS",
                    component_type: "trainer_data",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Training Specs",
                    icon: "TS",
                    component_type: "trainer_training",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Sampling",
                    icon: "SM",
                    component_type: "trainer_sampling",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Specialized",
                    icon: "SP",
                    component_type: "trainer_special",
                    rect: Rect::new(0, 0, 0, 0),
                },
            ],
        },
    ]
}

fn inference_component_groups() -> Vec<ComponentGroup> {
    vec![
        ComponentGroup {
            name: "Image Models",
            header_rect: Rect::new(0, 0, 0, 0),
            cards: vec![
                ComponentCard {
                    label: "SDXL Model",
                    icon: "SD",
                    component_type: "sdxl_model",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Flux Model",
                    icon: "FX",
                    component_type: "flux",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Z-Image Model",
                    icon: "ZI",
                    component_type: "z_image_model",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "HiDream-1",
                    icon: "HD",
                    component_type: "hidream_i1",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Sampler",
                    icon: "SM",
                    component_type: "sampler_v2",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Output",
                    icon: "OU",
                    component_type: "output",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "Image Display",
                    icon: "IM",
                    component_type: "image_display",
                    rect: Rect::new(0, 0, 0, 0),
                },
            ],
        },
        ComponentGroup {
            name: "Video Models",
            header_rect: Rect::new(0, 0, 0, 0),
            cards: vec![
                ComponentCard {
                    label: "WAN-VACE Video",
                    icon: "WV",
                    component_type: "wan_vace",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "AnimateDiff",
                    icon: "AD",
                    component_type: "animatediff",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "LTX Video",
                    icon: "LV",
                    component_type: "ltx_video",
                    rect: Rect::new(0, 0, 0, 0),
                },
            ],
        },
        ComponentGroup {
            name: "Display",
            header_rect: Rect::new(0, 0, 0, 0),
            cards: vec![
                ComponentCard {
                    label: "Media Display",
                    icon: "MD",
                    component_type: "media_display",
                    rect: Rect::new(0, 0, 0, 0),
                },
            ],
        },
        ComponentGroup {
            name: "Advanced",
            header_rect: Rect::new(0, 0, 0, 0),
            cards: vec![
                ComponentCard {
                    label: "Upscaler",
                    icon: "UP",
                    component_type: "upscaler",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "ControlNet",
                    icon: "CN",
                    component_type: "controlnet",
                    rect: Rect::new(0, 0, 0, 0),
                },
                ComponentCard {
                    label: "CLIP Text Encoder",
                    icon: "CL",
                    component_type: "clip_text_encoder",
                    rect: Rect::new(0, 0, 0, 0),
                },
            ],
        },
    ]
}

fn main() -> anyhow::Result<()> {
    #[cfg(all(unix, not(target_os = "macos")))]
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    env_logger::init();

    let (backend_tx, backend_rx) = mpsc::channel();
    request_backend_components(backend_tx.clone());

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::new(&event_loop, 1600, 900, "EriGui - Workflow Designer")?;
    let theme = Theme::alex_jammin();
    let mut app = App::new(theme.clone(), backend_tx.clone());
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

                renderer.begin_frame(Color::rgba(10, 10, 18, 255));
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
    let response = reqwest::blocking::get(&url).with_context(|| format!("requesting {url}"))?;
    if !response.status().is_success() {
        anyhow::bail!("backend status {}", response.status());
    }
    let text = response.text()?;
    let items = serde_json::from_str::<Vec<ApiComponentInfo>>(&text).map_err(|err| {
        let snippet = text.chars().take(200).collect::<String>();
        anyhow::anyhow!("error decoding response body: {err} · snippet: {snippet}")
    })?;
    Ok(items)
}

fn request_backend_components(tx: Sender<BackendMessage>) {
    thread::spawn(move || {
        const MAX_ATTEMPTS: usize = 10;
        const RETRY_DELAY_MS: u64 = 500;
        let mut last_err: Option<String> = None;
        for _attempt in 1..=MAX_ATTEMPTS {
            match fetch_backend_components() {
                Ok(list) => {
                    let _ = tx.send(BackendMessage::Connected(list.len()));
                    return;
                }
                Err(err) => {
                    last_err = Some(err.to_string());
                    thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }
        if let Some(err) = last_err {
            let _ = tx.send(BackendMessage::Error(format!(
                "Backend unreachable after {MAX_ATTEMPTS} attempts: {err}"
            )));
        }
    });
}

fn request_workflow_execution(tx: Sender<BackendMessage>, payload: WorkflowPayload) {
    thread::spawn(move || {
        let url = format!("{}/api/execute", BACKEND_BASE_URL);
        let result = (|| {
            let client = reqwest::blocking::Client::new();
            let response = client.post(&url).json(&payload).send()?;
            if !response.status().is_success() {
                anyhow::bail!("Backend status {}", response.status());
            }
            let json: serde_json::Value = response.json()?;
            if let Some(files) = json.get("files").and_then(|v| v.as_array()) {
                let paths: Vec<String> = files
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                Ok::<_, anyhow::Error>(serde_json::Value::Array(
                    paths.iter().map(|p| serde_json::Value::String(p.clone())).collect(),
                ))
            } else {
                Ok::<_, anyhow::Error>(json)
            }
        })();

        match result {
            Ok(val) => {
                if let Some(arr) = val.as_array() {
                    let paths: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    let _ = tx.send(BackendMessage::ExecutionFiles(paths));
                } else {
                    let _ = tx.send(BackendMessage::ExecutionFinished(Ok(format!(
                        "{val}"
                    ))));
                }
            }
            Err(err) => {
                let _ = tx.send(BackendMessage::ExecutionFinished(Err(err.to_string())));
            }
        }
    });
}
fn samples_component_groups() -> Vec<ComponentGroup> {
    vec![ComponentGroup {
        name: "Samples",
        header_rect: Rect::new(0, 0, 0, 0),
        cards: vec![
            ComponentCard {
                label: "Samples Browser",
                icon: "SB",
                component_type: "samples_view",
                rect: Rect::new(0, 0, 0, 0),
            },
            ComponentCard {
                label: "Image Display",
                icon: "IM",
                component_type: "image_display",
                rect: Rect::new(0, 0, 0, 0),
            },
        ],
    }]
}

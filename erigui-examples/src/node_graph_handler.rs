use std::fs;
use std::path::Path;

use erigui_core::{Point, Rect, Size, WidgetId};
use erigui_widgets::{
    Edge, Field, FieldKind, FieldValue, Graph, GraphState, Group, Node, NodeGraph, Port, Vec2f,
    WidgetManager,
};

pub struct GraphHandler {
    graph_id: WidgetId,
}

#[derive(Clone, Copy, Debug)]
pub enum Command {
    AddInput,
    AddOutput,
    DeleteSelected,
    FitView,
    ResetView,
    AddImageNode,
    ToggleGrid,
    ClearGraph,
    LoadSample,
}

#[derive(Clone, Copy, Debug)]
pub enum PaletteNodeKind {
    Model,
    Params,
    Vae,
    Generator,
}

fn text_field(id: usize, label: &str, default: &str) -> Field {
    Field {
        id,
        label: label.into(),
        kind: FieldKind::Text,
        value: FieldValue::Text(default.into()),
    }
}

fn number_field(
    id: usize,
    label: &str,
    default: f32,
    min: f32,
    max: f32,
    step: f32,
) -> Field {
    Field {
        id,
        label: label.into(),
        kind: FieldKind::Number { min, max, step },
        value: FieldValue::Number(default),
    }
}

fn select_field(id: usize, label: &str, options: &[&str], default: &str) -> Field {
    Field {
        id,
        label: label.into(),
        kind: FieldKind::Select {
            options: options.iter().map(|s| s.to_string()).collect(),
        },
        value: FieldValue::Select(default.into()),
    }
}

impl GraphHandler {
    pub fn attach(widgets: &mut WidgetManager) -> Self {
        let graph_id = widgets.add_widget(Box::new(NodeGraph::new(
            WidgetId::default(),
            default_graph(),
        )));

        // Demo should start without background grid lines for a cleaner look.
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(graph_id) {
            graph.set_grid_visible(false);
        }
        Self { graph_id }
    }

    pub fn graph_id(&self) -> WidgetId {
        self.graph_id
    }

    pub fn selected_node(&self, widgets: &WidgetManager) -> Option<Node> {
        widgets
            .get_typed::<NodeGraph>(self.graph_id)
            .and_then(|g| g.first_selected_node())
    }

    pub fn set_selected_title(&self, widgets: &mut WidgetManager, title: impl Into<String>) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            if let Some(sel) = graph.first_selected_node() {
                graph.set_node_title(sel.id, title.into());
            }
        }
    }

    pub fn graph_state(&self, widgets: &WidgetManager) -> Option<GraphState> {
        widgets
            .get_typed::<NodeGraph>(self.graph_id)
            .map(|graph| graph.state())
    }

    pub fn add_default_groups(&self, widgets: &mut WidgetManager, theme: &erigui_core::Theme) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.add_group(Group {
                id: 1,
                title: "Video extend".into(),
                rect: Rect::new(40, 60, 360, 420),
                color: theme.colors.primary.with_alpha(70),
            });
            graph.add_group(Group {
                id: 2,
                title: "Output".into(),
                rect: Rect::new(420, 220, 320, 260),
                color: theme.colors.secondary.with_alpha(90),
            });
        }
    }

    pub fn apply_commands(&self, widgets: &mut WidgetManager, cmds: Vec<Command>) {
        for cmd in cmds {
            match cmd {
                Command::AddInput => self.add_input(widgets),
                Command::AddOutput => self.add_output(widgets),
                Command::DeleteSelected => self.delete_selected(widgets),
                Command::FitView => self.fit_view(widgets),
                Command::ResetView => self.reset_view(widgets),
                Command::AddImageNode => self.add_image_node(widgets),
                Command::ToggleGrid => self.toggle_grid(widgets),
                Command::ClearGraph => self.clear_graph(widgets),
                Command::LoadSample => self.load_sample_graph(widgets),
            }
        }
    }

    pub fn save_to_path(
        &self,
        widgets: &WidgetManager,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        if let Some(graph) = widgets.get_typed::<NodeGraph>(self.graph_id) {
            let json = graph.save_json()?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn load_from_path(
        &self,
        widgets: &mut WidgetManager,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let data = fs::read_to_string(path)?;
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.load_json(&data)?;
        }
        Ok(())
    }

    fn add_input(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            if let Some(node) = graph.first_selected_node() {
                let label = format!("in{}", node.inputs.len());
                graph.add_input_port(node.id, label);
            }
        }
    }

    fn toggle_grid(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.toggle_grid();
        }
    }

    fn add_output(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            if let Some(node) = graph.first_selected_node() {
                let label = format!("out{}", node.outputs.len());
                graph.add_output_port(node.id, label);
            }
        }
    }

    fn delete_selected(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.delete_selected();
        }
    }

    fn fit_view(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.fit_view();
        }
    }

    fn reset_view(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.reset_view();
        }
    }

    fn clear_graph(&self, widgets: &mut WidgetManager) {
        self.replace_graph(widgets, Graph::default());
    }

    fn load_sample_graph(&self, widgets: &mut WidgetManager) {
        self.replace_graph(widgets, default_graph());
    }

    fn add_image_node(&self, widgets: &mut WidgetManager) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            let id = graph.next_node_id();
            let node = Node {
                id,
                title: format!("Image {}", id),
                position: Point::new(160, 160),
                size: Size::new(260, 320),
                inputs: vec![Port {
                    id: 0,
                    label: "mask".into(),
                    is_input: true,
                }],
                outputs: vec![Port {
                    id: 0,
                    label: "image".into(),
                    is_input: false,
                }],
                fields: vec![
                    Field {
                        id: 0,
                        label: "file".into(),
                        kind: FieldKind::FilePath {
                            extensions: vec!["png".into(), "jpg".into(), "jpeg".into()],
                        },
                        value: FieldValue::Text("choose file".into()),
                    },
                    Field {
                        id: 1,
                        label: "preview".into(),
                        kind: FieldKind::Select {
                            options: vec!["none".into(), "small".into(), "full".into()],
                        },
                        value: FieldValue::Select("none".into()),
                    },
                ],
                component_type: None,
            };
            graph.add_node(node);
        }
    }

    pub fn add_component_node(
        &self,
        widgets: &mut WidgetManager,
        component_type: &str,
        label: &str,
        position: Point,
    ) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            let id = graph.next_node_id();
            let node = Self::build_component_node(id, component_type, label, position);
            let is_model = component_type.to_lowercase().contains("model");
            graph.add_node(node);

            if is_model {
                // Auto-wire sampler, output, and image display for dropped models
                let sampler_id = graph.next_node_id();
                let sampler = Self::sampler_node(
                    sampler_id,
                    "Sampler",
                    Point::new(position.x + 280, position.y),
                    "sampler_v2",
                );
                graph.add_node(sampler);

                let output_id = graph.next_node_id();
                let output = Self::output_node(
                    output_id,
                    "Output",
                    Point::new(position.x + 560, position.y + 40),
                );
                graph.add_node(output);

                let display_id = graph.next_node_id();
                let display = Self::display_node(
                    display_id,
                    "Image Display",
                    Point::new(position.x + 820, position.y + 20),
                    "image_display",
                );
                graph.add_node(display);

                graph.graph.edges.push(Edge {
                    from_node: id,
                    from_port: 0,
                    to_node: sampler_id,
                    to_port: 0,
                });
                graph.graph.edges.push(Edge {
                    from_node: sampler_id,
                    from_port: 0,
                    to_node: output_id,
                    to_port: 0,
                });
                graph.graph.edges.push(Edge {
                    from_node: sampler_id,
                    from_port: 0,
                    to_node: display_id,
                    to_port: 0,
                });
            }
        }
    }

    pub fn add_palette_node(
        &self,
        widgets: &mut WidgetManager,
        kind: PaletteNodeKind,
        position: Point,
    ) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            let id = graph.next_node_id();
            let node = match kind {
                PaletteNodeKind::Model => Node {
                    id,
                    title: "Model Loader".into(),
                    position,
                    size: Size::new(240, 140),
                    inputs: vec![],
                    outputs: vec![
                        Port {
                            id: 0,
                            label: "pipeline".into(),
                            is_input: false,
                        },
                        Port {
                            id: 1,
                            label: "conditioning".into(),
                            is_input: false,
                        },
                        Port {
                            id: 2,
                            label: "pooled".into(),
                            is_input: false,
                        },
                    ],
                    fields: vec![
                        Field {
                            id: 0,
                            label: "Model Path".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text("model.safetensors".into()),
                        },
                        Field {
                            id: 1,
                            label: "VAE Path".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text(
                                "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors".into(),
                            ),
                        },
                    ],
                    component_type: Some("sdxl_model".into()),
                },
                PaletteNodeKind::Params => Node {
                    id,
                    title: "Sampler".into(),
                    position,
                    size: Size::new(280, 220),
                    inputs: vec![Port {
                        id: 0,
                        label: "pipeline".into(),
                        is_input: true,
                    }],
                    outputs: vec![Port {
                        id: 0,
                        label: "images".into(),
                        is_input: false,
                    }],
                    fields: vec![
                        Field {
                            id: 0,
                            label: "Prompt".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text("A cozy cabin in the snow".into()),
                        },
                        Field {
                            id: 1,
                            label: "Negative Prompt".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text("blurry, low quality".into()),
                        },
                        Field {
                            id: 2,
                            label: "Width".into(),
                            kind: FieldKind::Number {
                                min: 1.0,
                                max: 4096.0,
                                step: 64.0,
                            },
                            value: FieldValue::Number(1024.0),
                        },
                        Field {
                            id: 3,
                            label: "Height".into(),
                            kind: FieldKind::Number {
                                min: 1.0,
                                max: 4096.0,
                                step: 64.0,
                            },
                            value: FieldValue::Number(1024.0),
                        },
                        Field {
                            id: 4,
                            label: "Steps".into(),
                            kind: FieldKind::Number {
                                min: 1.0,
                                max: 200.0,
                                step: 1.0,
                            },
                            value: FieldValue::Number(20.0),
                        },
                        Field {
                            id: 5,
                            label: "CFG Scale".into(),
                            kind: FieldKind::Number {
                                min: 1.0,
                                max: 30.0,
                                step: 0.5,
                            },
                            value: FieldValue::Number(7.0),
                        },
                        Field {
                            id: 6,
                            label: "Sampler".into(),
                            kind: FieldKind::Select {
                                options: vec![
                                    "euler".into(),
                                    "ddim".into(),
                                    "dpmpp".into(),
                                    "heun".into(),
                                ],
                            },
                            value: FieldValue::Select("euler".into()),
                        },
                        Field {
                            id: 7,
                            label: "Seed".into(),
                            kind: FieldKind::Number {
                                min: -1.0,
                                max: 2_147_483_647.0,
                                step: 1.0,
                            },
                            value: FieldValue::Number(-1.0),
                        },
                        Field {
                            id: 8,
                            label: "Batch Size".into(),
                            kind: FieldKind::Number {
                                min: 1.0,
                                max: 8.0,
                                step: 1.0,
                            },
                            value: FieldValue::Number(1.0),
                        },
                    ],
                    component_type: Some("sampler_v2".into()),
                },
                PaletteNodeKind::Vae => Node {
                    id,
                    title: "VAE Loader".into(),
                    position,
                    size: Size::new(230, 120),
                    inputs: vec![],
                    outputs: vec![Port {
                        id: 0,
                        label: "pipeline".into(),
                        is_input: false,
                    }],
                    fields: vec![
                        Field {
                            id: 0,
                            label: "VAE Path".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text(
                                "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors".into(),
                            ),
                        },
                        Field {
                            id: 1,
                            label: "VAE Type".into(),
                            kind: FieldKind::Select {
                                options: vec!["standard".into(), "ema".into(), "mse".into()],
                            },
                            value: FieldValue::Select("standard".into()),
                        },
                    ],
                    component_type: Some("vae_model".into()),
                },
                PaletteNodeKind::Generator => Node {
                    id,
                    title: "Output".into(),
                    position,
                    size: Size::new(260, 160),
                    inputs: vec![Port {
                        id: 0,
                        label: "images".into(),
                        is_input: true,
                    }],
                    outputs: vec![Port {
                        id: 0,
                        label: "image".into(),
                        is_input: false,
                    }],
                    fields: vec![
                        Field {
                            id: 0,
                            label: "Output Directory".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text("/home/alex/Eri/output".into()),
                        },
                        Field {
                            id: 1,
                            label: "Filename Pattern".into(),
                            kind: FieldKind::Text,
                            value: FieldValue::Text("img_{seed}_{timestamp}".into()),
                        },
                        Field {
                            id: 2,
                            label: "Format".into(),
                            kind: FieldKind::Select {
                                options: vec!["PNG".into(), "JPEG".into(), "WEBP".into()],
                            },
                            value: FieldValue::Select("PNG".into()),
                        },
                    ],
                    component_type: Some("output".into()),
                },
            };
            graph.add_node(node);
        }
    }

    fn build_component_node(
        id: usize,
        component_type: &str,
        label: &str,
        position: Point,
    ) -> Node {
        let normalized = component_type.to_lowercase();
        match normalized.as_str() {
            "sdxl_model" | "sdxl" => Self::model_node(
                id,
                label,
                position,
                component_type,
                "/home/alex/SwarmUI/Models/Stable-Diffusion/OfficialStableDiffusion/sd_xl_base_1.0.safetensors",
            ),
            "flux" | "flux_model" => Self::model_node(
                id,
                label,
                position,
                "flux",
                "/home/alex/SwarmUI/Models/Flux/flux1-dev-fp16.safetensors",
            ),
            "z_image_model" | "z_image" => Self::model_node(
                id,
                label,
                position,
                "z_image_model",
                "Tongyi-MAI/Z-Image-Turbo",
            ),
            "hidream_i1" | "hidream" => Self::model_node(
                id,
                label,
                position,
                "hidream_i1",
                "/home/alex/SwarmUI/Models/HiDream/HiDream1.safetensors",
            ),
            "omnigen" => Self::model_node(
                id,
                label,
                position,
                "omnigen",
                "/home/alex/SwarmUI/Models/OmniGen/omnigen_base.safetensors",
            ),
            "sd35" | "sd3.5" => Self::model_node(
                id,
                label,
                position,
                "sd35",
                "/home/alex/SwarmUI/Models/Stable-Diffusion/SD3.5/sd3.5.safetensors",
            ),
            "lumina" => Self::model_node(
                id,
                label,
                position,
                "lumina",
                "/home/alex/SwarmUI/Models/Lumina/lumina.safetensors",
            ),
            "sampler_v2" | "sampler" => Self::sampler_node(id, label, position, component_type),
            "output" => Self::output_node(id, label, position),
            "wan_vace" | "wanvace" => Self::video_node(id, label, position, component_type),
            "ltx_video" | "ltxvideo" => Self::video_node(id, label, position, component_type),
            "animatediff" => Self::video_node(id, label, position, component_type),
            "image_display" => Self::display_node(id, label, position, component_type),
            "media_display" => Self::display_node(id, label, position, component_type),
            "upscaler" => Self::upscaler_node(id, label, position),
            "model_training" => Self::training_node(id, label, position),
            "controlnet" | "control_net" => Self::controlnet_node(id, label, position),
            "clip_text_encoder" | "clip" => Self::clip_node(id, label, position),
            "trainer_model" => Self::trainer_model_node(id, label, position),
            "trainer_data" => Self::trainer_data_node(id, label, position),
            "trainer_training" => Self::trainer_training_node(id, label, position),
            "trainer_sampling" => Self::trainer_sampling_node(id, label, position),
            "trainer_special" => Self::trainer_special_node(id, label, position),
            _ => Self::placeholder_node(id, label, position, component_type),
        }
    }

    fn model_node(
        id: usize,
        label: &str,
        position: Point,
        component_type: &str,
        checkpoint: &str,
    ) -> Node {
        let lower = component_type.to_lowercase();
        let is_z_image = lower.contains("z_image");
        let default_checkpoint = if is_z_image {
            "Tongyi-MAI/Z-Image-Turbo"
        } else {
            checkpoint
        };
        let default_vae = if is_z_image {
            "/home/alex/SwarmUI/Models/VAE/ae.safetensors"
        } else {
            "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors"
        };
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(260, 190),
            inputs: vec![],
            outputs: vec![
                Port {
                    id: 0,
                    label: "pipeline".into(),
                    is_input: false,
                },
                Port {
                    id: 1,
                    label: "conditioning".into(),
                    is_input: false,
                },
                Port {
                    id: 2,
                    label: "pooled".into(),
                    is_input: false,
                },
            ],
            fields: vec![
                text_field(0, "Model Path", default_checkpoint),
                text_field(
                    1,
                    "VAE Path",
                    default_vae,
                ),
                text_field(2, "LoRA Paths", ""),
                number_field(3, "LoRA Scale", 1.0, 0.0, 4.0, 0.1),
            ],
            component_type: Some(component_type.into()),
        }
    }

    fn sampler_node(id: usize, label: &str, position: Point, component_type: &str) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(280, 220),
            inputs: vec![Port {
                id: 0,
                label: "pipeline".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "images".into(),
                is_input: false,
            }],
            fields: vec![
                text_field(0, "Prompt", "A cozy cabin in the snow"),
                text_field(1, "Negative Prompt", "blurry, low quality"),
                number_field(2, "Width", 1024.0, 64.0, 4096.0, 64.0),
                number_field(3, "Height", 1024.0, 64.0, 4096.0, 64.0),
                number_field(4, "Steps", 20.0, 1.0, 200.0, 1.0),
                number_field(5, "CFG Scale", 7.0, 1.0, 30.0, 0.5),
                select_field(6, "Sampler", &["euler", "ddim", "dpmpp", "heun"], "euler"),
                number_field(7, "Seed", -1.0, -1.0, 2_147_483_647.0, 1.0),
                number_field(8, "Batch Size", 1.0, 1.0, 8.0, 1.0),
            ],
            component_type: Some(component_type.into()),
        }
    }

    fn output_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(260, 160),
            inputs: vec![Port {
                id: 0,
                label: "images".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "image".into(),
                is_input: false,
            }],
            fields: vec![
                text_field(0, "Output Directory", "/home/alex/Eri/output"),
                text_field(1, "Filename Pattern", "img_{seed}_{timestamp}"),
                select_field(2, "Format", &["PNG", "JPEG", "WEBP"], "PNG"),
            ],
            component_type: Some("output".into()),
        }
    }

    fn video_node(id: usize, label: &str, position: Point, component_type: &str) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(280, 200),
            inputs: vec![Port {
                id: 0,
                label: "pipeline".into(),
                is_input: true,
            }],
            outputs: vec![
                Port {
                    id: 0,
                    label: "video".into(),
                    is_input: false,
                },
                Port {
                    id: 1,
                    label: "frames".into(),
                    is_input: false,
                },
            ],
            fields: vec![
                text_field(0, "Prompt", "Describe the motion you want"),
                number_field(1, "Frames", 16.0, 4.0, 240.0, 4.0),
                number_field(2, "FPS", 12.0, 1.0, 60.0, 1.0),
            ],
            component_type: Some(component_type.into()),
        }
    }

    fn display_node(id: usize, label: &str, position: Point, component_type: &str) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(220, 140),
            inputs: vec![Port {
                id: 0,
                label: "images".into(),
                is_input: true,
            }],
            outputs: vec![],
            fields: vec![select_field(
                0,
                "Mode",
                &["thumbnail", "full"],
                "thumbnail",
            )],
            component_type: Some(component_type.into()),
        }
    }

    fn upscaler_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(260, 160),
            inputs: vec![Port {
                id: 0,
                label: "image".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "image".into(),
                is_input: false,
            }],
            fields: vec![
                select_field(0, "Model", &["4x-UltraSharp", "RealESRGAN"], "4x-UltraSharp"),
                number_field(1, "Scale", 4.0, 1.0, 8.0, 1.0),
            ],
            component_type: Some("upscaler".into()),
        }
    }

    fn training_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(300, 200),
            inputs: vec![Port {
                id: 0,
                label: "dataset".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "lora".into(),
                is_input: false,
            }],
            fields: vec![
                text_field(0, "Dataset Path", "/home/alex/datasets/sample"),
                number_field(1, "Epochs", 10.0, 1.0, 10_000.0, 1.0),
                number_field(2, "Learning Rate", 1e-4, 1e-6, 1e-2, 1e-5),
            ],
            component_type: Some("model_training".into()),
        }
    }

    fn controlnet_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(280, 200),
            inputs: vec![
                Port {
                    id: 0,
                    label: "pipeline".into(),
                    is_input: true,
                },
                Port {
                    id: 1,
                    label: "control image".into(),
                    is_input: true,
                },
            ],
            outputs: vec![Port {
                id: 0,
                label: "pipeline".into(),
                is_input: false,
            }],
            fields: vec![
                select_field(
                    0,
                    "Preprocessor",
                    &["canny", "openpose", "depth"],
                    "canny",
                ),
                number_field(1, "Weight", 1.0, 0.0, 2.0, 0.1),
            ],
            component_type: Some("controlnet".into()),
        }
    }

    fn clip_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(260, 160),
            inputs: vec![Port {
                id: 0,
                label: "text".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "conditioning".into(),
                is_input: false,
            }],
            fields: vec![text_field(0, "Prompt", "Describe the scene")],
            component_type: Some("clip_text_encoder".into()),
        }
    }

    fn placeholder_node(
        id: usize,
        label: &str,
        position: Point,
        component_type: &str,
    ) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(240, 140),
            inputs: vec![],
            outputs: vec![],
            fields: vec![text_field(
                0,
                "Notes",
                "Component not yet implemented",
            )],
            component_type: Some(component_type.into()),
        }
    }

    fn trainer_model_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(320, 360),
            inputs: vec![],
            outputs: vec![Port {
                id: 0,
                label: "model".into(),
                is_input: false,
            }],
            fields: vec![
                text_field(0, "Name or Path", "Tongyi-MAI/Z-Image-Turbo"),
                text_field(1, "VAE Path", "/home/alex/SwarmUI/Models/VAE/ae.safetensors"),
                text_field(2, "CLIP-L Path", "/home/alex/SwarmUI/Models/clip/clip_l.safetensors"),
                text_field(3, "T5 Path", "/home/alex/SwarmUI/Models/clip/t5xxl_fp16.safetensors"),
                select_field(4, "Network Type", &["lora", "lokr"], "lora"),
                number_field(5, "Linear", 16.0, 1.0, 128.0, 1.0),
                number_field(6, "Linear Alpha", 16.0, 1.0, 128.0, 1.0),
                select_field(7, "Is Flux", &["true", "false"], "false"),
                select_field(8, "Quantize", &["false", "true"], "false"),
            ],
            component_type: Some("trainer_model".into()),
        }
    }

    fn trainer_data_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(320, 360),
            inputs: vec![Port {
                id: 0,
                label: "model".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "data".into(),
                is_input: false,
            }],
            fields: vec![
                text_field(0, "Dataset Config", "examples/dataset.toml"),
                text_field(1, "Folder Path", "/home/anon/data/images/grayscale"),
                select_field(2, "Caption Ext", &["txt", "json"], "txt"),
                number_field(3, "Caption Dropout", 0.0, 0.0, 1.0, 0.05),
                select_field(4, "Shuffle Tokens", &["false", "true"], "false"),
                select_field(5, "Cache Latents", &["true", "false"], "true"),
                text_field(6, "Resolution", "512"),
                select_field(7, "Force Recache", &["false", "true"], "false"),
                text_field(8, "Data Root", "/home/anon/data/images/grayscale"),
            ],
            component_type: Some("trainer_data".into()),
        }
    }

    fn trainer_training_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(360, 420),
            inputs: vec![Port {
                id: 0,
                label: "data".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "train_spec".into(),
                is_input: false,
            }],
            fields: vec![
                number_field(0, "Batch Size", 1.0, 1.0, 32.0, 1.0),
                number_field(1, "Steps", 100.0, 1.0, 1_000_000.0, 10.0),
                number_field(2, "LR", 0.0001, 0.0000001, 0.01, 0.00001),
                number_field(3, "Grad Accum", 1.0, 1.0, 64.0, 1.0),
                select_field(4, "Train Backbone", &["true", "false"], "true"),
                select_field(5, "Train Text Encoder", &["false", "true"], "false"),
                select_field(6, "Gradient Checkpointing", &["true", "false"], "true"),
                select_field(7, "Noise Scheduler", &["flowmatch", "ddpm"], "flowmatch"),
                select_field(8, "Optimizer", &["adamw8bit", "adamw"], "adamw8bit"),
                select_field(9, "Mixed Precision", &["true", "false"], "true"),
                select_field(10, "Use Layer Streaming", &["true", "false"], "true"),
                number_field(11, "Streaming Memory GB", 40.0, 0.0, 80.0, 1.0),
                select_field(12, "Skip First Sample", &["true", "false"], "true"),
                select_field(13, "Disable Sampling", &["true", "false"], "true"),
                select_field(14, "Linear Timesteps", &["true", "false"], "true"),
                select_field(15, "Bypass Guidance Embedding", &["true", "false"], "true"),
                number_field(16, "Min SNR Gamma", 5.0, 0.0, 10.0, 0.5),
                select_field(17, "EMA Enabled", &["false", "true"], "false"),
                number_field(18, "EMA Decay", 0.99, 0.0, 0.9999, 0.01),
                select_field(19, "Checkpoint Policy", &["recompute", "full"], "recompute"),
            ],
            component_type: Some("trainer_training".into()),
        }
    }

    fn trainer_sampling_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(320, 320),
            inputs: vec![Port {
                id: 0,
                label: "train_spec".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "sampling".into(),
                is_input: false,
            }],
            fields: vec![
                select_field(0, "Enable Sampling", &["true", "false"], "false"),
                text_field(1, "Sampler", "flowmatch"),
                number_field(2, "Sample Every", 1000.0, 1.0, 1_000_000.0, 10.0),
                number_field(3, "Width", 1024.0, 64.0, 2048.0, 64.0),
                number_field(4, "Height", 1024.0, 64.0, 2048.0, 64.0),
                number_field(5, "Seed", 42.0, -1_000_000.0, 1_000_000.0, 1.0),
                number_field(6, "Guidance Scale", 7.5, 0.0, 20.0, 0.1),
                number_field(7, "Sample Steps", 20.0, 1.0, 200.0, 1.0),
                text_field(8, "Prompts", "test"),
                text_field(9, "Negative Prompt", ""),
                select_field(10, "Walk Seed", &["true", "false"], "true"),
            ],
            component_type: Some("trainer_sampling".into()),
        }
    }

    fn trainer_special_node(id: usize, label: &str, position: Point) -> Node {
        Node {
            id,
            title: label.into(),
            position,
            size: Size::new(360, 360),
            inputs: vec![Port {
                id: 0,
                label: "sampling".into(),
                is_input: true,
            }],
            outputs: vec![Port {
                id: 0,
                label: "trainer".into(),
                is_input: false,
            }],
            fields: vec![
                select_field(0, "Flux Cache Enabled", &["true", "false"], "true"),
                text_field(1, "Cache Dir", "/home/alex/EriDiffusion/cache_v2/latents"),
                select_field(2, "Cache Scope", &["persistent", "ephemeral"], "persistent"),
                text_field(3, "Cache Capacity", "16GiB"),
                number_field(4, "Warm Latents", 32.0, 0.0, 1024.0, 1.0),
                number_field(5, "Stats Every", 100.0, 1.0, 10_000.0, 10.0),
                select_field(6, "Force Recache", &["false", "true"], "false"),
                number_field(7, "Performance Log Every", 1.0, 1.0, 1000.0, 1.0),
                text_field(8, "Trigger Word", "woman"),
                select_field(9, "Device", &["cuda:0", "cuda:1", "cpu"], "cuda:0"),
            ],
            component_type: Some("trainer_special".into()),
        }
    }

    pub fn replace_graph(&self, widgets: &mut WidgetManager, graph_data: Graph) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.set_state(GraphState {
                graph: graph_data,
                pan: Vec2f::ZERO,
                zoom: 1.0,
                groups: Vec::new(),
            });
        }
    }

    pub fn set_image_preview(&self, widgets: &mut WidgetManager, node_id: usize, path: &str) {
        if let Some(graph) = widgets.get_typed_mut::<NodeGraph>(self.graph_id) {
            graph.refresh_preview_for(node_id, path);
        }
    }
}

pub fn default_graph() -> Graph {
    let model_node = Node {
        id: 1,
        title: "Model Loader".into(),
        position: Point::new(260, 190),
        size: Size::new(240, 140),
        inputs: vec![],
        outputs: vec![
            Port {
                id: 0,
                label: "pipeline".into(),
                is_input: false,
            },
            Port {
                id: 1,
                label: "conditioning".into(),
                is_input: false,
            },
            Port {
                id: 2,
                label: "pooled".into(),
                is_input: false,
            },
        ],
        fields: vec![
            Field {
                id: 0,
                label: "Model Path".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text(
                    "/home/alex/SwarmUI/Models/Stable-Diffusion/OfficialStableDiffusion/sd_xl_base_1.0.safetensors"
                        .into(),
                ),
            },
            Field {
                id: 1,
                label: "VAE Path".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text(
                    "/home/alex/SwarmUI/Models/VAE/sdxl_vae.safetensors".into(),
                ),
            },
        ],
        component_type: Some("sdxl_model".into()),
    };

    let sampler_node = Node {
        id: 2,
        title: "Sampler".into(),
        position: Point::new(540, 190),
        size: Size::new(280, 220),
        inputs: vec![Port {
            id: 0,
            label: "pipeline".into(),
            is_input: true,
        }],
        outputs: vec![Port {
            id: 0,
            label: "images".into(),
            is_input: false,
        }],
        fields: vec![
            Field {
                id: 0,
                label: "Prompt".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("A cozy cabin in the snow".into()),
            },
            Field {
                id: 1,
                label: "Negative Prompt".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("blurry, low quality".into()),
            },
            Field {
                id: 2,
                label: "Width".into(),
                kind: FieldKind::Number {
                    min: 1.0,
                    max: 4096.0,
                    step: 64.0,
                },
                value: FieldValue::Number(1024.0),
            },
            Field {
                id: 3,
                label: "Height".into(),
                kind: FieldKind::Number {
                    min: 1.0,
                    max: 4096.0,
                    step: 64.0,
                },
                value: FieldValue::Number(1024.0),
            },
            Field {
                id: 4,
                label: "Steps".into(),
                kind: FieldKind::Number {
                    min: 1.0,
                    max: 200.0,
                    step: 1.0,
                },
                value: FieldValue::Number(20.0),
            },
            Field {
                id: 5,
                label: "CFG Scale".into(),
                kind: FieldKind::Number {
                    min: 1.0,
                    max: 30.0,
                    step: 0.5,
                },
                value: FieldValue::Number(7.0),
            },
            Field {
                id: 6,
                label: "Sampler".into(),
                kind: FieldKind::Select {
                    options: vec!["euler".into(), "ddim".into(), "dpmpp".into(), "heun".into()],
                },
                value: FieldValue::Select("euler".into()),
            },
            Field {
                id: 7,
                label: "Seed".into(),
                kind: FieldKind::Number {
                    min: -1.0,
                    max: 2_147_483_647.0,
                    step: 1.0,
                },
                value: FieldValue::Number(-1.0),
            },
            Field {
                id: 8,
                label: "Batch Size".into(),
                kind: FieldKind::Number {
                    min: 1.0,
                    max: 8.0,
                    step: 1.0,
                },
                value: FieldValue::Number(1.0),
            },
        ],
        component_type: Some("sampler_v2".into()),
    };

    let output_node = Node {
        id: 3,
        title: "Output".into(),
        position: Point::new(840, 210),
        size: Size::new(260, 160),
        inputs: vec![Port {
            id: 0,
            label: "images".into(),
            is_input: true,
        }],
        outputs: vec![Port {
            id: 0,
            label: "images".into(),
            is_input: false,
        }],
        fields: vec![
            Field {
                id: 0,
                label: "Output Directory".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("/home/alex/Eri/output".into()),
            },
            Field {
                id: 1,
                label: "Filename Pattern".into(),
                kind: FieldKind::Text,
                value: FieldValue::Text("img_{seed}_{timestamp}".into()),
            },
            Field {
                id: 2,
                label: "Format".into(),
                kind: FieldKind::Select {
                    options: vec!["PNG".into(), "JPEG".into(), "WEBP".into()],
                },
                value: FieldValue::Select("PNG".into()),
            },
        ],
        component_type: Some("output".into()),
    };

    let preview_node = Node {
        id: 4,
        title: "Image Display".into(),
        position: Point::new(980, 220),
        size: Size::new(960, 820),
        inputs: vec![Port {
            id: 0,
            label: "images".into(),
            is_input: true,
        }],
        outputs: vec![],
        fields: vec![select_field(0, "Mode", &["thumbnail", "full"], "thumbnail")],
        component_type: Some("image_display".into()),
    };

    Graph {
        nodes: vec![model_node, sampler_node, output_node, preview_node],
        edges: vec![
            Edge {
                from_node: 1,
                from_port: 0,
                to_node: 2,
                to_port: 0,
            },
            Edge {
                from_node: 2,
                from_port: 0,
                to_node: 3,
                to_port: 0,
            },
            Edge {
                from_node: 2,
                from_port: 0,
                to_node: 4,
                to_port: 0,
            },
        ],
    }
}

pub fn trainer_graph() -> Graph {
    let nodes = vec![
        GraphHandler::trainer_model_node(1, "Model Specs", Point::new(260, 200)),
        GraphHandler::trainer_data_node(2, "Data Specs", Point::new(560, 200)),
        GraphHandler::trainer_training_node(3, "Training Specs", Point::new(880, 200)),
        GraphHandler::trainer_sampling_node(4, "Sampling", Point::new(1220, 200)),
        GraphHandler::trainer_special_node(5, "Specialized", Point::new(1540, 200)),
    ];
    Graph {
        nodes,
        edges: vec![
            Edge {
                from_node: 1,
                from_port: 0,
                to_node: 2,
                to_port: 0,
            },
            Edge {
                from_node: 2,
                from_port: 0,
                to_node: 3,
                to_port: 0,
            },
            Edge {
                from_node: 3,
                from_port: 0,
                to_node: 4,
                to_port: 0,
            },
            Edge {
                from_node: 4,
                from_port: 0,
                to_node: 5,
                to_port: 0,
            },
        ],
    }
}

pub fn samples_graph() -> Graph {
    let nodes = vec![
        Node {
            id: 1,
            title: "Samples Browser".into(),
            position: Point::new(260, 220),
            size: Size::new(360, 280),
            inputs: vec![],
            outputs: vec![],
            fields: vec![
                text_field(0, "Samples Folder", "/home/alex/Eri/backend/output"),
                select_field(1, "View Mode", &["grid", "list"], "grid"),
                number_field(2, "Refresh Seconds", 5.0, 1.0, 60.0, 1.0),
            ],
            component_type: Some("samples_view".into()),
        },
        Node {
            id: 2,
            title: "Image Display".into(),
            position: Point::new(640, 220),
            size: Size::new(320, 320),
            inputs: vec![Port {
                id: 0,
                label: "images".into(),
                is_input: true,
            }],
            outputs: vec![],
            fields: vec![select_field(0, "Mode", &["thumbnail", "full"], "thumbnail")],
            component_type: Some("image_display".into()),
        },
    ];
    Graph {
        nodes,
        edges: Vec::new(),
    }
}

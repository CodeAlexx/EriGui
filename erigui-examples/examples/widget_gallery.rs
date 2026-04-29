use erigui_core::{
    Color, DrawContext, LayoutConfig, LayoutMode, Margins, Point, Rect, Size, Theme, ThemeManager,
    Widget,
};
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::{
    Button, Container, Label, ListItem, ListView, TextAlign, TextInput, TreeNode,
    TreeView, WidgetId, WidgetManager,
};
use std::collections::HashMap;

// Container is a marker widget; it doesn't compute child layout. The
// host stores per-container LayoutConfigs in this sidecar map and uses
// them in `perform_container_layout` to position children.
struct App {
    widget_manager: WidgetManager,
    theme_manager: ThemeManager,
    root_container: WidgetId,
    layouts: HashMap<WidgetId, LayoutConfig>,
}

impl App {
    fn new() -> Self {
        let mut widget_manager = WidgetManager::new();
        let theme_manager = ThemeManager::new();
        let mut layouts: HashMap<WidgetId, LayoutConfig> = HashMap::new();

        // Create root container
        let root_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            root_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 16,
                padding: Margins::all(16),
                ..Default::default()
            },
        );

        // Title
        let title_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "EriGui Widget Gallery").with_align(TextAlign::Center),
        ));

        // Buttons section
        let button_section = Self::create_button_section(&mut widget_manager, &mut layouts);

        // Input section
        let input_section = Self::create_input_section(&mut widget_manager, &mut layouts);

        // List section
        let list_section = Self::create_list_section(&mut widget_manager, &mut layouts);

        // Tree section
        let tree_section = Self::create_tree_section(&mut widget_manager, &mut layouts);

        // Add all sections to root. Container is a marker now -- it just
        // tracks ids; the host walker positions them.
        let root = widget_manager.get_typed_mut::<Container>(root_id).unwrap();
        root.add_child(title_id);
        root.add_child(button_section);
        root.add_child(input_section);
        root.add_child(list_section);
        root.add_child(tree_section);

        Self {
            widget_manager,
            theme_manager,
            root_container: root_id,
            layouts,
        }
    }

    fn create_button_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Horizontal,
                spacing: 8,
                ..Default::default()
            },
        );

        // Create buttons
        let button1_id = widget_manager.add_widget(Box::new(
            Button::new(WidgetId::default(), "Click Me")
                .with_on_click(|| println!("Button 1 clicked!")),
        ));

        let button2_id =
            widget_manager.add_widget(Box::new(Button::new(WidgetId::default(), "Disabled")));
        widget_manager
            .get_mut(button2_id)
            .unwrap()
            .set_enabled(false);

        let button3_id =
            widget_manager.add_widget(Box::new(Button::new(WidgetId::default(), "Toggle Theme")));

        // Add buttons to container
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(button1_id);
        container.add_child(button2_id);
        container.add_child(button3_id);

        container_id
    }

    fn create_input_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Horizontal,
                spacing: 8,
                ..Default::default()
            },
        );

        let label_id =
            widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "Text Input:")));

        let input_id = widget_manager.add_widget(Box::new(
            TextInput::new(WidgetId::default())
                .with_placeholder("Type something...")
                .with_on_change(|text| println!("Text changed: {}", text)),
        ));

        // Add to container
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(input_id);

        container_id
    }

    fn create_list_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 8,
                ..Default::default()
            },
        );

        let label_id =
            widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "List View:")));

        // Create list items
        let items = vec![
            ListItem {
                id: "item1".to_string(),
                text: "Item 1".to_string(),
                icon: None,
                selected: true,
            },
            ListItem {
                id: "item2".to_string(),
                text: "Item 2".to_string(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item3".to_string(),
                text: "Item 3".to_string(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item4".to_string(),
                text: "Item 4".to_string(),
                icon: None,
                selected: false,
            },
            ListItem {
                id: "item5".to_string(),
                text: "Item 5".to_string(),
                icon: None,
                selected: false,
            },
        ];

        let list_id = widget_manager.add_widget(Box::new(
            ListView::new(WidgetId::default())
                .with_items(items)
                .with_on_selection_change(|index| println!("Selected item: {:?}", index)),
        ));

        // Add list directly to container (ScrollView was a Mojo-port stub
        // that's been removed; ListView now scrolls itself).
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(list_id);

        container_id
    }

    fn create_tree_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 8,
                ..Default::default()
            },
        );

        let label_id =
            widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "Tree View:")));

        // Create tree structure
        let root_node = TreeNode {
            id: "root".to_string(),
            text: "Root".to_string(),
            icon: None,
            expanded: true,
            selected: false,
            children: vec![
                TreeNode {
                    id: "branch1".to_string(),
                    text: "Branch 1".to_string(),
                    icon: None,
                    expanded: true,
                    selected: false,
                    children: vec![
                        TreeNode {
                            id: "leaf1.1".to_string(),
                            text: "Leaf 1.1".to_string(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                        TreeNode {
                            id: "leaf1.2".to_string(),
                            text: "Leaf 1.2".to_string(),
                            icon: None,
                            expanded: false,
                            selected: false,
                            children: vec![],
                        },
                    ],
                },
                TreeNode {
                    id: "branch2".to_string(),
                    text: "Branch 2".to_string(),
                    icon: None,
                    expanded: false,
                    selected: false,
                    children: vec![TreeNode {
                        id: "leaf2.1".to_string(),
                        text: "Leaf 2.1".to_string(),
                        icon: None,
                        expanded: false,
                        selected: false,
                        children: vec![],
                    }],
                },
            ],
        };

        let tree_id = widget_manager.add_widget(Box::new(
            TreeView::new(WidgetId::default())
                .with_root_nodes(vec![root_node])
                .with_on_selection_change(|path| println!("Selected path: {}", path)),
        ));

        // Add tree directly to container (ScrollView removed; tree-view's
        // own scroll handling, when added, would replace the wrapper anyway).
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(tree_id);

        container_id
    }

    fn layout_widgets(&mut self, size: Size) {
        // Layout root container
        let _theme_name = self.theme_manager.current().name.clone();

        if let Some(root) = self.widget_manager.get_mut(self.root_container) {
            let theme = Theme::light(); // Use a fresh theme instance
            root.layout(Rect::from_origin_size(Point::ZERO, size), &theme);
        }

        // Recursively layout children
        let theme = Theme::light(); // Use a fresh theme instance
        self.layout_children(self.root_container, &theme);
    }

    fn layout_children(&mut self, parent_id: WidgetId, theme: &Theme) {
        // If parent is a Container with a recorded layout config, run the
        // host-owned layout walker for it. Container itself does NOT
        // compute child bounds.
        if let Some(container) = self.widget_manager.get_typed::<Container>(parent_id) {
            if let Some(layout_info) = self.layouts.get(&parent_id).cloned() {
                let children = container.children().to_vec();
                let bounds = container.bounds();
                self.perform_container_layout(parent_id, &layout_info, &children, bounds, theme);
            }
        }

        // Get children IDs to recurse
        let children: Vec<WidgetId> = if let Some(parent) = self.widget_manager.get(parent_id) {
            parent.children().to_vec()
        } else {
            return;
        };

        // Recursively layout grandchildren
        for &child_id in &children {
            self.layout_children(child_id, theme);
        }
    }

    fn perform_container_layout(
        &mut self,
        _container_id: WidgetId,
        layout: &LayoutConfig,
        children: &[WidgetId],
        bounds: Rect,
        theme: &Theme,
    ) {
        let content_rect = bounds.inset(layout.padding.left);
        let spacing = layout.spacing;

        match layout.mode {
            LayoutMode::Vertical => {
                let mut y = content_rect.y();
                let child_height = if children.is_empty() {
                    0
                } else {
                    (content_rect.height() - spacing * (children.len() as i32 - 1))
                        / children.len() as i32
                };

                for &child_id in children {
                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        child.layout(
                            Rect::new(content_rect.x(), y, content_rect.width(), child_height),
                            theme,
                        );
                        y += child_height + spacing;
                    }
                }
            }

            LayoutMode::Horizontal => {
                let mut x = content_rect.x();
                let child_width = if children.is_empty() {
                    0
                } else {
                    (content_rect.width() - spacing * (children.len() as i32 - 1))
                        / children.len() as i32
                };

                for &child_id in children {
                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        child.layout(
                            Rect::new(x, content_rect.y(), child_width, content_rect.height()),
                            theme,
                        );
                        x += child_width + spacing;
                    }
                }
            }

            _ => {}
        }
    }

    fn draw_widget(&self, widget_id: WidgetId, renderer: &mut Renderer) {
        let theme = Theme::light(); // Use a fresh theme instance

        if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.draw(renderer, &theme);

            // Draw children
            for &child_id in widget.children() {
                self.draw_widget(child_id, renderer);
            }
        }
    }

    fn handle_event(&mut self, event: &erigui_core::Event) {
        // Simple event routing - in a real implementation, this would be more sophisticated
        let root_container = self.root_container;
        let theme = Theme::light(); // Use a fresh theme instance
        self.route_event(root_container, event, &theme);
    }

    fn route_event(
        &mut self,
        widget_id: WidgetId,
        event: &erigui_core::Event,
        theme: &Theme,
    ) -> erigui_core::EventResult {
        // First try children (top to bottom)
        let children: Vec<WidgetId> = if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.children().to_vec()
        } else {
            return erigui_core::EventResult::Ignored;
        };

        // Process children in reverse order (top-most first)
        for &child_id in children.iter().rev() {
            if self.route_event(child_id, event, theme).is_consumed() {
                return erigui_core::EventResult::Consumed;
            }
        }

        // Then try the widget itself
        if let Some(widget) = self.widget_manager.get_mut(widget_id) {
            widget.handle_event(event, theme)
        } else {
            erigui_core::EventResult::Ignored
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut renderer = Renderer::new(&event_loop, 800, 600, "EriGui - Widget Gallery")?;
    let mut app = App::new();

    // Initial layout
    app.layout_widgets(Size::new(800, 600));

    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;

    // Persistent translator so MouseInput clicks see the cached
    // last_cursor — see deprecation note on convert_window_event and
    // kitchen_sink commit a59a7f3.
    let mut translator = EventTranslator::new(renderer.viewport_size());

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        let new_size = Size::new(
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
                        translator.set_window_size(new_size);
                        app.layout_widgets(new_size);
                    }
                    _ => {}
                }

                // Convert and handle widget events
                if let Some(gui_event) = translator.translate(&event) {
                    app.handle_event(&gui_event);
                }
            }
            Event::AboutToWait => {
                renderer.begin_frame(Color::rgb(30, 30, 30));

                // Draw debug rect to verify rendering works
                renderer.set_color(Color::RED);
                renderer.fill_rect(Rect::new(10, 10, 100, 50));
                renderer.set_color(Color::WHITE);
                renderer.draw_text("Debug Test", Point::new(20, 30), 16);

                // Draw all widgets
                app.draw_widget(app.root_container, &mut renderer);

                renderer.end_frame();
            }
            _ => {}
        }
    }).unwrap();
    Ok(())
}

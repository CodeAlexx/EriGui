use erigui_core::{Color, LayoutConfig, LayoutMode, Margins, Point, Rect, Size, Theme, Widget};
use erigui_rendering::{EventTranslator, Renderer};
use erigui_widgets::{Container, Label, MenuBar, MenuItem, TextAlign, WidgetId, WidgetManager};
use std::collections::HashMap;

// Container is a marker widget; it doesn't compute child layout. The
// host stores per-container LayoutConfigs in this sidecar map and uses
// them in `perform_container_layout` to position children.
struct App {
    widget_manager: WidgetManager,
    root_container: WidgetId,
    layouts: HashMap<WidgetId, LayoutConfig>,
}

impl App {
    fn new() -> Self {
        let mut widget_manager = WidgetManager::new();
        let mut layouts: HashMap<WidgetId, LayoutConfig> = HashMap::new();

        // Create root container
        let root_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            root_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 0,
                padding: Margins::all(0),
                ..Default::default()
            },
        );

        // Create menu bar
        let menu_bar_id = widget_manager.add_widget(Box::new(MenuBar::new(WidgetId::default())));

        // Configure menu bar
        if let Some(menu_bar) = widget_manager.get_typed_mut::<MenuBar>(menu_bar_id) {
            // File menu
            let file_items = vec![
                MenuItem::new("New")
                    .with_shortcut("Ctrl+N")
                    .with_on_click(|| println!("New file")),
                MenuItem::new("Open...")
                    .with_shortcut("Ctrl+O")
                    .with_on_click(|| println!("Open file")),
                MenuItem::new("Save")
                    .with_shortcut("Ctrl+S")
                    .with_on_click(|| println!("Save file")),
                MenuItem::new("Save As...")
                    .with_shortcut("Ctrl+Shift+S")
                    .with_on_click(|| println!("Save as")),
                MenuItem::separator(),
                MenuItem::new("Exit")
                    .with_shortcut("Alt+F4")
                    .with_on_click(|| println!("Exit")),
            ];
            menu_bar.add_menu("File", file_items);

            // Edit menu
            let edit_items = vec![
                MenuItem::new("Undo")
                    .with_shortcut("Ctrl+Z")
                    .with_on_click(|| println!("Undo")),
                MenuItem::new("Redo")
                    .with_shortcut("Ctrl+Y")
                    .with_on_click(|| println!("Redo")),
                MenuItem::separator(),
                MenuItem::new("Cut")
                    .with_shortcut("Ctrl+X")
                    .with_on_click(|| println!("Cut")),
                MenuItem::new("Copy")
                    .with_shortcut("Ctrl+C")
                    .with_on_click(|| println!("Copy")),
                MenuItem::new("Paste")
                    .with_shortcut("Ctrl+V")
                    .with_on_click(|| println!("Paste")),
                MenuItem::separator(),
                MenuItem::new("Select All")
                    .with_shortcut("Ctrl+A")
                    .with_on_click(|| println!("Select all")),
            ];
            menu_bar.add_menu("Edit", edit_items);

            // View menu
            let view_items = vec![
                MenuItem::new("Zoom In")
                    .with_shortcut("Ctrl++")
                    .with_on_click(|| println!("Zoom in")),
                MenuItem::new("Zoom Out")
                    .with_shortcut("Ctrl+-")
                    .with_on_click(|| println!("Zoom out")),
                MenuItem::new("Reset Zoom")
                    .with_shortcut("Ctrl+0")
                    .with_on_click(|| println!("Reset zoom")),
                MenuItem::separator(),
                MenuItem::new("Full Screen")
                    .with_shortcut("F11")
                    .with_on_click(|| println!("Full screen")),
            ];
            menu_bar.add_menu("View", view_items);

            // Help menu
            let help_items = vec![
                MenuItem::new("Documentation").with_on_click(|| println!("Open documentation")),
                MenuItem::new("About").with_on_click(|| println!("About")),
            ];
            menu_bar.add_menu("Help", help_items);
        }

        // Create content area
        let content_container_id =
            widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            content_container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 20,
                padding: Margins::all(20),
                ..Default::default()
            },
        );

        // Add content labels
        let title_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Menu Demo").with_align(TextAlign::Center),
        ));

        let instruction_id = widget_manager.add_widget(Box::new(
            Label::new(
                WidgetId::default(),
                "Click on the menu items above to see the menu system in action",
            )
            .with_align(TextAlign::Center),
        ));

        let coord_info_id = widget_manager.add_widget(Box::new(
            Label::new(
                WidgetId::default(),
                "The menu uses integer coordinates for pixel-perfect rendering",
            )
            .with_align(TextAlign::Center)
            .with_color(Color::rgb(100, 150, 200)),
        ));

        // Build hierarchy. (Pre-strip code used `add_flex_child(...)` for
        // the content container; flex was already a no-op here -- the
        // example walker assigns flex height equally regardless.)
        let root = widget_manager.get_typed_mut::<Container>(root_id).unwrap();
        root.add_child(menu_bar_id);
        root.add_child(content_container_id);

        let content = widget_manager
            .get_typed_mut::<Container>(content_container_id)
            .unwrap();
        content.add_child(title_id);
        content.add_child(instruction_id);
        content.add_child(coord_info_id);

        Self {
            widget_manager,
            root_container: root_id,
            layouts,
        }
    }

    fn layout_widgets(&mut self, size: Size) {
        if let Some(root) = self.widget_manager.get_mut(self.root_container) {
            let theme = Theme::light();
            root.layout(Rect::from_origin_size(Point::ZERO, size), &theme);
        }

        let theme = Theme::light();
        self.layout_children(self.root_container, &theme);
    }

    fn layout_children(&mut self, parent_id: WidgetId, theme: &Theme) {
        if let Some(container) = self.widget_manager.get_typed::<Container>(parent_id) {
            if let Some(layout_info) = self.layouts.get(&parent_id).cloned() {
                let children = container.children().to_vec();
                let bounds = container.bounds();
                self.perform_container_layout(parent_id, &layout_info, &children, bounds, theme);
            }
        }

        let children: Vec<WidgetId> = if let Some(parent) = self.widget_manager.get(parent_id) {
            parent.children().to_vec()
        } else {
            return;
        };

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
                let mut remaining_height = content_rect.height();
                let mut flex_items = Vec::new();
                let mut fixed_height = 0;

                // First pass: measure fixed items (including menu bar).
                // A nested Container with a recorded layout is treated as
                // flex (it expands).
                let menu_bar_height = theme.typography.font_size_base + 12;
                for &child_id in children {
                    if self.widget_manager.get(child_id).is_some() {
                        // Check if it's a menu bar
                        if self.widget_manager.get_typed::<MenuBar>(child_id).is_some() {
                            fixed_height += menu_bar_height;
                        } else if self.widget_manager.get_typed::<Container>(child_id).is_some() {
                            if let Some(child_layout) = self.layouts.get(&child_id) {
                                if child_layout.mode != LayoutMode::None {
                                    flex_items.push(child_id);
                                    continue;
                                }
                            }
                            let child_height = 50;
                            fixed_height += child_height + spacing;
                        } else {
                            let child_height = 50; // Default height for labels/buttons
                            fixed_height += child_height + spacing;
                        }
                    }
                }

                remaining_height = (remaining_height - fixed_height).max(0);
                let flex_height = if flex_items.is_empty() {
                    0
                } else {
                    remaining_height / flex_items.len() as i32
                };

                // Second pass: layout all children
                for &child_id in children {
                    // Determine height before getting mutable reference
                    let is_menu_bar = self.widget_manager.get_typed::<MenuBar>(child_id).is_some();
                    let height = if is_menu_bar {
                        menu_bar_height
                    } else if flex_items.contains(&child_id) {
                        flex_height
                    } else {
                        50 // Default height
                    };

                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        child.layout(
                            Rect::new(content_rect.x(), y, content_rect.width(), height),
                            theme,
                        );
                    }
                    y += height + spacing;
                }
            }

            LayoutMode::Horizontal => {
                let mut x = content_rect.x();
                let child_count = children.len() as i32;
                let total_spacing = spacing * (child_count - 1).max(0);
                let available_width = content_rect.width() - total_spacing;

                for &child_id in children {
                    if let Some(child) = self.widget_manager.get_mut(child_id) {
                        let width = available_width / child_count;
                        child.layout(
                            Rect::new(x, content_rect.y(), width, content_rect.height()),
                            theme,
                        );
                        x += width + spacing;
                    }
                }
            }

            _ => {}
        }
    }

    fn draw_widget(&self, widget_id: WidgetId, renderer: &mut Renderer) {
        let theme = Theme::light();

        if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.draw(renderer, &theme);

            for &child_id in widget.children() {
                self.draw_widget(child_id, renderer);
            }
        }
    }

    /// Walk the tree and call `draw_dropdown_only` on every MenuBar so
    /// open dropdowns paint on top of the rest of the UI.
    fn draw_dropdowns(&self, renderer: &mut Renderer, theme: &Theme) {
        self.draw_dropdowns_walk(self.root_container, renderer, theme);
    }

    fn draw_dropdowns_walk(
        &self,
        widget_id: WidgetId,
        renderer: &mut Renderer,
        theme: &Theme,
    ) {
        if let Some(widget) = self.widget_manager.get(widget_id) {
            for &child_id in widget.children() {
                self.draw_dropdowns_walk(child_id, renderer, theme);
            }
            if let Some(menu_bar) = self.widget_manager.get_typed::<MenuBar>(widget_id) {
                menu_bar.draw_dropdown_only(renderer, theme);
            }
        }
    }

    fn handle_event(&mut self, event: &erigui_core::Event) {
        let root_container = self.root_container;
        let theme = Theme::light();
        self.route_event(root_container, event, &theme);
    }

    fn route_event(
        &mut self,
        widget_id: WidgetId,
        event: &erigui_core::Event,
        theme: &Theme,
    ) -> erigui_core::EventResult {
        let children: Vec<WidgetId> = if let Some(widget) = self.widget_manager.get(widget_id) {
            widget.children().to_vec()
        } else {
            return erigui_core::EventResult::Ignored;
        };

        // Route to children in reverse order (top to bottom)
        for &child_id in children.iter().rev() {
            if self.route_event(child_id, event, theme).is_consumed() {
                return erigui_core::EventResult::Consumed;
            }
        }

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
    let mut renderer = Renderer::new(&event_loop, 800, 600, "EriGui - Menu Demo")?;
    let mut app = App::new();

    app.layout_widgets(Size::new(800, 600));

    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;

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

                if let Some(gui_event) = translator.translate(&event) {
                    app.handle_event(&gui_event);
                }
            }
            Event::AboutToWait => {
                // Background must come from the same theme the labels
                // use, otherwise dark text lands on dark fill (invisible).
                let theme = Theme::light();
                renderer.begin_frame(theme.colors.background);
                app.draw_widget(app.root_container, &mut renderer);
                // Second pass: dropdowns must paint on top of everything
                // else (z-order). MenuBar::draw only paints the strip;
                // the host calls draw_dropdown_only after the rest of
                // the tree.
                app.draw_dropdowns(&mut renderer, &theme);
                renderer.end_frame();
            }
            _ => {}
        }
    }).unwrap();
    Ok(())
}

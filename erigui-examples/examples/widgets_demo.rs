use erigui_core::{Color, LayoutConfig, LayoutMode, Margins, Point, Rect, Size, Theme, Widget};
use erigui_rendering::{convert_window_event, Renderer};
use erigui_widgets::{
    Checkbox, Container, Label, ProgressBar, ProgressBarStyle, Slider, SliderOrientation,
    TextAlign, WidgetId, WidgetManager,
};
use std::collections::HashMap;
use std::time::Instant;

// Container is a marker widget; it doesn't compute child layout. The
// host stores per-container LayoutConfigs in this sidecar map and uses
// them in `perform_container_layout` to position children.
struct App {
    widget_manager: WidgetManager,
    root_container: WidgetId,
    progress_bar_id: WidgetId,
    animated_progress_id: WidgetId,
    last_update: Instant,
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
                spacing: 20,
                padding: Margins::all(20),
                ..Default::default()
            },
        );

        // Title
        let title_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "EriGui Widgets Demo").with_align(TextAlign::Center),
        ));

        // Checkbox section
        let checkbox_section = Self::create_checkbox_section(&mut widget_manager, &mut layouts);

        // Slider section
        let slider_section = Self::create_slider_section(&mut widget_manager, &mut layouts);

        // Progress bar section
        let (progress_section, progress_bar_id, animated_progress_id) =
            Self::create_progress_section(&mut widget_manager, &mut layouts);

        // Build hierarchy
        let root = widget_manager.get_typed_mut::<Container>(root_id).unwrap();
        root.add_child(title_id);
        root.add_child(checkbox_section);
        root.add_child(slider_section);
        root.add_child(progress_section);

        Self {
            widget_manager,
            root_container: root_id,
            progress_bar_id,
            animated_progress_id,
            last_update: Instant::now(),
            layouts,
        }
    }

    fn create_checkbox_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 10,
                ..Default::default()
            },
        );

        // Section label
        let label_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Checkboxes:").with_color(Color::rgb(100, 150, 200)),
        ));

        // Checkboxes
        let checkbox1_id = widget_manager.add_widget(Box::new(
            Checkbox::new(WidgetId::default(), "Enable notifications")
                .with_checked(true)
                .with_on_toggle(|checked| println!("Notifications: {}", checked)),
        ));

        let checkbox2_id = widget_manager.add_widget(Box::new(
            Checkbox::new(WidgetId::default(), "Auto-save documents")
                .with_on_toggle(|checked| println!("Auto-save: {}", checked)),
        ));

        let checkbox3_id = widget_manager.add_widget(Box::new(Checkbox::new(
            WidgetId::default(),
            "Disabled option",
        )));
        widget_manager
            .get_mut(checkbox3_id)
            .unwrap()
            .set_enabled(false);

        // Add to container
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(checkbox1_id);
        container.add_child(checkbox2_id);
        container.add_child(checkbox3_id);

        container_id
    }

    fn create_slider_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> WidgetId {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 10,
                ..Default::default()
            },
        );

        // Section label
        let label_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Sliders:").with_color(Color::rgb(100, 150, 200)),
        ));

        // Value label
        let value_label_id =
            widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "Volume: 50%")));

        // Horizontal slider
        let h_slider_id = {
            let value_label_id = value_label_id;
            widget_manager.add_widget(Box::new(
                Slider::new(WidgetId::default(), 0.0, 100.0, 50.0).with_on_value_changed(
                    move |value| {
                        println!("Volume: {:.0}%", value);
                    },
                ),
            ))
        };

        // Vertical slider container
        let v_container_id =
            widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            v_container_id,
            LayoutConfig {
                mode: LayoutMode::Horizontal,
                spacing: 20,
                ..Default::default()
            },
        );

        let brightness_label_id =
            widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "Brightness:")));

        let v_slider_id = widget_manager.add_widget(Box::new(
            Slider::new(WidgetId::default(), 0.0, 100.0, 75.0)
                .with_orientation(SliderOrientation::Vertical)
                .with_on_value_changed(|value| println!("Brightness: {:.0}%", value)),
        ));

        // Add to containers
        let v_container = widget_manager
            .get_typed_mut::<Container>(v_container_id)
            .unwrap();
        v_container.add_child(brightness_label_id);
        v_container.add_child(v_slider_id);

        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(value_label_id);
        container.add_child(h_slider_id);
        container.add_child(v_container_id);

        container_id
    }

    fn create_progress_section(
        widget_manager: &mut WidgetManager,
        layouts: &mut HashMap<WidgetId, LayoutConfig>,
    ) -> (WidgetId, WidgetId, WidgetId) {
        let container_id = widget_manager.add_widget(Box::new(Container::new(WidgetId::default())));
        layouts.insert(
            container_id,
            LayoutConfig {
                mode: LayoutMode::Vertical,
                spacing: 10,
                ..Default::default()
            },
        );

        // Section label
        let label_id = widget_manager.add_widget(Box::new(
            Label::new(WidgetId::default(), "Progress Bars:").with_color(Color::rgb(100, 150, 200)),
        ));

        // Solid progress bar
        let progress1_id = widget_manager.add_widget(Box::new(
            ProgressBar::new(WidgetId::default()).with_value(33.0),
        ));

        // Striped progress bar
        let progress2_id = widget_manager.add_widget(Box::new(
            ProgressBar::new(WidgetId::default())
                .with_value(67.0)
                .with_style(ProgressBarStyle::Striped)
                .with_color(Color::rgb(0, 200, 100)),
        ));

        // Animated progress bar
        let progress3_id = widget_manager.add_widget(Box::new(
            ProgressBar::new(WidgetId::default())
                .with_value(0.0)
                .with_style(ProgressBarStyle::Striped)
                .with_color(Color::rgb(255, 165, 0)),
        ));

        // Add to container
        let container = widget_manager
            .get_typed_mut::<Container>(container_id)
            .unwrap();
        container.add_child(label_id);
        container.add_child(progress1_id);
        container.add_child(progress2_id);
        container.add_child(progress3_id);

        (container_id, progress1_id, progress3_id)
    }

    fn update(&mut self) {
        // Animate the striped progress bars
        if let Some(progress) = self
            .widget_manager
            .get_typed_mut::<ProgressBar>(self.animated_progress_id)
        {
            progress.animate();

            // Also animate the value
            if self.last_update.elapsed().as_millis() > 50 {
                let current = progress.value();
                let new_value = if current >= 100.0 { 0.0 } else { current + 1.0 };
                progress.set_value(new_value);
                self.last_update = Instant::now();
            }
        }

        // Animate the second progress bar
        if let Some(progress) = self
            .widget_manager
            .get_typed_mut::<ProgressBar>(self.progress_bar_id)
        {
            progress.animate();
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

                for &child_id in children {
                    // Check for vertical slider before getting mutable reference
                    let is_vertical_slider = self
                        .widget_manager
                        .get_typed::<Slider>(child_id)
                        .map(|s| matches!(s.orientation(), SliderOrientation::Vertical))
                        .unwrap_or(false);

                    let height = if is_vertical_slider {
                        150 // Fixed height for vertical sliders
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
    let mut renderer = Renderer::new(&event_loop, 800, 600, "EriGui - Widgets Demo")?;
    let mut app = App::new();

    app.layout_widgets(Size::new(800, 600));

    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                        app.layout_widgets(Size::new(
                            physical_size.width as i32,
                            physical_size.height as i32,
                        ));
                    }
                    _ => {}
                }

                if let Some(gui_event) = convert_window_event(event, renderer.viewport_size()) {
                    app.handle_event(&gui_event);
                }
            }
            Event::AboutToWait => {
                app.update();
                renderer.begin_frame(Color::rgb(30, 30, 30));
                app.draw_widget(app.root_container, &mut renderer);
                renderer.end_frame();
            }
            _ => {}
        }
    }).unwrap();
    Ok(())
}

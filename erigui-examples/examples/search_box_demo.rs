use erigui_core::*;
use erigui_rendering::Renderer;
use erigui_widgets::*;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct SearchBoxDemo {
    // Different search boxes
    basic_search: SearchBox,
    history_search: SearchBox,
    suggestion_search: SearchBox,

    // Labels
    title_label: Label,
    basic_label: Label,
    history_label: Label,
    suggestion_label: Label,
    result_label: Label,

    // State
    last_search: String,
    search_results: Vec<String>,
}

impl SearchBoxDemo {
    fn new() -> Self {
        // Basic search box
        let basic_search = SearchBox::new(WidgetId::default())
            .with_placeholder("Search for something...")
            .with_on_search(|query| {
                println!("Basic search: {}", query);
            });

        // Search box with history
        let mut history_search = SearchBox::new(WidgetId::default())
            .with_placeholder("Search with history...")
            .with_on_search(|query| {
                println!("History search: {}", query);
            });

        // Pre-populate some history
        history_search.add_to_history("previous search 1".to_string());
        history_search.add_to_history("previous search 2".to_string());
        history_search.add_to_history("example query".to_string());

        // Search box with custom suggestions
        let suggestion_search = SearchBox::new(WidgetId::default())
            .with_placeholder("Type to see suggestions...")
            .with_debounce_delay(200)
            .with_suggestion_provider(|query| {
                // Simulate suggestions based on query
                if query.is_empty() {
                    vec![]
                } else {
                    vec![
                        format!("{} in documents", query),
                        format!("{} in projects", query),
                        format!("{} in settings", query),
                        format!("{} (exact match)", query),
                        format!("{} (fuzzy match)", query),
                    ]
                }
            })
            .with_on_search(|query| {
                println!("Suggestion search: {}", query);
            })
            .with_on_change(|query| {
                println!("Search query changing: {}", query);
            });

        Self {
            basic_search,
            history_search,
            suggestion_search,
            title_label: Label::new(WidgetId::default(), "SearchBox Demo"),
            basic_label: Label::new(WidgetId::default(), "Basic SearchBox:"),
            history_label: Label::new(
                WidgetId::default(),
                "SearchBox with History (click when empty):",
            ),
            suggestion_label: Label::new(WidgetId::default(), "SearchBox with Suggestions:"),
            result_label: Label::new(
                WidgetId::default(),
                "Press Enter to search, Escape to close suggestions",
            ),
            last_search: String::new(),
            search_results: Vec::new(),
        }
    }

    fn handle_search(&mut self, query: &str) {
        self.last_search = query.to_string();

        // Simulate search results
        if query.is_empty() {
            self.search_results.clear();
        } else {
            self.search_results = vec![
                format!("Result 1 for '{}'", query),
                format!("Result 2 for '{}'", query),
                format!("Result 3 for '{}'", query),
            ];
        }

        let result_text = if query.is_empty() {
            "Enter a search query and press Enter".to_string()
        } else {
            format!(
                "Found {} results for '{}'",
                self.search_results.len(),
                query
            )
        };

        self.result_label.set_text(result_text);
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let mut renderer =
        Renderer::new(&event_loop, 800, 600, "SearchBox Demo").expect("Failed to create renderer");

    let theme = Theme::dark();

    let mut demo = SearchBoxDemo::new();

    // Set up search callbacks
    demo.basic_search = demo.basic_search.with_on_search(|query| {
        println!("Searching for: {}", query);
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            WinitEvent::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size.width, physical_size.height);
                    }
                    _ => {}
                }

                // Convert window event to EriGui event
                if let Some(gui_event) =
                    erigui_rendering::window::convert_window_event(event, renderer.viewport_size())
                {
                    // Handle events for all search boxes
                    let basic_result = demo.basic_search.handle_event(&gui_event, &theme);
                    let history_result = demo.history_search.handle_event(&gui_event, &theme);
                    let suggestion_result = demo.suggestion_search.handle_event(&gui_event, &theme);

                    // Check if any search was triggered
                    if let Event::KeyPress(key_event) = &gui_event {
                        if key_event.key == Key::Enter {
                            if demo.basic_search.is_focused() {
                                let text = demo.basic_search.get_text();
                                demo.handle_search(&text);
                            } else if demo.history_search.is_focused() {
                                let text = demo.history_search.get_text();
                                demo.handle_search(&text);
                            } else if demo.suggestion_search.is_focused() {
                                let text = demo.suggestion_search.get_text();
                                demo.handle_search(&text);
                            }
                        }
                    }

                    if basic_result == EventResult::Consumed
                        || history_result == EventResult::Consumed
                        || suggestion_result == EventResult::Consumed
                    {
                        renderer.window().request_redraw();
                    }
                }
            }

            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let search_width = 400;
                let search_height = 36;
                let spacing = 50;

                let mut y = padding;

                // Title
                let title_size = demo
                    .title_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.title_label.layout(
                    Rect::new(
                        (window_size.width - title_size.width) / 2,
                        y,
                        title_size.width,
                        title_size.height,
                    ),
                    &theme,
                );
                y += title_size.height + spacing;

                // Basic search
                let basic_label_size = demo
                    .basic_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.basic_label.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        basic_label_size.width,
                        basic_label_size.height,
                    ),
                    &theme,
                );
                y += basic_label_size.height + 5;

                demo.basic_search.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        search_width,
                        search_height,
                    ),
                    &theme,
                );
                y += search_height + spacing;

                // History search
                let history_label_size = demo
                    .history_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.history_label.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        history_label_size.width,
                        history_label_size.height,
                    ),
                    &theme,
                );
                y += history_label_size.height + 5;

                demo.history_search.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        search_width,
                        search_height,
                    ),
                    &theme,
                );
                y += search_height + spacing;

                // Suggestion search
                let suggestion_label_size = demo
                    .suggestion_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.suggestion_label.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        suggestion_label_size.width,
                        suggestion_label_size.height,
                    ),
                    &theme,
                );
                y += suggestion_label_size.height + 5;

                demo.suggestion_search.layout(
                    Rect::new(
                        (window_size.width - search_width) / 2,
                        y,
                        search_width,
                        search_height,
                    ),
                    &theme,
                );
                y += search_height + spacing * 2;

                // Result label
                let result_size = demo
                    .result_label
                    .measure(&LayoutConstraints::default(), &theme);
                demo.result_label.layout(
                    Rect::new(
                        (window_size.width - result_size.width) / 2,
                        y,
                        result_size.width,
                        result_size.height,
                    ),
                    &theme,
                );

                // Draw
                renderer.begin_frame(theme.colors.background);

                demo.title_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.basic_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.basic_search
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.history_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.history_search
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.suggestion_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.suggestion_search
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.result_label
                    .draw(&mut renderer as &mut dyn DrawContext, &theme);

                renderer.end_frame();
            }

            _ => {}
        }
    });
}

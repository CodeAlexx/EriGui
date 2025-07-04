use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use std::path::Path;

struct FileDialogDemo {
    open_dialog: FileDialog,
    save_dialog: FileDialog,
    folder_dialog: FileDialog,
    
    open_button: Button,
    save_button: Button,
    folder_button: Button,
    
    info_label: Label,
    result_label: Label,
    
    show_open: bool,
    show_save: bool,
    show_folder: bool,
}

impl FileDialogDemo {
    fn new() -> Self {
        // Open file dialog
        let open_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Open)
            .with_filters(vec![
                FileFilter::new("All Files", vec!["*"]),
                FileFilter::new("Text Files", vec!["txt", "md"]),
                FileFilter::new("Images", vec!["png", "jpg", "jpeg", "gif"]),
                FileFilter::new("Rust Files", vec!["rs", "toml"]),
            ])
            .with_on_file_selected(|path| {
                println!("File selected: {}", path.display());
            })
            .with_on_cancel(|| {
                println!("Open dialog cancelled");
            });
        
        // Save file dialog
        let save_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::Save)
            .with_filters(vec![
                FileFilter::new("Text Files", vec!["txt"]),
                FileFilter::new("Markdown", vec!["md"]),
                FileFilter::new("All Files", vec!["*"]),
            ])
            .with_on_file_selected(|path| {
                println!("Save to: {}", path.display());
            })
            .with_on_cancel(|| {
                println!("Save dialog cancelled");
            });
        
        // Select folder dialog
        let folder_dialog = FileDialog::new(WidgetId::default(), FileDialogMode::SelectFolder)
            .with_on_file_selected(|path| {
                println!("Folder selected: {}", path.display());
            })
            .with_on_cancel(|| {
                println!("Folder dialog cancelled");
            });
        
        Self {
            open_dialog,
            save_dialog,
            folder_dialog,
            open_button: Button::new(WidgetId::default(), "Open File..."),
            save_button: Button::new(WidgetId::default(), "Save File..."),
            folder_button: Button::new(WidgetId::default(), "Select Folder..."),
            info_label: Label::new(WidgetId::default(), "FileDialog Demo - Click a button to open a dialog"),
            result_label: Label::new(WidgetId::default(), "No file selected"),
            show_open: false,
            show_save: false,
            show_folder: false,
        }
    }
    
    fn update_result(&mut self, path: &Path, action: &str) {
        let text = format!("{}: {}", action, path.display());
        self.result_label.set_text(text);
    }
}

fn main() {
    let event_loop = EventLoop::new();
    
    let mut renderer = Renderer::new(&event_loop, 800, 600, "FileDialog Demo")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    
    let mut demo = FileDialogDemo::new();
    
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
                if let Some(gui_event) = erigui_rendering::window::convert_window_event(event, renderer.viewport_size()) {
                    let mut redraw = false;
                    
                    // Handle button events
                    let open_btn_result = demo.open_button.handle_event(&gui_event, &theme);
                    let save_btn_result = demo.save_button.handle_event(&gui_event, &theme);
                    let folder_btn_result = demo.folder_button.handle_event(&gui_event, &theme);
                    
                    // Handle button clicks
                    if let Event::MouseButton(mouse_event) = &gui_event {
                        if !mouse_event.pressed && mouse_event.button == MouseButton::Left {
                            if demo.open_button.bounds().contains(mouse_event.position) {
                                demo.show_open = true;
                                demo.show_save = false;
                                demo.show_folder = false;
                                redraw = true;
                            } else if demo.save_button.bounds().contains(mouse_event.position) {
                                demo.show_open = false;
                                demo.show_save = true;
                                demo.show_folder = false;
                                redraw = true;
                            } else if demo.folder_button.bounds().contains(mouse_event.position) {
                                demo.show_open = false;
                                demo.show_save = false;
                                demo.show_folder = true;
                                redraw = true;
                            }
                        }
                    }
                    
                    // Handle dialog events
                    if demo.show_open {
                        let result = demo.open_dialog.handle_event(&gui_event, &theme);
                        if result == EventResult::Consumed {
                            redraw = true;
                        }
                        
                        // Update callbacks to handle closing
                        demo.open_dialog = demo.open_dialog
                            .with_on_file_selected(|path| {
                                println!("File opened: {}", path.display());
                            })
                            .with_on_cancel(|| {
                                println!("Open cancelled");
                            });
                    }
                    
                    if demo.show_save {
                        let result = demo.save_dialog.handle_event(&gui_event, &theme);
                        if result == EventResult::Consumed {
                            redraw = true;
                        }
                    }
                    
                    if demo.show_folder {
                        let result = demo.folder_dialog.handle_event(&gui_event, &theme);
                        if result == EventResult::Consumed {
                            redraw = true;
                        }
                    }
                    
                    if open_btn_result == EventResult::Consumed ||
                       save_btn_result == EventResult::Consumed ||
                       folder_btn_result == EventResult::Consumed ||
                       redraw {
                        renderer.window().request_redraw();
                    }
                }
            }
            
            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                let button_width = 150;
                let button_height = 40;
                let spacing = 20;
                
                // Info label
                let info_size = demo.info_label.measure(&LayoutConstraints::default(), &theme);
                demo.info_label.layout(Rect::new(
                    (window_size.width - info_size.width) / 2,
                    padding,
                    info_size.width,
                    info_size.height
                ), &theme);
                
                // Buttons
                let buttons_y = padding + info_size.height + spacing;
                let total_width = button_width * 3 + spacing * 2;
                let buttons_x = (window_size.width - total_width) / 2;
                
                demo.open_button.layout(Rect::new(
                    buttons_x,
                    buttons_y,
                    button_width,
                    button_height
                ), &theme);
                
                demo.save_button.layout(Rect::new(
                    buttons_x + button_width + spacing,
                    buttons_y,
                    button_width,
                    button_height
                ), &theme);
                
                demo.folder_button.layout(Rect::new(
                    buttons_x + (button_width + spacing) * 2,
                    buttons_y,
                    button_width,
                    button_height
                ), &theme);
                
                // Result label
                let result_size = demo.result_label.measure(&LayoutConstraints::default(), &theme);
                demo.result_label.layout(Rect::new(
                    (window_size.width - result_size.width) / 2,
                    buttons_y + button_height + spacing,
                    result_size.width,
                    result_size.height
                ), &theme);
                
                // Dialogs (centered)
                let dialog_size = Size::new(700, 500);
                let dialog_pos = Point::new(
                    (window_size.width - dialog_size.width) / 2,
                    (window_size.height - dialog_size.height) / 2
                );
                
                if demo.show_open {
                    demo.open_dialog.layout(Rect::new(
                        dialog_pos.x,
                        dialog_pos.y,
                        dialog_size.width,
                        dialog_size.height
                    ), &theme);
                }
                
                if demo.show_save {
                    demo.save_dialog.layout(Rect::new(
                        dialog_pos.x,
                        dialog_pos.y,
                        dialog_size.width,
                        dialog_size.height
                    ), &theme);
                }
                
                if demo.show_folder {
                    demo.folder_dialog.layout(Rect::new(
                        dialog_pos.x,
                        dialog_pos.y,
                        dialog_size.width,
                        dialog_size.height
                    ), &theme);
                }
                
                // Draw
                renderer.begin_frame(theme.colors.background);
                
                demo.info_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.open_button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.save_button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.folder_button.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.result_label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                
                // Draw active dialog
                if demo.show_open {
                    demo.open_dialog.draw(&mut renderer as &mut dyn DrawContext, &theme);
                }
                
                if demo.show_save {
                    demo.save_dialog.draw(&mut renderer as &mut dyn DrawContext, &theme);
                }
                
                if demo.show_folder {
                    demo.folder_dialog.draw(&mut renderer as &mut dyn DrawContext, &theme);
                }
                
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}
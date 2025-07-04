use erigui_core::*;
use erigui_widgets::*;
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

struct TreeViewDemo {
    tree_view: TreeView,
    label: Label,
    selected_path: String,
    last_mouse_pos: Point,
}

impl TreeViewDemo {
    fn new() -> Self {
        // Create the tree view
        let mut tree_view = TreeView::new(WidgetId::default())
            .with_on_selection_change(|_| {})
            .with_on_expand(|_, _| {});
        
        // Build a file system-like tree structure
        let mut root1 = TreeNode::new("root_home".to_string(), "Home".to_string());
        root1.icon = Some("🏠".to_string());
        
        let mut documents = TreeNode::new("documents".to_string(), "Documents".to_string());
        documents.icon = Some("📁".to_string());
        documents.add_child(TreeNode::new("resume.pdf".to_string(), "resume.pdf".to_string()));
        documents.add_child(TreeNode::new("cover_letter.doc".to_string(), "cover_letter.doc".to_string()));
        
        let mut projects = TreeNode::new("projects".to_string(), "Projects".to_string());
        projects.icon = Some("📁".to_string());
        
        let mut rust_project = TreeNode::new("rust_project".to_string(), "rust-gui".to_string());
        rust_project.icon = Some("📁".to_string());
        rust_project.add_child(TreeNode::new("cargo_toml".to_string(), "Cargo.toml".to_string()));
        rust_project.add_child(TreeNode::new("src_folder".to_string(), "src".to_string()));
        
        let mut src_folder = TreeNode::new("src".to_string(), "src".to_string());
        src_folder.icon = Some("📁".to_string());
        src_folder.add_child(TreeNode::new("main_rs".to_string(), "main.rs".to_string()));
        src_folder.add_child(TreeNode::new("lib_rs".to_string(), "lib.rs".to_string()));
        rust_project.children[1] = src_folder;
        
        projects.add_child(rust_project);
        projects.add_child(TreeNode::new("python_proj".to_string(), "data-analysis".to_string()));
        
        documents.add_child(projects);
        root1.add_child(documents);
        
        let mut pictures = TreeNode::new("pictures".to_string(), "Pictures".to_string());
        pictures.icon = Some("🖼️".to_string());
        pictures.add_child(TreeNode::new("vacation2023".to_string(), "Vacation 2023".to_string()));
        pictures.add_child(TreeNode::new("family".to_string(), "Family Photos".to_string()));
        root1.add_child(pictures);
        
        let mut downloads = TreeNode::new("downloads".to_string(), "Downloads".to_string());
        downloads.icon = Some("📥".to_string());
        downloads.add_child(TreeNode::new("installer.exe".to_string(), "installer.exe".to_string()));
        downloads.add_child(TreeNode::new("data.zip".to_string(), "data.zip".to_string()));
        root1.add_child(downloads);
        
        // Add another root
        let mut root2 = TreeNode::new("root_computer".to_string(), "Computer".to_string());
        root2.icon = Some("💻".to_string());
        
        let mut system = TreeNode::new("system".to_string(), "System".to_string());
        system.icon = Some("⚙️".to_string());
        
        let mut config = TreeNode::new("config".to_string(), "Configuration".to_string());
        config.add_child(TreeNode::new("settings.ini".to_string(), "settings.ini".to_string()));
        config.add_child(TreeNode::new("prefs.json".to_string(), "preferences.json".to_string()));
        system.add_child(config);
        
        let mut logs = TreeNode::new("logs".to_string(), "Logs".to_string());
        logs.add_child(TreeNode::new("system.log".to_string(), "system.log".to_string()));
        logs.add_child(TreeNode::new("error.log".to_string(), "error.log".to_string()));
        system.add_child(logs);
        
        root2.add_child(system);
        
        let mut apps = TreeNode::new("apps".to_string(), "Applications".to_string());
        apps.icon = Some("📱".to_string());
        apps.add_child(TreeNode::new("browser".to_string(), "Web Browser".to_string()));
        apps.add_child(TreeNode::new("editor".to_string(), "Text Editor".to_string()));
        apps.add_child(TreeNode::new("terminal".to_string(), "Terminal".to_string()));
        root2.add_child(apps);
        
        // Add a third root with more depth
        let mut root3 = TreeNode::new("root_network".to_string(), "Network".to_string());
        root3.icon = Some("🌐".to_string());
        
        let mut shared = TreeNode::new("shared".to_string(), "Shared Drives".to_string());
        
        let mut server1 = TreeNode::new("server1".to_string(), "Server-01".to_string());
        server1.icon = Some("🖥️".to_string());
        
        let mut public = TreeNode::new("public".to_string(), "Public".to_string());
        public.add_child(TreeNode::new("readme.txt".to_string(), "README.txt".to_string()));
        public.add_child(TreeNode::new("shared_data".to_string(), "shared_data.csv".to_string()));
        server1.add_child(public);
        
        let mut private = TreeNode::new("private".to_string(), "Private".to_string());
        private.add_child(TreeNode::new("confidential".to_string(), "confidential.doc".to_string()));
        server1.add_child(private);
        
        shared.add_child(server1);
        
        let mut server2 = TreeNode::new("server2".to_string(), "Server-02".to_string());
        server2.icon = Some("🖥️".to_string());
        server2.add_child(TreeNode::new("backups".to_string(), "Backups".to_string()));
        server2.add_child(TreeNode::new("archives".to_string(), "Archives".to_string()));
        shared.add_child(server2);
        
        root3.add_child(shared);
        
        tree_view.add_root_node(root1);
        tree_view.add_root_node(root2);
        tree_view.add_root_node(root3);
        
        // Expand some nodes by default
        if let Some(node) = tree_view.find_node_mut("root_home") {
            node.expanded = true;
        }
        if let Some(node) = tree_view.find_node_mut("documents") {
            node.expanded = true;
        }
        
        let label = Label::new(WidgetId::default(), "Click on items to select and expand/collapse");
        
        Self {
            tree_view,
            label,
            selected_path: String::new(),
            last_mouse_pos: Point::ZERO,
        }
    }
    
    fn handle_selection(&mut self, node_id: &str) {
        // Build the path to the selected node
        fn find_path(nodes: &[TreeNode], target_id: &str, current_path: &str) -> Option<String> {
            for node in nodes {
                let node_path = if current_path.is_empty() {
                    node.text.clone()
                } else {
                    format!("{} / {}", current_path, node.text)
                };
                
                if node.id == target_id {
                    return Some(node_path);
                }
                
                if let Some(path) = find_path(&node.children, target_id, &node_path) {
                    return Some(path);
                }
            }
            None
        }
        
        if let Some(path) = find_path(&self.tree_view.root_nodes, node_id, "") {
            self.selected_path = path;
            self.label.set_text(format!("Selected: {}", self.selected_path));
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    
    let mut renderer = Renderer::new(&event_loop, 800, 600, "TreeView Demo")
        .expect("Failed to create renderer");
    
    let theme = Theme::dark();
    
    let mut demo = TreeViewDemo::new();
    
    // Set up callbacks
    demo.tree_view = demo.tree_view.with_on_selection_change(|node_id| {
        println!("Selection changed: {}", node_id);
    });
    
    demo.tree_view = demo.tree_view.with_on_expand(|node_id, expanded| {
        println!("Node {} {}", node_id, if expanded { "expanded" } else { "collapsed" });
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
                if let Some(mut gui_event) = erigui_rendering::window::convert_window_event(event, renderer.viewport_size()) {
                    // Update mouse position for MouseMove events
                    if let Event::MouseMove(ref mut move_event) = gui_event {
                        let pos = move_event.position;
                        move_event.delta = Point::new(
                            pos.x - demo.last_mouse_pos.x,
                            pos.y - demo.last_mouse_pos.y
                        );
                        demo.last_mouse_pos = pos;
                    }
                    
                    // Handle events
                    let result = demo.tree_view.handle_event(&gui_event, &theme);
                    
                    // Check if selection changed
                    if matches!(gui_event, Event::MouseButton(_)) {
                        // Find selected node
                        for node in &demo.tree_view.root_nodes {
                            if let Some(selected) = find_selected_node(node) {
                                demo.handle_selection(&selected);
                                break;
                            }
                        }
                    }
                    
                    renderer.window().request_redraw();
                }
            }
            
            WinitEvent::RedrawRequested(_) => {
                // Layout
                let window_size = renderer.viewport_size();
                let padding = 20;
                
                // Layout label at top
                let label_size = demo.label.measure(&LayoutConstraints::default(), &theme);
                demo.label.layout(Rect::new(padding, padding, window_size.width - padding * 2, label_size.height), &theme);
                
                // Layout tree view below label
                let tree_constraints = LayoutConstraints {
                    min_width: Some(100),
                    max_width: Some(window_size.width - padding * 2),
                    min_height: Some(100),
                    max_height: Some(window_size.height - label_size.height - padding * 3),
                };
                
                let tree_size = demo.tree_view.measure(&tree_constraints, &theme);
                demo.tree_view.layout(
                    Rect::new(
                        padding,
                        padding * 2 + label_size.height,
                        tree_size.width,
                        tree_size.height
                    ),
                    &theme
                );
                
                // Draw frame
                renderer.begin_frame(theme.colors.background);
                demo.label.draw(&mut renderer as &mut dyn DrawContext, &theme);
                demo.tree_view.draw(&mut renderer as &mut dyn DrawContext, &theme);
                renderer.end_frame();
            }
            
            _ => {}
        }
    });
}

fn find_selected_node(node: &TreeNode) -> Option<String> {
    if node.selected {
        return Some(node.id.clone());
    }
    
    for child in &node.children {
        if let Some(id) = find_selected_node(child) {
            return Some(id);
        }
    }
    
    None
}
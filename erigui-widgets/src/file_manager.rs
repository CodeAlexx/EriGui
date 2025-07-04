use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, LayoutMode, LayoutConfig, Rect,
    Size, Theme, Widget, WidgetId, WidgetState, Margins,
};
use crate::{Container, TreeView, ListView, TextInput, Label, ListItem, TreeNode};
use std::any::Any;
use std::path::{Path, PathBuf};

pub struct FileManager {
    state: WidgetState,
    container_id: WidgetId,
    path_input_id: WidgetId,
    tree_view_id: WidgetId,
    list_view_id: WidgetId,
    status_label_id: WidgetId,
    current_path: PathBuf,
    show_hidden: bool,
    on_file_select: Option<Box<dyn FnMut(&Path)>>,
}

impl FileManager {
    pub fn new(
        id: WidgetId,
        container_id: WidgetId,
        path_input_id: WidgetId,
        tree_view_id: WidgetId,
        list_view_id: WidgetId,
        status_label_id: WidgetId,
    ) -> Self {
        Self {
            state: WidgetState::new(id),
            container_id,
            path_input_id,
            tree_view_id,
            list_view_id,
            status_label_id,
            current_path: PathBuf::from("/"),
            show_hidden: false,
            on_file_select: None,
        }
    }
    
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_path = path.into();
        self
    }
    
    pub fn with_on_file_select<F: FnMut(&Path) + 'static>(mut self, f: F) -> Self {
        self.on_file_select = Some(Box::new(f));
        self
    }
    
    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
    }
    
    pub fn refresh(&mut self) {
        // In a real implementation, this would read the file system
        // and update the tree and list views
    }
    
    pub fn navigate_to(&mut self, path: impl Into<PathBuf>) {
        self.current_path = path.into();
        self.refresh();
    }
    
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            let parent_path = parent.to_path_buf();
            self.navigate_to(parent_path);
        }
    }
    
    fn create_tree_nodes(&self, _path: &Path) -> Vec<TreeNode> {
        // In a real implementation, this would read the file system
        // For now, return mock data
        vec![
            TreeNode {
                id: "home".to_string(),
                text: "Home".to_string(),
                icon: Some("folder".to_string()),
                children: vec![
                    TreeNode::new("documents".to_string(), "Documents".to_string()),
                    TreeNode::new("downloads".to_string(), "Downloads".to_string()),
                    TreeNode::new("pictures".to_string(), "Pictures".to_string()),
                ],
                expanded: true,
                selected: false,
            },
            TreeNode {
                id: "root".to_string(),
                text: "Root".to_string(),
                icon: Some("folder".to_string()),
                children: vec![
                    TreeNode::new("etc".to_string(), "etc".to_string()),
                    TreeNode::new("usr".to_string(), "usr".to_string()),
                    TreeNode::new("var".to_string(), "var".to_string()),
                ],
                expanded: false,
                selected: false,
            },
        ]
    }
    
    fn create_list_items(&self, _path: &Path) -> Vec<ListItem> {
        // In a real implementation, this would read the file system
        // For now, return mock data
        vec![
            ListItem {
                id: "file1".to_string(),
                text: "document.txt".to_string(),
                icon: Some("file".to_string()),
                selected: false,
            },
            ListItem {
                id: "file2".to_string(),
                text: "image.png".to_string(),
                icon: Some("image".to_string()),
                selected: false,
            },
            ListItem {
                id: "folder1".to_string(),
                text: "Projects".to_string(),
                icon: Some("folder".to_string()),
                selected: false,
            },
        ]
    }
}

impl Widget for FileManager {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(
            constraints.max_width.unwrap_or(800),
            constraints.max_height.unwrap_or(600),
        )
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }
    
    fn draw(&self, _context: &mut dyn DrawContext, _theme: &Theme) {
        if !self.state.visible {
            return;
        }
        
        // The actual drawing is handled by the child widgets
        // FileManager just coordinates them
    }
    
    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
        // Events are handled by child widgets
        EventResult::Ignored
    }
    
    fn children(&self) -> &[WidgetId] {
        std::slice::from_ref(&self.container_id)
    }
    
    fn bounds(&self) -> Rect {
        self.state.bounds
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
    }
    
    fn is_visible(&self) -> bool {
        self.state.visible
    }
    
    fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }
    
    fn is_enabled(&self) -> bool {
        self.state.enabled
    }
    
    fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
    }
    
    fn is_focused(&self) -> bool {
        false
    }
    
    fn set_focused(&mut self, _focused: bool) {}
    
    fn can_focus(&self) -> bool {
        false
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn create_file_manager(widget_manager: &mut crate::WidgetManager) -> WidgetId {
    // Create container
    let container = Container::new(widget_manager.add_widget(Box::new(Container::new(WidgetId::default()))))
        .with_layout(LayoutConfig {
            mode: LayoutMode::Vertical,
            spacing: 4,
            padding: Margins::all(4),
            ..Default::default()
        });
    let container_id = container.id();
    
    // Create path input
    let path_input = TextInput::new(widget_manager.add_widget(Box::new(TextInput::new(WidgetId::default()))))
        .with_placeholder("Enter path...");
    let path_input_id = path_input.id();
    
    // Create horizontal split for tree and list
    let split_container = Container::new(widget_manager.add_widget(Box::new(Container::new(WidgetId::default()))))
        .with_layout(LayoutConfig {
            mode: LayoutMode::Horizontal,
            spacing: 4,
            ..Default::default()
        });
    let split_container_id = split_container.id();
    
    // Create tree view
    let tree_view = TreeView::new(widget_manager.add_widget(Box::new(TreeView::new(WidgetId::default()))));
    let tree_view_id = tree_view.id();
    
    // Create list view
    let list_view = ListView::new(widget_manager.add_widget(Box::new(ListView::new(WidgetId::default()))));
    let list_view_id = list_view.id();
    
    // Create status label
    let status_label = Label::new(
        widget_manager.add_widget(Box::new(Label::new(WidgetId::default(), "Ready"))),
        "Ready"
    );
    let status_label_id = status_label.id();
    
    // Build widget hierarchy
    widget_manager.get_typed_mut::<Container>(container_id).unwrap().add_child(path_input_id);
    widget_manager.get_typed_mut::<Container>(container_id).unwrap().add_flex_child(split_container_id, 1.0);
    widget_manager.get_typed_mut::<Container>(container_id).unwrap().add_child(status_label_id);
    
    widget_manager.get_typed_mut::<Container>(split_container_id).unwrap().add_flex_child(tree_view_id, 0.3);
    widget_manager.get_typed_mut::<Container>(split_container_id).unwrap().add_flex_child(list_view_id, 0.7);
    
    // Create file manager
    let file_manager = FileManager::new(
        widget_manager.add_widget(Box::new(FileManager::new(
            WidgetId::default(),
            container_id,
            path_input_id,
            tree_view_id,
            list_view_id,
            status_label_id,
        ))),
        container_id,
        path_input_id,
        tree_view_id,
        list_view_id,
        status_label_id,
    );
    
    file_manager.id()
}
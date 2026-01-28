use erigui_core::{Rect, WidgetId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AccessibilityRole {
    Button,
    CheckBox,
    RadioButton,
    TextInput,
    Label,
    Link,
    Menu,
    MenuItem,
    Tab,
    TabPanel,
    Tree,
    TreeItem,
    List,
    ListItem,
    ComboBox,
    Slider,
    ProgressBar,
    Dialog,
    Alert,
    ToolBar,
    ToolTip,
    ScrollBar,
    Separator,
    Window,
    Document,
    Application,
}

#[derive(Debug, Clone)]
pub struct AccessibilityState {
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub disabled: Option<bool>,
    pub focused: Option<bool>,
    pub pressed: Option<bool>,
    pub value: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub current_value: Option<f64>,
    pub level: Option<i32>,
    pub item_count: Option<i32>,
    pub item_index: Option<i32>,
}

impl Default for AccessibilityState {
    fn default() -> Self {
        Self {
            checked: None,
            selected: None,
            expanded: None,
            disabled: None,
            focused: None,
            pressed: None,
            value: None,
            min_value: None,
            max_value: None,
            current_value: None,
            level: None,
            item_count: None,
            item_index: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub id: WidgetId,
    pub role: AccessibilityRole,
    pub label: String,
    pub description: Option<String>,
    pub bounds: Rect,
    pub state: AccessibilityState,
    pub children: Vec<WidgetId>,
    pub parent: Option<WidgetId>,
    pub actions: Vec<AccessibilityAction>,
}

#[derive(Debug, Clone)]
pub enum AccessibilityAction {
    Click,
    Focus,
    Expand,
    Collapse,
    Check,
    Uncheck,
    Select,
    SetValue(String),
    ScrollIntoView,
}

pub struct AccessibilityManager {
    nodes: HashMap<WidgetId, AccessibilityNode>,
    focus_id: Option<WidgetId>,
    announcements: Vec<String>,
}

impl AccessibilityManager {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            focus_id: None,
            announcements: Vec::new(),
        }
    }

    pub fn register_node(&mut self, node: AccessibilityNode) {
        let id = node.id;

        // Update parent's children list
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                if !parent.children.contains(&id) {
                    parent.children.push(id);
                }
            }
        }

        self.nodes.insert(id, node);
    }

    pub fn unregister_node(&mut self, id: WidgetId) {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove from parent's children
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&child| child != id);
                }
            }

            // Clear focus if this node had it
            if self.focus_id == Some(id) {
                self.focus_id = None;
            }
        }
    }

    pub fn update_node_state(&mut self, id: WidgetId, state: AccessibilityState) {
        let node_info = self
            .nodes
            .get(&id)
            .map(|n| (n.label.clone(), n.role.clone(), n.state.clone()));

        if let Some(node) = self.nodes.get_mut(&id) {
            node.state = state.clone();
        }

        if let Some((label, role, old_state)) = node_info {
            // Create temporary node for announcement
            let temp_node = AccessibilityNode {
                id,
                role,
                label,
                description: None,
                bounds: Rect::default(),
                state,
                children: vec![],
                parent: None,
                actions: vec![],
            };
            self.announce_state_changes(&temp_node, &old_state);
        }
    }

    pub fn update_node_label(&mut self, id: WidgetId, label: String) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.label = label;
        }
    }

    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        self.focus_id = id;

        if let Some(id) = id {
            let node_info = self.nodes.get(&id).map(|n| {
                (
                    n.label.clone(),
                    n.role.clone(),
                    n.state.clone(),
                    n.description.clone(),
                )
            });
            if let Some((label, role, state, description)) = node_info {
                // Create a temporary node for announcement
                let temp_node = AccessibilityNode {
                    id,
                    role,
                    label,
                    description,
                    bounds: Rect::default(),
                    state,
                    children: vec![],
                    parent: None,
                    actions: vec![],
                };
                self.announce_focus(&temp_node);
            }
        }
    }

    pub fn get_focused_node(&self) -> Option<&AccessibilityNode> {
        self.focus_id.and_then(|id| self.nodes.get(&id))
    }

    pub fn announce(&mut self, message: String) {
        self.announcements.push(message);
    }

    pub fn get_announcements(&mut self) -> Vec<String> {
        std::mem::take(&mut self.announcements)
    }

    pub fn perform_action(&mut self, id: WidgetId, action: AccessibilityAction) -> bool {
        if let Some(node) = self.nodes.get(&id) {
            if node
                .actions
                .iter()
                .any(|a| std::mem::discriminant(a) == std::mem::discriminant(&action))
            {
                // Action is supported
                match &action {
                    AccessibilityAction::Click => {
                        self.announce(format!("{} clicked", node.label));
                    }
                    AccessibilityAction::Check => {
                        self.announce(format!("{} checked", node.label));
                    }
                    AccessibilityAction::Uncheck => {
                        self.announce(format!("{} unchecked", node.label));
                    }
                    AccessibilityAction::Expand => {
                        self.announce(format!("{} expanded", node.label));
                    }
                    AccessibilityAction::Collapse => {
                        self.announce(format!("{} collapsed", node.label));
                    }
                    _ => {}
                }
                return true;
            }
        }
        false
    }

    fn announce_focus(&mut self, node: &AccessibilityNode) {
        let mut announcement = node.label.clone();

        // Add role information
        match node.role {
            AccessibilityRole::Button => announcement.push_str(" button"),
            AccessibilityRole::CheckBox => announcement.push_str(" checkbox"),
            AccessibilityRole::RadioButton => announcement.push_str(" radio button"),
            AccessibilityRole::TextInput => announcement.push_str(" edit"),
            AccessibilityRole::ComboBox => announcement.push_str(" combo box"),
            AccessibilityRole::Slider => announcement.push_str(" slider"),
            _ => {}
        }

        // Add state information
        if let Some(checked) = node.state.checked {
            announcement.push_str(if checked { " checked" } else { " not checked" });
        }

        if let Some(selected) = node.state.selected {
            if selected {
                announcement.push_str(" selected");
            }
        }

        if let Some(expanded) = node.state.expanded {
            announcement.push_str(if expanded { " expanded" } else { " collapsed" });
        }

        if let Some(disabled) = node.state.disabled {
            if disabled {
                announcement.push_str(" disabled");
            }
        }

        if let Some(ref description) = node.description {
            announcement.push_str(". ");
            announcement.push_str(description);
        }

        self.announce(announcement);
    }

    fn announce_state_changes(&mut self, node: &AccessibilityNode, old_state: &AccessibilityState) {
        if node.state.checked != old_state.checked {
            if let Some(checked) = node.state.checked {
                self.announce(format!(
                    "{} {}",
                    node.label,
                    if checked { "checked" } else { "unchecked" }
                ));
            }
        }

        if node.state.expanded != old_state.expanded {
            if let Some(expanded) = node.state.expanded {
                self.announce(format!(
                    "{} {}",
                    node.label,
                    if expanded { "expanded" } else { "collapsed" }
                ));
            }
        }

        if node.state.value != old_state.value {
            if let Some(ref value) = node.state.value {
                self.announce(format!("{} {}", node.label, value));
            }
        }
    }
}

// Global accessibility manager
use std::sync::{Mutex, OnceLock};

static ACCESSIBILITY_MANAGER: OnceLock<Mutex<AccessibilityManager>> = OnceLock::new();

pub fn accessibility_manager() -> std::sync::MutexGuard<'static, AccessibilityManager> {
    ACCESSIBILITY_MANAGER
        .get_or_init(|| Mutex::new(AccessibilityManager::new()))
        .lock()
        .unwrap()
}

// Trait for widgets that support accessibility
pub trait Accessible {
    fn get_accessibility_node(&self) -> AccessibilityNode;
    fn update_accessibility(&self);
}

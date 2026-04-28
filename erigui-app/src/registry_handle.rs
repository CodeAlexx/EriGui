//! Bridge `erigui_nodes::NodeRegistry` to the widget-side
//! `NodeRegistryHandle` trait so the right-click "add node" menu (C4) can
//! query schemas and entries without erigui-widgets depending on
//! erigui-nodes.
//!
//! The widget side defines `AddMenuSchema`/`AddMenuFieldSpec`/
//! `AddMenuPortSpec` in widget-owned strings; we translate
//! `NodeSchema` → `AddMenuSchema` once per `schema()` call.

use std::sync::Arc;

use erigui_nodes::NodeRegistry;
use erigui_widgets::node_graph::add_menu::{
    AddMenuFieldSpec, AddMenuPortSpec, AddMenuSchema, NodeRegistryHandle, RegistryEntry,
};

/// Wraps `Arc<NodeRegistry>` so it satisfies the widget's
/// `NodeRegistryHandle` trait. Cheap to clone (the `Arc` shares the inner
/// registry).
pub struct RegistryHandle {
    inner: Arc<NodeRegistry>,
}

impl RegistryHandle {
    pub fn new(inner: Arc<NodeRegistry>) -> Self {
        Self { inner }
    }
}

impl NodeRegistryHandle for RegistryHandle {
    fn entries(&self) -> Vec<RegistryEntry> {
        // `NodeRegistry::by_category` is the only public iteration over
        // registered types. Flatten it back into a single list.
        let mut out = Vec::new();
        for (_cat, items) in self.inner.by_category() {
            for nt in items {
                let s = nt.schema();
                out.push(RegistryEntry {
                    type_id: nt.type_id().to_string(),
                    display_name: s.display_name.to_string(),
                    category: s.category.to_string(),
                });
            }
        }
        out
    }

    fn schema(&self, type_id: &str) -> Option<AddMenuSchema> {
        let nt = self.inner.get(type_id)?;
        let s = nt.schema();
        let inputs = s
            .inputs
            .into_iter()
            .map(|p| AddMenuPortSpec { name: p.name })
            .collect();
        let outputs = s
            .outputs
            .into_iter()
            .map(|p| AddMenuPortSpec { name: p.name })
            .collect();
        let fields = s
            .fields
            .into_iter()
            .map(|f| AddMenuFieldSpec {
                name: f.name,
                label: f.label,
                kind: f.kind,
                default: f.default,
            })
            .collect();
        Some(AddMenuSchema {
            display_name: s.display_name.to_string(),
            category: s.category.to_string(),
            inputs,
            outputs,
            fields,
        })
    }
}

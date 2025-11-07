use super::node::{ElementState, Node};
use crate::css::selectors::PseudoElement;
use scraper::Html;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Document {
    pub nodes: HashMap<usize, Node>,
    pub root: Option<usize>,
    pub(super) next_id: usize,
    pub(super) html: Option<Html>,
    pub(super) pseudo_children: HashMap<usize, HashMap<PseudoElement, usize>>,
}

// SAFETY: Document can be safely sent between threads because:
// 1. All Html accesses are protected by RwLock in the parent Zver struct
// 2. scraper::Html uses ego_tree internally which is safe
// 3. We guarantee proper synchronization through RwLock
unsafe impl Send for Document {}

// SAFETY: Document can be shared between threads (behind Arc<RwLock<>>) because:
// 1. All mutations require exclusive lock (RwLock::write)
// 2. All reads are performed under shared lock (RwLock::read)
unsafe impl Sync for Document {}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_id: 0,
            html: None,
            pseudo_children: HashMap::new(),
        }
    }

    pub fn attribute(&self, node_id: usize, name: &str) -> Option<String> {
        let node = self.nodes.get(&node_id)?;
        node.attributes.get(name).cloned()
    }

    pub fn pseudo_child_id(&self, parent_id: usize, pseudo: PseudoElement) -> Option<usize> {
        self.pseudo_children
            .get(&parent_id)
            .and_then(|children| children.get(&pseudo))
            .copied()
    }

    pub fn pseudo_children(&self, parent_id: usize) -> Option<&HashMap<PseudoElement, usize>> {
        self.pseudo_children.get(&parent_id)
    }

    /// Возвращает текущие UI-состояния элемента, если он существует.
    pub fn element_state(&self, node_id: usize) -> Option<ElementState> {
        self.nodes.get(&node_id).map(|node| node.element_state)
    }

    /// Обновляет битовое множество состояний (:hover, :focus и т.д.).
    pub fn set_element_state(
        &mut self,
        node_id: usize,
        state: ElementState,
        enabled: bool,
    ) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        if enabled {
            node.element_state.insert(state);
        } else {
            node.element_state.remove(state);
        }

        Ok(())
    }

    pub fn get_text_content(&self, node_id: usize) -> String {
        let mut text = String::new();
        self.collect_text_recursive(node_id, &mut text);
        text
    }

    pub(super) fn collect_text_recursive(&self, node_id: usize, text: &mut String) {
        if let Some(node) = self.nodes.get(&node_id) {
            if let Some(content) = &node.text_content {
                text.push_str(content);
            }
            for &child_id in &node.children {
                self.collect_text_recursive(child_id, text);
            }
        }
    }

    pub(super) fn remove_pseudo_node(&mut self, node_id: usize) {
        self.nodes.remove(&node_id);
    }

    pub(super) fn remove_all_pseudo_children(&mut self, parent_id: usize) {
        if let Some(children) = self.pseudo_children.remove(&parent_id) {
            for (_, node_id) in children {
                self.remove_pseudo_node(node_id);
            }
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

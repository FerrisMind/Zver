use super::document::Document;
use super::node::{ElementState, Node};
use crate::css::selectors::PseudoElement;
use std::collections::HashMap;

impl Document {
    pub fn sync_pseudo_elements(
        &mut self,
        pseudo_contents: &HashMap<usize, HashMap<PseudoElement, String>>,
    ) {
        let existing_parents: Vec<usize> = self.pseudo_children.keys().copied().collect();
        for parent_id in existing_parents {
            if !pseudo_contents.contains_key(&parent_id) {
                self.remove_all_pseudo_children(parent_id);
            }
        }

        for (&parent_id, pseudos) in pseudo_contents {
            if !self.nodes.contains_key(&parent_id) {
                continue;
            }

            let entry = self.pseudo_children.entry(parent_id).or_default();
            let current: Vec<PseudoElement> = entry.keys().copied().collect();
            for pseudo in current {
                if !pseudos.contains_key(&pseudo) {
                    self.remove_pseudo_child(parent_id, pseudo);
                }
            }

            for (&pseudo, content) in pseudos {
                self.upsert_pseudo_child(parent_id, pseudo, content);
            }

            if let Some(children) = self.pseudo_children.get(&parent_id)
                && children.is_empty()
            {
                self.pseudo_children.remove(&parent_id);
            }
        }
    }

    fn remove_pseudo_child(&mut self, parent_id: usize, pseudo: PseudoElement) {
        let node_id_to_remove = self
            .pseudo_children
            .get_mut(&parent_id)
            .and_then(|children| children.remove(&pseudo));

        if let Some(node_id) = node_id_to_remove {
            self.remove_pseudo_node(node_id);
        }

        if let Some(children) = self.pseudo_children.get(&parent_id)
            && children.is_empty()
        {
            self.pseudo_children.remove(&parent_id);
        }
    }

    fn upsert_pseudo_child(&mut self, parent_id: usize, pseudo: PseudoElement, content: &str) {
        let entry = self.pseudo_children.entry(parent_id).or_default();
        if let Some(&node_id) = entry.get(&pseudo) {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.text_content = Some(content.to_string());
            }
        } else {
            let node_id = self.next_id;
            self.next_id += 1;

            let node = Node {
                id: node_id,
                tag_name: None,
                attributes: HashMap::new(),
                text_content: Some(content.to_string()),
                children: Vec::new(),
                parent: Some(parent_id),
                element_state: ElementState::default(),
                pseudo_element: Some(pseudo),
            };

            self.nodes.insert(node_id, node);
            entry.insert(pseudo, node_id);
        }
    }
}

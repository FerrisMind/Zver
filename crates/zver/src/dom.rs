pub mod serialization;

use kuchikiki::{NodeData, NodeRef, traits::*};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub node_ref: NodeRef,
}

impl Node {
    pub fn is_element(&self) -> bool {
        self.tag_name.is_some()
    }

    pub fn tag_name(&self) -> Option<&str> {
        self.tag_name.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub nodes: HashMap<usize, Node>,
    pub root: Option<usize>,
    next_id: usize,
    dom_root: Option<NodeRef>,
    node_lookup: HashMap<usize, usize>,
}

unsafe impl Send for Document {}
unsafe impl Sync for Document {}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_id: 0,
            dom_root: None,
            node_lookup: HashMap::new(),
        }
    }

    fn node_ptr(node_ref: &NodeRef) -> usize {
        Rc::as_ptr(&node_ref.0) as usize
    }

    fn node_id_from_ref(&self, node_ref: &NodeRef) -> Option<usize> {
        self.node_lookup.get(&Self::node_ptr(node_ref)).copied()
    }

    pub fn node_ref(&self, node_id: usize) -> Option<NodeRef> {
        self.nodes.get(&node_id).map(|node| node.node_ref.clone())
    }

    pub fn document_root(&self) -> Option<NodeRef> {
        self.dom_root.clone()
    }

    fn select_from_node_ref(&self, node_ref: &NodeRef, selector: &str) -> Vec<usize> {
        match node_ref.select(selector) {
            Ok(selection) => selection
                .filter_map(|node| {
                    let node_ref = node.as_node();
                    self.node_id_from_ref(node_ref)
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn select_ids(&self, selector: &str) -> Vec<usize> {
        let Some(root) = self.dom_root.as_ref() else {
            return Vec::new();
        };

        self.select_from_node_ref(root, selector)
    }

    pub fn select_first_id(&self, selector: &str) -> Option<usize> {
        self.select_ids(selector).into_iter().next()
    }

    pub fn select_ids_from(&self, node_id: usize, selector: &str) -> Vec<usize> {
        let Some(node_ref) = self.node_ref(node_id) else {
            return Vec::new();
        };

        self.select_from_node_ref(&node_ref, selector)
    }

    pub fn attribute(&self, node_id: usize, name: &str) -> Option<String> {
        self.node_ref(node_id).and_then(|node_ref| {
            node_ref.as_element().and_then(|element| {
                element
                    .attributes
                    .borrow()
                    .get(name)
                    .map(|value| value.to_string())
            })
        })
    }

    pub async fn parse_html(&mut self, html: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dom = kuchikiki::parse_html().one(html);

        self.nodes.clear();
        self.root = None;
        self.next_id = 0;
        self.dom_root = Some(dom.clone());
        self.node_lookup.clear();

        let root_id = self.build_tree(&dom, None);

        let html_root_id = match self.select_first_id("html") {
            Some(id) => id,
            None => {
                let mut candidate = root_id;
                if let Some(root_node) = self.nodes.get(&root_id) {
                    if root_node.tag_name.is_none() {
                        for &child_id in &root_node.children {
                            if let Some(child_node) = self.nodes.get(&child_id)
                                && child_node.tag_name.as_deref() == Some("html")
                            {
                                candidate = child_id;
                                break;
                            }
                        }
                    } else {
                        candidate = root_id;
                    }
                }
                candidate
            }
        };

        self.root = Some(html_root_id);

        Ok(())
    }

    fn build_tree(&mut self, node_ref: &NodeRef, parent: Option<usize>) -> usize {
        let node_id = self.next_id;
        self.next_id += 1;

        let mut node = Node {
            id: node_id,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            parent,
            node_ref: node_ref.clone(),
        };

        match node_ref.data() {
            NodeData::Element(element) => {
                let name = element.name.local.to_string();
                node.tag_name = Some(name);
                for (attr_name, attr_value) in element.attributes.borrow().map.iter() {
                    node.attributes
                        .insert(attr_name.local.to_string(), attr_value.value.to_string());
                }
            }
            NodeData::Text(text) => {
                let text = text.borrow();
                node.text_content = Some(text.to_string());
            }
            _ => {}
        }

        self.node_lookup.insert(Self::node_ptr(node_ref), node_id);

        for child in node_ref.children() {
            let child_id = self.build_tree(&child, Some(node_id));
            node.children.push(child_id);
        }

        self.nodes.insert(node_id, node);
        node_id
    }

    pub fn query_selector(&self, selector: &str) -> Vec<usize> {
        self.select_ids(selector)
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<usize> {
        self.query_selector(selector)
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<usize> {
        let selector = format!("#{id}");
        self.select_first_id(&selector)
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        self.select_ids(tag)
    }

    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<usize> {
        let selector = format!(".{class_name}");
        self.select_ids(&selector)
    }

    pub fn get_text_content(&self, node_id: usize) -> String {
        self.node_ref(node_id)
            .map(|node| node.text_contents())
            .unwrap_or_default()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

use kuchikiki::{traits::TendrilSink, NodeData, NodeRef};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
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
}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_id: 0,
        }
    }

    pub async fn parse_html(&mut self, html: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dom = kuchikiki::parse_html().one(html);

        self.nodes.clear();
        self.root = None;
        self.next_id = 0;

        let root_id = self.build_tree(&dom);
        self.root = Some(root_id);

        Ok(())
    }

    fn build_tree(&mut self, node_ref: &NodeRef) -> usize {
        let node_id = self.next_id;
        self.next_id += 1;

        let mut node = Node {
            id: node_id,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            parent: None,
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

        for child in node_ref.children() {
            let child_id = self.build_tree(&child);
            if let Some(child_node) = self.nodes.get_mut(&child_id) {
                child_node.parent = Some(node_id);
            }
            node.children.push(child_id);
        }

        self.nodes.insert(node_id, node);
        node_id
    }

    pub fn query_selector(&self, selector: &str) -> Vec<usize> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| self.matches_selector(node, selector).then_some(*id))
            .collect()
    }

    fn matches_selector(&self, node: &Node, selector: &str) -> bool {
        match selector.chars().next() {
            Some('#') => node
                .attributes
                .get("id")
                .is_some_and(|id| id == &selector[1..]),
            Some('.') => node
                .attributes
                .get("class")
                .is_some_and(|class| class.split_whitespace().any(|c| c == &selector[1..])),
            _ => node.tag_name() == Some(selector),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}


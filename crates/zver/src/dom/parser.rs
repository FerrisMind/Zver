use super::document::Document;
use super::node::{ElementState, Node};
use scraper::Html;
use std::collections::HashMap;

impl Document {
    pub async fn parse_html(&mut self, html_str: &str) -> Result<(), Box<dyn std::error::Error>> {
        let html = Html::parse_document(html_str);

        self.nodes.clear();
        self.root = None;
        self.next_id = 0;
        self.pseudo_children.clear();

        // Строим дерево из scraper's Html
        let root_id = self.build_tree_from_html(&html);

        // Ищем <html> элемент
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
        self.html = Some(html);

        Ok(())
    }

    fn build_tree_from_html(&mut self, html: &Html) -> usize {
        let root = html.root_element();
        self.build_node_from_element(root, None)
    }

    fn build_node_from_element(
        &mut self,
        element: scraper::ElementRef,
        parent: Option<usize>,
    ) -> usize {
        use scraper::node::Node as ScraperNode;

        let node_id = self.next_id;
        self.next_id += 1;

        let mut node = Node {
            id: node_id,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            parent,
            element_state: ElementState::default(),
            pseudo_element: None,
        };

        // Получаем информацию об элементе
        let element_data = element.value();
        node.tag_name = Some(element_data.name().to_string());

        for (name, value) in element_data.attrs() {
            node.attributes.insert(name.to_string(), value.to_string());
        }

        // Обрабатываем детей
        for child in element.children() {
            match child.value() {
                ScraperNode::Element(_) => {
                    if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                        let child_id = self.build_node_from_element(child_elem, Some(node_id));
                        node.children.push(child_id);
                    }
                }
                ScraperNode::Text(text) => {
                    let text_node_id = self.next_id;
                    self.next_id += 1;

                    let text_node = Node {
                        id: text_node_id,
                        tag_name: None,
                        attributes: HashMap::new(),
                        text_content: Some(text.to_string()),
                        children: Vec::new(),
                        parent: Some(node_id),
                        element_state: ElementState::default(),
                        pseudo_element: None,
                    };

                    self.nodes.insert(text_node_id, text_node);
                    node.children.push(text_node_id);
                }
                _ => {}
            }
        }

        self.nodes.insert(node_id, node);
        node_id
    }
}

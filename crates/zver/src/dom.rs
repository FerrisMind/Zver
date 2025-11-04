pub mod serialization;

use kuchikiki::{NodeData, NodeRef, traits::TendrilSink};
use std::collections::HashMap;

// Подключаем scraper для продвинутого DOM querying
use scraper::{Html as ScraperHtml, Selector as ScraperSelector};

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
    scraper: Option<ScraperHtml>,
}

unsafe impl Send for Document {}
unsafe impl Sync for Document {}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_id: 0,
            scraper: None,
        }
    }

    pub async fn parse_html(&mut self, html: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Сохраняем парсинг для scraper
        self.scraper = Some(ScraperHtml::parse_document(html));

        let dom = kuchikiki::parse_html().one(html);

        self.nodes.clear();
        self.root = None;
        self.next_id = 0;

        // dom уже является корневым NodeRef
        let root_id = self.build_tree(&dom);

        // Находим настоящий корневой элемент (<html>)
        let mut html_root_id = root_id;
        if let Some(root_node) = self.nodes.get(&root_id) {
            // Если корневой узел - текстовый, ищем <html> среди детей
            if root_node.tag_name.is_none() {
                for &child_id in &root_node.children {
                    if let Some(child_node) = self.nodes.get(&child_id)
                        && child_node.tag_name.as_deref() == Some("html")
                    {
                        html_root_id = child_id;
                        break;
                    }
                }
            }
        }

        self.root = Some(html_root_id);

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
        // Используем scraper для точного поиска элементов
        if let Some(doc) = &self.scraper
            && let Ok(sel) = ScraperSelector::parse(selector)
        {
            let mut result_ids = Vec::new();

            for element in doc.select(&sel) {
                let name = element.value().name();
                let id_attr = element.value().id();
                let classes: Vec<_> = element.value().classes().collect();

                // Находим соответствующий узел в нашем дереве
                // Используем более точное сопоставление с учетом позиции в дереве
                if let Some(node_id) = self.find_matching_node(name, id_attr, &classes, &element) {
                    result_ids.push(node_id);
                }
            }

            return result_ids;
        }

        // Фоллбек: простые селекторы
        self.nodes
            .iter()
            .filter_map(|(id, node)| self.matches_selector(node, selector).then_some(*id))
            .collect()
    }

    fn find_matching_node(
        &self,
        tag_name: &str,
        id: Option<&str>,
        classes: &[&str],
        _element: &scraper::element_ref::ElementRef,
    ) -> Option<usize> {
        // Ищем узел по тегу, id и классам
        self.nodes.iter().find_map(|(node_id, node)| {
            if let Some(node_tag) = &node.tag_name
                && node_tag == tag_name
            {
                // Проверяем id
                let id_match = match id {
                    Some(expected_id) => node
                        .attributes
                        .get("id")
                        .map(|v| v == expected_id)
                        .unwrap_or(false),
                    None => true,
                };

                // Проверяем классы
                let class_match = if classes.is_empty() {
                    true
                } else {
                    node.attributes
                        .get("class")
                        .map(|cls| {
                            let existing: Vec<&str> = cls.split_whitespace().collect();
                            classes.iter().all(|c| existing.contains(c))
                        })
                        .unwrap_or(false)
                };

                if id_match && class_match {
                    return Some(*node_id);
                }
            }
            None
        })
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<usize> {
        self.query_selector(selector)
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<usize> {
        self.nodes.iter().find_map(|(node_id, node)| {
            node.attributes
                .get("id")
                .filter(|v| *v == id)
                .map(|_| *node_id)
        })
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| {
                node.tag_name
                    .as_ref()
                    .filter(|t| t.as_str() == tag)
                    .map(|_| *id)
            })
            .collect()
    }

    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<usize> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| {
                node.attributes
                    .get("class")
                    .filter(|cls| cls.split_whitespace().any(|c| c == class_name))
                    .map(|_| *id)
            })
            .collect()
    }

    pub fn get_text_content(&self, node_id: usize) -> String {
        let mut text = String::new();
        self.collect_text_recursive(node_id, &mut text);
        text
    }

    fn collect_text_recursive(&self, node_id: usize, text: &mut String) {
        if let Some(node) = self.nodes.get(&node_id) {
            if let Some(node_text) = &node.text_content {
                text.push_str(node_text);
            }
            for &child_id in &node.children {
                self.collect_text_recursive(child_id, text);
            }
        }
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

    pub fn has_scraper(&self) -> bool {
        self.scraper.is_some()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

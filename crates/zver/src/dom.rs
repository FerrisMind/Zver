use kuchikiki::{traits::TendrilSink, NodeData, NodeRef};
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
        // Если доступен scraper и селектор корректен — используем его для поиска элементов,
        // затем сопоставляем их с нашими узлами по тегу/id/class.
        if let Some(doc) = &self.scraper && let Ok(sel) = ScraperSelector::parse(selector) {
                let mut result_ids = Vec::new();
                for el in doc.select(&sel) {
                    let name = el.value().name();
                    let id_attr = el.value().id();
                    let class_iter = el.value().classes();
                    let class_set: Vec<String> = class_iter.map(|c| c.to_string()).collect();

                    // Находим первый подходящий узел (приближенное сопоставление)
                    if let Some((matched_id, _)) = self
                        .nodes
                        .iter()
                        .find(|(_nid, node)| match node.tag_name.as_deref() {
                            Some(t) if t == name => {
                                let id_ok = match id_attr {
                                    Some(idv) => node
                                        .attributes
                                        .get("id")
                                        .is_some_and(|v| v == idv),
                                    None => true,
                                };
                                let class_ok = if class_set.is_empty() {
                                    true
                                } else {
                                    node.attributes.get("class").map(|cls| {
                                        let existing: Vec<&str> = cls.split_whitespace().collect();
                                        class_set.iter().all(|c| existing.iter().any(|e| e == c))
                                    }).unwrap_or(false)
                                };
                                id_ok && class_ok
                            }
                            _ => false,
                        })
                    {
                        result_ids.push(*matched_id);
                    }
                }
                return result_ids;
        }

        // Фоллбек: простые селекторы (#id, .class, tag)
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

    pub fn has_scraper(&self) -> bool {
        self.scraper.is_some()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}


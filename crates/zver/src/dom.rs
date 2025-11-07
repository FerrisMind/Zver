pub mod serialization;

use crate::css::selectors::PseudoElement;
use bitflags::bitflags;
use scraper::{Html, Selector};
use std::collections::HashMap;

bitflags! {
    /// UI-состояния элемента, используемые в псевдоклассах.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ElementState: u16 {
        const HOVER = 1 << 0;
        const FOCUS = 1 << 1;
        const ACTIVE = 1 << 2;
        const DISABLED = 1 << 3;
        const CHECKED = 1 << 4;
        const INDETERMINATE = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub element_state: ElementState,
    pub pseudo_element: Option<PseudoElement>,
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
    html: Option<Html>,
    pseudo_children: HashMap<usize, HashMap<PseudoElement, usize>>,
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

    pub fn select_ids(&self, selector: &str) -> Vec<usize> {
        let Some(html) = self.html.as_ref() else {
            return Vec::new();
        };

        let Ok(selector) = Selector::parse(selector) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for element in html.select(&selector) {
            // Находим наш node_id по атрибутам элемента
            if let Some(id_attr) = element.attr("id")
                && let Some(&node_id) = self.nodes.iter().find_map(|(id, node)| {
                    if node.attributes.get("id") == Some(&id_attr.to_string()) {
                        Some(id)
                    } else {
                        None
                    }
                })
            {
                result.push(node_id);
                continue;
            }

            // Иначе ищем по tag_name
            let tag_name = element.value().name();
            for (&node_id, node) in &self.nodes {
                if node.tag_name.as_deref() == Some(tag_name) && !result.contains(&node_id) {
                    result.push(node_id);
                    break;
                }
            }
        }

        result
    }

    pub fn select_first_id(&self, selector: &str) -> Option<usize> {
        self.select_ids(selector).into_iter().next()
    }

    pub fn select_ids_from(&self, node_id: usize, selector: &str) -> Vec<usize> {
        // Для упрощения - выбираем из всего документа и фильтруем потомков
        self.select_ids(selector)
            .into_iter()
            .filter(|&id| {
                let mut current = self.nodes.get(&id);
                while let Some(n) = current {
                    if n.parent == Some(node_id) || n.id == node_id {
                        return true;
                    }
                    current = n.parent.and_then(|p| self.nodes.get(&p));
                }
                false
            })
            .collect()
    }

    pub fn attribute(&self, node_id: usize, name: &str) -> Option<String> {
        let node = self.nodes.get(&node_id)?;
        node.attributes.get(name).cloned()
    }

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

    pub fn pseudo_child_id(&self, parent_id: usize, pseudo: PseudoElement) -> Option<usize> {
        self.pseudo_children
            .get(&parent_id)
            .and_then(|children| children.get(&pseudo))
            .copied()
    }

    pub fn pseudo_children(&self, parent_id: usize) -> Option<&HashMap<PseudoElement, usize>> {
        self.pseudo_children.get(&parent_id)
    }

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

    fn collect_text_recursive(&self, node_id: usize, text: &mut String) {
        if let Some(node) = self.nodes.get(&node_id) {
            if let Some(content) = &node.text_content {
                text.push_str(content);
            }
            for &child_id in &node.children {
                self.collect_text_recursive(child_id, text);
            }
        }
    }

    fn remove_all_pseudo_children(&mut self, parent_id: usize) {
        if let Some(children) = self.pseudo_children.remove(&parent_id) {
            for (_, node_id) in children {
                self.remove_pseudo_node(node_id);
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

    fn remove_pseudo_node(&mut self, node_id: usize) {
        self.nodes.remove(&node_id);
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

    /// Создаёт новый элемент с заданным тегом
    pub fn create_element(&mut self, tag_name: &str) -> Result<usize, String> {
        let node_id = self.next_id;
        self.next_id += 1;

        let node = Node {
            id: node_id,
            tag_name: Some(tag_name.to_string()),
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            parent: None,
            element_state: ElementState::default(),
            pseudo_element: None,
        };

        self.nodes.insert(node_id, node);

        Ok(node_id)
    }

    /// Добавляет дочерний элемент к родителю
    pub fn append_child(&mut self, parent_id: usize, child_id: usize) -> Result<(), String> {
        // Обновляем внутренние связи
        if let Some(parent_node) = self.nodes.get_mut(&parent_id)
            && !parent_node.children.contains(&child_id)
        {
            parent_node.children.push(child_id);
        }

        if let Some(child_node) = self.nodes.get_mut(&child_id) {
            child_node.parent = Some(parent_id);
        }

        Ok(())
    }

    /// Удаляет дочерний элемент
    pub fn remove_child(&mut self, parent_id: usize, child_id: usize) -> Result<(), String> {
        // Обновляем внутренние связи
        if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
            parent_node.children.retain(|&id| id != child_id);
        }

        if let Some(child_node) = self.nodes.get_mut(&child_id) {
            child_node.parent = None;
        }

        self.remove_all_pseudo_children(child_id);

        Ok(())
    }

    /// Устанавливает атрибут элемента
    pub fn set_attribute(&mut self, node_id: usize, name: &str, value: &str) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        if node.tag_name.is_none() {
            return Err(format!("Node {} is not an element", node_id));
        }

        node.attributes.insert(name.to_string(), value.to_string());
        Ok(())
    }

    /// Получает значение атрибута
    pub fn get_attribute(&self, node_id: usize, name: &str) -> Option<String> {
        self.attribute(node_id, name)
    }

    /// Устанавливает текстовое содержимое элемента
    pub fn set_text_content(&mut self, node_id: usize, text: &str) -> Result<(), String> {
        // Получаем детей для удаления
        let children_to_remove: Vec<usize> = self
            .nodes
            .get(&node_id)
            .map(|node| node.children.clone())
            .unwrap_or_default();

        // Удаляем старых детей
        for child_id in children_to_remove {
            self.nodes.remove(&child_id);
        }

        // Создаем новый текстовый узел
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

        if let Some(node) = self.nodes.get_mut(&node_id) {
            let old_children = std::mem::take(&mut node.children);
            for child_id in old_children {
                if let Some(child_node) = self.nodes.get_mut(&child_id) {
                    child_node.parent = None;
                }
            }
        }

        Ok(())
    }

    /// Получает имя тега элемента
    pub fn get_tag_name(&self, node_id: usize) -> Option<String> {
        self.nodes
            .get(&node_id)
            .and_then(|node| node.tag_name.clone())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

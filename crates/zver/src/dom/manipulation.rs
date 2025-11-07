use super::document::Document;
use super::node::{ElementState, Node};
use std::collections::HashMap;

impl Document {
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

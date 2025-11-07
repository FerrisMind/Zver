use super::document::Document;
use scraper::Selector;

impl Document {
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
}

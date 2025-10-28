use super::Document;

/// Находит HTML элемент в DOM дереве
pub fn find_html_element(dom: &Document, start_node: usize) -> usize {
    // Рекурсивно ищем узел с тегом "html"
    if let Some(node) = dom.nodes.get(&start_node) {
        if node.tag_name.as_deref() == Some("html") {
            return start_node;
        }
        // Проверяем дочерние узлы
        for &child_id in &node.children {
            let found = find_html_element(dom, child_id);
            if found != usize::MAX {
                return found;
            }
        }
    }
    usize::MAX // Не найден
}

/// Сериализует DOM узел в HTML строку
pub fn serialize_node(dom: &Document, node_id: usize, html: &mut String) {
    if let Some(node) = dom.nodes.get(&node_id) {
        if let Some(tag) = &node.tag_name {
            html.push('<');
            html.push_str(tag);
            for (attr, value) in &node.attributes {
                html.push(' ');
                html.push_str(attr);
                html.push('=');
                html.push('"');
                html.push_str(value);
                html.push('"');
            }
            html.push('>');

            for &child in &node.children {
                serialize_node(dom, child, html);
            }

            html.push_str("</");
            html.push_str(tag);
            html.push('>');
        } else if let Some(text) = &node.text_content {
            html.push_str(text);
        }
    }
}

/// Сериализует весь DOM в HTML строку
pub fn serialize_dom(dom: &Document) -> String {
    let mut html = String::new();
    if let Some(root) = dom.root {
        let html_root = find_html_element(dom, root);
        serialize_node(dom, html_root, &mut html);
    }
    html
}

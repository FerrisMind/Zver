use super::Document;

/// Возвращает идентификатор элемента `<html>` или `usize::MAX`, если он не найден.
pub fn find_html_element(dom: &Document, _start_node: usize) -> usize {
    dom.select_first_id("html")
        .or(dom.root)
        .unwrap_or(usize::MAX)
}

/// Сериализует DOM узел в HTML строку
pub fn serialize_node(dom: &Document, node_id: usize, html: &mut String) {
    if let Some(node) = dom.nodes.get(&node_id) {
        if let Some(tag) = &node.tag_name {
            html.push_str(&format!("<{}", tag));
            for (name, value) in &node.attributes {
                html.push_str(&format!(r#" {}="{}""#, name, value));
            }
            html.push('>');

            for &child_id in &node.children {
                serialize_node(dom, child_id, html);
            }

            html.push_str(&format!("</{}>", tag));
        } else if let Some(text) = &node.text_content {
            html.push_str(text);
        }
    }
}

/// Сериализует весь DOM в HTML строку.
pub fn serialize_dom(dom: &Document) -> String {
    let mut html = String::new();
    if let Some(root_id) = dom.root {
        serialize_node(dom, root_id, &mut html);
    }
    html
}

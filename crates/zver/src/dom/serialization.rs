use super::Document;

/// Возвращает идентификатор элемента `<html>` или `usize::MAX`, если он не найден.
pub fn find_html_element(dom: &Document, _start_node: usize) -> usize {
    dom.select_first_id("html")
        .or(dom.root)
        .unwrap_or(usize::MAX)
}

/// Сериализует DOM узел в HTML строку, используя возможности kuchikiki.
pub fn serialize_node(dom: &Document, node_id: usize, html: &mut String) {
    if let Some(node_ref) = dom.node_ref(node_id) {
        html.push_str(&node_ref.to_string());
    }
}

/// Сериализует весь DOM в HTML строку.
pub fn serialize_dom(dom: &Document) -> String {
    if let Some(root_id) = dom.root
        && let Some(node_ref) = dom.node_ref(root_id)
    {
        return node_ref.to_string();
    }

    dom.document_root()
        .map(|node| node.to_string())
        .unwrap_or_default()
}

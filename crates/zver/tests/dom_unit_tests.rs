/// Unit tests for DOM module
/// Tests cover: selectors, attributes, text content, tree navigation
use zver::dom::Document;

#[tokio::test]
async fn test_parse_simple_html() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>Test</title></head>
            <body><p>Hello</p></body>
        </html>
    "#;

    let result = doc.parse_html(html).await;
    assert!(result.is_ok());
    assert!(doc.root.is_some());
}

#[tokio::test]
async fn test_get_element_by_id() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div id="main">Main content</div>
                <div id="sidebar">Sidebar</div>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    // Test existing ID
    let main_id = doc.get_element_by_id("main");
    assert!(main_id.is_some());

    let main_node = doc.nodes.get(&main_id.unwrap()).unwrap();
    assert_eq!(main_node.tag_name(), Some("div"));

    // Test non-existing ID
    let missing = doc.get_element_by_id("nonexistent");
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_get_elements_by_class_name() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div class="item">Item 1</div>
                <div class="item">Item 2</div>
                <div class="other">Other</div>
                <p class="item">Item 3</p>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    let items = doc.get_elements_by_class_name("item");
    assert_eq!(items.len(), 3, "Should find 3 elements with class 'item'");

    let other = doc.get_elements_by_class_name("other");
    assert_eq!(other.len(), 1, "Should find 1 element with class 'other'");

    let missing = doc.get_elements_by_class_name("nonexistent");
    assert_eq!(
        missing.len(),
        0,
        "Should find 0 elements with non-existent class"
    );
}

#[tokio::test]
async fn test_get_elements_by_tag_name() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>Test</title></head>
            <body>
                <p>Paragraph 1</p>
                <div><p>Paragraph 2</p></div>
                <p>Paragraph 3</p>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    let paragraphs = doc.get_elements_by_tag_name("p");
    assert_eq!(paragraphs.len(), 3, "Should find 3 <p> elements");

    let divs = doc.get_elements_by_tag_name("div");
    assert_eq!(divs.len(), 1, "Should find 1 <div> element");

    let spans = doc.get_elements_by_tag_name("span");
    assert_eq!(spans.len(), 0, "Should find 0 <span> elements");
}

#[tokio::test]
async fn test_attribute_access() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <a id="link1" href="https://example.com" target="_blank">Link</a>
                <img id="image1" src="image.png" alt="Test Image">
                <div id="noattr">No attributes</div>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    // Test href attribute
    let link_id = doc.get_element_by_id("link1").unwrap();
    let href = doc.attribute(link_id, "href");
    assert_eq!(href, Some("https://example.com".to_string()));

    let target = doc.attribute(link_id, "target");
    assert_eq!(target, Some("_blank".to_string()));

    // Test image alt attribute
    let img_id = doc.get_element_by_id("image1").unwrap();
    let alt = doc.attribute(img_id, "alt");
    assert_eq!(alt, Some("Test Image".to_string()));

    // Test non-existent attribute
    let missing = doc.attribute(link_id, "nonexistent");
    assert!(missing.is_none());

    // Test element with no attributes
    let div_id = doc.get_element_by_id("noattr").unwrap();
    let div_attr = doc.attribute(div_id, "class");
    assert!(div_attr.is_none());
}

#[tokio::test]
async fn test_get_text_content() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div id="simple">Simple text</div>
                <div id="nested">
                    <span>Nested</span> text
                </div>
                <div id="empty"></div>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    // Test simple text
    let simple_id = doc.get_element_by_id("simple").unwrap();
    let text = doc.get_text_content(simple_id);
    assert!(text.contains("Simple text"));

    // Test nested text
    let nested_id = doc.get_element_by_id("nested").unwrap();
    let nested_text = doc.get_text_content(nested_id);
    assert!(nested_text.contains("Nested"));
    assert!(nested_text.contains("text"));

    // Test empty element
    let empty_id = doc.get_element_by_id("empty").unwrap();
    let empty_text = doc.get_text_content(empty_id);
    assert!(empty_text.trim().is_empty() || empty_text.is_empty());
}

#[tokio::test]
async fn test_query_selector() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div class="container">
                    <p class="text">Paragraph 1</p>
                    <p class="text">Paragraph 2</p>
                </div>
                <div class="sidebar">
                    <p>Sidebar paragraph</p>
                </div>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    // Test class selector
    let text_paragraphs = doc.query_selector(".text");
    assert_eq!(text_paragraphs.len(), 2);

    // Test descendant selector
    let container_paragraphs = doc.query_selector(".container p");
    assert_eq!(container_paragraphs.len(), 2);

    // Test all paragraphs
    let all_paragraphs = doc.query_selector("p");
    assert_eq!(all_paragraphs.len(), 3);
}

#[tokio::test]
async fn test_parent_child_navigation() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div id="parent"><p id="child1">Child 1</p><p id="child2">Child 2</p></div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let parent_id = doc.get_element_by_id("parent").unwrap();
    let parent_node = doc.nodes.get(&parent_id).unwrap();

    // Test children count - HTML parser includes text nodes (whitespace), so filter for elements only
    let element_children: Vec<_> = parent_node
        .children
        .iter()
        .filter(|&&id| doc.nodes.get(&id).map(|n| n.is_element()).unwrap_or(false))
        .copied()
        .collect();
    assert_eq!(
        element_children.len(),
        2,
        "Parent should have 2 element children"
    );

    // Test child access
    let child1_id = doc.get_element_by_id("child1").unwrap();
    let child1_node = doc.nodes.get(&child1_id).unwrap();

    // Test parent reference
    assert_eq!(
        child1_node.parent,
        Some(parent_id),
        "Child should reference parent"
    );

    // Test sibling navigation through parent - find other element siblings
    let siblings: Vec<_> = element_children
        .iter()
        .filter(|&&id| id != child1_id)
        .copied()
        .collect();
    assert_eq!(siblings.len(), 1, "Should find 1 element sibling");
}

#[tokio::test]
async fn test_select_ids_from() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div id="outer">
                    <div class="inner">
                        <p>Inner paragraph</p>
                    </div>
                </div>
                <div class="inner">
                    <p>Outer paragraph</p>
                </div>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    let outer_id = doc.get_element_by_id("outer").unwrap();

    // Select only from within outer div
    let inner_paragraphs = doc.select_ids_from(outer_id, "p");
    assert_eq!(
        inner_paragraphs.len(),
        1,
        "Should find 1 paragraph inside outer div"
    );

    // Test that global selector finds both
    let all_paragraphs = doc.query_selector("p");
    assert_eq!(all_paragraphs.len(), 2, "Should find 2 paragraphs globally");
}

#[tokio::test]
async fn test_malformed_html_handling() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <div>Unclosed div
                <p>Paragraph</p>
            </body>
        </html>
    "#;

    // Should not panic, scraper handles malformed HTML gracefully
    let result = doc.parse_html(html).await;
    assert!(result.is_ok());

    // Should still be able to query
    let paragraphs = doc.get_elements_by_tag_name("p");
    assert_eq!(paragraphs.len(), 1);
}

#[tokio::test]
async fn test_empty_document() {
    let mut doc = Document::new();
    let html = "";

    let result = doc.parse_html(html).await;
    assert!(result.is_ok());

    // Empty document should have no elements
    let all_divs = doc.get_elements_by_tag_name("div");
    assert_eq!(all_divs.len(), 0);
}

#[tokio::test]
async fn test_complex_selectors() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <ul id="list">
                    <li class="item active">Item 1</li>
                    <li class="item">Item 2</li>
                    <li class="item active">Item 3</li>
                </ul>
            </body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    // Test multiple classes
    let active_items = doc.query_selector(".item.active");
    assert_eq!(active_items.len(), 2, "Should find 2 active items");

    // Test descendant selector with ID
    let list_items = doc.query_selector("#list li");
    assert_eq!(list_items.len(), 3, "Should find 3 list items");
}

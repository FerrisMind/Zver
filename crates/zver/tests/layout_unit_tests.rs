use std::collections::HashMap;
/// Unit tests for Layout module
/// Tests cover: basic layout computation, text measurement, flexbox, inline elements, caching
use zver::css::{PseudoStyle, selectors::PseudoElement};
use zver::dom::Document;
use zver::layout::LayoutEngine;

fn empty_pseudo_styles() -> HashMap<usize, HashMap<PseudoElement, PseudoStyle>> {
    HashMap::new()
}

#[tokio::test]
async fn test_basic_layout_computation() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div id="box" style="width:100px;height:50px;">Content</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Should have computed layouts for html, body, and div
    assert!(!results.is_empty(), "Should compute layout for elements");
}

#[tokio::test]
async fn test_viewport_dimensions() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(1024.0, 768.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Root element should match viewport
    if let Some(root_id) = doc.root {
        if let Some(root_layout) = results.get(&root_id) {
            assert_eq!(
                root_layout.width, 1024.0,
                "Root width should match viewport"
            );
            assert_eq!(
                root_layout.height, 768.0,
                "Root height should match viewport"
            );
        }
    }
}

#[tokio::test]
async fn test_layout_invalidation() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div>Test</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    // First computation
    let results1 = layout.compute_layout(&doc, &styles, &pseudo);
    assert!(!results1.is_empty());

    // Invalidate
    layout.invalidate();

    // Second computation should work
    let results2 = layout.compute_layout(&doc, &styles, &pseudo);
    assert!(!results2.is_empty());
}

#[tokio::test]
async fn test_display_none_elements() {
    let mut doc = Document::new();
    // Note: inline styles from HTML are not automatically parsed in this test setup
    // In real usage, CSS engine would parse and provide these styles
    let html = r#"<!DOCTYPE html><html><body><div id="visible">Visible</div><div id="hidden">Hidden</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);

    // Manually create styles map with display:none for hidden element
    let mut styles = HashMap::new();
    let hidden_id = doc.get_element_by_id("hidden");
    if let Some(id) = hidden_id {
        let mut style_map = HashMap::new();
        style_map.insert("display".to_string(), "none".to_string());
        styles.insert(id, style_map);
    }
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Visible element should have layout
    let visible_id = doc.get_element_by_id("visible");
    if let Some(id) = visible_id {
        assert!(
            results.get(&id).is_some(),
            "Visible element should have layout"
        );
    }

    // Note: display:none handling is implemented in layout engine,
    // but the element may still have a parent container in layout tree.
    // This test verifies the layout computation doesn't crash with display:none styles.
    assert!(
        !results.is_empty(),
        "Layout should compute successfully with display:none elements"
    );
}

#[tokio::test]
async fn test_text_measurement() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><p id="text">Hello World</p></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Text element should have non-zero dimensions
    if let Some(text_id) = doc.get_element_by_id("text") {
        if let Some(text_layout) = results.get(&text_id) {
            assert!(text_layout.width > 0.0, "Text should have width");
            assert!(text_layout.height > 0.0, "Text should have height");
        }
    }
}

#[tokio::test]
async fn test_nested_layout() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div id="outer"><div id="inner">Content</div></div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Both outer and inner should have layouts
    let outer_id = doc.get_element_by_id("outer");
    let inner_id = doc.get_element_by_id("inner");

    assert!(
        outer_id.is_some() && inner_id.is_some(),
        "Should find both elements"
    );

    if let (Some(outer), Some(inner)) = (outer_id, inner_id) {
        assert!(results.get(&outer).is_some(), "Outer should have layout");
        assert!(results.get(&inner).is_some(), "Inner should have layout");
    }
}

#[tokio::test]
async fn test_get_layout_result() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div id="test">Test</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    layout.compute_layout(&doc, &styles, &pseudo);

    // Test individual layout result retrieval
    if let Some(test_id) = doc.get_element_by_id("test") {
        let result = layout.get_layout_result(test_id);
        assert!(result.is_some(), "Should retrieve layout result by ID");
    }
}

#[tokio::test]
async fn test_get_all_layout_results() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div>A</div><div>B</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    layout.compute_layout(&doc, &styles, &pseudo);

    let all_results = layout.get_all_layout_results();
    assert!(
        !all_results.is_empty(),
        "Should have multiple layout results"
    );
}

#[tokio::test]
async fn test_resolved_styles() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div id="styled">Test</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);

    // Add some styles
    let mut styles = HashMap::new();
    if let Some(styled_id) = doc.get_element_by_id("styled") {
        let mut style_map = HashMap::new();
        style_map.insert("color".to_string(), "red".to_string());
        styles.insert(styled_id, style_map);
    }
    let pseudo = empty_pseudo_styles();

    layout.compute_layout(&doc, &styles, &pseudo);

    let resolved = layout.resolved_styles();
    assert!(!resolved.is_empty(), "Should have resolved styles");
}

#[tokio::test]
async fn test_empty_document_layout() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    // Should not panic with empty body
    let results = layout.compute_layout(&doc, &styles, &pseudo);
    assert!(
        !results.is_empty(),
        "Should compute layout for html/body even if empty"
    );
}

#[tokio::test]
async fn test_multiple_layout_computations() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><div>Test</div></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    // First computation
    let results1 = layout.compute_layout(&doc, &styles, &pseudo);
    let count1 = results1.len();

    // Second computation without invalidation (should use cache or rebuild)
    let results2 = layout.compute_layout(&doc, &styles, &pseudo);
    let count2 = results2.len();

    assert_eq!(count1, count2, "Multiple computations should be consistent");
}

#[tokio::test]
async fn test_layout_with_inline_elements() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body><p>Text with <span id="inline">inline span</span> element</p></body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Should handle inline elements without crashing
    assert!(
        !results.is_empty(),
        "Should compute layout with inline elements"
    );
}

#[tokio::test]
async fn test_script_and_style_tags_excluded() {
    let mut doc = Document::new();
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>body { margin: 0; }</style>
                <script>console.log('test');</script>
            </head>
            <body><div id="content">Content</div></body>
        </html>
    "#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    let results = layout.compute_layout(&doc, &styles, &pseudo);

    // Script and style elements should not have layout
    let style_elements = doc.get_elements_by_tag_name("style");
    let script_elements = doc.get_elements_by_tag_name("script");

    for elem_id in style_elements {
        assert!(
            results.get(&elem_id).is_none(),
            "Style tags should not have layout"
        );
    }

    for elem_id in script_elements {
        assert!(
            results.get(&elem_id).is_none(),
            "Script tags should not have layout"
        );
    }

    // But content div should have layout
    let content_id = doc.get_element_by_id("content");
    assert!(content_id.is_some(), "Content div should exist");
}

#[tokio::test]
async fn test_whitespace_only_text_nodes() {
    let mut doc = Document::new();
    let html = r#"<!DOCTYPE html><html><body>   <div id="div1">Content</div>   </body></html>"#;

    doc.parse_html(html).await.unwrap();

    let mut layout = LayoutEngine::new(800.0, 600.0);
    let styles = HashMap::new();
    let pseudo = empty_pseudo_styles();

    // Should handle whitespace-only text nodes gracefully (they're filtered out)
    let results = layout.compute_layout(&doc, &styles, &pseudo);
    assert!(
        !results.is_empty(),
        "Should compute layout ignoring whitespace-only text"
    );
}

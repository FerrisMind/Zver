//! Unit-тесты для Фазы 3: Псевдоклассы и псевдоэлементы

use zver::css::{selectors::PseudoElement, StyleEngine};
use zver::dom::{Document, ElementState};

#[tokio::test]
async fn test_structural_pseudo_classes() {
    let html = r#"
        <html>
            <body>
                <ul>
                    <li id="first">One</li>
                    <li id="second">Two</li>
                    <li id="third">Three</li>
                    <li id="fourth">Four</li>
                </ul>
            </body>
        </html>
    "#;

    let css = r#"
        li:first-child { color: red; }
        li:last-child { color: #0a141e; }
        li:nth-child(2) { color: #008000; }
    "#;

    let mut doc = Document::new();
    doc.parse_html(html).await.unwrap();

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    assert_eq!(
        style_value(&doc, &engine, "first", "color"),
        Some("rgba(255, 0, 0, 1)".to_string()),
        ":first-child must color the first list item"
    );
    assert_eq!(
        style_value(&doc, &engine, "second", "color"),
        Some("rgba(0, 128, 0, 1)".to_string()),
        ":nth-child(2) should match the second item"
    );
    assert_eq!(
        style_value(&doc, &engine, "fourth", "color"),
        Some("rgba(10, 20, 30, 1)".to_string()),
        ":last-child should override the trailing item"
    );
}


#[tokio::test]
async fn test_state_pseudo_classes() {
    let html = r#"
        <html>
            <body>
                <button id="hover-btn" class="hover-test">Hover</button>
                <button id="focus-btn" class="focus-test">Focus</button>
                <button id="active-btn" class="active-test">Active</button>
                <button id="disabled-btn" disabled>Disabled</button>
                <input id="check-input" class="check-test" type="checkbox" checked />
            </body>
        </html>
    "#;

    let css = r#"
        .hover-test:hover { color: #c80000; }
        .focus-test:focus { color: #0000c8; }
        .active-test:active { color: #009600; }
        button:disabled { background-color: #646464; }
        .check-test:checked { color: #0a0a0a; }
    "#;

    let mut doc = Document::new();
    doc.parse_html(html).await.unwrap();

    let hover_id = doc.get_element_by_id("hover-btn").unwrap();
    doc.set_element_state(hover_id, ElementState::HOVER, true).unwrap();
    let focus_id = doc.get_element_by_id("focus-btn").unwrap();
    doc.set_element_state(focus_id, ElementState::FOCUS, true).unwrap();
    let active_id = doc.get_element_by_id("active-btn").unwrap();
    doc.set_element_state(active_id, ElementState::ACTIVE, true).unwrap();

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    assert_eq!(
        style_value(&doc, &engine, "hover-btn", "color"),
        Some("rgba(200, 0, 0, 1)".to_string()),
        ":hover should react to ElementState::HOVER"
    );
    assert_eq!(
        style_value(&doc, &engine, "focus-btn", "color"),
        Some("rgba(0, 0, 200, 1)".to_string()),
        ":focus should read ElementState::FOCUS"
    );
    assert_eq!(
        style_value(&doc, &engine, "active-btn", "color"),
        Some("rgba(0, 150, 0, 1)".to_string()),
        ":active styling requires ElementState::ACTIVE"
    );
    assert_eq!(
        style_value(&doc, &engine, "disabled-btn", "background-color"),
        Some("rgba(100, 100, 100, 1)".to_string()),
        ":disabled should match via attribute"
    );
    assert_eq!(
        style_value(&doc, &engine, "check-input", "color"),
        Some("rgba(10, 10, 10, 1)".to_string()),
        ":checked should respond to the attribute"
    );
}


#[tokio::test]
async fn test_pseudo_elements() {
    let html = r#"
        <html>
            <body>
                <div id="banner" class="decorated">Hi</div>
            </body>
        </html>
    "#;

    let css = r#"
        .decorated::before { content: "[["; color: red; }
        .decorated::after { content: "]]"; color: blue; }
    "#;

    let mut doc = Document::new();
    doc.parse_html(html).await.unwrap();

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    let banner_id = doc.get_element_by_id("banner").unwrap();
    let pseudo_styles = engine
        .pseudo_element_styles
        .get(&banner_id)
        .expect("pseudo elements missing");

    let before_style = pseudo_styles
        .get(&PseudoElement::Before)
        .expect("::before style missing");
    assert_eq!(before_style.content, "[[");
    assert_eq!(
        before_style.properties.get("color").map(String::as_str),
        Some("rgba(255, 0, 0, 1)")
    );

    let after_style = pseudo_styles
        .get(&PseudoElement::After)
        .expect("::after style missing");
    assert_eq!(after_style.content, "]]");
    assert_eq!(
        after_style.properties.get("color").map(String::as_str),
        Some("rgba(0, 0, 255, 1)")
    );

    let contents = engine.pseudo_element_contents();
    doc.sync_pseudo_elements(&contents);

    let before_node = doc
        .pseudo_child_id(banner_id, PseudoElement::Before)
        .expect("missing ::before node");
    assert_eq!(
        doc.nodes.get(&before_node).and_then(|node| node.text_content.as_deref()),
        Some("[[")
    );

    let after_node = doc
        .pseudo_child_id(banner_id, PseudoElement::After)
        .expect("missing ::after node");
    assert_eq!(
        doc.nodes.get(&after_node).and_then(|node| node.text_content.as_deref()),
        Some("]]")
    );
}


#[tokio::test]
async fn test_nth_child_formula() {
    let html = r#"
        <html>
            <body>
                <ul>
                    <li id="item-1">Item 1</li>
                    <li id="item-2">Item 2</li>
                    <li id="item-3">Item 3</li>
                    <li id="item-4">Item 4</li>
                    <li id="item-5">Item 5</li>
                    <li id="item-6">Item 6</li>
                </ul>
            </body>
        </html>
    "#;

    let css = r#"
        li:nth-child(2n+1) { color: red; }
        li:nth-child(3n) { background-color: blue; }
        li:nth-child(4n+2) { color: #0a0a0a; }
    "#;

    let mut doc = Document::new();
    doc.parse_html(html).await.unwrap();

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    assert_eq!(
        style_value(&doc, &engine, "item-1", "color"),
        Some("rgba(255, 0, 0, 1)".to_string()),
        ":nth-child(2n+1) should hit the first item"
    );
    assert_eq!(
        style_value(&doc, &engine, "item-3", "background-color"),
        Some("rgba(0, 0, 255, 1)".to_string()),
        ":nth-child(3n) should paint multiples of three"
    );
    assert_eq!(
        style_value(&doc, &engine, "item-6", "color"),
        Some("rgba(10, 10, 10, 1)".to_string()),
        ":nth-child(4n+2) should override later items"
    );
}


#[tokio::test]
async fn test_pseudo_class_combination() {
    let html = r#"
        <html>
            <body>
                <ul>
                    <li id="combo-first">First</li>
                    <li>Second</li>
                </ul>
                <input id="combo-input" type="text" />
            </body>
        </html>
    "#;

    let css = r#"
        li:first-child:hover { color: blue; }
        input:focus:active { background-color: #ffa500; }
    "#;

    let mut doc = Document::new();
    doc.parse_html(html).await.unwrap();

    let first_id = doc.get_element_by_id("combo-first").unwrap();
    doc.set_element_state(first_id, ElementState::HOVER, true).unwrap();
    let input_id = doc.get_element_by_id("combo-input").unwrap();
    doc.set_element_state(input_id, ElementState::FOCUS, true).unwrap();
    doc.set_element_state(input_id, ElementState::ACTIVE, true).unwrap();

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    assert_eq!(
        style_value(&doc, &engine, "combo-first", "color"),
        Some("rgba(0, 0, 255, 1)".to_string()),
        ":first-child:hover should combine structural and state filters"
    );
    assert_eq!(
        style_value(&doc, &engine, "combo-input", "background-color"),
        Some("rgba(255, 165, 0, 1)".to_string()),
        ":focus:active should require both dynamic states"
    );
}


#[tokio::test]
async fn test_structural_pseudo_application() {
    let mut doc = Document::new();
    let html = r#"
        <html>
            <body>
                <ul>
                    <li id="first">A</li>
                    <li id="second">B</li>
                </ul>
            </body>
        </html>
    "#;
    doc.parse_html(html).await.unwrap();

    let css = r#"
        li:first-child { color: red; }
        li:last-child { color: blue; }
    "#;

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    let first_id = doc.select_ids("#first").into_iter().next().unwrap();
    let second_id = doc.select_ids("#second").into_iter().next().unwrap();

    let first_color = engine
        .computed_styles
        .get(&first_id)
        .and_then(|map| map.get("color"));
    assert_eq!(first_color, Some(&"rgba(255, 0, 0, 1)".to_string()));

    let second_color = engine
        .computed_styles
        .get(&second_id)
        .and_then(|map| map.get("color"));
    assert_eq!(second_color, Some(&"rgba(0, 0, 255, 1)".to_string()));
}

#[tokio::test]
async fn test_pseudo_element_nodes() {
    let mut doc = Document::new();
    let html = r#"
        <html>
            <body>
                <div id="banner" class="decorated">Hi</div>
            </body>
        </html>
    "#;
    doc.parse_html(html).await.unwrap();

    let css = r#"
        .decorated::before { content: "<-"; }
        .decorated::after { content: "->"; }
        .decorated { color: black; }
    "#;

    let mut engine = StyleEngine::new();
    engine.parse_css(css).unwrap();
    engine.apply_styles(&doc).unwrap();

    let banner_id = doc.select_ids("#banner").into_iter().next().unwrap();
    let pseudo_styles = engine
        .pseudo_element_styles
        .get(&banner_id)
        .expect("missing pseudo styles");
    assert_eq!(
        pseudo_styles.get(&PseudoElement::Before).map(|style| style.content.as_str()),
        Some("<-")
    );
    assert_eq!(
        pseudo_styles.get(&PseudoElement::After).map(|style| style.content.as_str()),
        Some("->")
    );

    let pseudo_contents = engine.pseudo_element_contents();
    doc.sync_pseudo_elements(&pseudo_contents);

    let before_id = doc
        .pseudo_child_id(banner_id, PseudoElement::Before)
        .expect("before pseudo node missing");
    let after_id = doc
        .pseudo_child_id(banner_id, PseudoElement::After)
        .expect("after pseudo node missing");

    assert_eq!(
        doc.nodes.get(&before_id).and_then(|node| node.text_content.as_deref()),
        Some("<-")
    );
    assert_eq!(
        doc.nodes.get(&after_id).and_then(|node| node.text_content.as_deref()),
        Some("->")
    );
}

fn style_value(
    doc: &Document,
    engine: &StyleEngine,
    element_id: &str,
    property: &str,
) -> Option<String> {
    let node_id = doc.get_element_by_id(element_id)?;
    engine
        .computed_styles
        .get(&node_id)
        .and_then(|map| map.get(property))
        .cloned()
}


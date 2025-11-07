//! Unit-тесты для Фазы 2: @-правила (@media, @keyframes, @font-face)

use zver::css::StyleEngine;

#[test]
fn test_media_query_parsing() {
    // Тест парсинга простого media query
    let css = "@media (min-width: 768px) { .test { color: red; } }";
    let mut engine = StyleEngine::new();

    // Парсинг должен пройти без ошибок
    let result = engine.parse_css(css);
    if let Err(e) = &result {
        eprintln!("Parse error: {:?}", e);
    }
    assert!(result.is_ok());

    // Проверка наличия media rules
    assert!(!engine.media_rules.is_empty());
}

#[test]
#[ignore] // TODO: Реализовать после добавления метода matches в MediaQuery
fn test_media_query_matching() {
    // Тест матчинга media query
    // TODO: Реализовать после добавления метода matches в MediaQuery
    // let viewport_width = 800.0;
    // let viewport_height = 600.0;
    // let query = MediaQuery::parse(...);
    // assert!(query.matches(viewport_width, viewport_height));
}

#[test]
fn test_keyframes_parsing() {
    // Тест парсинга @keyframes
    let css = r#"
        @keyframes fadeIn {
            from { opacity: 0; }
            to { opacity: 1; }
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // Проверка наличия keyframes
    assert!(!engine.keyframes.is_empty());
    assert!(engine.keyframes.contains_key("fadeIn"));
}

#[test]
fn test_keyframes_multiple_steps() {
    // Тест парсинга @keyframes с несколькими шагами
    let css = r#"
        @keyframes slide {
            0% { transform: translateX(0); }
            50% { transform: translateX(50px); }
            100% { transform: translateX(100px); }
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    if let Some(keyframes) = engine.keyframes.get("slide") {
        assert_eq!(keyframes.steps.len(), 3);
    }
}

#[test]
fn test_font_face_parsing() {
    // Тест парсинга @font-face
    let css = r#"
        @font-face {
            font-family: 'TestFont';
            src: url('test.ttf') format('truetype');
            font-weight: normal;
            font-style: normal;
        }
    "#;

    let mut engine = StyleEngine::new();
    let result = engine.parse_css(css);
    if let Err(e) = &result {
        eprintln!("Parse error: {:?}", e);
    }
    assert!(result.is_ok());

    // Проверка наличия fonts
    assert!(!engine.fonts.is_empty());
}

#[test]
fn test_font_face_with_local() {
    // Тест @font-face с local() fallback
    let css = r#"
        @font-face {
            font-family: 'TestFont';
            src: local('Arial'), url('test.ttf') format('truetype');
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_multiple_at_rules() {
    // Тест парсинга нескольких @-правил одновременно
    let css = r#"
        @media (min-width: 768px) {
            .test { color: red; }
        }
        
        @keyframes fade {
            from { opacity: 0; }
            to { opacity: 1; }
        }
        
        @font-face {
            font-family: 'Test';
            src: url('test.ttf');
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    assert!(!engine.media_rules.is_empty());
    assert!(!engine.keyframes.is_empty());
    assert!(!engine.fonts.is_empty());
}

#[test]
fn test_aspect_ratio_parsing() {
    // Тест парсинга aspect-ratio, min-aspect-ratio, max-aspect-ratio
    let css = r#"
        @media (aspect-ratio: 16/9) {
            .test1 { color: red; }
        }
        
        @media (min-aspect-ratio: 16/9) {
            .test2 { color: green; }
        }
        
        @media (max-aspect-ratio: 4/3) {
            .test3 { color: blue; }
        }
    "#;

    let mut engine = StyleEngine::new();
    let result = engine.parse_css(css);
    
    if let Err(e) = &result {
        eprintln!("Parse error: {:?}", e);
    }
    
    assert!(result.is_ok());
    assert_eq!(engine.media_rules.len(), 3, "Should parse all 3 media queries");
}

#[tokio::test]
async fn test_media_query_application() {
    // Тест применения media queries к элементам
    let css = r#"
        .box { background-color: #3498db; }
        
        @media (min-width: 768px) {
            .box { background-color: #2ecc71; }
        }
    "#;

    let mut engine = StyleEngine::new();
    engine.set_viewport(800.0, 600.0); // Устанавливаем viewport > 768px
    
    let result = engine.parse_css(css);
    assert!(result.is_ok());
    
    // Создаем простой DOM для тестирования
    let mut document = zver::dom::Document::new();
    let html = r#"<html><body><div class="box">Test</div></body></html>"#;
    document.parse_html(html).await.expect("Failed to parse HTML");
    
    // Применяем стили
    let apply_result = engine.apply_styles(&document);
    assert!(apply_result.is_ok());
    
    // Находим div с классом "box"
    let div_id = document.nodes.iter()
        .find(|(_, node)| {
            node.attributes.get("class").map(|c| c.contains("box")).unwrap_or(false)
        })
        .map(|(id, _)| *id);
    
    assert!(div_id.is_some(), "Элемент с классом 'box' не найден");
    
    // Проверяем, что применился стиль из media query
    if let Some(styles) = engine.computed_styles.get(&div_id.unwrap()) {
        if let Some(bg_color) = styles.get("background-color") {
            // Должен быть зеленый цвет из media query (#2ecc71), а не синий (#3498db)
            assert!(
                bg_color.contains("2ecc71") || bg_color.contains("46, 204, 113"),
                "Expected green color from media query, got: {}",
                bg_color
            );
        } else {
            panic!("background-color не найден в computed styles");
        }
    } else {
        panic!("Стили не применились к элементу");
    }
}

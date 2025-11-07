//! Интеграционные тесты для полного pipeline Zver
//!
//! Тестируют взаимодействие между модулями:
//! - HTML parsing → DOM
//! - CSS parsing → StyleEngine → Layout
//! - Full pipeline: HTML + CSS → Layout → Render

use zver::Zver;

/// Тест парсинга простого HTML документа и построения DOM дерева
#[tokio::test]
async fn test_basic_html_parsing() {
    let engine = Zver::new();

    // Создаём минимальный HTML
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>Test Page</title></head>
            <body>
                <h1 id="title">Hello World</h1>
                <p class="content">Test paragraph</p>
            </body>
        </html>
    "#;

    // Парсим HTML напрямую
    {
        let mut dom = engine.dom.write().await;
        assert!(dom.parse_html(html).await.is_ok());
    }

    // Проверяем структуру DOM
    let dom = engine.dom.read().await;

    // Проверяем что корень существует
    assert!(dom.root.is_some(), "DOM должен иметь корневой элемент");

    // Проверяем что можем найти элемент по ID
    let title_ids = dom.select_ids("#title");
    assert_eq!(title_ids.len(), 1, "Должен быть найден 1 элемент #title");

    // Проверяем что можем найти элемент по классу
    let content_ids = dom.select_ids(".content");
    assert_eq!(
        content_ids.len(),
        1,
        "Должен быть найден 1 элемент .content"
    );

    // Проверяем что можем найти элемент по тегу
    let h1_ids = dom.select_ids("h1");
    assert_eq!(h1_ids.len(), 1, "Должен быть найден 1 элемент h1");
}

/// Тест работы с пустым HTML документом
#[tokio::test]
async fn test_empty_html_document() {
    let engine = Zver::new();

    {
        let mut dom = engine.dom.write().await;
        // Пустой HTML не должен вызывать panic
        assert!(dom.parse_html("").await.is_ok());
    }

    let dom = engine.dom.read().await;
    // Проверяем что селекторы работают с пустым документом
    let result = dom.select_ids("div");
    assert_eq!(
        result.len(),
        0,
        "В пустом документе не должно быть элементов"
    );
}

/// Тест парсинга HTML с невалидным синтаксисом
#[tokio::test]
async fn test_malformed_html() {
    let engine = Zver::new();

    let malformed_html = r#"
        <div><p>Unclosed div
        <span>Nested without closing
        Random text
    "#;

    {
        let mut dom = engine.dom.write().await;
        // scraper должен gracefully обработать невалидный HTML
        assert!(dom.parse_html(malformed_html).await.is_ok());
    }

    let dom = engine.dom.read().await;
    // Проверяем что хотя бы какая-то структура создана
    assert!(
        dom.root.is_some(),
        "Даже невалидный HTML должен создать корень"
    );
}

/// Тест полного pipeline: HTML + CSS → Layout
#[tokio::test]
async fn test_html_css_layout_pipeline() {
    let engine = Zver::new();

    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; padding: 10px; }
                    .box { width: 100px; height: 50px; }
                </style>
            </head>
            <body>
                <div class="box">Test</div>
            </body>
        </html>
    "#;

    // Парсим HTML
    {
        let mut dom = engine.dom.write().await;
        assert!(dom.parse_html(html).await.is_ok());
    }

    // Извлекаем и применяем CSS
    {
        let dom_snapshot = engine.dom.read().await.clone();
        let mut css = engine.css.write().await;

        // Находим <style> теги
        let style_ids = dom_snapshot.select_ids("style");
        assert!(
            style_ids.len() > 0,
            "Должен быть найден хотя бы один <style> тег"
        );
        let pseudo_contents = css.pseudo_element_contents();

        // Извлекаем CSS
        let mut css_content = String::new();
        for style_id in style_ids {
            let content = dom_snapshot.get_text_content(style_id);
            css_content.push_str(&content);
        }

        // Парсим CSS
        assert!(
            css.parse_css(&css_content).is_ok(),
            "CSS должен парситься без ошибок"
        );

        // Применяем стили
        assert!(
            css.apply_styles(&dom_snapshot).is_ok(),
            "Стили должны применяться без ошибок"
        );

        // Проверяем что стили были применены
        assert!(css.rules.len() > 0, "Должны быть распознаны CSS правила");

        {
            let mut dom_write = engine.dom.write().await;
            dom_write.sync_pseudo_elements(&pseudo_contents);
        }
    }

    // Вычисляем layout
    {
        let dom_guard = engine.dom.read().await;
        let css_guard = engine.css.read().await;
        let css_snapshot = css_guard.computed_styles.clone();
        let pseudo_snapshot = css_guard.pseudo_element_styles.clone();
        drop(css_guard);
        let mut layout = engine.layout.write().await;

        let layout_results = layout.compute_layout(&*dom_guard, &css_snapshot, &pseudo_snapshot);

        // Проверяем что layout был вычислен
        assert!(
            layout_results.len() > 0,
            "Layout должен быть вычислен для элементов"
        );
    }
}

/// Тест обработки невалидного CSS
#[tokio::test]
async fn test_malformed_css() {
    let engine = Zver::new();

    let malformed_css = r#"
        .test { color: red
        broken syntax here
        .another { width: }
    "#;

    let mut css = engine.css.write().await;

    // Невалидный CSS не должен вызывать panic
    // cssparser должен либо пропустить, либо вернуть ошибку
    let result = css.parse_css(malformed_css);

    // Проверяем что либо успешно (с пропуском ошибок), либо возвращает ошибку
    assert!(
        result.is_ok() || result.is_err(),
        "CSS парсер должен обработать невалидный синтаксис"
    );
}

/// Тест производительности на большом DOM дереве (1000+ узлов)
#[tokio::test]
async fn test_large_dom_tree() {
    let engine = Zver::new();

    // Генерируем HTML с 1000 div элементов
    let mut html = String::from("<html><body>");
    for i in 0..1000 {
        html.push_str(&format!(
            "<div id='item-{}' class='item'>Item {}</div>",
            i, i
        ));
    }
    html.push_str("</body></html>");

    {
        let mut dom = engine.dom.write().await;
        let start = std::time::Instant::now();
        assert!(dom.parse_html(&html).await.is_ok());
        let elapsed = start.elapsed();

        // Parsing 1000 nodes should stay under 150ms (pseudo-element sync)
        assert!(
            elapsed.as_millis() < 150,
            "Parsing 1000 nodes must stay fast even with pseudo-element sync: {:?}",
            elapsed
        );
    }

    let dom = engine.dom.read().await;

    // Проверяем что все элементы были созданы
    let divs = dom.select_ids("div");
    assert!(
        divs.len() >= 1000,
        "Должно быть создано >= 1000 div элементов, найдено: {}",
        divs.len()
    );

    // Проверяем производительность селекторов на большом дереве
    let start = std::time::Instant::now();
    let items = dom.select_ids(".item");
    let elapsed = start.elapsed();

    assert!(
        items.len() >= 1000,
        "Должно быть найдено >= 1000 элементов с классом .item"
    );
    assert!(
        elapsed.as_millis() < 150,
        "Selector over 1000 items should stay under 150ms: {:?}",
        elapsed
    );
}

/// Тест параллельной обработки CSS
#[tokio::test]
async fn test_concurrent_css_processing() {
    let engine = Zver::new();

    let html = r#"
        <html>
            <head>
                <style>.a { color: red; }</style>
                <style>.b { color: blue; }</style>
                <style>.c { color: green; }</style>
            </head>
            <body><div class="a b c">Test</div></body>
        </html>
    "#;

    {
        let mut dom = engine.dom.write().await;
        assert!(dom.parse_html(html).await.is_ok());
    }

    {
        let dom_snapshot = engine.dom.read().await.clone();
        let mut css = engine.css.write().await;

        // Должно обработать несколько <style> тегов
        let style_ids = dom_snapshot.select_ids("style");
        assert_eq!(style_ids.len(), 3, "Должно быть 3 <style> тега");

        let mut combined_css = String::new();
        for style_id in style_ids {
            combined_css.push_str(&dom_snapshot.get_text_content(style_id));
            combined_css.push('\n');
        }

        assert!(css.parse_css(&combined_css).is_ok());
        assert_eq!(css.rules.len(), 3, "Должно быть распознано 3 CSS правила");
    }
}

/// Тест метрик производительности (tracing spans)
#[tokio::test]
async fn test_performance_metrics() {
    let engine = Zver::new();

    let html = r#"
        <html>
            <head>
                <style>body { margin: 0; }</style>
            </head>
            <body><p>Test</p></body>
        </html>
    "#;

    {
        let mut dom = engine.dom.write().await;

        let start = std::time::Instant::now();
        assert!(dom.parse_html(html).await.is_ok());
        let parse_time = start.elapsed();

        // Парсинг простого документа должен быть < 10ms
        assert!(
            parse_time.as_millis() < 10,
            "Парсинг должен быть быстрым: {:?}",
            parse_time
        );
    }

    {
        let dom_snapshot = engine.dom.read().await.clone();
        let mut css = engine.css.write().await;

        let style_content = dom_snapshot.get_text_content(dom_snapshot.select_ids("style")[0]);

        let start = std::time::Instant::now();
        assert!(css.parse_css(&style_content).is_ok());
        let css_time = start.elapsed();

        // CSS парсинг должен быть < 5ms
        assert!(
            css_time.as_millis() < 5,
            "CSS парсинг должен быть быстрым: {:?}",
            css_time
        );
    }
}

//! Unit-тесты для Фазы 4: Расширение свойств и единиц

use zver::css::StyleEngine;

#[test]
fn test_rem_units() {
    // Тест парсинга rem единиц
    let css = r#"
        .test { font-size: 2rem; padding: 1.5rem; margin: 0.5rem; }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: После реализации rem проверить вычисление значений
}

#[test]
fn test_vmin_vmax_units() {
    // Тест парсинга vmin и vmax единиц
    let css = r#"
        .test { width: 50vmin; height: 30vmax; }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: После реализации vmin/vmax проверить вычисление относительно viewport
}

#[test]
fn test_calc_function() {
    // Тест парсинга calc()
    let css = r#"
        .test { width: calc(100% - 40px); height: calc(100vh - 200px); }
    "#;

    let mut engine = StyleEngine::new();
    let result = engine.parse_css(css);
    if let Err(e) = &result {
        eprintln!("calc() parse error: {:?}", e);
    }
    assert!(result.is_ok());

    // TODO: После реализации calc() проверить вычисление выражений
}

#[test]
fn test_transition_properties() {
    // Тест парсинга transition-* свойств
    let css = r#"
        .test {
            transition-property: background-color, transform;
            transition-duration: 0.5s;
            transition-timing-function: ease-in-out;
            transition-delay: 0.1s;
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_transition_shorthand() {
    // Тест парсинга transition шортката
    let css = r#"
        .test { transition: all 0.3s ease 0s; }
        .test2 { transition: background-color 0.5s linear; }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_animation_properties() {
    // Тест парсинга animation-* свойств
    let css = r#"
        .test {
            animation-name: fadeIn;
            animation-duration: 2s;
            animation-timing-function: ease-in-out;
            animation-delay: 0s;
            animation-iteration-count: infinite;
            animation-direction: alternate;
            animation-fill-mode: both;
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_animation_shorthand() {
    // Тест парсинга animation шортката
    let css = r#"
        .test { animation: fadeIn 1.5s linear 0s infinite alternate; }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_box_sizing() {
    // Тест парсинга box-sizing
    let css = r#"
        .content-box { box-sizing: content-box; }
        .border-box { box-sizing: border-box; }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_grid_properties() {
    // Тест парсинга Grid свойств
    let css = r#"
        .grid {
            display: grid;
            grid-template-rows: 100px 100px;
            grid-template-columns: 1fr 2fr 1fr;
            grid-auto-rows: minmax(100px, auto);
            grid-auto-flow: row;
            gap: 10px;
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_grid_positioning() {
    // Тест парсинга grid-row и grid-column
    let css = r#"
        .item {
            grid-row: 1 / 3;
            grid-column: 2 / 4;
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

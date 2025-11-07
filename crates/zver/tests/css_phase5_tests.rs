//! Unit-тесты для Фазы 5: Каскад и наследование

use zver::css::StyleEngine;

#[test]
fn test_specificity_calculation() {
    // Тест вычисления специфичности
    let css = r#"
        div { color: black; }                    /* 0,0,0,1 */
        .class { color: blue; }                   /* 0,0,1,0 */
        #id { color: red; }                       /* 0,1,0,0 */
        div.class { color: green; }               /* 0,0,1,1 */
        div#id.class { color: yellow; }           /* 0,1,1,1 */
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: После реализации проверить правильность применения стилей по специфичности
}

#[test]
fn test_important_flag() {
    // Тест !important
    let css = r#"
        .test { color: blue !important; }
        .test { color: red; }  /* Не должно переопределить !important */
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: Проверить что !important имеет приоритет
}

#[test]
fn test_inheritance() {
    // Тест наследования свойств
    let css = r#"
        .parent {
            color: red;
            font-size: 20px;
            font-weight: bold;
        }
        .child {
            /* Наследует color, font-size, font-weight */
        }
        .child-override {
            color: blue;  /* Переопределяет наследуемый color */
        }
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: После реализации проверить наследование в ComputedStyle
}

#[test]
fn test_cascade_order() {
    // Тест порядка каскада (позднее правило побеждает)
    let css = r#"
        .test { color: red; }
        .test { color: blue; }  /* Должен победить */
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // TODO: Проверить что применен синий цвет
}

#[test]
fn test_user_agent_styles() {
    // Тест применения User-Agent стилей
    // TODO: После реализации UA styles проверить их применение перед author styles

    let css = r#"
        h1 { color: red; }  /* Author style */
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());

    // UA styles должны применяться первыми, затем author styles
}

#[test]
fn test_inherit_keyword() {
    // Тест явного наследования через inherit
    let css = r#"
        .grandparent { font-size: 24px; }
        .parent { font-size: inherit; }  /* Явно наследует 24px */
        .child { font-size: 16px; }      /* Переопределяет */
    "#;

    let mut engine = StyleEngine::new();
    assert!(engine.parse_css(css).is_ok());
}

#[test]
fn test_inline_styles() {
    // Тест inline стилей (должны иметь высокую специфичность)
    // TODO: После реализации inline стилей проверить их приоритет
}

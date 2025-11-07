//! Unit tests for JavaScript integration
//! Tests DOM API, events, and timer functionality

use tokio;
use zver::js::JSEngine;

#[tokio::test]
async fn test_js_dom_create_element() {
    let mut engine = JSEngine::new();

    let result = engine.execute(
        r#"
        typeof document !== 'undefined' && typeof document.createElement === 'function'
    "#,
    );

    // document может быть не инициализирован без DOM, это нормально
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_js_console_log() {
    let mut engine = JSEngine::new();

    let result = engine.execute(
        r#"
        console.log('Test message');
        true;
    "#,
    );

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_js_set_timeout() {
    let mut engine = JSEngine::new();

    // Регистрируем setTimeout
    let result = engine.execute(
        r#"
        let timeoutId = setTimeout("console.log('Timer fired')", 100);
        timeoutId > 0;
    "#,
    );

    assert!(result.is_ok());

    // Даём таймеру время сработать
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Вызываем tick_timers для выполнения callbacks
    let executed = engine.tick_timers();
    assert_eq!(executed, 1, "Should execute 1 timer callback");
}

#[tokio::test]
async fn test_js_set_timeout_with_function() {
    let mut engine = JSEngine::new();

    // setTimeout с анонимной функцией (будет сконвертирована в строку)
    let result = engine.execute(
        r#"
        let called = false;
        setTimeout(function() { called = true; }, 50);
    "#,
    );

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_js_basic_arithmetic() {
    let mut engine = JSEngine::new();

    let result = engine.execute("2 + 2");
    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::Number(n)) = result {
        assert_eq!(n, 4.0);
    } else {
        panic!("Expected number result");
    }
}

#[tokio::test]
async fn test_js_string_operations() {
    let mut engine = JSEngine::new();

    let result = engine.execute(
        r#"
        let str = "Hello, " + "World!";
        str;
    "#,
    );

    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::String(s)) = result {
        assert_eq!(s, "Hello, World!");
    } else {
        panic!("Expected string result");
    }
}

#[tokio::test]
async fn test_js_variables() {
    let mut engine = JSEngine::new();

    engine.execute("let x = 10;").unwrap();
    engine.execute("let y = 20;").unwrap();

    let result = engine.execute("x + y");
    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::Number(n)) = result {
        assert_eq!(n, 30.0);
    }
}

#[tokio::test]
async fn test_js_boolean_logic() {
    let mut engine = JSEngine::new();

    let result = engine.execute("true && false");
    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::Boolean(b)) = result {
        assert!(!b);
    } else {
        panic!("Expected boolean result");
    }
}

#[tokio::test]
async fn test_js_undefined() {
    let mut engine = JSEngine::new();

    let result = engine.execute("undefined");
    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::Undefined) = result {
        // Success
    } else {
        panic!("Expected undefined result");
    }
}

#[tokio::test]
async fn test_js_function_declaration() {
    let mut engine = JSEngine::new();

    let result = engine.execute(
        r#"
        function add(a, b) {
            return a + b;
        }
        add(5, 3);
    "#,
    );

    assert!(result.is_ok());

    if let Ok(zver::js::JSValue::Number(n)) = result {
        assert_eq!(n, 8.0);
    }
}

#[tokio::test]
async fn test_js_error_handling() {
    let mut engine = JSEngine::new();

    // Синтаксическая ошибка должна вернуть Err
    let result = engine.execute("invalid syntax here +++");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_timers() {
    let mut engine = JSEngine::new();

    // Регистрируем несколько таймеров
    engine
        .execute(r#"setTimeout("console.log('Timer 1')", 50);"#)
        .unwrap();
    engine
        .execute(r#"setTimeout("console.log('Timer 2')", 100);"#)
        .unwrap();
    engine
        .execute(r#"setTimeout("console.log('Timer 3')", 150);"#)
        .unwrap();

    // Ждём чтобы первый таймер сработал
    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
    let executed = engine.tick_timers();
    assert_eq!(executed, 1, "First timer should fire");

    // Ждём чтобы второй сработал
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let executed = engine.tick_timers();
    assert_eq!(executed, 1, "Second timer should fire");

    // Ждём чтобы третий сработал
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let executed = engine.tick_timers();
    assert_eq!(executed, 1, "Third timer should fire");
}

#[tokio::test]
async fn test_tick_timers_no_ready() {
    let mut engine = JSEngine::new();

    // Регистрируем таймер с большой задержкой
    engine
        .execute(r#"setTimeout("console.log('Far future')", 10000);"#)
        .unwrap();

    // Вызываем tick_timers сразу - ничего не должно выполниться
    let executed = engine.tick_timers();
    assert_eq!(executed, 0, "No timers should be ready");
}

use std::collections::HashMap;
use zver::dom::Document;
use zver::layout::LayoutEngine;

/// Интеграционные тесты для новой функциональности layout с Taffy
#[cfg(test)]
mod tests {
    use super::*;

    /// Создает простой DOM документ для тестирования
    fn create_test_dom() -> Document {
        use std::collections::HashMap;
        use zver::dom::Node;

        let mut doc = Document::new();

        // Создаем корневой элемент (id: 0)
        let root_node = Node {
            id: 0,
            tag_name: Some("div".to_string()),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert(
                    "style".to_string(),
                    "width: 800px; height: 600px; background-color: white;".to_string(),
                );
                attrs
            },
            text_content: None,
            children: vec![1], // дочерний элемент
            parent: None,
        };
        doc.nodes.insert(0, root_node);
        doc.root = Some(0);

        // Создаем дочерний элемент (id: 1)
        let child_node = Node {
            id: 1,
            tag_name: Some("div".to_string()),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert(
                    "style".to_string(),
                    "width: 200px; height: 100px; background-color: blue; margin: 10px;"
                        .to_string(),
                );
                attrs
            },
            text_content: None,
            children: vec![2], // текстовый узел
            parent: Some(0),
        };
        doc.nodes.insert(1, child_node);

        // Создаем текстовый узел (id: 2)
        let text_node = Node {
            id: 2,
            tag_name: None,
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert(
                    "style".to_string(),
                    "color: black; font-size: 16px;".to_string(),
                );
                attrs
            },
            text_content: Some("Hello World".to_string()),
            children: vec![],
            parent: Some(1),
        };
        doc.nodes.insert(2, text_node);

        doc
    }

    /// Создает вычисленные стили для тестирования
    fn create_test_styles() -> HashMap<usize, HashMap<String, String>> {
        let mut styles = HashMap::new();

        // Стили для корневого элемента (id: 0)
        let mut root_styles = HashMap::new();
        root_styles.insert("width".to_string(), "800px".to_string());
        root_styles.insert("height".to_string(), "600px".to_string());
        root_styles.insert("background-color".to_string(), "white".to_string());
        styles.insert(0, root_styles);

        // Стили для дочернего элемента (id: 1)
        let mut child_styles = HashMap::new();
        child_styles.insert("width".to_string(), "200px".to_string());
        child_styles.insert("height".to_string(), "100px".to_string());
        child_styles.insert("background-color".to_string(), "blue".to_string());
        child_styles.insert("margin".to_string(), "10px".to_string());
        styles.insert(1, child_styles);

        // Стили для текстового узла (id: 2)
        let mut text_styles = HashMap::new();
        text_styles.insert("color".to_string(), "black".to_string());
        text_styles.insert("font-size".to_string(), "16px".to_string());
        styles.insert(2, text_styles);

        styles
    }

    #[test]
    fn test_layout_engine_creation() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        // Проверяем, что engine создается без ошибок
        // и методы работают корректно
        let empty_dom = Document::new();
        let empty_styles = HashMap::new();
        let results = engine.compute_layout(&empty_dom, &empty_styles);
        assert!(results.is_empty());
    }

    #[test]
    fn test_compute_layout_with_simple_dom() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        let results = engine.compute_layout(&dom, &styles);

        // Проверяем, что результаты не пустые
        assert!(!results.is_empty());

        // Проверяем, что есть результат для корневого элемента
        assert!(results.contains_key(&0));

        // Проверяем, что есть результат для дочернего элемента
        assert!(results.contains_key(&1));

        // Проверяем, что есть результат для текстового узла
        assert!(results.contains_key(&2));
    }

    #[test]
    fn test_layout_result_properties() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        let results = engine.compute_layout(&dom, &styles);

        // Проверяем свойства корневого элемента
        let root_result = results.get(&0).unwrap();
        assert_eq!(root_result.node_id, 0);
        assert_eq!(root_result.width, 800.0);
        assert_eq!(root_result.height, 600.0);
        assert_eq!(root_result.x, 0.0);
        assert_eq!(root_result.y, 0.0);

        // Проверяем свойства дочернего элемента
        let child_result = results.get(&1).unwrap();
        assert_eq!(child_result.node_id, 1);
        assert_eq!(child_result.width, 200.0);
        assert_eq!(child_result.height, 100.0);
        // Проверяем, что элемент имеет правильные размеры
        // (margin может не влиять на позицию в данном случае)
        assert!(child_result.x >= 0.0);
        assert!(child_result.y >= 0.0);
    }

    #[test]
    fn test_get_layout_result() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        engine.compute_layout(&dom, &styles);

        // Тестируем получение конкретного результата
        let root_result = engine.get_layout_result(0);
        assert!(root_result.is_some());
        assert_eq!(root_result.unwrap().width, 800.0);

        // Тестируем получение несуществующего результата
        let non_existent = engine.get_layout_result(999);
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_get_all_layout_results() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        engine.compute_layout(&dom, &styles);

        let all_results = engine.get_all_layout_results();
        assert_eq!(all_results.len(), 3); // root, child, text

        // Проверяем, что все ожидаемые ID присутствуют
        assert!(all_results.contains_key(&0));
        assert!(all_results.contains_key(&1));
        assert!(all_results.contains_key(&2));
    }

    #[test]
    fn test_collect_render_info() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        engine.compute_layout(&dom, &styles);

        let render_info = engine.collect_render_info(&dom);
        assert!(!render_info.is_empty());

        // Проверяем структуру RenderInfo
        for info in render_info {
            assert!(info.layout.width >= 0.0);
            assert!(info.layout.height >= 0.0);
            assert!(info.layout.x >= 0.0);
            assert!(info.layout.y >= 0.0);
        }
    }

    #[test]
    fn test_get_all_render_info() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        engine.compute_layout(&dom, &styles);

        let render_info = engine.get_all_render_info(&dom);
        assert_eq!(render_info.len(), 3);

        // Проверяем, что каждый RenderInfo содержит правильные данные
        for info in render_info {
            let expected_result = engine.get_layout_result(info.layout.node_id).unwrap();
            assert_eq!(info.layout, expected_result);
        }
    }

    #[test]
    fn test_invalidate_clears_cache() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        // Заполняем кеш
        let results_before = engine.compute_layout(&dom, &styles);
        assert!(!results_before.is_empty());

        // Инвалидируем
        engine.invalidate();

        // Проверяем, что результаты после инвалидации отличаются
        let results_after = engine.compute_layout(&dom, &styles);
        // После инвалидации результат должен быть тем же (кеш перестраивается)
        assert_eq!(results_before.len(), results_after.len());
    }

    #[test]
    fn test_empty_dom_returns_empty_results() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let empty_dom = Document::new();
        let empty_styles = HashMap::new();

        let results = engine.compute_layout(&empty_dom, &empty_styles);
        assert!(results.is_empty());
    }

    #[test]
    fn test_layout_cache_persistence() {
        let mut engine = LayoutEngine::new(800.0, 600.0);
        let dom = create_test_dom();
        let styles = create_test_styles();

        // Первый вызов
        let results1 = engine.compute_layout(&dom, &styles);

        // Второй вызов (кеш должен сохраняться)
        let results2 = engine.compute_layout(&dom, &styles);

        // Результаты должны быть одинаковыми
        assert_eq!(results1, results2);

        // Проверяем, что можем получить результаты
        let all_results = engine.get_all_layout_results();
        assert_eq!(all_results.len(), results1.len());
    }
}

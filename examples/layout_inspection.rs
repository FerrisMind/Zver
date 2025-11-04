use zver::Zver;

/// Пример демонстрации работы с новым Layout API и RenderInfo
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация движка
    let engine = Zver::new();

    // Загрузка локального HTML файла
    let mut path = std::env::current_dir().unwrap();
    path.push("examples");
    path.push("index.html");
    let url = format!("file://{}", path.display());
    println!("Загружаем: {}", url);

    match engine.load_url(&url).await {
        Ok(_) => {
            println!("✅ Страница успешно загружена!");

            // Получение информации о DOM
            let dom = engine.dom.read().await;
            println!("📄 DOM содержит {} узлов", dom.nodes.len());

            // Получение информации о layout через новый API
            let layout = engine.layout.read().await;
            let layout_results = layout.get_all_layout_results();
            println!("📐 Layout результатов: {}", layout_results.len());

            // Получение RenderInfo для всех узлов
            let render_info = layout.get_all_render_info(&dom);
            println!("🎨 RenderInfo элементов: {}", render_info.len());

            // Анализ результатов layout
            println!("\n📊 Анализ layout результатов:");
            let mut total_area = 0.0;
            let mut text_nodes = 0;
            let mut element_nodes = 0;

            for info in &render_info {
                let area = info.layout.width * info.layout.height;
                total_area += area;

                if info.node.tag_name.is_none() {
                    text_nodes += 1;
                    if let Some(text) = &info.node.text_content {
                        let preview = if text.chars().count() > 20 {
                            format!("{}...", text.chars().take(20).collect::<String>())
                        } else {
                            text.clone()
                        };
                        println!(
                            "  📝 Текст: \"{}\" - {:.0}x{:.0} at ({:.0},{:.0})",
                            preview,
                            info.layout.width,
                            info.layout.height,
                            info.layout.x,
                            info.layout.y
                        );
                    }
                } else {
                    element_nodes += 1;
                    if let Some(tag) = &info.node.tag_name {
                        println!(
                            "  🏷️  <{}> - {:.0}x{:.0} at ({:.0},{:.0})",
                            tag,
                            info.layout.width,
                            info.layout.height,
                            info.layout.x,
                            info.layout.y
                        );
                    }
                }
            }

            println!("\n📈 Статистика:");
            println!("  Общая площадь: {:.0} px²", total_area);
            println!("  Текстовых узлов: {}", text_nodes);
            println!("  Элементных узлов: {}", element_nodes);

            // Демонстрация получения конкретного результата layout
            if let Some(root_id) = dom.root {
                if let Some(root_result) = layout.get_layout_result(root_id) {
                    println!("\n🌳 Корневой элемент:");
                    println!(
                        "  Размер: {:.0}x{:.0}",
                        root_result.width, root_result.height
                    );
                    println!("  Позиция: ({:.0},{:.0})", root_result.x, root_result.y);
                    println!(
                        "  Контентная область: {:.0}x{:.0} at ({:.0},{:.0})",
                        root_result.content_width,
                        root_result.content_height,
                        root_result.content_x,
                        root_result.content_y
                    );
                }
            }

            // Получение информации о CSS
            let css = engine.css.read().await;
            println!("\n🎨 CSS информация:");
            println!("  Правил: {}", css.rules.len());
            println!("  Вычисленных стилей: {}", css.computed_styles.len());
        }
        Err(e) => {
            println!("❌ Ошибка загрузки: {}", e);
        }
    }

    Ok(())
}

// Для запуска примера:
// cargo run --example layout_inspection

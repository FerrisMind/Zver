use zver::Zver;

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

            if let Some(root) = dom.root {
                println!("🌳 Корневой узел: {:?}", root);
            }

            // Получение информации о layout
            let layout = engine.layout.read().await;
            let layout_results = layout.get_all_layout_results();
            if !layout_results.is_empty() {
                println!("📐 Layout результатов: {}", layout_results.len());

                // Показываем информацию о первых нескольких результатах
                for (i, (node_id, result)) in layout_results.iter().take(5).enumerate() {
                    println!(
                        "  {}: узел {} - {:.0}x{:.0} at ({:.0},{:.0})",
                        i + 1,
                        node_id,
                        result.width,
                        result.height,
                        result.x,
                        result.y
                    );
                }

                if layout_results.len() > 5 {
                    println!("  ... и еще {} результатов", layout_results.len() - 5);
                }
            } else {
                println!("⚠️  Layout результаты не найдены");
            }

            // Получение информации о CSS
            let css = engine.css.read().await;
            println!("🎨 CSS правил: {}", css.rules.len());
            println!("📊 Вычисленных стилей: {}", css.computed_styles.len());

            // Сериализация DOM обратно в HTML
            let html = zver::dom::serialization::serialize_dom(&dom);
            println!("📝 Сериализованный HTML ({} символов):", html.len());

            // Показываем первые 200 символов
            let preview = if html.len() > 200 {
                format!("{}...", &html[..200])
            } else {
                html
            };
            println!("{}", preview);
        }
        Err(e) => {
            println!("❌ Ошибка загрузки: {}", e);
        }
    }

    Ok(())
}

// Для запуска примера:
// cargo run --example basic_usage

//! Пример для тестирования CSS фаз
//! 
//! Запуск:
//! ```bash
//! cargo run --example test_css_phases -- phase2_media_queries
//! ```

use zver::Zver;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let test_file = if args.len() > 1 {
        args[1].as_str()
    } else {
        println!("Использование: cargo run --example test_css_phases -- <test_file>");
        println!("\nДоступные тесты:");
        println!("  phase2_media_queries");
        println!("  phase2_keyframes");
        println!("  phase2_font_face");
        println!("  phase3_pseudo_classes");
        println!("  phase3_pseudo_elements");
        println!("  phase4_properties_units");
        println!("  phase4_grid_properties");
        println!("  phase5_cascade_inheritance");
        println!("  phase6_grid_layout");
        println!("  phase7_animations_transitions");
        return Ok(());
    };

    let engine = Zver::new();

    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push(format!("{}.html", test_file));
    
    let url = format!("file://{}", path.display());
    println!("Загружаем: {}", url);

    match engine.load_url(&url).await {
        Ok(_) => {
            println!("✅ Страница успешно загружена!");

            // Получение информации о DOM
            let dom = engine.dom.read().await;
            println!("📄 DOM содержит {} узлов", dom.nodes.len());

            if let Some(root) = dom.root {
                println!("🌳 Корневой узел: {}", root);
            }

            // Получение информации о CSS
            let css = engine.css.read().await;
            println!("📝 CSS правил: {}", css.rules.len());
            println!("📱 Media правил: {}", css.media_rules.len());
            println!("🎬 Keyframes: {}", css.keyframes.len());
            println!("🔤 Шрифтов: {}", css.fonts.len());

            // Получение информации о layout
            let layout = engine.layout.read().await;
            let layout_results = layout.get_all_layout_results();
            println!("📐 Layout результатов: {}", layout_results.len());

            // Получение RenderInfo
            let render_info = layout.get_all_render_info(&dom);
            println!("🎨 RenderInfo элементов: {}", render_info.len());

            // Вывод первых 10 элементов
            println!("\n📊 Первые 10 элементов:");
            for (i, info) in render_info.iter().take(10).enumerate() {
                if let Some(node) = dom.nodes.get(&info.node_id) {
                    let tag = node.tag_name.as_deref().unwrap_or("text");
                    println!("  {}: {} - bounds: {:?}", i + 1, tag, info.bounds);
                }
            }

            println!("\n✅ Тест завершен успешно!");
        }
        Err(e) => {
            eprintln!("❌ Ошибка загрузки: {}", e);
            return Err(e);
        }
    }

    Ok(())
}


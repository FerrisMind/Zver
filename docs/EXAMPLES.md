# Примеры использования Zver

Данный документ содержит практические примеры использования браузерного движка Zver для различных сценариев.

## Базовые примеры

### 1. Простая загрузка HTML страницы

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Создание экземпляра движка
    let engine = Zver::new();
    
    // Загрузка локального HTML файла
    engine.load_url("file://example.html").await?;
    
    // Получение информации о загруженной странице
    let dom = engine.dom.read().await;
    println!("✅ Загружено {} DOM узлов", dom.nodes.len());
    
    Ok(())
}
```

### 2. Анализ структуры DOM

```rust
use zver::Zver;
use zver::dom::serialization::serialize_dom;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    engine.load_url("file://complex.html").await?;
    
    let dom = engine.dom.read().await;
    
    // Поиск элементов по селекторам
    let divs = dom.select_ids("div");
    let classes = dom.select_ids(".container");
    let ids = dom.select_ids("#header");
    
    println!("📊 Статистика DOM:");
    println!("  Всего узлов: {}", dom.nodes.len());
    println!("  <div> элементов: {}", divs.len());
    println!("  .container классов: {}", classes.len());
    println!("  #header ID: {}", ids.len());
    
    // Обход дерева DOM
    if let Some(root_id) = dom.root {
        print_dom_tree(&dom, root_id, 0);
    }
    
    // Сериализация обратно в HTML
    let html = serialize_dom(&dom);
    println!("\n📝 Сериализованный HTML ({} символов)", html.len());
    
    Ok(())
}

fn print_dom_tree(dom: &zver::dom::Document, node_id: usize, depth: usize) {
    let indent = "  ".repeat(depth);
    
    if let Some(node) = dom.nodes.get(&node_id) {
        match &node.tag_name {
            Some(tag) => {
                let attrs = node.attributes.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(" ");
                
                if attrs.is_empty() {
                    println!("{}📄 <{}>", indent, tag);
                } else {
                    println!("{}📄 <{} {}>", indent, tag, attrs);
                }
            }
            None => {
                if let Some(text) = &node.text_content {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        println!("{}📝 \"{}\"", indent, trimmed);
                    }
                }
            }
        }
        
        // Рекурсивный обход дочерних элементов
        for &child_id in &node.children {
            print_dom_tree(dom, child_id, depth + 1);
        }
    }
}
```

### 3. Работа с CSS стилями

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Создание простого HTML документа
    {
        let mut dom = engine.dom.write().await;
        dom.parse_html(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body { 
                        font-family: Arial, sans-serif; 
                        margin: 20px;
                        background-color: #f0f0f0;
                    }
                    .header { 
                        color: #333; 
                        font-size: 24px;
                        margin-bottom: 10px;
                    }
                    .content { 
                        background: white; 
                        padding: 15px;
                        border-radius: 5px;
                        box-shadow: 0 2px 5px rgba(0,0,0,0.1);
                    }
                    #special { 
                        color: red; 
                        font-weight: bold; 
                    }
                </style>
            </head>
            <body>
                <div class="header">Заголовок страницы</div>
                <div class="content">
                    <p>Обычный параграф текста.</p>
                    <p id="special">Специальный параграф с ID.</p>
                </div>
            </body>
            </html>
        "#).await?;
    }
    
    // Применение CSS стилей
    {
        let dom = engine.dom.read().await;
        let mut css = engine.css.write().await;
        
        // CSS уже извлечен из <style> тега при парсинге HTML
        css.apply_styles(&dom)?;
        
        println!("🎨 CSS обработка завершена:");
        println!("  Правил: {}", css.rules.len());
        println!("  Вычисленных стилей: {}", css.computed_styles.len());
        
        // Анализ вычисленных стилей
        for (node_id, style) in &css.computed_styles {
            if let Some(node) = dom.nodes.get(node_id) {
                let element_info = match &node.tag_name {
                    Some(tag) => {
                        let class = node.attributes.get("class")
                            .map(|c| format!(".{}", c))
                            .unwrap_or_default();
                        let id = node.attributes.get("id")
                            .map(|i| format!("#{}", i))
                            .unwrap_or_default();
                        format!("<{}{}{}> ", tag, class, id)
                    }
                    None => "текст ".to_string(),
                };
                
                println!("  {} - цвет: {:?}, фон: {:?}", 
                    element_info, style.color, style.background_color);
            }
        }
    }
    
    Ok(())
}
```

### 4. Layout вычисления

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // HTML с Flexbox layout
    {
        let mut dom = engine.dom.write().await;
        dom.parse_html(r#"
            <div style="display: flex; width: 800px; height: 600px; flex-direction: column;">
                <div style="flex: 0 0 60px; background: #333;">Header</div>
                <div style="display: flex; flex: 1;">
                    <div style="flex: 0 0 200px; background: #ddd;">Sidebar</div>
                    <div style="flex: 1; background: #fff;">Main Content</div>
                </div>
                <div style="flex: 0 0 40px; background: #666;">Footer</div>
            </div>
        "#).await?;
    }
    
    // Применение стилей и вычисление layout
    {
        let dom = engine.dom.read().await;
        let mut css = engine.css.write().await;
        css.apply_styles(&dom)?;
        
        let css_snapshot = css.computed_styles.clone();
        drop(css); // Освобождаем блокировку CSS
        
        let mut layout = engine.layout.write().await;
        layout.compute_layout(&dom, &css_snapshot);
        
        // Анализ результатов layout
        let results = layout.get_all_layout_results();
        println!("📐 Layout результаты:");
        
        for (node_id, result) in results {
            if let Some(node) = dom.nodes.get(node_id) {
                let description = match &node.text_content {
                    Some(text) if !text.trim().is_empty() => text.trim(),
                    _ => match &node.tag_name {
                        Some(tag) => tag,
                        None => "узел",
                    }
                };
                
                println!("  {}: {:.0}×{:.0} at ({:.0}, {:.0})", 
                    description, result.width, result.height, result.x, result.y);
            }
        }
    }
    
    Ok(())
}
```

## Продвинутые примеры

### 5. JavaScript интеграция

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // HTML с JavaScript
    {
        let mut dom = engine.dom.write().await;
        dom.parse_html(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>JavaScript Test</title>
            </head>
            <body>
                <div id="content">Исходный контент</div>
                <script>
                    console.log("JavaScript загружен!");
                    
                    // Простые вычисления
                    var result = 10 + 20;
                    console.log("Результат:", result);
                    
                    // Работа с объектами
                    var user = {
                        name: "Иван",
                        age: 25,
                        greet: function() {
                            return "Привет, " + this.name + "!";
                        }
                    };
                    
                    console.log(user.greet());
                </script>
            </body>
            </html>
        "#).await?;
    }
    
    // JavaScript уже исполнен при загрузке HTML
    // Дополнительное исполнение JavaScript
    {
        let mut js = engine.js.write().await;
        
        // Выполнение дополнительного кода
        js.execute(r#"
            function calculateArea(width, height) {
                return width * height;
            }
            
            var area = calculateArea(10, 20);
            console.log("Площадь:", area);
        "#)?;
        
        // Получение результата вычислений
        let result = js.evaluate("calculateArea(15, 25)")?;
        println!("🔢 Результат JS вычисления: {}", result);
        
        // Работа с массивами
        js.execute(r#"
            var numbers = [1, 2, 3, 4, 5];
            var sum = numbers.reduce(function(a, b) { return a + b; }, 0);
            console.log("Сумма массива:", sum);
        "#)?;
        
        let sum_result = js.evaluate("sum")?;
        println!("📊 Сумма массива: {}", sum_result);
    }
    
    Ok(())
}
```

### 6. Сетевая загрузка ресурсов

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Настройка сетевого движка
    {
        let mut network = engine.network.write().await;
        network.set_user_agent("Zver Browser 1.0 (Example)");
    }
    
    // Загрузка различных типов ресурсов
    println!("🌐 Тестирование сетевых запросов...");
    
    // 1. Загрузка HTML страницы
    match engine.load_url("https://httpbin.org/html").await {
        Ok(_) => {
            let dom = engine.dom.read().await;
            println!("✅ HTML загружен: {} узлов", dom.nodes.len());
        }
        Err(e) => println!("❌ Ошибка загрузки HTML: {}", e),
    }
    
    // 2. Прямая загрузка ресурсов
    {
        let mut network = engine.network.write().await;
        
        // JSON данные
        match network.fetch("https://httpbin.org/json").await {
            Ok(json_data) => {
                println!("✅ JSON загружен: {} символов", json_data.len());
                println!("📄 Первые 100 символов: {}", 
                    &json_data.chars().take(100).collect::<String>());
            }
            Err(e) => println!("❌ Ошибка загрузки JSON: {}", e),
        }
        
        // Тестирование кэширования
        println!("🔄 Повторная загрузка (должна использовать кэш)...");
        let start = std::time::Instant::now();
        match network.fetch("https://httpbin.org/json").await {
            Ok(_) => {
                let duration = start.elapsed();
                println!("✅ Повторная загрузка за {:?}", duration);
            }
            Err(e) => println!("❌ Ошибка повторной загрузки: {}", e),
        }
    }
    
    Ok(())
}
```

### 7. Обработка ошибок и восстановление

```rust
use zver::Zver;

#[tokio::main]
async fn main() {
    let engine = Zver::new();
    
    // Тестирование различных типов ошибок
    println!("🧪 Тестирование обработки ошибок...");
    
    // 1. Некорректный URL
    match engine.load_url("invalid://url").await {
        Ok(_) => println!("✅ Неожиданный успех"),
        Err(e) => println!("❌ Ожидаемая ошибка URL: {}", e),
    }
    
    // 2. Несуществующий файл
    match engine.load_url("file://nonexistent.html").await {
        Ok(_) => println!("✅ Неожиданный успех"),
        Err(e) => println!("❌ Ожидаемая ошибка файла: {}", e),
    }
    
    // 3. Некорректный HTML
    {
        let mut dom = engine.dom.write().await;
        match dom.parse_html("<div><p>Незакрытый тег").await {
            Ok(_) => println!("✅ HTML парсинг с ошибками успешен (graceful degradation)"),
            Err(e) => println!("❌ Ошибка парсинга HTML: {}", e),
        }
    }
    
    // 4. Некорректный CSS
    {
        let dom = engine.dom.read().await;
        let mut css = engine.css.write().await;
        
        match css.parse_css("invalid css { property: ; }") {
            Ok(_) => println!("✅ CSS парсинг с ошибками успешен (игнорирование некорректных правил)"),
            Err(e) => println!("❌ Ошибка парсинга CSS: {}", e),
        }
    }
    
    // 5. Некорректный JavaScript
    {
        let mut js = engine.js.write().await;
        match js.execute("invalid javascript syntax {") {
            Ok(_) => println!("✅ Неожиданный успех JS"),
            Err(e) => println!("❌ Ожидаемая ошибка JS: {}", e),
        }
        
        // Проверка, что движок продолжает работать после ошибки
        match js.execute("console.log('JS движок работает после ошибки');") {
            Ok(_) => println!("✅ JS движок восстановился после ошибки"),
            Err(e) => println!("❌ JS движок не восстановился: {}", e),
        }
    }
    
    // 6. Успешная загрузка после ошибок
    println!("\n🔄 Тестирование восстановления...");
    match engine.load_url("file://test.html").await {
        Ok(_) => {
            let dom = engine.dom.read().await;
            println!("✅ Успешная загрузка после ошибок: {} узлов", dom.nodes.len());
        }
        Err(e) => println!("❌ Ошибка восстановления: {}", e),
    }
}
```

### 8. Производительность и профилирование

```rust
use zver::Zver;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Создание большого HTML документа для тестирования производительности
    let large_html = generate_large_html(1000); // 1000 элементов
    
    println!("⚡ Тестирование производительности...");
    println!("📄 HTML размер: {} символов", large_html.len());
    
    // 1. Парсинг DOM
    let start = Instant::now();
    {
        let mut dom = engine.dom.write().await;
        dom.parse_html(&large_html).await?;
    }
    let dom_time = start.elapsed();
    
    let dom = engine.dom.read().await;
    println!("🔍 DOM парсинг: {:?} ({} узлов)", dom_time, dom.nodes.len());
    
    // 2. CSS обработка
    let start = Instant::now();
    {
        let mut css = engine.css.write().await;
        css.apply_styles(&dom)?;
    }
    let css_time = start.elapsed();
    
    let css = engine.css.read().await;
    println!("🎨 CSS обработка: {:?} ({} стилей)", css_time, css.computed_styles.len());
    
    // 3. Layout вычисления
    let start = Instant::now();
    {
        let css_snapshot = css.computed_styles.clone();
        drop(css);
        
        let mut layout = engine.layout.write().await;
        layout.compute_layout(&dom, &css_snapshot);
    }
    let layout_time = start.elapsed();
    
    let layout = engine.layout.read().await;
    let results = layout.get_all_layout_results();
    println!("📐 Layout вычисления: {:?} ({} результатов)", layout_time, results.len());
    
    // 4. Общее время
    let total_time = dom_time + css_time + layout_time;
    println!("⏱️  Общее время: {:?}", total_time);
    
    // 5. Статистика производительности
    let nodes_per_ms = dom.nodes.len() as f64 / total_time.as_millis() as f64;
    println!("📊 Производительность: {:.2} узлов/мс", nodes_per_ms);
    
    Ok(())
}

fn generate_large_html(count: usize) -> String {
    let mut html = String::from(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                .container { display: flex; flex-wrap: wrap; }
                .item { 
                    width: 200px; 
                    height: 100px; 
                    margin: 10px; 
                    padding: 15px;
                    background: #f0f0f0;
                    border: 1px solid #ccc;
                }
                .item h3 { color: #333; margin: 0 0 10px 0; }
                .item p { color: #666; margin: 0; }
            </style>
        </head>
        <body>
            <div class="container">
    "#);
    
    for i in 0..count {
        html.push_str(&format!(r#"
            <div class="item">
                <h3>Элемент {}</h3>
                <p>Описание элемента номер {}. Это тестовый контент для проверки производительности парсинга и рендеринга.</p>
            </div>
        "#, i + 1, i + 1));
    }
    
    html.push_str(r#"
            </div>
        </body>
        </html>
    "#);
    
    html
}
```

## Интеграция с GUI

### 9. Использование с egui

```rust
use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Zver Integration Example",
        native_options,
        Box::new(|_cc| Ok(Box::<ZverIntegrationApp>::default())),
    )
}

struct ZverIntegrationApp {
    engine: Arc<Zver>,
    runtime: Arc<Runtime>,
    url: String,
    html_content: String,
    status: String,
}

impl Default for ZverIntegrationApp {
    fn default() -> Self {
        Self {
            engine: Arc::new(Zver::new()),
            runtime: Arc::new(Runtime::new().expect("Failed to create runtime")),
            url: "file://example.html".to_string(),
            html_content: String::new(),
            status: "Готов к загрузке".to_string(),
        }
    }
}

impl eframe::App for ZverIntegrationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Zver Browser Engine Integration");
            
            // URL ввод и загрузка
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_singleline(&mut self.url);
                
                if ui.button("Загрузить").clicked() {
                    self.load_page();
                }
            });
            
            ui.separator();
            ui.label(&self.status);
            
            // Отображение HTML контента
            if !self.html_content.is_empty() {
                ui.separator();
                ui.heading("HTML Контент:");
                
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.html_content)
                                .desired_width(f32::INFINITY)
                                .code_editor(),
                        );
                    });
            }
        });
    }
}

impl ZverIntegrationApp {
    fn load_page(&mut self) {
        let url = self.url.clone();
        let engine = self.engine.clone();
        
        self.status = "Загрузка...".to_string();
        
        let result = self.runtime.block_on(async move {
            engine.load_url(&url).await
        });
        
        match result {
            Ok(_) => {
                self.status = "Загружено успешно".to_string();
                self.update_content();
            }
            Err(e) => {
                self.status = format!("Ошибка: {}", e);
            }
        }
    }
    
    fn update_content(&mut self) {
        let engine = self.engine.clone();
        
        self.html_content = self.runtime.block_on(async move {
            let dom = engine.dom.read().await;
            zver::dom::serialization::serialize_dom(&dom)
        });
    }
}
```

## Тестирование

### 10. Unit тесты

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_basic_html_parsing() {
        let engine = Zver::new();
        
        {
            let mut dom = engine.dom.write().await;
            let result = dom.parse_html("<div>Hello World</div>").await;
            assert!(result.is_ok());
        }
        
        let dom = engine.dom.read().await;
        assert!(dom.nodes.len() > 0);
        assert!(dom.root.is_some());
    }
    
    #[tokio::test]
    async fn test_css_parsing() {
        let engine = Zver::new();
        
        // Создание простого DOM
        {
            let mut dom = engine.dom.write().await;
            dom.parse_html("<div class='test'>Content</div>").await.unwrap();
        }
        
        // Применение CSS
        {
            let dom = engine.dom.read().await;
            let mut css = engine.css.write().await;
            
            let result = css.parse_css(".test { color: red; }");
            assert!(result.is_ok());
            
            let result = css.apply_styles(&dom);
            assert!(result.is_ok());
            
            assert!(css.computed_styles.len() > 0);
        }
    }
    
    #[tokio::test]
    async fn test_javascript_execution() {
        let engine = Zver::new();
        
        let mut js = engine.js.write().await;
        
        // Простое выполнение
        let result = js.execute("var x = 10;");
        assert!(result.is_ok());
        
        // Получение результата
        let result = js.evaluate("x * 2");
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_layout_computation() {
        let engine = Zver::new();
        
        // HTML с inline стилями
        {
            let mut dom = engine.dom.write().await;
            dom.parse_html(r#"<div style="width: 100px; height: 50px;">Test</div>"#).await.unwrap();
        }
        
        // Применение стилей и layout
        {
            let dom = engine.dom.read().await;
            let mut css = engine.css.write().await;
            css.apply_styles(&dom).unwrap();
            
            let css_snapshot = css.computed_styles.clone();
            drop(css);
            
            let mut layout = engine.layout.write().await;
            layout.compute_layout(&dom, &css_snapshot);
            
            let results = layout.get_all_layout_results();
            assert!(results.len() > 0);
            
            // Проверка размеров
            let has_correct_size = results.values().any(|r| r.width == 100.0 && r.height == 50.0);
            assert!(has_correct_size, "Layout должен содержать элемент 100x50");
        }
    }
}
```

Эти примеры покрывают основные сценарии использования Zver и демонстрируют возможности движка. Каждый пример можно запустить отдельно или интегрировать в более крупное приложение.
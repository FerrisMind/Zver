# API Документация Zver

Данный документ описывает публичный API браузерного движка Zver.

## Основной класс Zver

### Создание экземпляра

```rust
use zver::Zver;

let engine = Zver::new();
// или
let engine = Zver::default();
```

### Основные методы

#### `load_url(url: &str) -> Result<(), Box<dyn std::error::Error>>`

Загружает и обрабатывает HTML страницу по указанному URL.

**Параметры:**
- `url` - URL для загрузки (поддерживает `http://`, `https://`, `file://`)

**Возвращает:**
- `Ok(())` при успешной загрузке
- `Err(...)` при ошибке загрузки или парсинга

**Пример:**
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Загрузка локального файла
    engine.load_url("file://index.html").await?;
    
    // Загрузка с веб-сервера
    engine.load_url("https://example.com").await?;
    
    Ok(())
}
```

## Компоненты движка

Zver предоставляет доступ к внутренним компонентам через Arc<RwLock<T>>:

### DOM Engine

```rust
// Чтение DOM
let dom = engine.dom.read().await;
println!("Количество узлов: {}", dom.nodes.len());

// Получение текстового содержимого
let text = dom.get_text_content(node_id);

// Поиск элементов по селектору
let elements = dom.select_ids("div.container");
```

#### Основные методы DOM

- `parse_html(html: &str) -> Result<(), Error>` - Парсинг HTML строки
- `get_text_content(node_id: usize) -> String` - Получение текста узла
- `select_ids(selector: &str) -> Vec<usize>` - Поиск по CSS селектору
- `attribute(node_id: usize, name: &str) -> Option<String>` - Получение атрибута

### CSS Engine

```rust
// Чтение CSS движка
let css = engine.css.read().await;
println!("CSS правил: {}", css.rules.len());
println!("Вычисленных стилей: {}", css.computed_styles.len());

// Запись в CSS движок
let mut css = engine.css.write().await;
css.parse_css("body { color: red; }")?;
```

#### Основные методы CSS

- `parse_css(css: &str) -> Result<(), Error>` - Парсинг CSS строки
- `apply_styles(dom: &Document) -> Result<(), Error>` - Применение стилей к DOM
- `get_computed_style(node_id: usize) -> Option<&ComputedStyle>` - Получение вычисленного стиля

### Layout Engine

```rust
// Чтение layout движка
let layout = engine.layout.read().await;
let results = layout.get_all_layout_results();

for (node_id, result) in results {
    println!("Узел {}: {}x{} at ({}, {})", 
        node_id, result.width, result.height, result.x, result.y);
}
```

#### Основные методы Layout

- `compute_layout(dom: &Document, styles: &HashMap<usize, ComputedStyle>) -> HashMap<usize, LayoutResult>` - Вычисление layout
- `get_all_layout_results() -> &HashMap<usize, LayoutResult>` - Получение всех результатов
- `collect_render_info(dom: &Document) -> Vec<RenderInfo>` - Сбор информации для рендеринга

### JavaScript Engine

```rust
// Исполнение JavaScript
let mut js = engine.js.write().await;
js.execute("console.log('Hello from Zver!');")?;

// Получение результата
let result = js.evaluate("2 + 2")?;
println!("Результат: {}", result);
```

#### Основные методы JS

- `execute(code: &str) -> Result<(), Error>` - Исполнение JavaScript кода
- `evaluate(expression: &str) -> Result<JsValue, Error>` - Вычисление выражения
- `with_dom(dom: Arc<RwLock<Document>>) -> Self` - Привязка к DOM

### Network Engine

```rust
// Загрузка ресурса
let mut network = engine.network.write().await;
let content = network.fetch("https://example.com/style.css").await?;

// Очистка кэша
network.clear_cache_for_url("https://example.com");
```

#### Основные методы Network

- `fetch(url: &str) -> Result<String, Error>` - Загрузка ресурса
- `clear_cache_for_url(url: &str)` - Очистка кэша для URL
- `set_user_agent(agent: &str)` - Установка User-Agent

### Render Engine

```rust
// Рендеринг
let mut render = engine.render.write().await;
render.paint(&layout, &dom).await?;

// Получение текстуры
let texture = render.get_rendered_texture();
```

## Структуры данных

### LayoutResult

```rust
pub struct LayoutResult {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

### ComputedStyle

```rust
pub struct ComputedStyle {
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub font_size: Option<f32>,
    pub margin: Margin,
    pub padding: Padding,
    pub display: Display,
    // ... другие CSS свойства
}
```

### RenderInfo

```rust
pub struct RenderInfo {
    pub node_id: usize,
    pub layout: LayoutResult,
    pub style: ComputedStyle,
    pub content: String,
}
```

## Обработка ошибок

Zver использует стандартную Rust систему обработки ошибок:

```rust
use zver::Zver;

#[tokio::main]
async fn main() {
    let engine = Zver::new();
    
    match engine.load_url("invalid://url").await {
        Ok(_) => println!("Успешно загружено"),
        Err(e) => {
            eprintln!("Ошибка загрузки: {}", e);
            
            // Обработка конкретных типов ошибок
            if let Some(network_error) = e.downcast_ref::<NetworkError>() {
                eprintln!("Сетевая ошибка: {}", network_error);
            }
        }
    }
}
```

## Примеры использования

### Базовая загрузка и анализ

```rust
use zver::Zver;
use zver::dom::serialization::serialize_dom;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Загрузка страницы
    engine.load_url("file://test.html").await?;
    
    // Анализ DOM
    let dom = engine.dom.read().await;
    println!("DOM узлов: {}", dom.nodes.len());
    
    // Сериализация обратно в HTML
    let html = serialize_dom(&dom);
    println!("HTML: {}", html);
    
    // Анализ layout
    let layout = engine.layout.read().await;
    let results = layout.get_all_layout_results();
    
    for (node_id, result) in results.iter().take(5) {
        println!("Узел {}: {:.1}x{:.1}", node_id, result.width, result.height);
    }
    
    Ok(())
}
```

### Работа с CSS

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Загрузка HTML
    {
        let mut dom = engine.dom.write().await;
        dom.parse_html(r#"
            <div class="container">
                <p id="text">Hello World</p>
            </div>
        "#).await?;
    }
    
    // Добавление CSS
    {
        let dom = engine.dom.read().await;
        let mut css = engine.css.write().await;
        
        css.parse_css(r#"
            .container { 
                width: 300px; 
                padding: 20px; 
            }
            #text { 
                color: blue; 
                font-size: 16px; 
            }
        "#)?;
        
        css.apply_styles(&dom)?;
    }
    
    // Вычисление layout
    {
        let dom = engine.dom.read().await;
        let css = engine.css.read().await;
        let mut layout = engine.layout.write().await;
        
        layout.compute_layout(&dom, &css.computed_styles);
    }
    
    Ok(())
}
```

### Исполнение JavaScript

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Загрузка HTML с JavaScript
    engine.load_url("file://app.html").await?;
    
    // Дополнительное исполнение JS
    {
        let mut js = engine.js.write().await;
        
        // Выполнение кода
        js.execute(r#"
            function updateTitle() {
                document.title = "Updated by Zver";
            }
            updateTitle();
        "#)?;
        
        // Получение значения
        let result = js.evaluate("2 + 2")?;
        println!("JS результат: {}", result);
    }
    
    Ok(())
}
```

## Конфигурация

### Настройка размеров viewport

```rust
use zver::layout::LayoutEngine;

let mut layout = LayoutEngine::new(1920.0, 1080.0); // Full HD
// или
let mut layout = LayoutEngine::new(375.0, 667.0);   // iPhone размер
```

### Настройка сети

```rust
// Пользовательский User-Agent
let mut network = engine.network.write().await;
network.set_user_agent("Zver Browser 1.0");
```

## Производительность

### Рекомендации по оптимизации

1. **Используйте read() вместо write()** когда возможно
2. **Минимизируйте время удержания блокировок**
3. **Кэшируйте результаты layout** для статичного контента
4. **Используйте параллельную обработку** для больших DOM деревьев

```rust
// Хорошо: быстрое чтение
{
    let dom = engine.dom.read().await;
    let node_count = dom.nodes.len();
} // блокировка освобождена

// Плохо: долгое удержание блокировки
let dom = engine.dom.read().await;
// ... много операций ...
let node_count = dom.nodes.len();
```

## Отладка

### Включение логирования

```rust
env_logger::init();
// или
tracing_subscriber::fmt::init();
```

### Получение отладочной информации

```rust
// Статистика DOM
let dom = engine.dom.read().await;
println!("DOM статистика:");
println!("  Узлов: {}", dom.nodes.len());
println!("  Корень: {:?}", dom.root);

// Статистика CSS
let css = engine.css.read().await;
println!("CSS статистика:");
println!("  Правил: {}", css.rules.len());
println!("  Стилей: {}", css.computed_styles.len());

// Статистика Layout
let layout = engine.layout.read().await;
let results = layout.get_all_layout_results();
println!("Layout статистика:");
println!("  Результатов: {}", results.len());
```
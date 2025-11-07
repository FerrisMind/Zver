# Архитектура Zver Browser Engine

Данный документ описывает внутреннюю архитектуру браузерного движка Zver, принципы проектирования и взаимодействие компонентов.

## Общий обзор

Zver построен на модульной архитектуре с четким разделением ответственности между компонентами. Каждый модуль отвечает за конкретную область функциональности и взаимодействует с другими через определенные интерфейсы.

```
┌─────────────────────────────────────────────────────────────┐
│                    Zver Engine Core                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │    DOM      │  │    CSS      │  │      Layout         │  │
│  │   Engine    │  │   Engine    │  │      Engine         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ JavaScript  │  │   Network   │  │      Render         │  │
│  │   Engine    │  │   Engine    │  │      Engine         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Resource Loader                            │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Принципы проектирования

### 1. Модульность
- Каждый компонент инкапсулирован в отдельном модуле
- Четкие интерфейсы между модулями
- Возможность независимого тестирования и разработки

### 2. Асинхронность
- Все операции ввода-вывода асинхронны (Tokio)
- Неблокирующая обработка сетевых запросов
- Параллельная обработка CSS и layout вычислений

### 3. Потокобезопасность
- Использование Arc<RwLock<T>> для разделяемого состояния
- Минимизация времени удержания блокировок
- Избежание deadlock'ов через упорядоченное получение блокировок

### 4. Производительность
- Параллельная обработка через Rayon
- Кэширование вычислений
- Ленивые вычисления где возможно
- GPU-ускоренный рендеринг

## Компоненты системы

### DOM Engine (`src/dom/`)

**Назначение:** Парсинг HTML и управление деревом документа.

**Основные файлы:**
- `dom.rs` - Основная структура Document и Node
- `serialization.rs` - Сериализация DOM обратно в HTML

**Ключевые структуры:**
```rust
pub struct Document {
    pub nodes: HashMap<usize, Node>,
    pub root: Option<usize>,
    next_id: usize,
}

pub struct Node {
    pub id: usize,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}
```

**Зависимости:**
- `kuchikiki` - HTML парсинг
- `selectors` - CSS селекторы для поиска элементов

**Поток обработки:**
1. HTML строка → kuchikiki парсер
2. Построение внутреннего представления Node
3. Создание индексированной структуры HashMap<usize, Node>
4. Поддержка CSS селекторов для поиска элементов

### CSS Engine (`src/css/`)

**Назначение:** Парсинг CSS и применение стилей к DOM элементам.

**Основные файлы:**
- `mod.rs` - Основная структура StyleEngine
- `parser.rs` - CSS парсинг через cssparser
- `selectors.rs` - Обработка CSS селекторов
- `properties.rs` - CSS свойства и их значения
- `color.rs` - Работа с цветами
- `fonts.rs` - Управление шрифтами
- `animations.rs` - CSS анимации
- `media_queries.rs` - Медиа-запросы

**Ключевые структуры:**
```rust
pub struct StyleEngine {
    pub rules: Vec<CSSRule>,
    pub computed_styles: HashMap<usize, ComputedStyle>,
    parsed_selectors: Vec<SelectorList<SimpleSelector>>,
    selector_cache: HashMap<String, CompiledSelector>,
}

pub struct ComputedStyle {
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub font_size: Option<f32>,
    pub margin: Margin,
    pub padding: Padding,
    pub display: Display,
    // ... другие свойства
}
```

**Зависимости:**
- `cssparser` - Парсинг CSS синтаксиса
- `selectors` - Компиляция и сопоставление селекторов

**Поток обработки:**
1. CSS строка → cssparser → AST
2. Компиляция селекторов через selectors crate
3. Сопоставление селекторов с DOM элементами
4. Вычисление каскада и специфичности
5. Создание ComputedStyle для каждого элемента

### Layout Engine (`src/layout/`)

**Назначение:** Вычисление позиций и размеров элементов.

**Основные файлы:**
- `compute.rs` - Основная логика layout вычислений
- `types.rs` - Типы данных для layout
- `taffy_integration.rs` - Интеграция с Taffy layout engine
- `text_measure.rs` - Измерение текста
- `styles.rs` - Преобразование CSS стилей в Taffy стили
- `render.rs` - Подготовка данных для рендеринга

**Ключевые структуры:**
```rust
pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
    taffy: Taffy,
    node_map: HashMap<usize, NodeId>, // DOM ID -> Taffy NodeId
    layout_results: HashMap<usize, LayoutResult>,
    resolved_styles: HashMap<usize, ResolvedStyle>,
}

pub struct LayoutResult {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

**Зависимости:**
- `taffy` - Layout engine с поддержкой Flexbox/Grid
- `fontdue` - Измерение текста

**Поток обработки:**
1. ComputedStyle → Taffy Style конвертация
2. Построение Taffy дерева узлов
3. Вычисление layout через Taffy
4. Извлечение результатов в LayoutResult
5. Подготовка RenderInfo для рендеринга

### Render Engine (`src/render/`)

**Назначение:** GPU-рендеринг элементов на экран.

**Основные файлы:**
- `mod.rs` - Основная структура RenderEngine
- `initialization.rs` - Инициализация WGPU
- `operations.rs` - Операции рендеринга
- `types.rs` - Типы данных для рендеринга
- `utils.rs` - Вспомогательные функции

**Ключевые структуры:**
```rust
pub struct RenderEngine {
    device: Option<Device>,
    queue: Option<Queue>,
    surface: Option<Surface>,
    render_pipeline: Option<RenderPipeline>,
}

pub struct RenderInfo {
    pub node_id: usize,
    pub layout: LayoutResult,
    pub style: ComputedStyle,
    pub content: String,
}
```

**Зависимости:**
- `wgpu` - GPU рендеринг
- `wgpu_text` - Рендеринг текста
- `bytemuck` - Безопасное приведение типов для GPU

**Поток обработки:**
1. RenderInfo → GPU буферы
2. Создание render pipeline
3. Рендеринг примитивов (прямоугольники, текст)
4. Композиция финального изображения

### JavaScript Engine (`src/js.rs`)

**Назначение:** Исполнение JavaScript кода и DOM API.

**Ключевые структуры:**
```rust
pub struct JSEngine {
    context: Context,
    dom_ref: Option<Arc<RwLock<Document>>>,
}
```

**Зависимости:**
- `boa_engine` - JavaScript движок

**Поток обработки:**
1. JavaScript код → Boa парсер
2. Исполнение в изолированном контексте
3. DOM API через привязки к Document
4. Обработка событий и колбэков

### Network Engine (`src/network.rs`)

**Назначение:** Загрузка ресурсов по сети.

**Ключевые структуры:**
```rust
pub struct NetworkEngine {
    client: Client,
    cache: HashMap<String, CachedResource>,
}
```

**Зависимости:**
- `reqwest` - HTTP клиент
- `tokio` - Асинхронные операции

**Поток обработки:**
1. URL запрос → HTTP/HTTPS/File загрузка
2. Кэширование ответов
3. Обработка редиректов и ошибок
4. Возврат содержимого как String

### Resource Loader (`src/resource_loader.rs`)

**Назначение:** Координация загрузки различных типов ресурсов.

**Поток обработки:**
1. Анализ типа ресурса (HTML, CSS, JS, изображения)
2. Делегирование загрузки Network Engine
3. Обработка зависимостей между ресурсами
4. Уведомление о завершении загрузки

## Поток обработки страницы

### 1. Инициализация
```rust
let engine = Zver::new();
```
- Создание всех компонентов
- Инициализация WGPU для рендеринга
- Настройка JavaScript контекста

### 2. Загрузка URL
```rust
engine.load_url("https://example.com").await?;
```

**Детальный поток:**

1. **Network Engine** загружает HTML
2. **DOM Engine** парсит HTML в дерево узлов
3. **CSS Engine** извлекает и парсит CSS из `<style>` тегов
4. **CSS Engine** применяет стили к DOM элементам
5. **JavaScript Engine** исполняет код из `<script>` тегов
6. **Layout Engine** вычисляет позиции элементов
7. **Render Engine** отрисовывает элементы

### 3. Обновление и перерендеринг

При изменении DOM через JavaScript:
1. DOM изменения → уведомление CSS Engine
2. Пересчет затронутых стилей
3. Инкрементальный пересчет layout
4. Частичный перерендеринг

## Управление памятью

### Стратегии оптимизации

1. **Reference Counting:** Arc для разделяемых данных
2. **Copy-on-Write:** Клонирование только при изменении
3. **Pooling:** Переиспользование объектов рендеринга
4. **Lazy Loading:** Загрузка ресурсов по требованию

### Избежание утечек памяти

- Слабые ссылки для циклических зависимостей
- Автоматическая очистка кэшей
- Ограничение размера кэшей

## Обработка ошибок

### Стратегия обработки

1. **Graceful Degradation:** Продолжение работы при частичных ошибках
2. **Error Propagation:** Использование Result<T, E> для всех fallible операций
3. **Logging:** Подробное логирование для отладки
4. **Recovery:** Автоматическое восстановление после ошибок

### Типы ошибок

```rust
#[derive(Debug, thiserror::Error)]
pub enum ZverError {
    #[error("DOM parsing error: {0}")]
    DomError(String),
    
    #[error("CSS parsing error: {0}")]
    CssError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("JavaScript error: {0}")]
    JsError(String),
    
    #[error("Render error: {0}")]
    RenderError(String),
}
```

## Тестирование

### Стратегия тестирования

1. **Unit Tests:** Тестирование отдельных компонентов
2. **Integration Tests:** Тестирование взаимодействия компонентов
3. **End-to-End Tests:** Тестирование полного цикла обработки
4. **Performance Tests:** Бенчмарки производительности

### Тестовые данные

- `test.html`, `test_phase1.html` и т.д. - HTML файлы для тестирования
- Модульные тесты в каждом компоненте
- Интеграционные тесты в `tests/` директории

## Расширяемость

### Добавление новых CSS свойств

1. Добавить свойство в `ComputedStyle`
2. Реализовать парсинг в `css/properties.rs`
3. Добавить обработку в Layout Engine
4. Реализовать рендеринг в Render Engine

### Добавление новых HTML элементов

1. Расширить DOM парсинг
2. Добавить специфичную обработку в Layout
3. Реализовать рендеринг элемента

### Добавление JavaScript API

1. Создать привязки в JS Engine
2. Реализовать методы в соответствующих компонентах
3. Добавить обработку событий

## Производительность

### Критические пути

1. **CSS Selector Matching:** Оптимизировано через кэширование
2. **Layout Computation:** Использует эффективный Taffy engine
3. **Text Rendering:** GPU-ускоренный через wgpu_text
4. **DOM Traversal:** Оптимизированные структуры данных

### Профилирование

```bash
# Профилирование CPU
cargo build --release
perf record --call-graph=dwarf ./target/release/zver-egui
perf report

# Профилирование памяти
valgrind --tool=massif ./target/release/zver-egui
```

## Будущие улучшения

### Краткосрочные (1-3 месяца)

- Поддержка CSS Grid Layout
- Расширенные JavaScript API
- Оптимизация рендеринга текста
- Поддержка изображений

### Среднесрочные (3-6 месяцев)

- WebAssembly интеграция
- Поддержка веб-компонентов
- CSS анимации и переходы
- Мобильная оптимизация

### Долгосрочные (6+ месяцев)

- Многопроцессная архитектура
- Поддержка WebGL
- Расширенная поддержка HTML5 API
- Плагинная система

## Заключение

Архитектура Zver спроектирована для обеспечения высокой производительности, модульности и расширяемости. Четкое разделение ответственности между компонентами позволяет независимо развивать каждую область функциональности, а асинхронная архитектура обеспечивает отзывчивость пользовательского интерфейса.
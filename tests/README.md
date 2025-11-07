# Тестовые файлы для реализации CSS поддержки

Этот каталог содержит тестовые HTML/CSS файлы для каждой фазы плана полной поддержки CSS в Zver Browser Engine.

## Структура тестов

### Фаза 2: @-правила

- **`phase2_media_queries.html`** - Тестирование @media queries
  - min-width, max-width
  - orientation
  - min-height, max-height
  - screen vs print
  - not modifier
  - aspect-ratio

- **`phase2_keyframes.html`** - Тестирование @keyframes
  - Простые анимации (from/to)
  - Множественные шаги (0%, 50%, 100%)
  - Infinite анимации
  - Комплексные анимации с несколькими свойствами

- **`phase2_font_face.html`** - Тестирование @font-face
  - Загрузка шрифтов из файлов
  - font-weight и font-style варианты
  - local() fallback
  - font-display
  - unicode-range

### Фаза 3: Псевдоклассы и псевдоэлементы

- **`phase3_pseudo_classes.html`** - Тестирование псевдоклассов
  - Структурные: :first-child, :last-child, :nth-child, :only-child
  - Типовые: :first-of-type, :last-of-type, :nth-of-type
  - Состояния: :hover, :focus, :active, :disabled, :checked

- **`phase3_pseudo_elements.html`** - Тестирование псевдоэлементов
  - ::before и ::after (текст, эмодзи, background)
  - ::first-letter (drop cap)
  - ::first-line
  - Комбинации псевдоэлементов

### Фаза 4: Расширение свойств и единиц

- **`phase4_properties_units.html`** - Тестирование новых единиц и свойств
  - rem единицы
  - vmin, vmax единицы
  - calc() функция
  - transition-* свойства
  - animation-* свойства
  - box-sizing

- **`phase4_grid_properties.html`** - Тестирование Grid свойств
  - grid-template-rows/columns
  - grid-auto-rows/columns
  - grid-auto-flow
  - grid-gap
  - grid-row/grid-column
  - repeat() и minmax()
  - grid-template-areas

### Фаза 5: Каскад и наследование

- **`phase5_cascade_inheritance.html`** - Тестирование каскада
  - Базовое наследование
  - Переопределение свойств
  - Специфичность селекторов
  - !important
  - Inline стили
  - Порядок правил
  - Наследуемые vs ненаследуемые свойства
  - User-Agent стили

### Фаза 6: Grid Layout

- **`phase6_grid_layout.html`** - Тестирование полной Grid Layout интеграции
  - Простые Grid макеты
  - Разные размеры колонок/строк
  - auto-rows
  - Размещение элементов (span)
  - Именованные линии
  - minmax() и auto-fit/auto-fill
  - Выравнивание
  - row-gap и column-gap

### Фаза 7: Анимации и Transitions (runtime)

- **`phase7_animations_transitions.html`** - Тестирование runtime анимаций
  - Transitions: цвет, transform, множественные свойства
  - Transition delay и timing functions
  - Animations: slide, rotate, bounce, color change
  - Animation properties: duration, delay, iteration, direction, fill-mode

## Использование

### Запуск тестов через zver-egui

1. Откройте `zver-egui` приложение
2. Загрузите тестовый файл:
   ```rust
   let url = format!("file://{}", std::path::Path::new("tests/phase2_media_queries.html").canonicalize()?);
   engine.load_url(&url).await?;
   ```

### Запуск через примеры

Создайте новый пример в `examples/`:

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push("phase2_media_queries.html");
    let url = format!("file://{}", path.display());
    
    engine.load_url(&url).await?;
    
    // Проверка результатов
    let dom = engine.dom.read().await;
    println!("DOM nodes: {}", dom.nodes.len());
    
    Ok(())
}
```

### Запуск unit-тестов

```bash
# Все CSS тесты
cargo test --package zver --test css_phase2_tests
cargo test --package zver --test css_phase3_tests
cargo test --package zver --test css_phase4_tests
cargo test --package zver --test css_phase5_tests

# Конкретный тест
cargo test --package zver test_media_query_parsing
```

## Ожидаемые результаты

### Фаза 2 (✅ Завершена)
- ✅ Парсинг @media, @keyframes, @font-face
- ⚠️ Runtime применение @media (требует viewport context)
- ⚠️ Runtime анимации (требует animation loop)
- ⚠️ Загрузка шрифтов по сети (требует network.rs интеграцию)

### Фаза 3 (⬜️ В процессе)
- ⬜️ Структурные псевдоклассы
- ⬜️ Псевдоклассы состояния (требуют UI события)
- ⬜️ Псевдоэлементы (требуют виртуальные узлы)

### Фаза 4 (⬜️ В процессе)
- ⬜️ Новые единицы (rem, vmin, vmax, calc)
- ⬜️ Transition/Animation свойства
- ⬜️ Grid свойства (полная поддержка)

### Фаза 5 (⬜️ В процессе)
- ⬜️ User-Agent стили
- ⬜️ Полное наследование
- ⬜️ Cascade origin

### Фаза 6 (⬜️ В процессе)
- ⬜️ Полная Grid Layout интеграция с Taffy

### Фаза 7 (⬜️ В процессе)
- ⬜️ Transition engine
- ⬜️ Animation engine
- ⬜️ Интеграция с render loop

## Отладка

Если тесты не проходят:

1. **Проверьте парсинг CSS:**
   ```rust
   let mut engine = StyleEngine::new();
   match engine.parse_css(css) {
       Ok(_) => println!("✅ CSS parsed successfully"),
       Err(e) => println!("❌ Parse error: {}", e),
   }
   ```

2. **Проверьте применение стилей:**
   ```rust
   let dom = engine.dom.read().await;
   let css = engine.css.read().await;
   let computed = css.computed_styles.get(&node_id);
   println!("Computed styles: {:?}", computed);
   ```

3. **Проверьте layout:**
   ```rust
   let layout = engine.layout.read().await;
   let render_info = layout.get_all_render_info(&dom);
   for info in render_info {
       println!("Node {}: {:?}", info.node_id, info.bounds);
   }
   ```

## Примечания

- Некоторые тесты требуют runtime реализации (анимации, transitions, :hover)
- Media queries требуют viewport context для матчинга
- Псевдоэлементы требуют создания виртуальных узлов в DOM
- Grid Layout требует полной интеграции с Taffy


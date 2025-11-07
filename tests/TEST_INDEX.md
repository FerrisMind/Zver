# Индекс тестовых файлов

## Быстрый доступ к тестам

### Фаза 2: @-правила ✅

| Файл | Описание | Статус |
|------|----------|--------|
| `phase2_media_queries.html` | Тестирование @media queries | ✅ Парсинг работает |
| `phase2_keyframes.html` | Тестирование @keyframes | ✅ Парсинг работает |
| `phase2_font_face.html` | Тестирование @font-face | ✅ Парсинг работает |

**Unit-тесты:** `crates/zver/tests/css_phase2_tests.rs`

### Фаза 3: Псевдоклассы и псевдоэлементы ⬜️

| Файл | Описание | Статус |
|------|----------|--------|
| `phase3_pseudo_classes.html` | Структурные и state псевдоклассы | ⬜️ В разработке |
| `phase3_pseudo_elements.html` | ::before, ::after, ::first-letter, ::first-line | ⬜️ В разработке |

**Unit-тесты:** `crates/zver/tests/css_phase3_tests.rs`

### Фаза 4: Расширение свойств и единиц ⬜️

| Файл | Описание | Статус |
|------|----------|--------|
| `phase4_properties_units.html` | rem, vmin, vmax, calc(), transitions, animations | ⬜️ В разработке |
| `phase4_grid_properties.html` | Grid свойства (grid-template-*, grid-auto-*, etc.) | ⬜️ В разработке |

**Unit-тесты:** `crates/zver/tests/css_phase4_tests.rs`

### Фаза 5: Каскад и наследование ⬜️

| Файл | Описание | Статус |
|------|----------|--------|
| `phase5_cascade_inheritance.html` | Специфичность, !important, наследование, UA styles | ⬜️ В разработке |

**Unit-тесты:** `crates/zver/tests/css_phase5_tests.rs`

### Фаза 6: Grid Layout ⬜️

| Файл | Описание | Статус |
|------|----------|--------|
| `phase6_grid_layout.html` | Полная Grid Layout интеграция с Taffy | ⬜️ В разработке |

### Фаза 7: Анимации и Transitions (runtime) ⬜️

| Файл | Описание | Статус |
|------|----------|--------|
| `phase7_animations_transitions.html` | Runtime transitions и animations | ⬜️ В разработке |

## Запуск тестов

### Через пример

```bash
cargo run --example test_css_phases -- phase2_media_queries
```

### Через unit-тесты

```bash
# Все тесты Фазы 2
cargo test --package zver --test css_phase2_tests

# Конкретный тест
cargo test --package zver test_media_query_parsing
```

### Через zver-egui

Загрузите HTML файл через `file://` протокол в приложении.

## Структура тестов

Каждый HTML файл содержит:
- ✅ Примеры использования CSS функций
- ✅ Комментарии с ожидаемым поведением
- ✅ Визуальные индикаторы для проверки
- ✅ Код CSS для справки

## Примечания

- ⚠️ Некоторые тесты требуют runtime реализации (анимации, :hover)
- ⚠️ Media queries требуют viewport context
- ⚠️ Псевдоэлементы требуют виртуальных узлов в DOM
- ✅ Парсинг @-правил работает (Фаза 2)


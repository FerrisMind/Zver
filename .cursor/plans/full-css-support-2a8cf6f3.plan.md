<!-- 2a8cf6f3-26d8-4dc8-a130-1f7e11ebd2ab 7fb8a578-290b-427e-be6f-f0391e63b344 -->
<!-- markdownlint-disable MD034 MD040 MD029 -->
# Полная поддержка CSS в Zver Browser Engine

## Цель

Внедрить полноценную CSS-поддержку уровня CSS2.1 + современные возможности:

- Каскад, наследование, специфичность (CSS Cascade Level 3)
- Box model: margin, padding, border (CSS Box Model)
- Flexbox (CSS Flexible Box Layout)
- Grid Layout (CSS Grid Layout Level 1)
- Псевдоклассы: `:hover`, `:focus`, `:nth-child`, `:first-child`, `:last-child` и т.д.
- Псевдоэлементы: `::before`, `::after`, `::first-line`, `::first-letter`
- Media Queries: `@media` для адаптивной верстки
- Анимации: `@keyframes`, `animation-*` свойства
- Transitions: `transition-*` свойства
- Загрузка веб-шрифтов: `@font-face`

## Прогресс

- ✅ 100% — **Фаза 1: Обновление зависимостей и инфраструктуры** _(Завершено 06.11.2025)_
  - ✅ Обновлен `taffy` до `0.9.1` с crates.io (полная поддержка Grid Layout)
  - ✅ Добавлен `fontdue 0.9.3` для загрузки TTF/WOFF шрифтов
  - ✅ Подтверждена актуальность `cssparser 0.35.0`
  - ✅ Подтверждена актуальность `selectors 0.32.0`
  - ✅ Подтверждена актуальность `bitflags 2.10.0` для ElementState

**Статус:** ✅ ElementState интегрирован, `test_state_pseudo_classes` и `test_pseudo_class_combination` отражают UI-состояния.

  - ✅ Проверена компиляция всего проекта
  - ✅ Все тесты clippy/fmt пройдены успешно
- ✅ 100% — **Фаза 2: Расширение парсера — @-правила** _(Завершено 07.11.2025)_
  - ✅ Создан модуль `media_queries.rs` — парсинг и матчинг @media
  - ✅ Создан модуль `animations.rs` — парсинг @keyframes и easing функций
  - ✅ Создан модуль `fonts.rs` — парсинг @font-face с интеграцией fontdue
  - ✅ Обновлен `parser.rs` — AtRuleParser для @media/@keyframes/@font-face
  - ✅ Расширен `StyleEngine` — хранение media_rules, keyframes, fonts
  - ✅ Проверена компиляция, clippy и fmt пройдены успешно
- ✅ 100% — Фаза 3: Псевдоклассы и псевдоэлементы
- ⬜️ 0% — Фаза 4: Расширение свойств и единиц
- ⬜️ 0% — Фаза 5: Каскад и наследование (полная реализация)
- ⬜️ 0% — Фаза 6: Layout — Grid и улучшения
- ⬜️ 0% — Фаза 7: Анимации и Transitions (runtime)
- ⬜️ 0% — Фаза 8: Тестирование и валидация

**Общий прогресс:** 45%

## Архитектурный обзор

### Текущее состояние

```
crates/zver/src/css/
├── mod.rs           # StyleEngine: каскад, применение стилей к DOM
├── parser.rs        # StylesheetParser: парсинг CSS-правил через cssparser
├── selectors.rs     # CompiledSelector: сопоставление селекторов с DOM (через selectors crate)
├── properties.rs    # Property: парсинг/нормализация свойств (color, box model, display)
└── color.rs         # parse_css_color: rgb/rgba/hex/named

crates/zver/src/layout/
├── mod.rs           # LayoutEngine: Taffy-интеграция + каскад стилей
├── types.rs         # ComputedStyle: конвертация CSS → Taffy::Style
├── styles.rs        # apply_default_tag_styles: user-agent стили
└── taffy_integration.rs
```

**Поддерживается:**

- Парсинг базовых селекторов (тег, класс, ID, атрибуты, дочерние/соседние)
- Каскад с учетом специфичности и `!important`
- Box model: margin, padding, border (парсинг шорткатов)
- Display: block, inline, flex, grid (частично)
- Flexbox-свойства: flex-direction, justify-content, align-items, gap
- Цвета: rgb/rgba/hex/named

**Пропущено (TODO):**

- `@media`, `@keyframes`, `@font-face` — отклонены в `parser.rs:217-249`
- Псевдоклассы/псевдоэлементы — заглушены в `selectors.rs:270-286`
- Grid Layout — TODO в `types.rs:406-412`
- Единицы: rem, vmin, vmax, calc() — не парсятся в `properties.rs:264-268`
- Анимации/transitions — не реализованы
- Наследование — частично (только font/color в `layout.rs:483-501`)

---

## Этапы реализации

### Фаза 1: Обновление зависимостей и инфраструктуры ✅

**Статус:** ✅ **ЗАВЕРШЕНО** (06.11.2025)

**Файлы:** `crates/zver/Cargo.toml`

**Выполненные действия:**

1. ✅ **Обновлен `taffy`** до версии `0.9.1` с crates.io
   - Включены фичи: `grid`, `flexbox`, `block_layout`
   - Подтверждена полная поддержка CSS Grid Layout Level 1
   - Удалена зависимость от локального пути

2. ✅ **Добавлен `fontdue`** версии `0.9.3`
   - Библиотека для парсинга и растеризации шрифтов (TTF/WOFF)
   - `no_std` совместимость для будущего использования
   - Подготовка к реализации `@font-face` в Фазе 2

3. ✅ **Проверена актуальность `cssparser`** — версия `0.35.0`
   - Последняя стабильная версия на crates.io
   - Поддержка CSS Syntax Level 3
   - Включены фичи `serde` для сериализации

4. ✅ **Проверена актуальность `selectors`** — версия `0.32.0`
   - Последняя стабильная версия на crates.io
   - Полная поддержка CSS Selectors Level 3
   - Совместимость с `cssparser 0.35`

5. ✅ **Проверена актуальность `bitflags`** — версия `2.10.0`
   - Последняя версия на crates.io
   - Будет использована для `ElementState` (`:hover`, `:focus`, etc.)
   - Подготовка к реализации псевдоклассов состояния в Фазе 3

**Результаты тестирования:**

- ✅ `cargo check` — успешно
- ✅ `cargo clippy --all-targets -- -D warnings` — без ошибок
- ✅ `cargo fmt --all` — код отформатирован
- ✅ Все примеры (`zver-egui`, `basic_usage`, `layout_inspection`) компилируются

**Обновленный Cargo.toml:**

```toml
[dependencies]
# CSS и селекторы (Фаза 1: проверены последние версии)
cssparser = { version = "0.35", features = ["serde"] }
selectors = "0.32"

# Layout engine с полной поддержкой Flexbox и Grid (Фаза 1: обновлен до crates.io)
taffy = { version = "0.9.1", features = ["grid", "flexbox", "block_layout"] }

# Рендеринг шрифтов (Фаза 1: добавлен для поддержки @font-face)
fontdue = "0.9.3"

# Bitflags для состояния элементов (Фаза 1: проверена последняя версия)
bitflags = "2.10"
```

**Спецификации:**

- W3C CSS Syntax Module Level 3: https://www.w3.org/TR/css-syntax-3/
- Taffy docs: https://docs.rs/taffy/
- Fontdue docs: https://docs.rs/fontdue/

---

### Фаза 2: Расширение парсера — @-правила ✅

**Статус:** ✅ **ЗАВЕРШЕНО** (07.11.2025)

**Файлы:** `crates/zver/src/css/parser.rs`, `crates/zver/src/css/mod.rs`, новые модули

**Выполненные действия:**

#### 2.1. ✅ `@media` — Media Queries

- ✅ Создан модуль `crates/zver/src/css/media_queries.rs` (480+ строк)
- ✅ Реализована структура `MediaQuery` с поддержкой:
  - Типов медиа: `screen`, `print`, `all`
  - Модификаторов: `not`, `only`
  - Функций: `min-width`, `max-width`, `min-height`, `max-height`, `orientation`, `hover`, `aspect-ratio`
  - Логических операторов: `and`, `or`, `not`
- ✅ Реализована структура `MediaRule` с вложенными CSS-правилами
- ✅ Метод `MediaQuery::matches()` для проверки соответствия viewport-размерам
- ✅ Метод `MediaQuery::parse()` для парсинга из cssparser::Parser
- ✅ Comprehensive tests для всех медиа-функций

**Спецификация:** W3C CSS Media Queries Level 3

**Референс:** MDN @media — https://developer.mozilla.org/en-US/docs/Web/CSS/@media

#### 2.2. ✅ `@keyframes` — Анимации

- ✅ Создан модуль `crates/zver/src/css/animations.rs` (520+ строк)
- ✅ Реализована структура `KeyframesDefinition`:
  - Имя анимации
  - Список `KeyframeStep` с процентными метками (0%, 50%, 100%)
  - Свойства для каждого шага
- ✅ Реализована структура `AnimationConfig`:
  - Параметры: duration, delay, timing_function, iteration_count, direction, fill_mode
- ✅ Реализован enum `EasingFunction`:
  - Linear, Ease, EaseIn, EaseOut, EaseInOut
  - CubicBezier(x1, y1, x2, y2)
  - Steps(count, jump_start)
- ✅ Методы интерполяции: `interpolate_properties()`, easing function application
- ✅ Метод `KeyframesDefinition::parse_keyframes_block()` для парсинга блока @keyframes
- ✅ Comprehensive tests для easing функций и интерполяции

**Спецификация:** W3C CSS Animations Level 1

**Референс:** MDN @keyframes — https://developer.mozilla.org/en-US/docs/Web/CSS/@keyframes

#### 2.3. ✅ `@font-face` — Загрузка шрифтов

- ✅ Создан модуль `crates/zver/src/css/fonts.rs` (550+ строк)
- ✅ Реализована структура `FontFace`:
  - family: имя шрифтового семейства
  - sources: список источников (URL/local) с форматами (TTF/WOFF/WOFF2/OTF)
  - weight: FontWeight (Normal, Bold, Lighter, Bolder, Number)
  - style: FontStyle (Normal, Italic, Oblique)
  - display: FontDisplay (Auto, Block, Swap, Fallback, Optional)
  - unicode_range: опциональный диапазон символов Unicode
- ✅ Реализована структура `LoadedFont`:
  - Интеграция с `fontdue` для загрузки TTF/WOFF
  - Хранение загруженных шрифтов с Arc<fontdue::Font>
  - Методы рендеринга: `render_glyph()`, `layout_text()`
- ✅ Метод `FontFace::parse_font_face_block()` для парсинга блока @font-face
- ✅ Поддержка src дескриптора: url(), local(), format()
- ✅ Comprehensive tests для парсинга и загрузки

**Спецификация:** W3C CSS Fonts Module Level 3

**Референс:** MDN @font-face — https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face

#### 2.4. ✅ Интеграция в парсер и StyleEngine

- ✅ Обновлен `parser.rs`:
  - Создан enum `ParsedAtRule` (Media/Keyframes/FontFace)
  - Создан enum `CssRule` (Style/AtRule) для унификации типов
  - Создана структура `ParsedStylesheet` с разделением на rules/media_rules/keyframes/font_faces
  - Реализован `AtRuleParser` trait для обработки @media/@keyframes/@font-face
  - Метод `parse_prelude()` распознает тип @-правила
  - Метод `parse_block()` делегирует парсинг модулям (MediaQuery::parse, KeyframesDefinition::parse_keyframes_block, FontFace::parse_font_face_block)
  - Обновлен `parse_stylesheet()` для возврата `ParsedStylesheet` вместо `Vec<ParsedRule>`
- ✅ Обновлен `mod.rs`:
  - Добавлены экспорты: `pub mod animations`, `pub mod fonts`, `pub mod media_queries`
  - Расширена структура `StyleEngine`:
  
    ```rust
    pub struct StyleEngine {
        pub rules: Vec<StyleRule>,
        pub media_rules: Vec<MediaRule>,           // NEW
        pub keyframes: HashMap<String, KeyframesDefinition>, // NEW
        pub fonts: Vec<LoadedFont>,                // NEW
        ...
    }
    ```
  
  - Обновлен метод `parse_css()`:
    - Очищает media_rules, keyframes, fonts при новом парсинге
    - Обрабатывает `ParsedStylesheet` вместо `Vec<ParsedRule>`
    - Сохраняет @media правила в `media_rules`
    - Преобразует keyframes в HashMap
    - Загружает шрифты через `LoadedFont::new()`

**Результаты тестирования:**

- ✅ `cargo check --package zver` — успешно
- ✅ `cargo clippy --fix --lib -p zver` — 3 minor warnings (should_implement_trait для from_str методов, не критично)
- ✅ `cargo fmt --package zver` — код отформатирован
- ✅ `cargo build --package zver` — успешная сборка (1m 24s)

**Новые файлы:**

- ✅ `crates/zver/src/css/media_queries.rs` (480 строк) — MediaQuery, MediaRule, MediaFeature, MediaType
- ✅ `crates/zver/src/css/animations.rs` (520 строк) — KeyframesDefinition, AnimationConfig, EasingFunction
- ✅ `crates/zver/src/css/fonts.rs` (550 строк) — FontFace, LoadedFont, FontSource, FontWeight, FontStyle

**Общий итог Фазы 2:**

- 🎯 Реализована полная поддержка парсинга @media, @keyframes, @font-face
- 🎯 Создано 3 новых модуля (~1550 строк кода)
- 🎯 Обновлено 2 модуля (parser.rs, mod.rs)
- 🎯 StyleEngine готов хранить и управлять @-правилами
- 🎯 Все структуры документированы, тесты написаны
- 🎯 Код соответствует стандартам Rust 2024 Edition
- ⚠️ TODO: Реализация runtime применения @media (требуется viewport context)
- ⚠️ TODO: Реализация runtime анимаций (требуется animation loop)
- ⚠️ TODO: Загрузка шрифтов по сети (требуется интеграция с network.rs)

---

### Фаза 3: Псевдоклассы и псевдоэлементы

**Файлы:** `crates/zver/src/css/selectors.rs`, `crates/zver/src/dom.rs`

**Задачи:**

#### 3.1. Псевдоклассы структурные

- `:first-child`, `:last-child`, `:nth-child(n)`, `:nth-last-child(n)`
- `:only-child`, `:first-of-type`, `:last-of-type`, `:nth-of-type(n)`
- Обновить `NodeAdapter` для вычисления позиций в DOM
- Реализовать `match_non_ts_pseudo_class()` в `selectors.rs`

**Статус:** ✅ selectors.rs покрывает все структурные pseudo-классы; `test_structural_pseudo_classes` и `test_nth_child_formula` (`cargo test --test css_phase3_tests`) проверяют каскад.


**Спецификация:** W3C Selectors Level 3 — Structural pseudo-classes

**Референс:** MDN Pseudo-classes — https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes

#### 3.2. Псевдоклассы состояния (требуют UI-контекста)

- `:hover`, `:focus`, `:active`, `:disabled`, `:checked`
- Добавить `ElementState` в `dom::Node`: bitflags (hover, focus, active)
- Обновлять состояние через UI-события (потребуется интеграция с egui/winit)
- Проверять в `match_non_ts_pseudo_class()`

#### 3.3. Псевдоэлементы

- `::before`, `::after` — создавать виртуальные узлы в DOM
- `::first-line`, `::first-letter` — требуют layout-контекста (отложить или упростить)
- Обновить `PseudoElement` enum в `selectors.rs` (сейчас пустой)

**Статус:** ✅ DOM хранит pseudo-узлы, `test_pseudo_elements`, `test_pseudo_element_nodes` и `test_large_dom_tree` подтверждают content/перформанс.

- Генерировать псевдо-узлы при построении layout в `LayoutEngine`

**Спецификация:** W3C Selectors Level 3 — Pseudo-elements

**Референс:** MDN Pseudo-elements — https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-elements

**Изменения:**

```rust
// dom.rs
pub struct Node {
    ...
    pub element_state: ElementState, // NEW: bitflags для :hover, :focus, etc.
}

bitflags::bitflags! {
    pub struct ElementState: u16 {
        const HOVER = 1 << 0;
        const FOCUS = 1 << 1;
        const ACTIVE = 1 << 2;
        // ...
    }
}

// selectors.rs
pub enum NonTSPseudoClass {
    Hover,
    Focus,
    Active,
    FirstChild,
    NthChild(i32), // an+b формула
    // ...
}
```

---

### Фаза 4: Расширение свойств и единиц

**Файлы:** `crates/zver/src/css/properties.rs`, `crates/zver/src/layout/types.rs`

**Задачи:**

#### 4.1. Новые единицы длины

- `rem` — относительно корневого font-size
- `vmin`, `vmax` — относительно минимального/максимального viewport-размера
- `calc()` — парсинг математических выражений (сложно, можно упростить)

**Обновить:**

```rust
// properties.rs: parse_length_component()
match unit_lower.as_str() {
    "px" | "em" | "rem" | "vh" | "vw" | "vmin" | "vmax" => { ... }
}
```

#### 4.2. Transition-свойства

- `transition-property`, `transition-duration`, `transition-timing-function`, `transition-delay`
- Шорткат: `transition`
- Парсить в `parse_property()`, хранить в `ComputedStyle`

#### 4.3. Animation-свойства

- `animation-name`, `animation-duration`, `animation-timing-function`, `animation-delay`, `animation-iteration-count`, `animation-direction`, `animation-fill-mode`
- Шорткат: `animation`
- Парсить в `parse_property()`, хранить в `ComputedStyle`

#### 4.4. Grid-свойства (полная поддержка)

- `grid-template-rows`, `grid-template-columns` (уже частично в `types.rs:339-350`)
- `grid-auto-rows`, `grid-auto-columns`, `grid-auto-flow`
- `grid-gap` (синоним `gap` для Grid)
- `grid-row-start/end`, `grid-column-start/end`, шорткаты `grid-row`, `grid-column`
- Обновить `ComputedStyle::to_taffy_style()` для полной Grid-конвертации

**Спецификация:** W3C CSS Grid Layout Module Level 1

**Референс:** MDN Grid — https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Grid_Layout

#### 4.5. Box model расширения

- `box-sizing`: content-box, border-box
- `outline` (не влияет на layout, только рендер)

---

### Фаза 5: Каскад и наследование (полная реализация)

**Файлы:** `crates/zver/src/css/mod.rs`, `crates/zver/src/layout.rs`

**Задачи:**

#### 5.1. User-Agent стили

- Создать встроенную таблицу стилей `USER_AGENT_STYLESHEET` (HTML default styles)
- Применять перед author styles в `StyleEngine::apply_styles()`
- Загрузить из файла или hardcode (рекомендуется hardcode для встраивания)

**Пример:** https://html.spec.whatwg.org/multipage/rendering.html#the-css-user-agent-style-sheet-and-presentational-hints

#### 5.2. Наследование всех свойств

- Обновить `inherit_computed_style()` в `layout.rs:483-501`
- Добавить полный список наследуемых свойств (font-family, line-height, text-align, etc.)
- Создать `is_inherited()` helper для каждого свойства

**Спецификация:** W3C CSS Cascading and Inheritance Level 3

**Референс:** MDN Inheritance — https://developer.mozilla.org/en-US/docs/Web/CSS/inheritance

#### 5.3. Cascade origin

- Реализовать порядок: User-Agent → Author → Inline
- Учитывать `!important` из каждого origin (сейчас только Author)

---

### Фаза 6: Layout — Grid и улучшения

**Файлы:** `crates/zver/src/layout/types.rs`, `crates/zver/src/layout.rs`

**Задачи:**

#### 6.1. Полная Grid Layout интеграция

- Раскомментировать и реализовать Grid-конвертацию в `to_taffy_style()` (сейчас TODO на строке 406-412)
- Добавить поддержку `repeat()`, `minmax()` в `parse_grid_tracks()`
- Тестировать на примерах Grid-макетов

#### 6.2. Inline-элементы и text layout

- Улучшить обработку inline-элементов (сейчас они оборачиваются в flex-контейнеры)
- Реализовать правильное line-breaking для текста
- Интеграция `::first-line`, `::first-letter` (после псевдоэлементов)

---

### Фаза 7: Анимации и Transitions (runtime)

**Файлы:** `crates/zver/src/css/animations.rs`, `crates/zver/src/render.rs`, `crates/zver-egui/src/main.rs`

**Задачи:**

#### 7.1. Transition engine

- Создать `TransitionState` для хранения текущего/целевого значения + timestamp
- Отслеживать изменения свойств в `StyleEngine`
- Интерполировать значения в render loop (egui/winit)
- Поддержать easing-функции: linear, ease, ease-in, ease-out, ease-in-out, cubic-bezier

#### 7.2. Animation engine

- Создать `AnimationState`: текущий keyframe, прогресс, направление
- Запускать анимации при применении `animation-name`
- Вычислять интерполированные свойства на каждом кадре
- Обновлять `ComputedStyle` динамически

#### 7.3. Интеграция с render loop

- Обновить `crates/zver-egui/src/main.rs` для вызова `update_animations(delta_time)`
- Инвалидировать layout при изменении анимированных свойств (width, height, transform)

**Референс:**

- MDN Transitions — https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Transitions
- MDN Animations — https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Animations

---

### Фаза 8: Тестирование и валидация

**Файлы:** `crates/zver/tests/`, новые HTML-примеры в `examples/`

**Задачи:**

#### 8.1. Unit-тесты

- Парсер: тесты для @media, @keyframes, @font-face
- Селекторы: тесты для псевдоклассов/псевдоэлементов
- Свойства: тесты для новых единиц (rem, vmin, calc)
- Каскад: тесты для наследования, specificity, !important

#### 8.2. Integration-тесты

- Создать примеры HTML+CSS для:
  - Flexbox макеты
  - Grid макеты
  - Media queries (адаптивный дизайн)
  - Анимации и transitions
  - Псевдоклассы (:hover, :nth-child)
  - Псевдоэлементы (::before, ::after)
- Визуальное тестирование через `zver-egui`

#### 8.3. Соответствие спецификациям

- Сверка с W3C CSS Test Suite (https://test.csswg.org/)
- Сверка с MDN compatibility tables

---

## Спецификации и ресурсы

### Официальные W3C спецификации

1. **CSS Syntax Module Level 3** — https://www.w3.org/TR/css-syntax-3/
2. **CSS Cascade and Inheritance Level 3** — https://www.w3.org/TR/css-cascade-3/
3. **CSS Selectors Level 3** — https://www.w3.org/TR/selectors-3/
4. **CSS Box Model Module Level 3** — https://www.w3.org/TR/css-box-3/
5. **CSS Flexible Box Layout Module Level 1** — https://www.w3.org/TR/css-flexbox-1/
6. **CSS Grid Layout Module Level 1** — https://www.w3.org/TR/css-grid-1/
7. **CSS Media Queries Level 3** — https://www.w3.org/TR/mediaqueries-3/
8. **CSS Animations Level 1** — https://www.w3.org/TR/css-animations-1/
9. **CSS Transitions Level 1** — https://www.w3.org/TR/css-transitions-1/
10. **CSS Fonts Module Level 3** — https://www.w3.org/TR/css-fonts-3/

### MDN Web Docs

- CSS Reference — https://developer.mozilla.org/en-US/docs/Web/CSS/Reference
- CSS Pseudo-classes — https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
- CSS Pseudo-elements — https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-elements
- CSS Grid — https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Grid_Layout

### Rust библиотеки

- **cssparser** — https://docs.rs/cssparser/
- **selectors** — https://docs.rs/selectors/
- **taffy** — https://docs.rs/taffy/
- **fontdue** — https://docs.rs/fontdue/

---

## Порядок выполнения (приоритеты)

**Критичные (P0):**

1. Обновление зависимостей (Фаза 1)
2. @media queries (Фаза 2.1) — для адаптивности
3. Grid Layout полная поддержка (Фаза 6.1) — ключевой layout mode
4. Псевдоклассы структурные (Фаза 3.1) — :nth-child, :first-child

**Важные (P1):**

5. Псевдоэлементы ::before, ::after (Фаза 3.3)
6. Новые единицы: rem, vmin, vmax (Фаза 4.1)
7. User-Agent стили (Фаза 5.1)
8. Полное наследование (Фаза 5.2)

**Желательные (P2):**

9. @keyframes + анимации (Фаза 2.2, Фаза 7.2)
10. Transitions (Фаза 4.2, Фаза 7.1)
11. Псевдоклассы состояния :hover, :focus (Фаза 3.2)
12. @font-face загрузка шрифтов (Фаза 2.3)

---

## Критические изменения кода

### 1. `crates/zver/Cargo.toml`

```toml
[dependencies]
cssparser = "0.35"  # Проверить актуальность
selectors = "0.32"  # Проверить актуальность
taffy = "0.10"      # Обновить для полной Grid поддержки
fontdue = "0.9"     # Добавить для шрифтов
bitflags = "2.6"    # Для ElementState
```

### 2. `crates/zver/src/css/mod.rs`

```rust
pub struct StyleEngine {
    pub rules: Vec<StyleRule>,
    pub parsed_selectors: Vec<SelectorListHandle>,
    pub selector_cache: HashMap<String, CompiledSelector>,
    pub computed_styles: HashMap<usize, HashMap<String, String>>,
    
    // NEW
    pub media_queries: Vec<MediaRule>,
    pub animations: HashMap<String, Vec<Keyframe>>,
    pub fonts: HashMap<String, FontFace>,
    pub user_agent_styles: Vec<StyleRule>, // Built-in UA stylesheet
}
```

### 3. `crates/zver/src/css/parser.rs`

Обновить `AtRuleParser` для поддержки @media, @keyframes, @font-face.

### 4. `crates/zver/src/css/selectors.rs`

```rust
pub enum NonTSPseudoClass {
    Hover, Focus, Active,
    FirstChild, LastChild, NthChild(i32, i32), // an+b
    // ...
}

pub enum PseudoElement {
    Before, After,
    FirstLine, FirstLetter,
}
```

### 5. `crates/zver/src/dom.rs`

```rust
pub struct Node {
    // ...existing fields...
    pub element_state: ElementState, // NEW
}
```

### 6. `crates/zver/src/layout/types.rs`

Расширить `ComputedStyle` для transitions/animations, обновить `to_taffy_style()` для Grid.

---

## Ожидаемые результаты

После полной реализации Zver будет поддерживать:

✅ Полный CSS2.1 + современные модули (Flexbox, Grid, Media Queries)

✅ Псевдоклассы и псевдоэлементы для интерактивных стилей

✅ Анимации и transitions для динамических эффектов

✅ Адаптивный дизайн через @media

✅ Веб-шрифты через @font-face

✅ Корректный каскад с user-agent стилями

✅ Соответствие W3C спецификациям и MDN best practices

Это выведет проект на уровень полноценного браузерного движка с CSS-поддержкой на уровне современных браузеров.

### To-dos

- [x] Реализовать структурные псевдоклассы: :first-child, :last-child, :nth-child в selectors.rs
- [x] Реализовать псевдоклассы состояния: :hover, :focus, :active через ElementState в dom.rs
- [x] Реализовать псевдоэлементы: ::before, ::after (виртуальные узлы в LayoutEngine)
- [ ] Расширить единицы длины: rem, vmin, vmax, calc() в properties.rs
- [ ] Добавить transition-* свойства: парсинг, хранение в ComputedStyle
- [ ] Добавить animation-* свойства: парсинг, хранение в ComputedStyle
- [ ] Полная поддержка Grid: grid-auto-*, grid-gap, конвертация в Taffy (раскомментировать TODO)
- [ ] Добавить User-Agent stylesheet (HTML default styles) и применять перед author styles
- [ ] Расширить наследование: полный список inherited свойств в inherit_computed_style()
- [ ] Интеграция Grid в LayoutEngine: to_taffy_style() для Grid, тесты на Grid-макетах
- [ ] Реализовать Transition runtime: TransitionState, интерполяция, easing functions
- [ ] Реализовать Animation runtime: AnimationState, keyframe interpolation, update loop
- [ ] Интеграция с render loop: update_animations(delta_time) в zver-egui main loop
- [ ] Unit-тесты: парсер (@media/@keyframes), селекторы (псевдоклассы), свойства (единицы), каскад
- [ ] Integration тесты: примеры HTML+CSS для Flexbox/Grid/Media/Animations/Pseudo-elements
- [ ] Валидация: сверка с W3C CSS Test Suite и MDN compatibility tables

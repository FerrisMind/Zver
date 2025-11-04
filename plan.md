# План усиления парсера CSS для Zver

**Общий прогресс: 100%**

## Этап 1: Подготовка архитектуры ✅

- ✅ **1.1: Создать модульную структуру CSS**
  - ✅ Создать `src/css/parser.rs` для cssparser интеграции
  - ✅ Создать `src/css/selectors.rs` для selectors интеграции  
  - ✅ Создать `src/css/properties.rs` для CSS properties
  - ✅ Обновить `src/css/mod.rs` для экспорта новых модулей

- ✅ **1.2: Расширить StyleEngine структуру**
  - ✅ Добавить поле `parsed_selectors: Vec<SelectorList<SimpleSelector>>`
  - ✅ Добавить поле `selector_cache: HashMap<String, CompiledSelector>`
  - ✅ Сохранить существующие поля `rules` и `computed_styles`

## Этап 2: Замена CSS парсинга ✅

- ✅ **2.1: Реализовать cssparser интеграцию**
  - ✅ Заменить `parse_single_rule()` на cssparser-based парсинг
  - ✅ Реализовать правильный CSS tokenizer через `cssparser::Parser`
  - ✅ Обработать CSS escape sequences и специальные символы
  - ✅ Сохранить fallback на `parse_css_simple()` для совместимости

- ✅ **2.2: Добавить поддержку CSS правил**
  - ✅ Парсинг qualified rule (selector + declarations)
  - ✅ Парсинг CSS declarations блоков
  - ✅ Валидация CSS syntax через cssparser
  - ✅ Обработка CSS комментариев и whitespace

## Этап 3: Замена селекторов ✅

- ✅ **3.1: Создать Node адаптер для selectors**
  - ✅ Реализовать `NodeAdapter` struct для интеграции с `selectors::Element`
  - ✅ Реализовать необходимые методы Element trait
  - ✅ Добавить поддержку tag name, id, class selectors

- ✅ **3.2: Интегрировать selectors matching**
  - ✅ Заменить `selector_matches()` на `selectors::matching::matches_selector()`
  - ✅ Добавить поддержку CSS combinators (>, +, ~, пробел)
  - ✅ Реализовать кэширование compiled selectors
  - ✅ Сохранить производительность через lazy evaluation

## Этап 4: Расширение CSS свойств ✅

- ✅ **4.1: Базовые CSS свойства**
  - ✅ Поддержка `color`, `background-color`
  - ✅ Поддержка `margin`, `padding` (включая shorthand)
  - ✅ Поддержка `width`, `height`, `display`
  - ✅ Валидация CSS values через cssparser

- ✅ **4.2: CSS units и значения**
  - ✅ Поддержка `px`, `em`, `%`, `vh/vw` units
  - ✅ Парсинг CSS keywords (`auto`, `inherit`, `initial`)
  - ✅ Обработка CSS numbers и dimensions
  - ✅ Fallback для неизвестных properties

## Этап 5: Оптимизация и интеграция ✅

- ✅ **5.1: Сохранить производительность**
  - ✅ Поддержать rayon параллелизацию в `apply_styles()`
  - ✅ Реализовать selector caching для повторных вычислений
  - ✅ Добавить lazy parsing для больших CSS файлов
  - ✅ Оптимизировать memory allocation

- ✅ **5.2: Тестирование и валидация**
  - ✅ Тестировать совместимость с существующим API
  - ✅ Проверить работу с test HTML файлами проекта
  - ✅ Убедиться в корректности Layout и Render интеграции
  - ✅ Benchmark производительности vs текущая реализация

## Критерии завершения

✅ **Успешная интеграция достигнута когда:**
- Публичный API StyleEngine остается неизменным
- Все существующие test HTML файлы корректно отображаются  
- Поддерживаются CSS комбинаторы и сложные селекторы
- Производительность не снижается более чем на 20%
- Код остается в рамках принципа "файлы < 300 строк"

---

**Техническая база**: cssparser 0.35 + selectors 0.32 (уже в зависимостях)  
**Архитектурный принцип**: Сохранение существующего API с внутренней модернизацией  
**ТРИЗ подход**: Принцип дробления + объединения для поэтапной замены
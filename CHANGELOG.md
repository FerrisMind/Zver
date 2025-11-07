# Changelog

Все значимые изменения в проекте Zver будут документированы в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
и проект следует [семантическому версионированию](https://semver.org/lang/ru/).

## [Unreleased]

### Планируется
- Поддержка CSS анимаций и переходов
- Расширенные JavaScript API для DOM манипуляций
- Поддержка изображений (PNG, JPEG, SVG)
- WebAssembly интеграция
- Мобильная оптимизация

## [0.6.0-alpha.1] - 2025-11-07

### Добавлено
- Полноценный браузерный интерфейс с мультивкладками в `zver-egui` (`feat(browser): Implement comprehensive browser interface with multi-tab support`).
- Расширенная панель DevTools в `zver-egui` с вкладками (Elements/Console/Network/Performance) и докингом справа (`feat(zver-egui): Enhance DevTools panel layout and positioning`).
- Улучшенный рендеринг layout и debug-визуализации в демо-приложении (`feat(zver-egui): Enhance layout rendering and debug visualization`, `feat(zver-egui): Enhance UI rendering and layout visualization`).
- Поддержка улучшенного inline layout и рендеринга (`feat(layout): Enhance inline element handling and layout rendering`).
- Улучшенный парсинг цветов и поддержка нескольких форматов (`feat(css): Enhance color parsing and support for multiple color formats`).
- Интеграция Boa JavaScript engine и обновление зависимостей (`feat(project): Integrate Boa JavaScript engine and enhance project dependencies`).

### Изменено
- Улучшен DOM-парсинг и операции (`refactor(dom): Enhance DOM parsing and querying capabilities`).
- Реструктурирован CSS-модуль и улучшен парсинг (`refactor(css): Restructure CSS module and enhance parsing capabilities`).
- Улучшен загрузчик ресурсов (`refactor(resource_loader): Improve resource loading initialization and error handling`).
- Рефакторинг структуры проекта и зависимостей (`refactor(project): Restructure project modules and update dependencies`).
- Консолидация оптимизаций DOM/CSS/layout (коммит `42f6eca`).

### Удалено
- Временные экспериментальные файлы и резервные копии тестов, не являющиеся частью публичного API
  (`chore: clean repository structure and gitignore`).

### Исправлено
- Очистка репозитория и `.gitignore` от артефактов сборки и временных файлов
  (`chore: clean repository structure and gitignore`).

## [0.1.0] - 2024-11-06

### Добавлено

#### Основная функциональность
- **Браузерный движок Zver** - основная библиотека с модульной архитектурой
- **GUI демо приложение** на базе egui для визуализации и тестирования
- **Асинхронная архитектура** на базе Tokio для неблокирующих операций

#### DOM Engine
- Парсинг HTML через библиотеку kuchikiki
- Построение дерева DOM с индексированными узлами
- Поддержка CSS селекторов для поиска элементов
- Сериализация DOM обратно в HTML
- Обход дерева и манипуляции с узлами

#### CSS Engine  
- Современный CSS парсер на базе cssparser 0.35
- Поддержка CSS селекторов через selectors 0.32
- Обработка CSS комбинаторов (>, +, ~, пробел)
- Кэширование скомпилированных селекторов
- Поддержка основных CSS свойств:
  - Цвета (color, background-color)
  - Размеры (width, height, margin, padding)
  - Отображение (display: block, inline, flex, grid)
  - Шрифты (font-size, font-family)
- Параллельная обработка CSS правил через Rayon

#### Layout Engine
- Интеграция с Taffy 0.9.1 для layout вычислений
- Полная поддержка Flexbox layout
- Базовая поддержка CSS Grid
- Измерение текста через fontdue
- Viewport-адаптивные вычисления
- Оптимизированные структуры данных для layout результатов

#### Render Engine
- GPU-ускоренный рендеринг через WGPU 27.0
- Поддержка Vulkan, Metal, DirectX 12
- Рендеринг базовых примитивов (прямоугольники, текст)
- Текстовый рендеринг через wgpu_text
- Композиция слоев и элементов

#### JavaScript Engine
- JavaScript исполнение через Boa Engine 0.21
- Базовые DOM API привязки
- Поддержка console.log и базовых объектов
- Изолированный контекст исполнения
- Обработка ошибок JavaScript

#### Network Engine
- HTTP/HTTPS запросы через reqwest 0.12
- Поддержка file:// протокола для локальных файлов
- Кэширование загруженных ресурсов
- Настраиваемый User-Agent
- Асинхронная загрузка с таймаутами

#### Resource Loader
- Координация загрузки различных типов ресурсов
- Управление зависимостями между ресурсами
- Автоматическое извлечение CSS из <style> тегов
- Автоматическое исполнение JavaScript из <script> тегов

### GUI Демо приложение (zver-egui)
- Интерактивный интерфейс для загрузки и просмотра веб-страниц
- Визуализация DOM структуры
- Отображение layout результатов с отладочными наложениями
- Чистый рендеринг без отладочной информации
- Просмотр HTML исходного кода
- Статистика производительности (DOM, CSS, Layout)
- Поддержка локальных файлов и HTTP URL

### Примеры и документация
- **basic_usage.rs** - базовый пример использования API
- **layout_inspection.rs** - пример анализа layout результатов
- Подробная API документация
- Руководство по архитектуре
- Инструкции по сборке и развертыванию
- Примеры интеграции с различными GUI фреймворками

### Тестирование
- Unit тесты для всех основных компонентов
- Интеграционные тесты полного цикла обработки
- Тестовые HTML файлы различной сложности
- Бенчмарки производительности
- CI/CD настройка для GitHub Actions

### Инфраструктура
- Workspace структура с двумя crate'ами
- Оптимизированные профили сборки
- Подробное логирование и отладка
- Кроссплатформенная поддержка (Windows, macOS, Linux)
- Docker контейнеризация

### Производительность
- Параллельная обработка CSS через Rayon
- Кэширование селекторов и layout результатов
- Оптимизированные структуры данных
- GPU-ускоренный рендеринг
- Ленивые вычисления где возможно

### Зависимости
- **cssparser 0.35** - CSS парсинг с поддержкой serde
- **selectors 0.32** - CSS селекторы
- **kuchikiki 0.8** - HTML парсинг
- **taffy 0.9.1** - Layout движок с Flexbox/Grid
- **boa_engine 0.21** - JavaScript движок
- **wgpu 27.0** - GPU рендеринг
- **tokio 1.48** - асинхронный runtime
- **reqwest 0.12** - HTTP клиент
- **egui 0.33** - GUI фреймворк для демо
- **fontdue 0.9.3** - рендеринг шрифтов

### Известные ограничения
- Ограниченная поддержка CSS свойств (базовый набор)
- Простые JavaScript API (без событий DOM)
- Нет поддержки изображений
- Нет CSS анимаций
- Базовая обработка ошибок

---

## Формат записей

### [Версия] - ГГГГ-ММ-ДД

#### Добавлено
- Новая функциональность

#### Изменено  
- Изменения в существующей функциональности

#### Устарело
- Функциональность, которая будет удалена в будущих версиях

#### Удалено
- Удаленная функциональность

#### Исправлено
- Исправления багов

#### Безопасность
- Исправления уязвимостей

---

## Ссылки

- [Unreleased]: https://github.com/FerrisMind/Zver/compare/v0.6.0-alpha.1...HEAD
- [0.6.0-alpha.1]: https://github.com/FerrisMind/Zver/releases/tag/v0.6.0-alpha.1
- [0.1.0]: https://github.com/FerrisMind/Zver/releases/tag/v0.1.0
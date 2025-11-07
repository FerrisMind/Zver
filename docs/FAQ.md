# Часто задаваемые вопросы (FAQ)

## Общие вопросы

### Что такое Zver?

Zver — это современный браузерный движок, написанный на Rust. Он предназначен для парсинга HTML, обработки CSS, исполнения JavaScript и рендеринга веб-страниц с акцентом на производительность и модульность.

### Почему Rust?

Rust обеспечивает:
- **Безопасность памяти** без сборщика мусора
- **Высокую производительность** сравнимую с C/C++
- **Параллелизм без гонок данных**
- **Богатую экосистему** библиотек
- **Кроссплатформенность**

### Чем Zver отличается от других браузерных движков?

- **Модульная архитектура** — каждый компонент независим
- **Асинхронность по умолчанию** — все операции неблокирующие
- **GPU-ускоренный рендеринг** через WGPU
- **Простота интеграции** — можно использовать как библиотеку
- **Современные технологии** — Taffy для layout, Boa для JavaScript

## Установка и настройка

### Какие системные требования?

**Минимальные:**
- Rust 1.75+ (edition 2024)
- 4 GB RAM
- GPU с поддержкой Vulkan/Metal/DirectX 12

**Рекомендуемые:**
- 8+ GB RAM
- SSD накопитель
- Современная видеокарта

### Как установить Zver?

```bash
# Клонирование репозитория
git clone https://github.com/your-username/zver.git
cd zver

# Сборка проекта
cargo build --release

# Запуск GUI демо
cargo run --bin zver-egui
```

### Почему не компилируется на Windows?

Убедитесь, что установлены:
- Visual Studio Build Tools или Visual Studio Community
- Windows SDK
- Rust через rustup

```powershell
# Установка через winget
winget install Microsoft.VisualStudio.2022.BuildTools
winget install Rustlang.Rustup
```

### Ошибки WGPU на Linux

Установите Vulkan драйверы:

```bash
# Ubuntu/Debian
sudo apt install mesa-vulkan-drivers vulkan-utils

# Fedora
sudo dnf install mesa-vulkan-drivers vulkan-tools

# Arch Linux
sudo pacman -S vulkan-radeon vulkan-intel vulkan-mesa-layers
```

## Использование

### Как загрузить HTML страницу?

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    engine.load_url("file://index.html").await?;
    Ok(())
}
```

### Поддерживаются ли HTTPS запросы?

Да, через reqwest с rustls:

```rust
// HTTPS поддерживается из коробки
engine.load_url("https://example.com").await?;
```

### Как получить доступ к DOM?

```rust
// Чтение DOM
let dom = engine.dom.read().await;
println!("Узлов: {}", dom.nodes.len());

// Поиск элементов
let divs = dom.select_ids("div");
let classes = dom.select_ids(".container");
```

### Как исполнить JavaScript?

```rust
// Получение доступа к JS движку
let mut js = engine.js.write().await;

// Исполнение кода
js.execute("console.log('Hello World!');")?;

// Получение результата
let result = js.evaluate("2 + 2")?;
```

### Поддерживается ли CSS Grid?

Да, через интеграцию с Taffy:

```css
.container {
    display: grid;
    grid-template-columns: 1fr 2fr 1fr;
    grid-gap: 10px;
}
```

## Производительность

### Насколько быстр Zver?

Производительность зависит от сложности страницы:
- **Простые страницы:** ~1000 узлов/мс
- **Средние страницы:** ~500 узлов/мс  
- **Сложные страницы:** ~200 узлов/мс

### Как оптимизировать производительность?

1. **Используйте release сборку:**
```bash
cargo build --release
```

2. **Включите нативные оптимизации:**
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

3. **Минимизируйте время блокировок:**
```rust
// Хорошо
{
    let dom = engine.dom.read().await;
    let count = dom.nodes.len();
} // блокировка освобождена

// Плохо
let dom = engine.dom.read().await;
// ... много операций ...
```

### Почему медленно работает в debug режиме?

Debug сборки не оптимизированы. Используйте:

```bash
# Release сборка
cargo run --release

# Или оптимизированный debug
[profile.dev]
opt-level = 1
```

## Разработка

### Как добавить поддержку нового CSS свойства?

1. **Добавить в ComputedStyle:**
```rust
pub struct ComputedStyle {
    // ... существующие поля
    pub new_property: Option<NewPropertyType>,
}
```

2. **Реализовать парсинг:**
```rust
// В css/properties.rs
fn parse_new_property(value: &str) -> Option<NewPropertyType> {
    // логика парсинга
}
```

3. **Добавить в layout обработку:**
```rust
// В layout/compute.rs
if let Some(value) = style.new_property {
    // применение свойства
}
```

### Как добавить JavaScript API?

1. **Создать привязку в JS движке:**
```rust
// В js.rs
pub fn add_dom_api(&mut self) {
    // регистрация функций
}
```

2. **Реализовать методы:**
```rust
fn js_get_element_by_id(id: &str) -> Option<NodeId> {
    // поиск элемента
}
```

### Как запустить тесты?

```bash
# Все тесты
cargo test

# Конкретный модуль
cargo test css::tests

# С выводом
cargo test -- --nocapture
```

### Как профилировать код?

```bash
# Linux (perf)
cargo build --release
perf record ./target/release/zver-egui
perf report

# macOS (Instruments)
instruments -t "Time Profiler" ./target/release/zver-egui
```

## Ошибки и отладка

### "failed to create wgpu device"

Проблема с GPU драйверами. Попробуйте:

```bash
# Принудительное использование OpenGL
export WGPU_BACKEND=gl

# Или программный рендеринг
export WGPU_BACKEND=cpu
```

### "CSS parsing error"

CSS парсер строгий. Проверьте синтаксис:

```css
/* Правильно */
.class { color: red; }

/* Неправильно */
.class { color red }
```

### "JavaScript execution error"

Проверьте синтаксис JavaScript:

```javascript
// Правильно
console.log("Hello");

// Неправильно
console.log("Hello"
```

### Утечки памяти

Используйте инструменты профилирования:

```bash
# Valgrind (Linux)
valgrind --tool=massif ./target/release/zver-egui

# AddressSanitizer
export RUSTFLAGS=-Zsanitizer=address
cargo +nightly build
```

### Медленная компиляция

Оптимизируйте процесс сборки:

```bash
# Параллельная сборка
export CARGO_BUILD_JOBS=8

# Кэширование с sccache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

## Интеграция

### Можно ли использовать Zver в веб-приложении?

Да, через WebAssembly:

```rust
// Добавить в Cargo.toml
[lib]
crate-type = ["cdylib"]

// Компиляция в WASM
cargo build --target wasm32-unknown-unknown
```

### Как интегрировать с существующим GUI?

Zver предоставляет API для интеграции:

```rust
// Получение результатов рендеринга
let render_info = layout.collect_render_info(&dom);

// Отрисовка в вашем GUI фреймворке
for info in render_info {
    your_gui.draw_rect(info.layout, info.style);
}
```

### Поддержка мобильных платформ?

Частично. Требуется дополнительная работа:
- Android: через JNI
- iOS: через C FFI
- Flutter: через dart:ffi

## Лицензирование

### Под какой лицензией распространяется Zver?

MIT License — можно использовать в коммерческих проектах.

### Можно ли использовать в закрытом ПО?

Да, MIT лицензия это разрешает.

### Нужно ли указывать авторство?

Да, требуется сохранить copyright notice в коде.

## Сообщество

### Где получить помощь?

- **GitHub Issues** — для багов и вопросов
- **GitHub Discussions** — для общих обсуждений
- **Discord** — для быстрой помощи (ссылка в README)

### Как внести вклад?

1. Форкните репозиторий
2. Создайте ветку для изменений
3. Следуйте [руководству по участию](CONTRIBUTING.md)
4. Создайте Pull Request

### Планы развития?

См. [план развития](plan.md) и GitHub Issues с меткой "enhancement".

## Сравнение с аналогами

### Zver vs Servo

| Аспект | Zver | Servo |
|--------|------|-------|
| Размер | Компактный | Большой |
| Сложность | Простой | Сложный |
| Интеграция | Легкая | Сложная |
| Производительность | Хорошая | Отличная |
| Поддержка стандартов | Базовая | Полная |

### Zver vs Tauri WebView

| Аспект | Zver | Tauri |
|--------|------|-------|
| Движок | Собственный | Системный |
| Контроль | Полный | Ограниченный |
| Размер | Больше | Меньше |
| Кастомизация | Высокая | Низкая |
| Стабильность | Развивается | Стабильная |

### Когда использовать Zver?

**Используйте Zver если:**
- Нужен полный контроль над рендерингом
- Требуется кастомизация движка
- Важна модульность архитектуры
- Разрабатываете специализированный браузер

**Не используйте Zver если:**
- Нужна полная совместимость с веб-стандартами
- Критичен размер приложения
- Требуется максимальная стабильность
- Достаточно системного WebView

## Устранение неполадок

### Проблемы сборки

```bash
# Очистка кэша
cargo clean
rm -rf ~/.cargo/registry/cache

# Обновление toolchain
rustup update

# Переустановка зависимостей
cargo update
```

### Проблемы runtime

```bash
# Включение отладки
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Проверка системы
vulkaninfo  # Linux
# Или system_profiler SPDisplaysDataType  # macOS
```

### Получение поддержки

При создании issue укажите:
- Версию Rust (`rustc --version`)
- ОС и версию
- Полный текст ошибки
- Минимальный воспроизводимый пример
- Шаги для воспроизведения

---

Если ваш вопрос не освещен в FAQ, создайте issue в репозитории или обратитесь в сообщество.
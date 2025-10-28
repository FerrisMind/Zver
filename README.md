# Zver Browser Engine

Zver - это экспериментальный браузерный движок, написанный на Rust. Проект создан для изучения принципов работы браузеров и включает в себя парсинг HTML/CSS, построение DOM, вычисление layout и рендеринг.

## 🚀 Особенности

- **HTML парсинг** - поддержка HTML5 с использованием `html5ever`
- **CSS движок** - парсинг и применение CSS стилей
- **Layout движок** - вычисление позиций и размеров элементов с использованием Taffy
- **JavaScript движок** - выполнение JavaScript с помощью Boa
- **Сетевой модуль** - загрузка ресурсов по HTTP/HTTPS
- **Модульная архитектура** - четкое разделение компонентов
- **GUI интерфейс** - демо-приложение на egui

## 📁 Структура проекта

```
Zver/
├── crates/
│   ├── zver/                    # Основной движок
│   │   ├── src/
│   │   │   ├── css/            # CSS парсинг и стили
│   │   │   ├── dom/            # DOM структуры и сериализация
│   │   │   ├── layout/         # Layout движок
│   │   │   ├── render/         # Рендеринг (wgpu)
│   │   │   ├── css.rs          # CSS движок
│   │   │   ├── dom.rs          # DOM парсер
│   │   │   ├── js.rs           # JavaScript движок
│   │   │   ├── layout.rs       # Layout координатор
│   │   │   ├── network.rs      # Сетевой модуль
│   │   │   ├── render.rs       # Рендер координатор
│   │   │   └── lib.rs          # Главный API
│   │   └── Cargo.toml
│   └── zver-egui/              # GUI демо-приложение
│       ├── src/
│       │   ├── egui_integration.rs  # Интеграция с egui
│       │   └── main.rs         # GUI приложение
│       └── Cargo.toml
└── README.md
```

## 🛠️ Технологии

### Основной движок (`zver`)
- **html5ever** - HTML5 парсинг
- **cssparser** - CSS парсинг
- **taffy** - Layout движок
- **boa_engine** - JavaScript движок
- **wgpu** - GPU рендеринг
- **reqwest** - HTTP клиент
- **scraper** - DOM селекторы
- **tokio** - Асинхронное выполнение

### GUI приложение (`zver-egui`)
- **eframe/egui** - Immediate mode GUI
- **tokio** - Асинхронная среда выполнения

## 🚦 Быстрый старт

### Требования
- Rust 1.70+
- Cargo

### Установка и запуск

1. Клонируйте репозиторий:
```bash
git clone https://github.com/your-username/zver.git
cd zver
```

2. Запустите GUI демо:
```bash
cargo run -p zver-egui
```

3. Или используйте движок как библиотеку:
```bash
cargo add --path crates/zver
```

## 📖 Использование

### Базовое использование движка

```rust
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    
    // Загрузка и обработка HTML страницы
    engine.load_url("https://example.com").await?;
    
    // Получение DOM
    let dom = engine.dom.read().await;
    println!("DOM узлов: {}", dom.nodes.len());
    
    // Получение layout
    let layout = engine.layout.read().await;
    if let Some(tree) = &layout.layout_tree {
        println!("Layout размер: {}x{}", tree.dimensions.width, tree.dimensions.height);
    }
    
    Ok(())
}
```

### GUI демо-приложение

GUI приложение предоставляет интерактивный интерфейс для:
- Загрузки веб-страниц по URL
- Просмотра HTML исходного кода
- Визуализации layout дерева
- Отладочного рендеринга

## 🏗️ Архитектура

### Основные компоненты

1. **DOM модуль** (`dom.rs`)
   - Парсинг HTML в DOM дерево
   - Сериализация DOM обратно в HTML
   - Поддержка CSS селекторов

2. **CSS модуль** (`css.rs`)
   - Парсинг CSS правил
   - Применение стилей к DOM элементам
   - Поддержка цветов и базовых свойств

3. **Layout модуль** (`layout.rs`)
   - Вычисление позиций и размеров элементов
   - Поддержка блочной и инлайн модели
   - Интеграция с Taffy для flexbox

4. **Render модуль** (`render.rs`)
   - GPU рендеринг через wgpu
   - Поддержка текста и изображений
   - MSAA антиалиасинг

5. **Network модуль** (`network.rs`)
   - HTTP/HTTPS запросы
   - Кэширование ресурсов
   - Загрузка изображений

6. **JavaScript модуль** (`js.rs`)
   - Выполнение JavaScript кода
   - DOM API интеграция
   - Базовые веб API

### Принципы дизайна

- **Модульность** - каждый компонент независим и тестируем
- **Асинхронность** - использование tokio для неблокирующих операций
- **Безопасность** - использование Rust для предотвращения ошибок памяти
- **Производительность** - GPU рендеринг и эффективные структуры данных

## 🧪 Тестирование

```bash
# Запуск всех тестов
cargo test

# Тесты конкретного модуля
cargo test -p zver dom

# Проверка кода
cargo clippy -- -D warnings

# Форматирование
cargo fmt
```

## 📊 Текущий статус

### ✅ Реализовано
- [x] HTML5 парсинг
- [x] Базовый CSS парсинг и применение
- [x] DOM построение и манипуляции
- [x] Layout вычисления (блочная модель)
- [x] Базовый рендеринг
- [x] Сетевые запросы
- [x] JavaScript выполнение (базовое)
- [x] GUI демо-приложение

### 🚧 В разработке
- [ ] Полная поддержка CSS (flexbox, grid)
- [ ] Продвинутый рендеринг (тени, градиенты)
- [ ] Больше JavaScript API
- [ ] Поддержка форм
- [ ] Обработка событий

### 📋 Планы
- [ ] WebAssembly поддержка
- [ ] Сетевая безопасность
- [ ] Accessibility
- [ ] Мобильная поддержка
- [ ] Расширения/плагины

## 🤝 Вклад в проект

Мы приветствуем вклад в развитие проекта! Пожалуйста:

1. Форкните репозиторий
2. Создайте ветку для новой функции (`git checkout -b feature/amazing-feature`)
3. Зафиксируйте изменения (`git commit -m 'Add amazing feature'`)
4. Отправьте в ветку (`git push origin feature/amazing-feature`)
5. Откройте Pull Request

### Правила разработки
- Все файлы должны быть меньше 300 строк
- Код должен проходить `cargo clippy` без предупреждений
- Добавляйте тесты для новой функциональности
- Следуйте существующему стилю кода

## 📄 Лицензия

Этот проект лицензирован под MIT License - см. файл [LICENSE](LICENSE) для деталей.

## 🙏 Благодарности

- [html5ever](https://github.com/servo/html5ever) - HTML парсинг
- [taffy](https://github.com/DioxusLabs/taffy) - Layout движок
- [boa](https://github.com/boa-dev/boa) - JavaScript движок
- [wgpu](https://github.com/gfx-rs/wgpu) - GPU рендеринг
- [egui](https://github.com/emilk/egui) - GUI фреймворк

## 📞 Контакты

- GitHub Issues: [Создать issue](https://github.com/your-username/zver/issues)
- Документация: [docs.rs/zver](https://docs.rs/zver)

---

**Zver** - это образовательный проект для изучения внутреннего устройства браузеров. Не предназначен для продакшн использования.
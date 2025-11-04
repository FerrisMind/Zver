# Быстрый старт Zver

## 🚀 Запуск GUI демо

```bash
# Клонируйте репозиторий
git clone https://github.com/your-username/zver.git
cd zver

# Запустите GUI приложение
cargo run -p zver-egui
```

В GUI приложении:
1. Введите URL (например: `file://examples/index.html`)
2. Нажмите "Load" для загрузки страницы
3. Используйте кнопки для просмотра HTML и layout

## 📚 Использование как библиотеки

```rust
// Cargo.toml
[dependencies]
zver = { path = "crates/zver" }
tokio = { version = "1.48", features = ["full"] }

// main.rs
use zver::Zver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Zver::new();
    engine.load_url("https://example.com").await?;
    
    let dom = engine.dom.read().await;
    println!("Загружено {} DOM узлов", dom.nodes.len());
    
    Ok(())
}
```

## 🧪 Запуск примера

```bash
cargo run --example basic_usage -p zver
```

## 🔧 Разработка

```bash
# Проверка кода
cargo clippy -- -D warnings

# Тесты
cargo test

# Форматирование
cargo fmt
```

## 📁 Тестовые файлы

- `examples/index.html` - демо HTML страница
- `examples/basic_usage.rs` - пример использования API

## ⚡ Быстрые команды

```bash
# Сборка всего проекта
cargo build

# Запуск только движка
cargo run -p zver --example basic_usage

# Запуск GUI
cargo run -p zver-egui

# Проверка без сборки
cargo check
```
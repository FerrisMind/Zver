# Руководство по сборке Zver

Данный документ описывает процесс сборки, тестирования и развертывания браузерного движка Zver.

## Системные требования

### Минимальные требования

- **Rust:** 1.75+ (edition 2024)
- **Cargo:** Поставляется с Rust
- **ОС:** Windows 10+, macOS 10.15+, Linux (Ubuntu 20.04+)
- **RAM:** 4 GB (рекомендуется 8 GB)
- **GPU:** Поддержка Vulkan, Metal или DirectX 12 для WGPU

### Дополнительные зависимости

#### Windows
```powershell
# Установка Rust через rustup
winget install Rustlang.Rustup

# Или через официальный установщик
# https://rustup.rs/
```

#### macOS
```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установка Xcode Command Line Tools (если не установлены)
xcode-select --install
```

#### Linux (Ubuntu/Debian)
```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установка системных зависимостей
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev

# Для WGPU (Vulkan поддержка)
sudo apt install -y libvulkan1 mesa-vulkan-drivers vulkan-utils
```

#### Linux (Fedora/RHEL)
```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установка системных зависимостей
sudo dnf install -y gcc gcc-c++ openssl-devel pkg-config

# Для WGPU
sudo dnf install -y vulkan-loader vulkan-tools mesa-vulkan-drivers
```

## Клонирование репозитория

```bash
# Клонирование основного репозитория
git clone https://github.com/your-username/zver.git
cd zver

# Проверка структуры проекта
ls -la
```

## Сборка проекта

### Быстрая сборка (Debug)

```bash
# Сборка всех компонентов
cargo build

# Сборка только основной библиотеки
cargo build -p zver

# Сборка только GUI демо
cargo build -p zver-egui
```

### Оптимизированная сборка (Release)

```bash
# Полная release сборка
cargo build --release

# С дополнительными оптимизациями
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Проверка сборки

```bash
# Проверка синтаксиса без сборки
cargo check

# Проверка с clippy
cargo clippy

# Форматирование кода
cargo fmt
```

## Запуск приложений

### GUI демо приложение

```bash
# Debug версия
cargo run --bin zver-egui

# Release версия (быстрее)
cargo run --release --bin zver-egui
```

### Примеры использования

```bash
# Базовый пример
cargo run --example basic_usage

# Пример анализа layout
cargo run --example layout_inspection

# Все примеры
cargo run --example basic_usage
cargo run --example layout_inspection
```

## Тестирование

### Запуск тестов

```bash
# Все тесты
cargo test

# Тесты с подробным выводом
cargo test -- --nocapture

# Тесты конкретного модуля
cargo test css::tests

# Тесты конкретного пакета
cargo test -p zver

# Интеграционные тесты
cargo test --test integration_tests
```

### Тестирование производительности

```bash
# Бенчмарки (если есть)
cargo bench

# Профилирование с perf (Linux)
cargo build --release
perf record --call-graph=dwarf ./target/release/zver-egui
perf report

# Профилирование памяти с valgrind (Linux)
valgrind --tool=massif ./target/release/zver-egui
```

## Конфигурация сборки

### Профили сборки

Проект использует оптимизированные профили в `Cargo.toml`:

```toml
[profile.dev]
opt-level = 1        # Базовая оптимизация для debug
debug = true         # Отладочная информация

[profile.release]
opt-level = 3        # Максимальная оптимизация
lto = true          # Link Time Optimization
codegen-units = 1   # Лучшая оптимизация
panic = "abort"     # Меньший размер бинарника

[profile.dev.package."*"]
opt-level = 2       # Оптимизация зависимостей в debug
```

### Переменные окружения

```bash
# Уровень логирования
export RUST_LOG=debug
export RUST_LOG=zver=trace,wgpu=warn

# Бэкенд для WGPU
export WGPU_BACKEND=vulkan  # Linux
export WGPU_BACKEND=metal   # macOS
export WGPU_BACKEND=dx12    # Windows

# Оптимизации компилятора
export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld"
```

## Отладка

### Включение логирования

```rust
// В main.rs или lib.rs
env_logger::init();

// Или с более детальной конфигурацией
use tracing_subscriber;
tracing_subscriber::fmt::init();
```

### Отладка WGPU

```bash
# Подробная информация о GPU
export RUST_LOG=wgpu_core=debug,wgpu_hal=debug

# Валидация GPU операций
export WGPU_VALIDATION=1
```

### Отладка CSS парсинга

```bash
# Подробная информация о CSS
export RUST_LOG=zver::css=trace
```

## Оптимизация производительности

### Компиляция с оптимизациями

```bash
# Максимальная оптимизация
RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release

# С Link Time Optimization
RUSTFLAGS="-C lto=fat" cargo build --release

# Оптимизация размера бинарника
RUSTFLAGS="-C opt-level=z" cargo build --release
```

### Профилирование

#### Linux (perf)
```bash
# Установка perf
sudo apt install linux-perf  # Ubuntu
sudo dnf install perf        # Fedora

# Профилирование
cargo build --release
perf record --call-graph=dwarf ./target/release/zver-egui
perf report
```

#### macOS (Instruments)
```bash
# Сборка с отладочными символами
cargo build --release

# Запуск через Instruments
instruments -t "Time Profiler" ./target/release/zver-egui
```

#### Windows (Visual Studio)
```powershell
# Сборка с PDB файлами
cargo build --release

# Использование Visual Studio Profiler или Intel VTune
```

## Упаковка и распространение

### Создание релизных бинарников

```bash
# Сборка для текущей платформы
cargo build --release

# Копирование бинарников
cp target/release/zver-egui ./zver-egui
# Windows: copy target\release\zver-egui.exe .\zver-egui.exe
```

### Cross-compilation

```bash
# Установка target'ов
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Сборка для Windows из Linux
cargo build --release --target x86_64-pc-windows-gnu

# Сборка для macOS из Linux (требует дополнительной настройки)
cargo build --release --target x86_64-apple-darwin
```

### Docker сборка

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libvulkan1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zver-egui /usr/local/bin/
CMD ["zver-egui"]
```

```bash
# Сборка Docker образа
docker build -t zver:latest .

# Запуск (требует X11 forwarding для GUI)
docker run --rm -it \
  -e DISPLAY=$DISPLAY \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  zver:latest
```

## CI/CD настройка

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta]

    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
        components: rustfmt, clippy
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Install system dependencies (Linux)
      if: runner.os == 'Linux'
      run: |
        sudo apt update
        sudo apt install -y libvulkan1 mesa-vulkan-drivers
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Build release
      run: cargo build --release --verbose
```

## Устранение проблем

### Частые проблемы

#### 1. Ошибки компиляции WGPU

```bash
# Проверка поддержки GPU
vulkaninfo  # Linux
# Или установка Mesa драйверов
sudo apt install mesa-vulkan-drivers
```

#### 2. Ошибки линковки

```bash
# Linux: установка системных библиотек
sudo apt install build-essential pkg-config libssl-dev

# macOS: установка Xcode tools
xcode-select --install
```

#### 3. Медленная сборка

```bash
# Использование sccache для кэширования
cargo install sccache
export RUSTC_WRAPPER=sccache

# Параллельная сборка
export CARGO_BUILD_JOBS=8
```

#### 4. Проблемы с зависимостями

```bash
# Очистка кэша Cargo
cargo clean
rm -rf ~/.cargo/registry/cache

# Обновление зависимостей
cargo update
```

### Отладка runtime ошибок

#### Панические ошибки
```bash
# Подробные stack trace
export RUST_BACKTRACE=full
export RUST_BACKTRACE=1
```

#### Проблемы с GPU
```bash
# Принудительное использование CPU рендеринга
export WGPU_BACKEND=gl
export MESA_GL_VERSION_OVERRIDE=3.3
```

#### Проблемы с памятью
```bash
# Использование AddressSanitizer
export RUSTFLAGS=-Zsanitizer=address
cargo +nightly build --target x86_64-unknown-linux-gnu
```

## Документация

### Генерация документации

```bash
# Генерация документации
cargo doc

# Открытие в браузере
cargo doc --open

# Документация с приватными элементами
cargo doc --document-private-items
```

### Проверка документации

```bash
# Тестирование примеров в документации
cargo test --doc

# Проверка ссылок в документации
cargo doc --no-deps
```

## Релизный процесс

### Подготовка релиза

1. **Обновление версий** в `Cargo.toml`
2. **Обновление CHANGELOG.md**
3. **Запуск полного тестирования**
4. **Создание git тега**
5. **Сборка релизных бинарников**
6. **Публикация на GitHub Releases**

```bash
# Пример скрипта релиза
#!/bin/bash
VERSION="0.2.0"

# Обновление версий
sed -i "s/version = \".*\"/version = \"$VERSION\"/" crates/*/Cargo.toml

# Коммит изменений
git add .
git commit -m "Release v$VERSION"
git tag "v$VERSION"

# Сборка релизных бинарников
cargo build --release

# Создание архивов
tar -czf "zver-v$VERSION-linux.tar.gz" -C target/release zver-egui
# zip для Windows и macOS...

# Пуш изменений
git push origin main --tags
```

Это руководство покрывает все аспекты сборки и развертывания Zver. При возникновении проблем обращайтесь к разделу устранения неполадок или создавайте issue в репозитории проекта.
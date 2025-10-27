use eframe::egui::{self, TextEdit};
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Zver egui demo",
        native_options,
        Box::new(|_cc| Ok(Box::<ZverApp>::default())),
    )
}

struct ZverApp {
    engine: Arc<Zver>,
    runtime: Arc<Runtime>,
    url: String,
    status: String,
    last_html: String,
    dom_stats: String,
    layout_stats: String,
    js_stats: String,
    show_html_source: bool,
    show_visual_layout_window: bool,
    show_clean_render_window: bool,
}

impl Default for ZverApp {
    fn default() -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
        let engine = Arc::new(Zver::new());

        Self {
            engine,
            runtime,
            url: "file://index.html".to_string(),
            status: "Готов к загрузке. Введите URL и нажмите Load.".to_string(),
            last_html: String::new(),
            dom_stats: "DOM не загружен".to_string(),
            layout_stats: "Layout не вычислен".to_string(),
            js_stats: "JS не исполнен".to_string(),
            show_html_source: true,
            show_visual_layout_window: false,
            show_clean_render_window: false,
        }
    }
}

impl eframe::App for ZverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Zver + egui");

            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_singleline(&mut self.url);
                if ui.button("Load").clicked() {
                    let url = self.url.clone();
                    let engine_for_load = self.engine.clone();
                    self.status = "Loading...".to_string();

                    let load_result = self
                        .runtime
                        .block_on(async move { engine_for_load.load_url(&url).await });

                    match &load_result {
                        Ok(_) => {
                            self.status = "Loaded".to_string();
                            // Обновляем отображение HTML после загрузки
                            let engine_for_dom = self.engine.clone();
                            (
                                self.last_html,
                                self.dom_stats,
                                self.layout_stats,
                                self.js_stats,
                            ) = self.runtime.block_on(async {
                                let dom = engine_for_dom.dom.read().await;
                                let layout = engine_for_dom.layout.read().await;

                                // Сериализация HTML
                                let mut html = String::new();
                                if let Some(root) = dom.root {
                                    let html_root = find_html_element(&dom, root);
                                    serialize_node(&dom, html_root, &mut html);
                                }

                                // Статистика DOM
                                let dom_stats = format!(
                                    "DOM узлов: {}, Корень: {:?}, Scraper: {}",
                                    dom.nodes.len(),
                                    dom.root,
                                    dom.has_scraper()
                                );

                                // Статистика Layout
                                let layout_stats = if let Some(tree) = &layout.layout_tree {
                                    format!(
                                        "Layout вычислен: {}x{}",
                                        tree.dimensions.width, tree.dimensions.height
                                    )
                                } else {
                                    "Layout не вычислен".to_string()
                                };

                                // Статистика JS
                                let js_stats = "JS исполнен (Boa)".to_string();

                                (html, dom_stats, layout_stats, js_stats)
                            });
                        }
                        Err(err) => {
                            self.status = format!("Error: {err}");
                        }
                    }
                }

                if ui.button("Reload").clicked() {
                    let url = self.url.clone();
                    let engine_for_load = self.engine.clone();
                    self.status = "Reloading...".to_string();

                    // Очищаем кеш перед загрузкой
                    self.runtime.block_on(async {
                        let mut network = engine_for_load.network.write().await;
                        network.clear_cache_for_url(&url);
                    });

                    let load_result = self
                        .runtime
                        .block_on(async move { engine_for_load.load_url(&url).await });

                    match &load_result {
                        Ok(_) => {
                            self.status = "Reloaded".to_string();
                            // Обновляем отображение HTML после загрузки
                            let engine_for_dom = self.engine.clone();
                            (
                                self.last_html,
                                self.dom_stats,
                                self.layout_stats,
                                self.js_stats,
                            ) = self.runtime.block_on(async {
                                let dom = engine_for_dom.dom.read().await;
                                let layout = engine_for_dom.layout.read().await;

                                // Сериализация HTML
                                let mut html = String::new();
                                if let Some(root) = dom.root {
                                    let html_root = find_html_element(&dom, root);
                                    serialize_node(&dom, html_root, &mut html);
                                }

                                // Статистика DOM
                                let dom_stats = format!(
                                    "DOM узлов: {}, Корень: {:?}, Scraper: {}",
                                    dom.nodes.len(),
                                    dom.root,
                                    dom.has_scraper()
                                );

                                // Статистика Layout
                                let layout_stats = if let Some(tree) = &layout.layout_tree {
                                    format!(
                                        "Layout вычислен: {}x{}",
                                        tree.dimensions.width, tree.dimensions.height
                                    )
                                } else {
                                    "Layout не вычислен".to_string()
                                };

                                // Статистика JS
                                let js_stats = "JS исполнен (Boa)".to_string();

                                (html, dom_stats, layout_stats, js_stats)
                            });
                        }
                        Err(err) => {
                            self.status = format!("Error: {err}");
                        }
                    }
                }

                if ui.button("Refresh HTML").clicked() {
                    // Обновляем отображение HTML из движка
                    let engine_for_dom = self.engine.clone();
                    (self.last_html, self.dom_stats, self.layout_stats) =
                        self.runtime.block_on(async {
                            let dom = engine_for_dom.dom.read().await;
                            let layout = engine_for_dom.layout.read().await;

                            // Сериализация HTML
                            let mut html = String::new();
                            if let Some(root) = dom.root {
                                // Находим настоящий HTML элемент
                                let html_root = find_html_element(&dom, root);
                                serialize_node(&dom, html_root, &mut html);
                            }

                            // Статистика DOM
                            let dom_stats = format!(
                                "DOM узлов: {}, Корень: {:?}, Scraper: {}",
                                dom.nodes.len(),
                                dom.root,
                                dom.has_scraper()
                            );

                            // Статистика Layout
                            let layout_stats = if let Some(tree) = &layout.layout_tree {
                                format!(
                                    "Layout вычислен: {}x{}",
                                    tree.dimensions.width, tree.dimensions.height
                                )
                            } else {
                                "Layout не вычислен".to_string()
                            };

                            (html, dom_stats, layout_stats)
                        });
                }
            });

            ui.separator();
            ui.label(&self.status);
            ui.label(&self.dom_stats);
            ui.label(&self.layout_stats);
            ui.label(&self.js_stats);

            ui.separator();

            // Кнопки для управления окнами и секциями
            ui.horizontal(|ui| {
                if ui
                    .button(if self.show_html_source {
                        "Hide HTML Source"
                    } else {
                        "Show HTML Source"
                    })
                    .clicked()
                {
                    self.show_html_source = !self.show_html_source;
                }

                if ui.button("Open Visual Layout Window").clicked() {
                    self.show_visual_layout_window = true;
                }

                if ui.button("Open Clean Render Window").clicked() {
                    self.show_clean_render_window = true;
                }
            });

            // HTML Source с прокруткой (опционально показываем)
            if self.show_html_source {
                ui.separator();
                ui.heading("HTML Source:");

                // Автоматически обновляем HTML только если он уже был загружен
                if self.last_html.is_empty() && !self.status.contains("Готов к загрузке")
                {
                    let engine_for_dom = self.engine.clone();
                    (
                        self.last_html,
                        self.dom_stats,
                        self.layout_stats,
                        self.js_stats,
                    ) = self.runtime.block_on(async {
                        let dom = engine_for_dom.dom.read().await;
                        let layout = engine_for_dom.layout.read().await;

                        // Сериализация HTML
                        let mut html = String::new();
                        if let Some(root) = dom.root {
                            let html_root = find_html_element(&dom, root);
                            serialize_node(&dom, html_root, &mut html);
                        }

                        // Статистика DOM
                        let dom_stats = format!(
                            "DOM узлов: {}, Корень: {:?}, Scraper: {}",
                            dom.nodes.len(),
                            dom.root,
                            dom.has_scraper()
                        );

                        // Статистика Layout
                        let layout_stats = if let Some(tree) = &layout.layout_tree {
                            format!(
                                "Layout вычислен: {}x{}",
                                tree.dimensions.width, tree.dimensions.height
                            )
                        } else {
                            "Layout не вычислен".to_string()
                        };

                        // Статистика JS
                        let js_stats = "JS исполнен (Boa)".to_string();

                        (html, dom_stats, layout_stats, js_stats)
                    });
                }

                // Область с прокруткой для HTML исходника
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::multiline(&mut self.last_html)
                                .desired_width(f32::INFINITY)
                                .code_editor(),
                        );
                    });
            }
        });

        // Рендерим окна каждый фрейм, если они открыты
        if self.show_visual_layout_window {
            let engine_clone = self.engine.clone();
            let runtime_clone = self.runtime.clone();

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("visual_layout"),
                egui::ViewportBuilder::default()
                    .with_title("Visual Layout Render")
                    .with_inner_size([850.0, 700.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        runtime_clone.block_on(async {
                            let layout = engine_clone.layout.read().await;
                            let dom = engine_clone.dom.read().await;

                            if let Some(tree) = &layout.layout_tree {
                                ui.label(format!(
                                    "Root dimensions: x={}, y={}, w={}, h={}",
                                    tree.dimensions.x,
                                    tree.dimensions.y,
                                    tree.dimensions.width,
                                    tree.dimensions.height
                                ));
                                ui.label(format!("Root children count: {}", tree.children.len()));

                                let mut text_nodes = 0;
                                let mut empty_nodes = 0;
                                for node in dom.nodes.values() {
                                    if let Some(text) = &node.text_content
                                        && !text.trim().is_empty()
                                    {
                                        text_nodes += 1;
                                    }
                                    if node.text_content.is_none() && node.tag_name.is_some() {
                                        empty_nodes += 1;
                                    }
                                }
                                ui.label(format!(
                                    "DOM: {} nodes with text, {} element nodes",
                                    text_nodes, empty_nodes
                                ));

                                ui.separator();

                                egui::ScrollArea::both().show(ui, |ui| {
                                    // Увеличиваем размер canvas для полного отображения
                                    let canvas_width = tree.dimensions.width.max(1200.0);
                                    let canvas_height = tree.dimensions.height.max(1000.0);

                                    let (response, painter) = ui.allocate_painter(
                                        egui::vec2(canvas_width, canvas_height),
                                        egui::Sense::hover(),
                                    );

                                    render_layout_tree_in_painter(
                                        &painter,
                                        response.rect.min,
                                        tree,
                                        &dom,
                                        0,
                                    );
                                });
                            } else {
                                ui.label("Layout tree не построен");
                            }
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Пользователь закрыл окно
                    }
                },
            );
        }

        if self.show_clean_render_window {
            let engine_clone = self.engine.clone();
            let runtime_clone = self.runtime.clone();

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("clean_render"),
                egui::ViewportBuilder::default()
                    .with_title("Clean Render")
                    .with_inner_size([1024.0, 768.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        runtime_clone.block_on(async {
                            let layout = engine_clone.layout.read().await;
                            let dom = engine_clone.dom.read().await;

                            if let Some(tree) = &layout.layout_tree {
                                egui::ScrollArea::both().show(ui, |ui| {
                                    let (response, painter) = ui.allocate_painter(
                                        egui::vec2(
                                            tree.dimensions.width.max(800.0),
                                            tree.dimensions.height.max(600.0),
                                        ),
                                        egui::Sense::hover(),
                                    );

                                    painter.rect_filled(response.rect, 0.0, egui::Color32::WHITE);

                                    render_clean_layout(&painter, response.rect.min, tree, &dom);
                                });
                            } else {
                                ui.label("Layout tree не построен. Загрузите страницу.");
                            }
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Пользователь закрыл окно
                    }
                },
            );
        }
    }
}

fn find_html_element(dom: &zver::dom::Document, start_node: usize) -> usize {
    // Рекурсивно ищем узел с тегом "html"
    if let Some(node) = dom.nodes.get(&start_node) {
        if node.tag_name.as_deref() == Some("html") {
            return start_node;
        }
        // Проверяем дочерние узлы
        for &child_id in &node.children {
            let found = find_html_element(dom, child_id);
            if found != usize::MAX {
                return found;
            }
        }
    }
    usize::MAX // Не найден
}

fn serialize_node(dom: &zver::dom::Document, node_id: usize, html: &mut String) {
    if let Some(node) = dom.nodes.get(&node_id) {
        if let Some(tag) = &node.tag_name {
            html.push('<');
            html.push_str(tag);
            for (attr, value) in &node.attributes {
                html.push(' ');
                html.push_str(attr);
                html.push('=');
                html.push('"');
                html.push_str(value);
                html.push('"');
            }
            html.push('>');

            for &child in &node.children {
                serialize_node(dom, child, html);
            }

            html.push_str("</");
            html.push_str(tag);
            html.push('>');
        } else if let Some(text) = &node.text_content {
            html.push_str(text);
        }
    }
}

// Визуальный рендеринг layout tree в egui с использованием painter
fn render_layout_tree_in_painter(
    painter: &egui::Painter,
    offset: egui::Pos2,
    node: &zver::layout::LayoutNode,
    dom: &zver::dom::Document,
    _depth: usize,
) {
    use egui::{Color32, Rect, Stroke, Vec2};

    // Получаем размеры и позицию
    let x = node.dimensions.x;
    let y = node.dimensions.y;
    let width = node.dimensions.width;
    let height = node.dimensions.height;

    // Пропускаем узлы с нулевыми размерами
    if width <= 0.0 || height <= 0.0 {
        // Всё равно обрабатываем детей
        for child in &node.children {
            render_layout_tree_in_painter(painter, offset, child, dom, 0);
        }
        return;
    }

    // Определяем цвет на основе CSS стилей или типа узла
    let (bg_color, text_info, text_content) = if let Some(dom_node) = dom.nodes.get(&node.dom_node)
    {
        // Пытаемся получить background-color из CSS
        let bg = if let Some(bg_css) = &node.style.background_color {
            parse_css_color(bg_css).unwrap_or_else(|| {
                // Fallback на цвет по типу тега
                get_default_color_for_tag(&dom_node.tag_name)
            })
        } else {
            get_default_color_for_tag(&dom_node.tag_name)
        };

        let info = if let Some(tag) = &dom_node.tag_name {
            format!("<{}>", tag)
        } else if let Some(text) = &dom_node.text_content {
            let trimmed = text.trim();
            let char_count = trimmed.chars().count();
            if char_count > 30 {
                let truncated: String = trimmed.chars().take(30).collect();
                format!("\"{}...\"", truncated)
            } else {
                format!("\"{}\"", trimmed)
            }
        } else {
            "node".to_string()
        };

        // Получаем текстовый контент для рендеринга
        let text = dom_node.text_content.clone();

        (bg, info, text)
    } else {
        (Color32::WHITE, "unknown".to_string(), None)
    };

    // Рисуем прямоугольник для этого узла (с учётом offset)
    let rect = Rect::from_min_size(
        egui::pos2(offset.x + x, offset.y + y),
        Vec2::new(width.max(1.0), height.max(1.0)),
    );

    painter.rect_filled(rect, 0.0, bg_color);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, Color32::DARK_GRAY),
        egui::StrokeKind::Outside,
    );

    // ИСПРАВЛЕНИЕ: Рендерим текст ТОЛЬКО если он есть в этом конкретном узле
    // Не собираем рекурсивно из детей - это предотвращает дублирование
    if let Some(text) = text_content {
        let trimmed = text.trim();
        if !trimmed.is_empty() && width > 10.0 && height > 10.0 {
            // Получаем цвет текста из CSS или используем чёрный по умолчанию
            let text_color = if let Some(color_css) = &node.style.color {
                parse_css_color(color_css).unwrap_or(Color32::BLACK)
            } else {
                Color32::BLACK
            };

            let font_size = node.style.font_size.clamp(8.0, 32.0);
            let text_pos = rect.min + Vec2::new(4.0, 4.0);

            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                trimmed,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }
    } else {
        // Добавляем отладочную информацию только для контейнеров без текста
        if width > 50.0 && height > 15.0 {
            // Полупрозрачный фон для отладочного текста
            let debug_bg = Rect::from_min_size(
                rect.min,
                Vec2::new((text_info.len() as f32 * 6.0 + 10.0).min(width), 14.0),
            );
            painter.rect_filled(debug_bg, 0.0, Color32::from_black_alpha(128));

            let text_pos = rect.min + Vec2::new(2.0, 2.0);
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                format!("{} {}x{}", text_info, width as i32, height as i32),
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );
        }
    }

    // Рекурсивно рендерим дочерние узлы
    for child in &node.children {
        render_layout_tree_in_painter(painter, offset, child, dom, 0);
    }
}

// Парсинг CSS цвета в egui Color32
fn parse_css_color(color_str: &str) -> Option<egui::Color32> {
    let color_str = color_str.trim();

    // Hex формат: #RRGGBB или #RGB
    if let Some(hex) = color_str.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(egui::Color32::from_rgb(r, g, b));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            return Some(egui::Color32::from_rgb(r, g, b));
        }
    }

    // rgb() формат: rgb(r, g, b)
    if let Some(rgb) = color_str.strip_prefix("rgb(")
        && let Some(rgb) = rgb.strip_suffix(')')
    {
        let parts: Vec<&str> = rgb.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            return Some(egui::Color32::from_rgb(r, g, b));
        }
    }

    // Именованные цвета
    match color_str.to_lowercase().as_str() {
        "white" => Some(egui::Color32::WHITE),
        "black" => Some(egui::Color32::BLACK),
        "red" => Some(egui::Color32::RED),
        "green" => Some(egui::Color32::GREEN),
        "blue" => Some(egui::Color32::BLUE),
        "yellow" => Some(egui::Color32::YELLOW),
        "gray" | "grey" => Some(egui::Color32::GRAY),
        _ => None,
    }
}

// Цвет по умолчанию для типа тега (fallback)
fn get_default_color_for_tag(tag_name: &Option<String>) -> egui::Color32 {
    if let Some(tag) = tag_name {
        match tag.as_str() {
            "body" => egui::Color32::from_rgb(240, 240, 240),
            "h1" | "h2" | "h3" => egui::Color32::from_rgb(200, 220, 255),
            "div" => egui::Color32::from_rgb(220, 255, 220),
            "p" => egui::Color32::from_rgb(255, 255, 220),
            _ => egui::Color32::from_rgb(255, 240, 240),
        }
    } else {
        egui::Color32::from_rgb(255, 255, 200) // Текстовые узлы
    }
}

// Чистовой рендеринг без отладочной информации
fn render_clean_layout(
    painter: &egui::Painter,
    offset: egui::Pos2,
    node: &zver::layout::LayoutNode,
    dom: &zver::dom::Document,
) {
    use egui::{Color32, FontId, Rect, Vec2};

    let x = node.dimensions.x;
    let y = node.dimensions.y;
    let width = node.dimensions.width;
    let height = node.dimensions.height;

    // Пропускаем узлы с нулевыми размерами
    if width <= 0.0 || height <= 0.0 {
        for child in &node.children {
            render_clean_layout(painter, offset, child, dom);
        }
        return;
    }

    // Получаем DOM узел и его информацию
    let (bg_color, text_content, tag_name) = if let Some(dom_node) = dom.nodes.get(&node.dom_node) {
        let bg = if let Some(bg_css) = &node.style.background_color {
            parse_css_color(bg_css).unwrap_or(Color32::TRANSPARENT)
        } else {
            Color32::TRANSPARENT
        };

        // ИСПРАВЛЕНИЕ: Берем текст ТОЛЬКО из этого узла, не из детей
        // Это предотвращает дублирование текста
        let text = dom_node.text_content.clone().unwrap_or_default();

        (bg, text, dom_node.tag_name.clone())
    } else {
        (Color32::TRANSPARENT, String::new(), None)
    };

    // Рисуем прямоугольник только если есть цвет фона
    if bg_color != Color32::TRANSPARENT {
        let rect = Rect::from_min_size(
            egui::pos2(offset.x + x, offset.y + y),
            Vec2::new(width.max(1.0), height.max(1.0)),
        );
        painter.rect_filled(rect, 0.0, bg_color);
    }

    // Рендерим текстовый контент
    let trimmed = text_content.trim();
    if !trimmed.is_empty() && width > 10.0 && height > 10.0 {
        let text_color = if let Some(color_css) = &node.style.color {
            parse_css_color(color_css).unwrap_or(Color32::BLACK)
        } else {
            Color32::BLACK
        };

        // Определяем размер шрифта в зависимости от тега
        let mut font_size = node.style.font_size;
        if let Some(tag) = &tag_name {
            font_size = match tag.as_str() {
                "h1" => 32.0,
                "h2" => 28.0,
                "h3" => 24.0,
                "h4" => 20.0,
                "h5" => 18.0,
                "h6" => 16.0,
                _ => font_size,
            };
        }

        font_size = font_size.clamp(8.0, 48.0);

        let rect = Rect::from_min_size(
            egui::pos2(offset.x + x, offset.y + y),
            Vec2::new(width, height),
        );
        let text_pos = rect.min + Vec2::new(4.0, 4.0);

        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            trimmed,
            FontId::proportional(font_size),
            text_color,
        );
    }

    // Рекурсивно рендерим дочерние узлы
    for child in &node.children {
        render_clean_layout(painter, offset, child, dom);
    }
}

use eframe::egui::{self, TextEdit};
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;
use zver::dom::serialization::serialize_dom;
mod egui_integration;
use egui_integration::{render_clean_layout, render_layout_tree_in_painter};

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

impl ZverApp {
    fn update_html_and_stats(&mut self) {
        let engine_clone = self.engine.clone();
        (
            self.last_html,
            self.dom_stats,
            self.layout_stats,
            self.js_stats,
        ) = self.runtime.block_on(async {
            let dom = engine_clone.dom.read().await;
            let layout = engine_clone.layout.read().await;

            // Сериализация HTML
            let html = serialize_dom(&dom);

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
                    let engine_clone = self.engine.clone();
                    self.status = "Loading...".to_string();

                    let load_result = self
                        .runtime
                        .block_on(async move { engine_clone.load_url(&url).await });

                    match load_result {
                        Ok(_) => {
                            self.status = "Loaded".to_string();
                            self.update_html_and_stats();
                        }
                        Err(err) => {
                            self.status = format!("Error: {err}");
                        }
                    }
                }

                if ui.button("Reload").clicked() {
                    let url = self.url.clone();
                    let engine_clone = self.engine.clone();
                    self.status = "Reloading...".to_string();

                    // Очищаем кеш перед загрузкой
                    self.runtime.block_on(async {
                        let mut network = engine_clone.network.write().await;
                        network.clear_cache_for_url(&url);
                    });

                    let load_result = self
                        .runtime
                        .block_on(async move { engine_clone.load_url(&url).await });

                    match load_result {
                        Ok(_) => {
                            self.status = "Reloaded".to_string();
                            self.update_html_and_stats();
                        }
                        Err(err) => {
                            self.status = format!("Error: {err}");
                        }
                    }
                }

                if ui.button("Refresh HTML").clicked() {
                    self.update_html_and_stats();
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

            // HTML Source с прокруткой
            if self.show_html_source {
                ui.separator();
                ui.heading("HTML Source:");

                // Автоматически обновляем HTML если он пустой
                if self.last_html.is_empty() && !self.status.contains("Готов к загрузке")
                {
                    self.update_html_and_stats();
                }

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
            self.render_visual_layout_window(ctx);
        }

        if self.show_clean_render_window {
            self.render_clean_render_window(ctx);
        }
    }
}

impl ZverApp {
    fn render_visual_layout_window(&mut self, ctx: &egui::Context) {
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

    fn render_clean_render_window(&mut self, ctx: &egui::Context) {
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

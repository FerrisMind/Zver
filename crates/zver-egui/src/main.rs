use eframe::egui::{self, TextEdit};
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;
use zver::dom::serialization::serialize_dom;
mod egui_integration;
use egui_integration::{render_clean_layout_from_results, render_layout_results_in_painter};

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
    show_debug_overlays: bool,
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
            show_debug_overlays: false,
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
                "DOM узлов: {}, Корень: {:?}, Selectors: scraper",
                dom.nodes.len(),
                dom.root,
            );

            // Статистика Layout
            let layout_results = layout.get_all_layout_results();
            let layout_stats = if !layout_results.is_empty() {
                format!("Layout вычислен: {} элементов", layout_results.len())
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
            
            // Статус и логи с возможностью копирования
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.add(
                    TextEdit::singleline(&mut self.status.as_str())
                        .desired_width(f32::INFINITY)
                );
            });
            
            ui.horizontal(|ui| {
                ui.label("DOM:");
                ui.add(
                    TextEdit::singleline(&mut self.dom_stats.as_str())
                        .desired_width(f32::INFINITY)
                );
            });
            
            ui.horizontal(|ui| {
                ui.label("Layout:");
                ui.add(
                    TextEdit::singleline(&mut self.layout_stats.as_str())
                        .desired_width(f32::INFINITY)
                );
            });
            
            ui.horizontal(|ui| {
                ui.label("JS:");
                ui.add(
                    TextEdit::singleline(&mut self.js_stats.as_str())
                        .desired_width(f32::INFINITY)
                );
            });

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

                if ui
                    .button(if self.show_debug_overlays {
                        "Hide Debug Overlay"
                    } else {
                        "Show Debug Overlay"
                    })
                    .clicked()
                {
                    self.show_debug_overlays = !self.show_debug_overlays;
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
                    let show_overlays = self.show_debug_overlays;
                    let engine = Arc::clone(&engine_clone);
                    let runtime = Arc::clone(&runtime_clone);

                    runtime.block_on(async move {
                        let layout = engine.layout.read().await;
                        let dom = engine.dom.read().await;
                        let resolved_styles = layout.resolved_styles().clone();

                        let render_info = layout.collect_render_info(&dom);
                        if !render_info.is_empty() {
                            ui.label(format!("Layout results: {} elements", render_info.len()));

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

                            // Calculate actual content dimensions
                            let mut max_x = 0.0f32;
                            let mut max_y = 0.0f32;
                            for info in &render_info {
                                let right = info.layout.x + info.layout.width;
                                let bottom = info.layout.y + info.layout.height;
                                max_x = max_x.max(right);
                                max_y = max_y.max(bottom);
                            }
                            
                            // Add padding
                            let canvas_width = (max_x + 20.0).max(800.0);
                            let canvas_height = (max_y + 20.0).max(600.0);

                            egui::ScrollArea::both()
                                .auto_shrink(false)
                                .show(ui, |ui| {
                                    let (response, painter) = ui.allocate_painter(
                                        egui::vec2(canvas_width, canvas_height),
                                        egui::Sense::hover(),
                                    );

                                    render_layout_results_in_painter(
                                        &painter,
                                        response.rect.min,
                                        &render_info,
                                        &resolved_styles,
                                        show_overlays,
                                    );
                                });
                        } else {
                            ui.label("Layout результаты не найдены");
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
                    let engine = Arc::clone(&engine_clone);
                    let runtime = Arc::clone(&runtime_clone);

                    runtime.block_on(async move {
                        let layout = engine.layout.read().await;
                        let dom = engine.dom.read().await;
                        let resolved_styles = layout.resolved_styles().clone();

                        let render_info = layout.collect_render_info(&dom);
                        if !render_info.is_empty() {
                            // Calculate actual content dimensions
                            let mut max_x = 0.0f32;
                            let mut max_y = 0.0f32;
                            for info in &render_info {
                                let right = info.layout.x + info.layout.width;
                                let bottom = info.layout.y + info.layout.height;
                                max_x = max_x.max(right);
                                max_y = max_y.max(bottom);
                            }
                            
                            // Add padding
                            let canvas_width = (max_x + 20.0).max(800.0);
                            let canvas_height = (max_y + 20.0).max(600.0);

                            egui::ScrollArea::both()
                                .auto_shrink(false)
                                .show(ui, |ui| {
                                    let (response, painter) = ui.allocate_painter(
                                        egui::vec2(canvas_width, canvas_height),
                                        egui::Sense::hover(),
                                    );

                                    painter.rect_filled(response.rect, 0.0, egui::Color32::WHITE);

                                    render_clean_layout_from_results(
                                        &painter,
                                        response.rect.min,
                                        &render_info,
                                        &resolved_styles,
                                    );
                                });
                        } else {
                            ui.label("Layout результаты не найдены. Загрузите страницу.");
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

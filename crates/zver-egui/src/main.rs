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
    runtime: Runtime,
    url: String,
    status: String,
    last_html: String,
    dom_stats: String,
    layout_stats: String,
    js_stats: String,
}

impl Default for ZverApp {
    fn default() -> Self {
        let runtime = Runtime::new().expect("failed to create tokio runtime");
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
                            (self.last_html, self.dom_stats, self.layout_stats, self.js_stats) = self.runtime.block_on(async {
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
                                    format!("Layout вычислен: {}x{}", tree.dimensions.width, tree.dimensions.height)
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
                            (self.last_html, self.dom_stats, self.layout_stats, self.js_stats) = self.runtime.block_on(async {
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
                                    format!("Layout вычислен: {}x{}", tree.dimensions.width, tree.dimensions.height)
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
                    (self.last_html, self.dom_stats, self.layout_stats) = self.runtime.block_on(async {
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
                            format!("Layout вычислен: {}x{}", tree.dimensions.width, tree.dimensions.height)
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

            // Автоматически обновляем HTML только если он уже был загружен
            if self.last_html.is_empty() && !self.status.contains("Готов к загрузке") {
                let engine_for_dom = self.engine.clone();
                (self.last_html, self.dom_stats, self.layout_stats, self.js_stats) = self.runtime.block_on(async {
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
                        format!("Layout вычислен: {}x{}", tree.dimensions.width, tree.dimensions.height)
                    } else {
                        "Layout не вычислен".to_string()
                    };

                    // Статистика JS
                    let js_stats = "JS исполнен (Boa)".to_string();

                    (html, dom_stats, layout_stats, js_stats)
                });
            }

            ui.add(TextEdit::multiline(&mut self.last_html).desired_width(f32::INFINITY));
        });
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

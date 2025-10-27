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
}

impl Default for ZverApp {
    fn default() -> Self {
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        let engine = Arc::new(Zver::new());
        Self {
            engine,
            runtime,
            url: "https://example.com".to_string(),
            status: String::from("Введите URL и нажмите Load"),
            last_html: String::new(),
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

                    self.status = match &load_result {
                        Ok(_) => "Loaded".to_string(),
                        Err(err) => format!("Error: {err}"),
                    };

                    if load_result.is_ok() {
                        let engine_for_dom = self.engine.clone();
                        self.last_html = self.runtime.block_on(async move {
                            let dom = engine_for_dom.dom.read().await;
                            let mut html = String::new();
                            if let Some(root) = dom.root {
                                serialize_node(&dom, root, &mut html);
                            }
                            html
                        });
                    }
                }
            });

            ui.separator();
            ui.label(&self.status);
            ui.add(TextEdit::multiline(&mut self.last_html).desired_width(f32::INFINITY));
        });
        ctx.request_repaint();
    }
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

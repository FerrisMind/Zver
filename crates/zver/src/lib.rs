pub mod css;
pub mod dom;
pub mod js;
pub mod layout;
pub mod network;
pub mod render;
pub mod resource_loader;

use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Zver {
    pub dom: Arc<RwLock<dom::Document>>,
    pub css: Arc<RwLock<css::StyleEngine>>,
    pub layout: Arc<RwLock<layout::LayoutEngine>>,
    pub render: Arc<RwLock<render::RenderEngine>>,
    pub network: Arc<RwLock<network::NetworkEngine>>,
    pub js: Arc<RwLock<js::JSEngine>>,
    pub resource_loader: Arc<RwLock<resource_loader::ResourceLoader>>,
}

impl Zver {
    pub fn new() -> Self {
        let dom = Arc::new(RwLock::new(dom::Document::new()));
        let js = Arc::new(RwLock::new(js::JSEngine::new().with_dom(dom.clone())));

        Self {
            dom,
            css: Arc::new(RwLock::new(css::StyleEngine::new())),
            layout: Arc::new(RwLock::new(layout::LayoutEngine::new(800.0, 600.0))),
            render: Arc::new(RwLock::new(render::RenderEngine::new())),
            network: Arc::new(RwLock::new(network::NetworkEngine::new())),
            js,
            resource_loader: Arc::new(RwLock::new(resource_loader::ResourceLoader::new())),
        }
    }

    pub async fn load_url(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Инициализируем resource_loader при первом использовании
        {
            let mut loader = self.resource_loader.write().await;
            loader.init().await;
        }

        let html = {
            let mut network = self.network.write().await;
            network.fetch(url).await?
        };

        {
            let mut dom = self.dom.write().await;
            dom.parse_html(&html).await?;
        }

        {
            let dom_snapshot = self.dom.read().await.clone();
            let mut css = self.css.write().await;

            // Параллельно извлекаем CSS из всех <style> тегов
            let css_contents: Vec<String> = find_style_nodes(&dom_snapshot, dom_snapshot.root)
                .into_par_iter()
                .map(|style_node_id| {
                    let mut content = String::new();
                    extract_css_from_single_node(&dom_snapshot, style_node_id, &mut content);
                    content
                })
                .collect();

            let combined_css = css_contents.join("\n");
            css.parse_css(&combined_css)?;
            css.apply_styles(&dom_snapshot)?;
        }

        // Исполняем JavaScript из <script> тегов
        {
            let dom_snapshot = self.dom.read().await.clone();
            let mut js_engine = self.js.write().await;
            let mut js_content = String::new();
            extract_js_from_dom(&dom_snapshot, dom_snapshot.root, &mut js_content);

            if !js_content.is_empty()
                && let Err(e) = js_engine.execute(&js_content)
            {
                eprintln!("JavaScript execution error: {}", e);
            }
        }

        {
            let dom_snapshot = self.dom.read().await.clone();
            let css_snapshot = self.css.read().await.computed_styles.clone();
            let mut layout = self.layout.write().await;
            let _layout_results = layout.compute_layout(&dom_snapshot, &css_snapshot);
        }

        {
            let layout = self.layout.read().await;
            let dom_snapshot = self.dom.read().await.clone();
            let mut render = self.render.write().await;
            render.paint(&layout, &dom_snapshot).await?;
        }

        Ok(())
    }
}

impl Default for Zver {
    fn default() -> Self {
        Self::new()
    }
}

fn find_style_nodes(dom: &dom::Document, node_id: Option<usize>) -> Vec<usize> {
    match node_id {
        Some(id) => {
            let mut nodes = Vec::new();
            if dom.nodes.get(&id).and_then(|node| node.tag_name.as_deref()) == Some("style") {
                nodes.push(id);
            }
            nodes.extend(dom.select_ids_from(id, "style"));
            nodes
        }
        None => dom.select_ids("style"),
    }
}

fn extract_css_from_single_node(dom: &dom::Document, node_id: usize, css_content: &mut String) {
    let content = dom.get_text_content(node_id);
    if !content.is_empty() {
        css_content.push_str(&content);
        if !content.ends_with('\n') {
            css_content.push('\n');
        }
    }
}

#[allow(dead_code)]
fn extract_css_from_dom(dom: &dom::Document, node_id: Option<usize>, css_content: &mut String) {
    for style_id in find_style_nodes(dom, node_id) {
        extract_css_from_single_node(dom, style_id, css_content);
    }
}

fn extract_js_from_dom(dom: &dom::Document, node_id: Option<usize>, js_content: &mut String) {
    let mut script_nodes = match node_id {
        Some(id) => {
            let mut nodes = Vec::new();
            if dom.nodes.get(&id).and_then(|node| node.tag_name.as_deref()) == Some("script") {
                nodes.push(id);
            }
            nodes.extend(dom.select_ids_from(id, "script"));
            nodes
        }
        None => dom.select_ids("script"),
    };

    script_nodes.sort_unstable();
    script_nodes.dedup();

    for script_id in script_nodes {
        if let Some(src) = dom.attribute(script_id, "src") {
            js_content.push_str(&format!("// External script: {src}\n"));
        } else {
            let content = dom.get_text_content(script_id);
            if !content.is_empty() {
                js_content.push_str(&content);
                if !content.ends_with('\n') {
                    js_content.push('\n');
                }
            }
        }
    }
}

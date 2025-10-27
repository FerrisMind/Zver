pub mod dom;
pub mod css;
pub mod layout;
pub mod render;
pub mod network;
pub mod js;
pub mod resource_loader;

use std::sync::Arc;
use tokio::sync::RwLock;
use rayon::prelude::*;

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

            if !js_content.is_empty() && let Err(e) = js_engine.execute(&js_content) {
                eprintln!("JavaScript execution error: {}", e);
            }
        }

        {
            let dom_snapshot = self.dom.read().await.clone();
            let css_snapshot = self.css.read().await.computed_styles.clone();
            let mut layout = self.layout.write().await;
            layout.compute(&dom_snapshot, &css_snapshot);
        }

        {
            let layout = self.layout.read().await;
            let mut render = self.render.write().await;
            render.paint(&layout).await?;
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
    let mut style_nodes = Vec::new();
    find_style_nodes_recursive(dom, node_id, &mut style_nodes);
    style_nodes
}

fn find_style_nodes_recursive(dom: &dom::Document, node_id: Option<usize>, style_nodes: &mut Vec<usize>) {
    if let Some(node_id) = node_id
        && let Some(node) = dom.nodes.get(&node_id) {
        // Если это <style> тег, добавляем в список
        if node.tag_name.as_deref() == Some("style") {
            style_nodes.push(node_id);
        }

        // Рекурсивно обрабатываем дочерние узлы
        for &child_id in &node.children {
            find_style_nodes_recursive(dom, Some(child_id), style_nodes);
        }
    }
}

fn extract_css_from_single_node(dom: &dom::Document, node_id: usize, css_content: &mut String) {
    if let Some(node) = dom.nodes.get(&node_id) {
        // Ищем текстовые дочерние узлы
        for &child_id in &node.children {
            if let Some(child) = dom.nodes.get(&child_id)
                && let Some(text) = &child.text_content {
                css_content.push_str(text);
                css_content.push('\n');
            }
        }
    }
}

#[allow(dead_code)]
fn extract_css_from_dom(dom: &dom::Document, node_id: Option<usize>, css_content: &mut String) {
    if let Some(node_id) = node_id
        && let Some(node) = dom.nodes.get(&node_id) {
        // Если это <style> тег, извлекаем его содержимое
        if node.tag_name.as_deref() == Some("style") {
            // Ищем текстовые дочерние узлы
            for &child_id in &node.children {
                if let Some(child) = dom.nodes.get(&child_id)
                    && let Some(text) = &child.text_content {
                    css_content.push_str(text);
                    css_content.push('\n');
                }
            }
        }

        // Рекурсивно обрабатываем дочерние узлы
        for &child_id in &node.children {
            extract_css_from_dom(dom, Some(child_id), css_content);
        }
    }
}

fn extract_js_from_dom(dom: &dom::Document, node_id: Option<usize>, js_content: &mut String) {
    if let Some(node_id) = node_id
        && let Some(node) = dom.nodes.get(&node_id) {
        // Если это <script> тег, извлекаем его содержимое
        if node.tag_name.as_deref() == Some("script") {
            // Ищем текстовые дочерние узлы или src атрибут
            if let Some(src) = node.attributes.get("src") {
                // Для демонстрации просто добавляем комментарий
                // В реальности нужно было бы загрузить внешний скрипт
                js_content.push_str(&format!("// External script: {}\n", src));
            } else {
                // Ищем текстовые дочерние узлы
                for &child_id in &node.children {
                    if let Some(child) = dom.nodes.get(&child_id)
                        && let Some(text) = &child.text_content {
                        js_content.push_str(text);
                        js_content.push('\n');
                    }
                }
            }
        }

        // Рекурсивно обрабатываем дочерние узлы
        for &child_id in &node.children {
            extract_js_from_dom(dom, Some(child_id), js_content);
        }
    }
}

pub mod dom;
pub mod css;
pub mod layout;
pub mod render;
pub mod network;
pub mod js;

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct Zver {
    pub dom: Arc<RwLock<dom::Document>>,
    pub css: Arc<RwLock<css::StyleEngine>>,
    pub layout: Arc<RwLock<layout::LayoutEngine>>,
    pub render: Arc<RwLock<render::RenderEngine>>,
    pub network: Arc<RwLock<network::NetworkEngine>>,
}

impl Zver {
    pub fn new() -> Self {
        Self {
            dom: Arc::new(RwLock::new(dom::Document::new())),
            css: Arc::new(RwLock::new(css::StyleEngine::new())),
            layout: Arc::new(RwLock::new(layout::LayoutEngine::new(800.0, 600.0))),
            render: Arc::new(RwLock::new(render::RenderEngine::new())),
            network: Arc::new(RwLock::new(network::NetworkEngine::new())),
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
            css.parse_css("*")?;
            css.apply_styles(&dom_snapshot)?;
        }

        {
            let dom_snapshot = self.dom.read().await.clone();
            let css_snapshot = self.css.read().await.computed_styles.clone();
            let mut layout = self.layout.write().await;
            layout.compute(&dom_snapshot, &css_snapshot);
        }

        {
            let layout = self.layout.read().await;
            let render = self.render.read().await;
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


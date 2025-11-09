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
use tracing::{debug, instrument};

/// Главный интерфейс браузерного движка Zver.
///
/// Координирует работу всех подсистем: DOM, CSS, Layout, Render, Network, JS.
///
/// # Lock Ordering
///
/// **КРИТИЧНО**: Для предотвращения deadlocks всегда захватывайте блокировки в следующем порядке:
/// 1. DOM (Level 1)
/// 2. CSS (Level 2)  
/// 3. Layout (Level 3)
/// 4. Render (Level 4)
/// 5. Network (Level 5)
/// 6. JS (Level 6)
/// 7. ResourceLoader (Level 7)
///
/// Никогда не захватывайте блокировку более высокого уровня, удерживая блокировку низкого уровня.
///
/// # Примеры
///
/// ```no_run
/// use zver::Zver;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let engine = Zver::new();
///     engine.load_url("https://example.com").await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Zver {
    /// DOM tree (Lock Level 1)
    pub dom: Arc<RwLock<dom::Document>>,
    /// CSS engine (Lock Level 2)
    pub css: Arc<RwLock<css::StyleEngine>>,
    /// Layout engine (Lock Level 3)
    pub layout: Arc<RwLock<layout::LayoutEngine>>,
    /// Render engine (Lock Level 4)
    pub render: Arc<RwLock<render::RenderEngine>>,
    /// Network engine (Lock Level 5)
    pub network: Arc<RwLock<network::NetworkEngine>>,
    /// JavaScript engine (Lock Level 6)
    pub js: Arc<RwLock<js::JSEngine>>,
    /// Resource loader (Lock Level 7)
    pub resource_loader: Arc<RwLock<resource_loader::ResourceLoader>>,
}

impl Zver {
    /// Выполняет произвольный JavaScript-код в контексте текущего движка.
    ///
    /// Синхронная обёртка над [`JSEngine::execute()`], использующая существующий RwLock.
    /// Не изменяет публичные контракты и модель блокировок.
    pub fn eval_js(&self, code: &str) -> Result<crate::js::JSValue, String> {
        // Используем существующий js: Arc<RwLock<JSEngine>>
        // Без изменения lock ordering: JS — самый высокий уровень.
        let mut js_engine = self.js.blocking_write();
        js_engine.execute(code).map_err(|e| e.to_string())
    }
    /// Создаёт новый экземпляр браузерного движка с настройками по умолчанию.
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр `Zver` с viewport размером 800x600.
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

    /// Загружает и рендерит HTML документ по заданному URL.
    ///
    /// Выполняет полный пайплайн обработки:
    /// 1. Инициализация загрузчика ресурсов
    /// 2. Загрузка HTML через сеть
    /// 3. Парсинг DOM
    /// 4. Извлечение и применение CSS
    /// 5. Выполнение JavaScript
    /// 6. Вычисление layout
    /// 7. Рендеринг
    ///
    /// # Аргументы
    ///
    /// * `url` - URL для загрузки. Поддерживаются схемы: `http://`, `https://`, `file://`
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// # use zver::Zver;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = Zver::new();
    /// engine.load_url("file://./index.html").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку если:
    /// - Не удалось загрузить URL
    /// - HTML невалидный
    /// - CSS содержит критические ошибки парсинга
    ///
    /// # Lock ordering
    ///
    /// Соблюдается безопасный порядок блокировок: DOM -> CSS -> Layout -> Render
    #[instrument(skip(self), fields(url = %url))]
    pub async fn load_url(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Инициализируем resource_loader при первом использовании
        {
            let mut loader = self.resource_loader.write().await;
            loader.init().await;
        }

        let html = {
            let _span = tracing::debug_span!("fetch_html").entered();
            let mut network = self.network.write().await;
            network.fetch(url).await?
        };

        {
            let _span = tracing::debug_span!("parse_dom").entered();
            let mut dom = self.dom.write().await;
            dom.parse_html(&html).await?;
        }

        let pseudo_contents = {
            let _span = tracing::debug_span!("process_css").entered();
            // TODO(Phase 3): ��⨬���஢��� - ᮧ���� ��������� snapshot ��� CSS extraction
            // ����� �ॡ���� clone() ��� rayon parallel processing ('static lifetime)
            let dom_snapshot = self.dom.read().await.clone();
            let mut css = self.css.write().await;

            // ��ࠫ���쭮 ��������� CSS �� ��� <style> ⥣��
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

            debug!("Processed {} CSS rules", css.rules.len());
            css.pseudo_element_contents()
        };

        {
            let mut dom = self.dom.write().await;
            dom.sync_pseudo_elements(&pseudo_contents);
        }

        // Исполняем JavaScript из <script> тегов
        // NOTE: Требуется clone() для изоляции - JS может мутировать DOM через callbacks
        {
            let _span = tracing::debug_span!("execute_js").entered();
            let dom_snapshot = self.dom.read().await.clone();
            let mut js_engine = self.js.write().await;

            // Reset JavaScript context to prevent "duplicate lexical declaration" errors
            // This is necessary because const/let declarations cannot be redeclared in the same scope
            js_engine.reset_context();

            let mut js_content = String::new();
            extract_js_from_dom(&dom_snapshot, dom_snapshot.root, &mut js_content);

            if !js_content.is_empty() {
                tracing::debug!(
                    "Executing JavaScript code ({} chars): {}",
                    js_content.len(),
                    &js_content[..js_content.len().min(100)]
                );

                if let Err(e) = js_engine.execute(&js_content) {
                    eprintln!("JavaScript execution error: {}", e);
                }
            }
        }

        // Вычисляем layout
        // OPTIMIZATION: Используем guard вместо clone() для экономии памяти и CPU
        // compute_layout() не мутирует DOM и работает быстро, поэтому
        // удержание read lock безопасно и не создаёт bottleneck
        {
            let _span = tracing::debug_span!("compute_layout").entered();
            let css_guard = self.css.read().await;
            let css_snapshot = css_guard.computed_styles.clone();
            let pseudo_snapshot = css_guard.pseudo_element_styles.clone();
            drop(css_guard);
            let mut layout = self.layout.write().await;

            // Берём DOM guard на время вычислений вместо полного clone()
            let dom_guard = self.dom.read().await;
            let layout_results = layout.compute_layout(&dom_guard, &css_snapshot, &pseudo_snapshot);
            // Guard автоматически освобождается здесь

            debug!("Computed layout for {} nodes", layout_results.len());
        }

        // Рендеринг
        // TODO(Phase 2): Создать RenderSnapshot вместо полного clone()
        // Render нужны только геометрия + тексты, не всё дерево
        {
            let _span = tracing::debug_span!("render").entered();
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

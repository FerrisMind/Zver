use crate::css::{PseudoStyle, selectors::PseudoElement};
use crate::dom::Document;
use crate::layout::builder::TreeBuilder;
use crate::layout::metrics::{FontMetrics, TextMeasureContext, text_measure_function};
use crate::layout::render::RenderInfo;
use crate::layout::types::{ComputedStyle, LayoutResult};
use std::collections::HashMap;
use taffy::prelude::*;

pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,

    // Taffy layout engine с поддержкой текстовых контекстов
    taffy: TaffyTree<Option<TextMeasureContext>>,

    // Кеширование результатов
    root_node: Option<NodeId>,
    node_mapping: HashMap<usize, NodeId>, // DOM ID -> Taffy NodeId
    layout_cache: HashMap<usize, LayoutResult>, // Результаты layout по DOM ID
    resolved_styles: HashMap<usize, ComputedStyle>,
}

// SAFETY: LayoutEngine can be safely sent between threads because:
// 1. TaffyTree is designed to be Send (contains no thread-local data)
// 2. All access is protected by RwLock in the parent Zver struct
// 3. HashMap and Vec are Send when their contents are Send
// 4. TextMeasureContext and LayoutResult are plain data structures
unsafe impl Send for LayoutEngine {}

// SAFETY: LayoutEngine can be shared between threads (behind Arc<RwLock<>>) because:
// 1. All mutations require exclusive lock (RwLock::write)
// 2. All reads are performed under shared lock (RwLock::read)
// 3. No interior mutability without synchronization
unsafe impl Sync for LayoutEngine {}

impl LayoutEngine {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            taffy: TaffyTree::new(),
            root_node: None,
            node_mapping: HashMap::new(),
            layout_cache: HashMap::new(),
            resolved_styles: HashMap::new(),
        }
    }

    /// Сбрасывает состояние при изменении DOM/CSS
    pub fn invalidate(&mut self) {
        if let Some(root) = self.root_node.take() {
            let _ = self.taffy.remove(root);
        }
        self.taffy.clear();
        self.node_mapping.clear();
        self.layout_cache.clear();
        self.resolved_styles.clear();
    }

    /// Вычисляет layout с использованием Taffy
    pub fn compute_layout(
        &mut self,
        document: &Document,
        styles: &HashMap<usize, HashMap<String, String>>,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
    ) -> HashMap<usize, LayoutResult> {
        // 1. Строим Taffy дерево с контекстами для текста
        if self
            .build_taffy_tree_with_contexts(document, styles, pseudo_styles)
            .is_none()
        {
            return HashMap::new();
        }

        // 2. Вычисляем лейауты с измерением текста
        self.compute_taffy_layouts();

        // 3. Извлекаем и кешируем результаты
        self.extract_and_cache_results(document);

        // 4. Возвращаем результаты
        self.layout_cache.clone()
    }

    /// Получает результат layout для конкретного узла
    pub fn get_layout_result(&self, node_id: usize) -> Option<LayoutResult> {
        self.layout_cache.get(&node_id).copied()
    }

    /// Получает все результаты layout
    pub fn get_all_layout_results(&self) -> &HashMap<usize, LayoutResult> {
        &self.layout_cache
    }

    /// Возвращает карту вычисленных стилей после применения каскада и наследования
    pub fn resolved_styles(&self) -> &HashMap<usize, ComputedStyle> {
        &self.resolved_styles
    }

    /// Строит Taffy дерево с контекстами для текстовых узлов
    fn build_taffy_tree_with_contexts(
        &mut self,
        document: &Document,
        styles: &HashMap<usize, HashMap<String, String>>,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
    ) -> Option<NodeId> {
        // Очищаем старое состояние
        if let Some(root) = self.root_node.take() {
            let _ = self.taffy.remove(root);
        }
        self.taffy.clear();
        self.node_mapping.clear();
        self.layout_cache.clear();
        self.resolved_styles.clear();

        let root_id = document.root?;

        let mut builder = TreeBuilder {
            taffy: &mut self.taffy,
            node_mapping: &mut self.node_mapping,
            resolved_styles: &mut self.resolved_styles,
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
        };

        let (taffy_root, _) =
            builder.build_node_recursive(document, root_id, styles, pseudo_styles, None)?;
        self.root_node = Some(taffy_root);
        Some(taffy_root)
    }

    /// Вычисляет layout с измерением текста
    fn compute_taffy_layouts(&mut self) {
        if let Some(root) = self.root_node {
            let font_metrics = FontMetrics::default();

            // Вычисляем layout, учитывая измерение текста
            let _ = self.taffy.compute_layout_with_measure(
                root,
                taffy::Size {
                    width: taffy::AvailableSpace::Definite(self.viewport_width),
                    height: taffy::AvailableSpace::Definite(self.viewport_height),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    if let Some(text_ctx) = node_context.as_ref().and_then(|opt| opt.as_ref()) {
                        // Это текстовый узел - измеряем текст
                        text_measure_function(
                            known_dimensions,
                            available_space,
                            Some(text_ctx),
                            &font_metrics,
                        )
                    } else {
                        // Это обычный элемент - используем заданные размеры (если есть)
                        known_dimensions.unwrap_or(taffy::Size::ZERO)
                    }
                },
            );
        }
    }

    /// Извлекает результаты layout из Taffy и кеширует их
    fn extract_and_cache_results(&mut self, document: &Document) {
        self.layout_cache.clear();

        if let Some(root_id) = document.root
            && let Some(&taffy_root) = self.node_mapping.get(&root_id)
        {
            self.extract_node_layout(document, root_id, taffy_root, 0.0, 0.0);
        }
    }

    fn extract_node_layout(
        &mut self,
        document: &Document,
        dom_node_id: usize,
        taffy_id: NodeId,
        parent_x: f32,
        parent_y: f32,
    ) {
        if let Ok(layout) = self.taffy.layout(taffy_id) {
            let abs_x = parent_x + layout.location.x;
            let abs_y = parent_y + layout.location.y;

            let layout_result = LayoutResult {
                node_id: dom_node_id,
                x: abs_x,
                y: abs_y,
                width: layout.size.width,
                height: layout.size.height,
                content_x: abs_x + layout.border.left + layout.padding.left,
                content_y: abs_y + layout.border.top + layout.padding.top,
                content_width: layout.content_size.width,
                content_height: layout.content_size.height,
            };
            self.layout_cache.insert(dom_node_id, layout_result);

            if let Some(dom_node) = document.nodes.get(&dom_node_id) {
                for &child_dom_id in &dom_node.children {
                    if let Some(&child_taffy_id) = self.node_mapping.get(&child_dom_id) {
                        self.extract_node_layout(
                            document,
                            child_dom_id,
                            child_taffy_id,
                            abs_x,
                            abs_y,
                        );
                    }
                }
            }
            if let Some(pseudo_children) = document.pseudo_children(dom_node_id) {
                for &pseudo_dom_id in pseudo_children.values() {
                    if let Some(&pseudo_taffy_id) = self.node_mapping.get(&pseudo_dom_id) {
                        self.extract_node_layout(
                            document,
                            pseudo_dom_id,
                            pseudo_taffy_id,
                            abs_x,
                            abs_y,
                        );
                    }
                }
            }
        }
    }

    /// Собирает информацию для рендеринга из кеша результатов
    pub fn collect_render_info(&self, document: &Document) -> Vec<RenderInfo> {
        let mut render_list = Vec::new();

        if let Some(root_id) = document.root {
            self.collect_render_info_recursive(&mut render_list, document, root_id);
        }

        render_list
    }

    /// Получает RenderInfo для всех результатов layout
    pub fn get_all_render_info(&self, document: &Document) -> Vec<RenderInfo> {
        self.layout_cache
            .iter()
            .filter_map(|(&node_id, &layout_result)| {
                document
                    .nodes
                    .get(&node_id)
                    .map(|node| RenderInfo::new(layout_result, node.clone()))
            })
            .collect()
    }

    /// Рекурсивно собирает информацию для рендеринга
    fn collect_render_info_recursive(
        &self,
        render_list: &mut Vec<RenderInfo>,
        document: &Document,
        dom_node_id: usize,
    ) {
        if let (Some(layout_result), Some(dom_node)) = (
            self.layout_cache.get(&dom_node_id),
            document.nodes.get(&dom_node_id),
        ) {
            render_list.push(RenderInfo {
                layout: *layout_result,
                node: dom_node.clone(),
                z_index: 0, // TODO: вычислить z-index
            });

            // Рекурсивно обрабатываем детей
            for &child_id in &dom_node.children {
                self.collect_render_info_recursive(render_list, document, child_id);
            }
            if let Some(pseudo_children) = document.pseudo_children(dom_node_id) {
                for &pseudo_dom_id in pseudo_children.values() {
                    self.collect_render_info_recursive(render_list, document, pseudo_dom_id);
                }
            }
        }
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

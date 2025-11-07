pub mod render;
pub mod styles;
pub mod taffy_integration;
pub mod types;

pub use render::*;
pub use types::*;

use crate::css::{PseudoStyle, selectors::PseudoElement};
use crate::dom::Document;
use crate::layout::styles::apply_default_tag_styles;
use std::collections::HashMap;
use taffy::prelude::*;

/// Метрики шрифта для измерения текста
struct FontMetrics {
    char_width: f32,  // коэффициент ширины символа относительно font_size
    char_height: f32, // коэффициент высоты строки относительно font_size
}

/// Функция измерения текста для Taffy (свободная функция)
fn text_measure_function_impl(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<taffy::AvailableSpace>,
    node_context: Option<&TextMeasureContext>,
    font_metrics: &FontMetrics,
) -> taffy::Size<f32> {
    // Если размеры уже известны и положительные, возвращаем их
    if let taffy::Size {
        width: Some(width),
        height: Some(height),
    } = known_dimensions
        && width > 0.0
        && height > 0.0
    {
        return taffy::Size { width, height };
    }

    // Если нет текстового контекста, возвращаем нулевой размер
    let Some(text_ctx) = node_context else {
        return taffy::Size::ZERO;
    };

    // Применяем алгоритм измерения текста
    let words: Vec<&str> = text_ctx.content.split_whitespace().collect();
    if words.is_empty() {
        return taffy::Size::ZERO;
    }

    let char_width = text_ctx.font_size * font_metrics.char_width;
    let line_height = text_ctx.font_size * font_metrics.char_height;

    let min_line_length: usize = words.iter().map(|word| word.len()).max().unwrap_or(0);
    let max_line_length: usize =
        words.iter().map(|word| word.len()).sum::<usize>() + words.len().saturating_sub(1);

    let width = known_dimensions
        .width
        .unwrap_or_else(|| match available_space.width {
            taffy::AvailableSpace::MinContent => min_line_length as f32 * char_width,
            taffy::AvailableSpace::MaxContent => max_line_length as f32 * char_width,
            taffy::AvailableSpace::Definite(w) => w
                .min(max_line_length as f32 * char_width)
                .max(min_line_length as f32 * char_width),
        });

    let height = known_dimensions.height.unwrap_or_else(|| {
        let chars_per_line = (width / char_width).floor() as usize;
        if chars_per_line == 0 {
            return line_height;
        }

        let mut line_count = 1;
        let mut current_line_length = 0;

        for word in &words {
            if current_line_length == 0 {
                current_line_length = word.len();
            } else if current_line_length + word.len() + 1 > chars_per_line {
                line_count += 1;
                current_line_length = word.len();
            } else {
                current_line_length += word.len() + 1;
            }
        }

        line_count as f32 * line_height
    });

    taffy::Size { width, height }
}

pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,

    // Taffy layout engine с поддержкой текстовых контекстов
    taffy: TaffyTree<Option<TextMeasureContext>>,

    // Кеширование результатов
    root_node: Option<NodeId>,
    node_mapping: HashMap<usize, NodeId>, // DOM ID -> Taffy NodeId
    layout_cache: HashMap<usize, LayoutResult>, // Результаты layout по DOM ID
    resolved_styles: HashMap<usize, types::ComputedStyle>,
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

    /// Вычисляет layout с использованием Taffy (новый API)
    pub fn compute_layout(
        &mut self,
        document: &Document,
        styles: &HashMap<usize, HashMap<String, String>>,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
    ) -> HashMap<usize, LayoutResult> {
        // 1. Строим Taffy дерево с контекстами для текста (очищает старое состояние)
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

    /// Возвращает карту вычисленных стилей после применения каскада и наследования.
    pub fn resolved_styles(&self) -> &HashMap<usize, types::ComputedStyle> {
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
        let (taffy_root, _) =
            self.build_node_recursive(document, root_id, styles, pseudo_styles, None)?;
        self.root_node = Some(taffy_root);
        Some(taffy_root)
    }

    /// Рекурсивно строит узел Taffy дерева
    fn build_node_recursive(
        &mut self,
        document: &Document,
        dom_node_id: usize,
        styles: &HashMap<usize, HashMap<String, String>>,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
        parent_style: Option<&ComputedStyle>,
    ) -> Option<(NodeId, crate::layout::types::Display)> {
        // Получаем стили для узла
        let node_styles = styles.get(&dom_node_id).cloned().unwrap_or_default();

        let mut computed_style = ComputedStyle::from_css_properties(&node_styles);

        if let Some(node) = document.nodes.get(&dom_node_id) {
            apply_default_tag_styles(&mut computed_style, &node.tag_name);
        }

        inherit_computed_style(&mut computed_style, parent_style);

        // Текстовые узлы рассматриваем как inline по умолчанию
        if document
            .nodes
            .get(&dom_node_id)
            .is_some_and(|node| node.tag_name.is_none())
        {
            computed_style.display = crate::layout::types::Display::Inline;
        }

        let node_display = computed_style.display;

        // Для корневого элемента принудительно устанавливаем размеры viewport
        if document.root == Some(dom_node_id) {
            computed_style.width = crate::layout::types::Size::Px(self.viewport_width);
            computed_style.height = crate::layout::types::Size::Px(self.viewport_height);
            computed_style.display = crate::layout::types::Display::Block;
        }

        // Пропускаем элементы с display: none
        if matches!(computed_style.display, crate::layout::types::Display::None) {
            return None;
        }

        self.resolved_styles
            .insert(dom_node_id, computed_style.clone());

        // Пропускаем служебные теги, которые не должны рендериться
        if let Some(node) = document.nodes.get(&dom_node_id)
            && let Some(tag) = &node.tag_name
        {
            match tag.as_str() {
                "script" | "style" | "head" | "meta" | "link" | "title" => return None,
                _ => {}
            }
        }

        // Пропускаем пустые текстовые узлы
        if let Some(node) = document.nodes.get(&dom_node_id)
            && node.tag_name.is_none()
        {
            if let Some(text) = &node.text_content {
                if text.trim().is_empty() {
                    return None;
                }
            } else {
                return None;
            }
        }

        // Создаем контекст для текстовых узлов
        let context = if let Some(node) = document.nodes.get(&dom_node_id) {
            if node.tag_name.is_none() {
                // текстовый узел
                Some(TextMeasureContext {
                    content: node.text_content.clone().unwrap_or_default(),
                    font_size: computed_style.font_size,
                    font_weight: computed_style.font_weight,
                    font_style: computed_style.font_style,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Рекурсивно создаем детей
        let mut taffy_children = Vec::new();
        let mut inline_group: Vec<NodeId> = Vec::new();

        if let Some((before_id, before_display)) = self.build_pseudo_element_node(
            document,
            dom_node_id,
            PseudoElement::Before,
            pseudo_styles,
            &computed_style,
        ) {
            self.push_layout_child(
                before_id,
                before_display,
                &mut inline_group,
                &mut taffy_children,
            );
        }

        if let Some(dom_node) = document.nodes.get(&dom_node_id) {
            for &child_dom_id in &dom_node.children {
                if let Some((child_taffy_id, child_display)) = self.build_node_recursive(
                    document,
                    child_dom_id,
                    styles,
                    pseudo_styles,
                    Some(&computed_style),
                ) {
                    self.push_layout_child(
                        child_taffy_id,
                        child_display,
                        &mut inline_group,
                        &mut taffy_children,
                    );
                }
            }
        }

        if let Some((after_id, after_display)) = self.build_pseudo_element_node(
            document,
            dom_node_id,
            PseudoElement::After,
            pseudo_styles,
            &computed_style,
        ) {
            self.push_layout_child(
                after_id,
                after_display,
                &mut inline_group,
                &mut taffy_children,
            );
        }

        self.flush_inline_group(&mut inline_group, &mut taffy_children);

        // Создаем Taffy узел
        let taffy_node_id = if context.is_some() {
            // Текстовый узел - leaf с контекстом
            self.taffy
                .new_leaf_with_context(computed_style.to_taffy_style(), context)
                .ok()?
        } else if taffy_children.is_empty() {
            // Обычный элемент без детей - leaf без контекста
            self.taffy.new_leaf(computed_style.to_taffy_style()).ok()?
        } else {
            // Элемент с детьми - контейнер
            self.taffy
                .new_with_children(computed_style.to_taffy_style(), &taffy_children)
                .ok()?
        };

        // Сохраняем mapping
        self.node_mapping.insert(dom_node_id, taffy_node_id);

        Some((taffy_node_id, node_display))
    }

    fn create_inline_container(&mut self, children: &[NodeId]) -> Option<NodeId> {
        if children.is_empty() {
            return None;
        }

        use taffy::style::{AlignItems, Display as TaffyDisplay, FlexDirection, FlexWrap};

        let style = taffy::Style {
            display: TaffyDisplay::Flex,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: Some(AlignItems::FlexStart),
            ..Default::default()
        };

        self.taffy.new_with_children(style, children).ok()
    }

    fn push_layout_child(
        &mut self,
        child_id: NodeId,
        display: crate::layout::types::Display,
        inline_group: &mut Vec<NodeId>,
        taffy_children: &mut Vec<NodeId>,
    ) {
        if matches!(display, crate::layout::types::Display::Inline) {
            inline_group.push(child_id);
        } else {
            self.flush_inline_group(inline_group, taffy_children);
            taffy_children.push(child_id);
        }
    }

    fn flush_inline_group(
        &mut self,
        inline_group: &mut Vec<NodeId>,
        taffy_children: &mut Vec<NodeId>,
    ) {
        if inline_group.is_empty() {
            return;
        }

        if let Some(container_id) = self.create_inline_container(inline_group) {
            taffy_children.push(container_id);
        }
        inline_group.clear();
    }

    fn build_pseudo_element_node(
        &mut self,
        document: &Document,
        owner_id: usize,
        pseudo: PseudoElement,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
        parent_style: &ComputedStyle,
    ) -> Option<(NodeId, crate::layout::types::Display)> {
        let styles_for_owner = pseudo_styles.get(&owner_id)?;
        let pseudo_style = styles_for_owner.get(&pseudo)?;
        let pseudo_node_id = document.pseudo_child_id(owner_id, pseudo)?;

        let mut computed_style = ComputedStyle::from_css_properties(&pseudo_style.properties);
        if !pseudo_style.properties.contains_key("display") {
            computed_style.display = crate::layout::types::Display::Inline;
        }
        inherit_computed_style(&mut computed_style, Some(parent_style));

        if matches!(computed_style.display, crate::layout::types::Display::None) {
            return None;
        }

        let text_content = document
            .nodes
            .get(&pseudo_node_id)
            .and_then(|node| node.text_content.clone())
            .filter(|content| !content.is_empty());

        let context = text_content.map(|content| TextMeasureContext {
            content,
            font_size: computed_style.font_size,
            font_weight: computed_style.font_weight,
            font_style: computed_style.font_style,
        });

        let taffy_node_id = if let Some(ctx) = context {
            self.taffy
                .new_leaf_with_context(computed_style.to_taffy_style(), Some(ctx))
                .ok()?
        } else {
            self.taffy.new_leaf(computed_style.to_taffy_style()).ok()?
        };

        self.resolved_styles
            .insert(pseudo_node_id, computed_style.clone());
        self.node_mapping.insert(pseudo_node_id, taffy_node_id);

        Some((taffy_node_id, computed_style.display))
    }

    /// Вычисляет layout с измерением текста
    fn compute_taffy_layouts(&mut self) {
        if let Some(root) = self.root_node {
            let font_metrics = FontMetrics {
                char_width: 0.6,  // эвристика: символ ≈ 0.6 от font_size
                char_height: 1.2, // line height ≈ 1.2 от font_size
            };

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
                        text_measure_function_impl(
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

fn inherit_computed_style(style: &mut ComputedStyle, parent: Option<&ComputedStyle>) {
    if let Some(parent) = parent {
        if style.color.is_none() {
            style.color = parent.color.clone();
        }
        if style.background_color.is_none() {
            style.background_color = parent.background_color.clone();
        }
        if (style.font_size - 16.0).abs() < f32::EPSILON {
            style.font_size = parent.font_size;
        }
        if matches!(style.font_weight, FontWeight::Normal) {
            style.font_weight = parent.font_weight;
        }
        if matches!(style.font_style, FontStyle::Normal) {
            style.font_style = parent.font_style;
        }
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

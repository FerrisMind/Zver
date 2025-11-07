use crate::css::{PseudoStyle, selectors::PseudoElement};
use crate::dom::Document;
use crate::layout::metrics::TextMeasureContext;
use crate::layout::styles::apply_default_tag_styles;
use crate::layout::types::{ComputedStyle, Display};
use std::collections::HashMap;
use taffy::prelude::*;

/// Вспомогательная структура для построения Taffy дерева
pub struct TreeBuilder<'a> {
    pub taffy: &'a mut TaffyTree<Option<TextMeasureContext>>,
    pub node_mapping: &'a mut HashMap<usize, NodeId>,
    pub resolved_styles: &'a mut HashMap<usize, ComputedStyle>,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl<'a> TreeBuilder<'a> {
    /// Рекурсивно строит узел Taffy дерева
    pub fn build_node_recursive(
        &mut self,
        document: &Document,
        dom_node_id: usize,
        styles: &HashMap<usize, HashMap<String, String>>,
        pseudo_styles: &HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,
        parent_style: Option<&ComputedStyle>,
    ) -> Option<(NodeId, Display)> {
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
            computed_style.display = Display::Inline;
        }

        let node_display = computed_style.display;

        // Для корневого элемента принудительно устанавливаем размеры viewport
        if document.root == Some(dom_node_id) {
            computed_style.width = crate::layout::types::Size::Px(self.viewport_width);
            computed_style.height = crate::layout::types::Size::Px(self.viewport_height);
            computed_style.display = Display::Block;
        }

        // Пропускаем элементы с display: none
        if matches!(computed_style.display, Display::None) {
            return None;
        }

        self.resolved_styles
            .insert(dom_node_id, computed_style.clone());

        // Пропускаем служебные теги
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
            self.taffy
                .new_leaf_with_context(computed_style.to_taffy_style(), context)
                .ok()?
        } else if taffy_children.is_empty() {
            self.taffy.new_leaf(computed_style.to_taffy_style()).ok()?
        } else {
            self.taffy
                .new_with_children(computed_style.to_taffy_style(), &taffy_children)
                .ok()?
        };

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
        display: Display,
        inline_group: &mut Vec<NodeId>,
        taffy_children: &mut Vec<NodeId>,
    ) {
        if matches!(display, Display::Inline) {
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
    ) -> Option<(NodeId, Display)> {
        let styles_for_owner = pseudo_styles.get(&owner_id)?;
        let pseudo_style = styles_for_owner.get(&pseudo)?;
        let pseudo_node_id = document.pseudo_child_id(owner_id, pseudo)?;

        let mut computed_style = ComputedStyle::from_css_properties(&pseudo_style.properties);
        if !pseudo_style.properties.contains_key("display") {
            computed_style.display = Display::Inline;
        }
        inherit_computed_style(&mut computed_style, Some(parent_style));

        if matches!(computed_style.display, Display::None) {
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
}

/// Применяет наследование стилей от родителя
pub fn inherit_computed_style(style: &mut ComputedStyle, parent: Option<&ComputedStyle>) {
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
        if matches!(style.font_weight, crate::layout::types::FontWeight::Normal) {
            style.font_weight = parent.font_weight;
        }
        if matches!(style.font_style, crate::layout::types::FontStyle::Normal) {
            style.font_style = parent.font_style;
        }
    }
}

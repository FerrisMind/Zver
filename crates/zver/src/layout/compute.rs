use super::styles::{apply_css_styles, apply_default_tag_styles};
use super::types::*;
use crate::dom::{Document, Node};
use std::collections::HashMap;

/// Строит layout узел из DOM узла
pub fn build_layout_node(
    document: &Document,
    node_id: usize,
    styles: &HashMap<usize, HashMap<String, String>>,
    viewport_width: f32,
    viewport_height: f32,
    parent_style: Option<&ComputedStyle>,
) -> Option<LayoutNode> {
    let node = document.nodes.get(&node_id)?;

    // Пропускаем служебные теги, которые не должны рендериться
    if let Some(tag) = &node.tag_name {
        match tag.as_str() {
            "script" | "style" | "head" | "meta" | "link" | "title" => return None,
            _ => {}
        }
    }

    // Создаём базовый стиль с наследованием
    let mut style = ComputedStyle::default();

    // Применяем стили по умолчанию для HTML тегов
    apply_default_tag_styles(&mut style, &node.tag_name);

    // Наследуем от родителя
    inherit_properties(&mut style, parent_style);

    // Применяем CSS стили
    if let Some(node_styles) = styles.get(&node_id) {
        apply_css_styles(&mut style, node_styles);
    }

    if matches!(style.display, super::types::Display::None) {
        return None;
    }

    let mut children = Vec::new();
    for &child_id in &node.children {
        if let Some(child) = build_layout_node(
            document,
            child_id,
            styles,
            viewport_width,
            viewport_height,
            Some(&style),
        ) {
            children.push(child);
        }
    }

    let mut layout_node = LayoutNode {
        style,
        dimensions: Dimensions::default(),
        children,
        dom_node: node_id,
    };

    compute_dimensions(&mut layout_node, viewport_width, viewport_height, node);
    Some(layout_node)
}

/// Наследует свойства от родительского стиля
fn inherit_properties(style: &mut ComputedStyle, parent: Option<&ComputedStyle>) {
    if let Some(parent) = parent {
        // Наследуем цвет только если он не установлен
        if style.color.is_none() {
            style.color = parent.color.clone();
        }
        // Наследуем размер шрифта только если он дефолтный
        if style.font_size == 16.0 {
            style.font_size = parent.font_size;
        }
        // Наследуем background только если не установлен
        if style.background_color.is_none() {
            style.background_color = parent.background_color.clone();
        }
        // Наследуем font-weight и font-style ТОЛЬКО если они Normal
        if matches!(style.font_weight, FontWeight::Normal) {
            style.font_weight = parent.font_weight;
        }
        if matches!(style.font_style, FontStyle::Normal) {
            style.font_style = parent.font_style;
        }
    }
}

/// Вычисляет размеры узла
fn compute_dimensions(
    node: &mut LayoutNode,
    viewport_width: f32,
    viewport_height: f32,
    dom_node: &Node,
) {
    let is_text_node = dom_node.tag_name.is_none() && dom_node.text_content.is_some();

    node.dimensions.width = match node.style.width {
        super::types::Size::Px(px) => px,
        super::types::Size::Percent(percent) => viewport_width * percent,
        super::types::Size::Auto => {
            if is_text_node {
                // Примерная ширина текста
                if let Some(text) = &dom_node.text_content {
                    let char_count = text.trim().chars().count() as f32;
                    let char_width = node.style.font_size * 0.6;
                    (char_count * char_width).min(viewport_width)
                } else {
                    0.0
                }
            } else {
                match node.style.display {
                    super::types::Display::Block | super::types::Display::Flex => viewport_width,
                    super::types::Display::Inline => {
                        let children_width: f32 = node
                            .children
                            .iter()
                            .map(|child| child.dimensions.width)
                            .sum();

                        if children_width == 0.0 && dom_node.text_content.is_some() {
                            if let Some(text) = &dom_node.text_content {
                                let char_count = text.trim().chars().count() as f32;
                                let char_width = node.style.font_size * 0.6;
                                (char_count * char_width).min(viewport_width)
                            } else {
                                0.0
                            }
                        } else {
                            children_width.max(10.0)
                        }
                    }
                    super::types::Display::None => 0.0,
                }
            }
        }
    };

    node.dimensions.height = match node.style.height {
        super::types::Size::Px(px) => px,
        super::types::Size::Percent(percent) => viewport_height * percent,
        super::types::Size::Auto => {
            if is_text_node {
                node.style.font_size * 1.4
            } else {
                let mut height = 0.0;
                for child in &node.children {
                    height += child.dimensions.height;
                }
                height.max(node.style.font_size * 1.2)
            }
        }
    };
}

/// Вычисляет позиции всех узлов относительно родителя
pub fn compute_positions(node: &mut LayoutNode, parent_x: f32, parent_y: f32) {
    node.dimensions.x = parent_x;
    node.dimensions.y = parent_y;

    let mut current_y = parent_y;

    for child in &mut node.children {
        match child.style.display {
            super::types::Display::Block => {
                compute_positions(child, parent_x, current_y);
                current_y += child.dimensions.height;
            }
            super::types::Display::Inline => {
                compute_positions(child, parent_x, current_y);
            }
            super::types::Display::Flex => {
                compute_positions(child, parent_x, current_y);
                current_y += child.dimensions.height;
            }
            super::types::Display::None => {}
        }
    }
}

use super::{FontWeight, LayoutNode};
use crate::css::color::{Color, parse_css_color};
use crate::dom::Document;

/// Информация о рендеринге узла
#[derive(Debug, Clone)]
pub struct RenderInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub background_color: Option<Color>,
    pub text_color: Color,
    pub text_content: Option<String>,
    pub tag_name: Option<String>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub is_text_node: bool,
}

impl RenderInfo {
    /// Создает информацию о рендеринге из layout узла
    pub fn from_layout_node(node: &LayoutNode, dom: &Document) -> Self {
        let x = node.dimensions.x;
        let y = node.dimensions.y;
        let width = node.dimensions.width;
        let height = node.dimensions.height;

        let (background_color, text_content, tag_name, text_color) =
            if let Some(dom_node) = dom.nodes.get(&node.dom_node) {
                let bg = if let Some(bg_css) = &node.style.background_color {
                    parse_css_color(bg_css)
                } else {
                    None
                };

                let text_color = if let Some(color_css) = &node.style.color {
                    parse_css_color(color_css).unwrap_or(Color::BLACK)
                } else {
                    Color::BLACK
                };

                let text = dom_node.text_content.clone();
                let tag = dom_node.tag_name.clone();

                (bg, text, tag, text_color)
            } else {
                (None, None, None, Color::BLACK)
            };

        let is_text_node = tag_name.is_none();
        let font_size = node.style.font_size.clamp(8.0, 48.0);

        Self {
            x,
            y,
            width,
            height,
            background_color,
            text_color,
            text_content,
            tag_name,
            font_size,
            font_weight: node.style.font_weight,
            is_text_node,
        }
    }

    /// Проверяет, нужно ли рендерить этот узел
    pub fn should_render(&self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }

    /// Проверяет, нужно ли рендерить текст
    pub fn should_render_text(&self) -> bool {
        self.is_text_node
            && self
                .text_content
                .as_ref()
                .is_some_and(|t| !t.trim().is_empty())
            && self.width > 10.0
            && self.height > 10.0
    }

    /// Проверяет, нужно ли рендерить фон
    pub fn should_render_background(&self) -> bool {
        self.background_color.is_some() && self.background_color != Some(Color::TRANSPARENT)
    }
}

/// Собирает информацию о рендеринге для всех узлов в дереве
pub fn collect_render_info(node: &LayoutNode, dom: &Document, result: &mut Vec<RenderInfo>) {
    let info = RenderInfo::from_layout_node(node, dom);
    if info.should_render() {
        result.push(info);
    }

    // Рекурсивно обрабатываем дочерние узлы
    for child in &node.children {
        collect_render_info(child, dom, result);
    }
}

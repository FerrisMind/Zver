use crate::dom::Document;
use std::collections::HashMap;
use taffy::prelude::*;
use taffy::style::{AlignItems, Dimension, JustifyContent};

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub style: ComputedStyle,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutNode>,
    pub dom_node: usize, // ID узла в DOM
}

#[derive(Debug, Clone, Default)]
pub struct Dimensions {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub width: Size,
    pub height: Size,
    pub background_color: Option<String>,
    pub color: Option<String>,
    pub font_size: f32,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            position: Position::Static,
            width: Size::Auto,
            height: Size::Auto,
            background_color: None,
            color: None,
            font_size: 16.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    None,
    Flex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Auto,
    Px(f32),
    Percent(f32),
}

pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
    pub layout_tree: Option<LayoutNode>,
    taffy: TaffyTree<()>,
}

unsafe impl Send for LayoutEngine {}
unsafe impl Sync for LayoutEngine {}

impl LayoutEngine {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            layout_tree: None,
            taffy: TaffyTree::new(),
        }
    }

    pub fn compute(
        &mut self,
        document: &Document,
        styles: &HashMap<usize, HashMap<String, String>>,
    ) -> Option<&LayoutNode> {
        let root_id = document.root?;

        // Строим Taffy-дерево из DOM
        let taffy_root = build_taffy_tree(&mut self.taffy, document, root_id, styles);

        // Вычисляем лейаут
        let _ = self
            .taffy
            .compute_layout(
                taffy_root,
                taffy::Size {
                    width: AvailableSpace::Definite(self.viewport_width),
                    height: AvailableSpace::Definite(self.viewport_height),
                },
            )
            .ok();

        // Параллельно строим нашу структурированную модель для дальнейшего рендера
        self.layout_tree = build_layout_node(document, root_id, styles, self, None);
        
        // Устанавливаем позиции всех узлов
        if let Some(tree) = &mut self.layout_tree {
            compute_positions(tree, 0.0, 0.0);
        }
        
        self.layout_tree.as_ref()
    }

    pub fn layout_tree(&self) -> Option<&LayoutNode> {
        self.layout_tree.as_ref()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

fn compute_style_for_node(
    styles: &HashMap<usize, HashMap<String, String>>,
    node_id: usize,
    parent: Option<&ComputedStyle>,
) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    inherit_properties(&mut style, parent);

    let node_styles = styles.get(&node_id).cloned().unwrap_or_default();

    if let Some(display) = node_styles.get("display") {
        style.display = match display.as_str() {
            "block" => Display::Block,
            "inline" => Display::Inline,
            "none" => Display::None,
            "flex" => Display::Flex,
            _ => style.display,
        };
    }

    if let Some(position) = node_styles.get("position") {
        style.position = match position.as_str() {
            "relative" => Position::Relative,
            "absolute" => Position::Absolute,
            _ => Position::Static,
        };
    }

    if let Some(width) = node_styles.get("width") {
        style.width = parse_size(width);
    }

    if let Some(height) = node_styles.get("height") {
        style.height = parse_size(height);
    }

    if let Some(color) = node_styles.get("background-color") {
        style.background_color = Some(color.clone());
    }

    if let Some(color) = node_styles.get("color") {
        style.color = Some(color.clone());
    }

    if let Some(size) = node_styles
        .get("font-size")
        .and_then(|v| v.parse::<f32>().ok())
    {
        style.font_size = size;
    }

    style
}

fn parse_size(value: &str) -> Size {
    let value = value.trim();
    if let Some(px) = value.strip_suffix("px") {
        px.trim().parse::<f32>().map(Size::Px).unwrap_or(Size::Auto)
    } else if let Some(percent) = value.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .map(Size::Percent)
            .unwrap_or(Size::Auto)
    } else if let Ok(px) = value.parse::<f32>() {
        Size::Px(px)
    } else {
        Size::Auto
    }
}

fn build_layout_node(
    document: &Document,
    node_id: usize,
    styles: &HashMap<usize, HashMap<String, String>>,
    engine: &LayoutEngine,
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

    let style = compute_style_for_node(styles, node_id, parent_style);

    if matches!(style.display, Display::None) {
        return None;
    }

    let mut children = Vec::new();
    for &child_id in &node.children {
        if let Some(child) = build_layout_node(document, child_id, styles, engine, Some(&style)) {
            children.push(child);
        }
    }

    let mut layout_node = LayoutNode {
        style,
        dimensions: Dimensions::default(),
        children,
        dom_node: node_id,
    };

    compute_dimensions(&mut layout_node, engine, &node);
    Some(layout_node)
}

fn inherit_properties(style: &mut ComputedStyle, parent: Option<&ComputedStyle>) {
    if let Some(parent) = parent {
        if style.color.is_none() {
            style.color = parent.color.clone();
        }
        if style.font_size <= 0.0 {
            style.font_size = parent.font_size;
        }
    }
}

fn compute_dimensions(node: &mut LayoutNode, engine: &LayoutEngine, dom_node: &crate::dom::Node) {
    // Для текстовых узлов устанавливаем размеры на основе текста
    let is_text_node = dom_node.tag_name.is_none() && dom_node.text_content.is_some();
    
    node.dimensions.width = match node.style.width {
        Size::Px(px) => px,
        Size::Percent(percent) => engine.viewport_width * percent / 100.0,
        Size::Auto => {
            if is_text_node {
                // Примерная ширина текста: количество символов * ширина символа
                if let Some(text) = &dom_node.text_content {
                    let char_count = text.trim().chars().count() as f32;
                    let char_width = node.style.font_size * 0.6; // Примерная ширина символа
                    (char_count * char_width).min(engine.viewport_width)
                } else {
                    0.0
                }
            } else {
                match node.style.display {
                    Display::Block | Display::Flex => engine.viewport_width,
                    Display::Inline => node
                        .children
                        .iter()
                        .map(|child| child.dimensions.width)
                        .sum::<f32>(),
                    Display::None => 0.0,
                }
            }
        }
    };

    node.dimensions.height = match node.style.height {
        Size::Px(px) => px,
        Size::Percent(percent) => engine.viewport_height * percent / 100.0,
        Size::Auto => {
            if is_text_node {
                // Высота текста = размер шрифта * line-height
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

// Вычисление позиций всех узлов относительно родителя
fn compute_positions(node: &mut LayoutNode, parent_x: f32, parent_y: f32) {
    node.dimensions.x = parent_x;
    node.dimensions.y = parent_y;
    
    let mut current_y = parent_y;
    
    for child in &mut node.children {
        // Блочные элементы располагаются вертикально
        match child.style.display {
            Display::Block => {
                compute_positions(child, parent_x, current_y);
                current_y += child.dimensions.height;
            }
            Display::Inline => {
                // Инлайновые элементы (текст) располагаются в той же строке
                compute_positions(child, parent_x, current_y);
            }
            Display::Flex => {
                compute_positions(child, parent_x, current_y);
                current_y += child.dimensions.height;
            }
            Display::None => {}
        }
    }
}

fn build_taffy_tree(
    taffy: &mut TaffyTree<()>,
    document: &Document,
    node_id: usize,
    styles: &HashMap<usize, HashMap<String, String>>,
) -> NodeId {
    let node_styles = styles.get(&node_id).cloned().unwrap_or_default();

    // Определяем display и создаем соответствующий Style
    let display_str = node_styles.get("display").map(|s| s.as_str());

    let mut style = match display_str {
        Some("flex") => Style {
            display: taffy::prelude::Display::Flex,
            ..Default::default()
        },
        Some("grid") => Style {
            display: taffy::prelude::Display::Grid,
            ..Default::default()
        },
        Some("none") => Style {
            display: taffy::prelude::Display::None,
            ..Default::default()
        },
        _ => Style {
            display: taffy::prelude::Display::Block,
            ..Default::default()
        },
    };

    // Применяем размеры
    if let Some(width) = node_styles.get("width") {
        style.size.width = parse_dimension(width);
    }
    if let Some(height) = node_styles.get("height") {
        style.size.height = parse_dimension(height);
    }

    // Минимальные и максимальные размеры
    if let Some(min_width) = node_styles.get("min-width") {
        style.min_size.width = parse_dimension(min_width);
    }
    if let Some(min_height) = node_styles.get("min-height") {
        style.min_size.height = parse_dimension(min_height);
    }
    if let Some(max_width) = node_styles.get("max-width") {
        style.max_size.width = parse_dimension(max_width);
    }
    if let Some(max_height) = node_styles.get("max-height") {
        style.max_size.height = parse_dimension(max_height);
    }

    // Padding
    if let Some(padding) = node_styles.get("padding") {
        let dim = parse_length_percentage(padding);
        style.padding = taffy::geometry::Rect {
            left: dim,
            right: dim,
            top: dim,
            bottom: dim,
        };
    }
    if let Some(p) = node_styles.get("padding-left") {
        style.padding.left = parse_length_percentage(p);
    }
    if let Some(p) = node_styles.get("padding-right") {
        style.padding.right = parse_length_percentage(p);
    }
    if let Some(p) = node_styles.get("padding-top") {
        style.padding.top = parse_length_percentage(p);
    }
    if let Some(p) = node_styles.get("padding-bottom") {
        style.padding.bottom = parse_length_percentage(p);
    }

    // Margin
    if let Some(margin) = node_styles.get("margin") {
        let dim = parse_length_percentage_auto(margin);
        style.margin = taffy::geometry::Rect {
            left: dim,
            right: dim,
            top: dim,
            bottom: dim,
        };
    }
    if let Some(m) = node_styles.get("margin-left") {
        style.margin.left = parse_length_percentage_auto(m);
    }
    if let Some(m) = node_styles.get("margin-right") {
        style.margin.right = parse_length_percentage_auto(m);
    }
    if let Some(m) = node_styles.get("margin-top") {
        style.margin.top = parse_length_percentage_auto(m);
    }
    if let Some(m) = node_styles.get("margin-bottom") {
        style.margin.bottom = parse_length_percentage_auto(m);
    }

    // Flexbox properties
    if let Some(flex_direction) = node_styles.get("flex-direction") {
        style.flex_direction = match flex_direction.as_str() {
            "row" => FlexDirection::Row,
            "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column,
            "column-reverse" => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        };
    }

    if let Some(flex_grow) = node_styles.get("flex-grow") {
        style.flex_grow = flex_grow.parse().unwrap_or(0.0);
    }

    if let Some(flex_shrink) = node_styles.get("flex-shrink") {
        style.flex_shrink = flex_shrink.parse().unwrap_or(1.0);
    }

    if let Some(justify_content) = node_styles.get("justify-content") {
        style.justify_content = Some(match justify_content.as_str() {
            "flex-start" => JustifyContent::Start,
            "flex-end" => JustifyContent::End,
            "center" => JustifyContent::Center,
            "space-between" => JustifyContent::SpaceBetween,
            "space-around" => JustifyContent::SpaceAround,
            "space-evenly" => JustifyContent::SpaceEvenly,
            _ => JustifyContent::Start,
        });
    }

    if let Some(align_items) = node_styles.get("align-items") {
        style.align_items = Some(match align_items.as_str() {
            "flex-start" => AlignItems::Start,
            "flex-end" => AlignItems::End,
            "center" => AlignItems::Center,
            "stretch" => AlignItems::Stretch,
            _ => AlignItems::Stretch,
        });
    }

    // Gap
    if let Some(gap) = node_styles.get("gap") {
        let dim = parse_length_percentage(gap);
        style.gap = taffy::geometry::Size {
            width: dim,
            height: dim,
        };
    }

    // Создаем дочерние узлы
    let mut children_ids: Vec<NodeId> = Vec::new();
    if let Some(node) = document.nodes.get(&node_id) {
        for &child in &node.children {
            let child_style = styles.get(&child).cloned().unwrap_or_default();
            if child_style.get("display").map(|s| s.as_str()) == Some("none") {
                continue;
            }
            let child_id = build_taffy_tree(taffy, document, child, styles);
            children_ids.push(child_id);
        }
    }

    taffy.new_with_children(style, &children_ids).unwrap()
}

fn parse_dimension(value: &str) -> Dimension {
    let value = value.trim();
    if value == "auto" {
        return Dimension::auto();
    }

    if let Some(px) = value.strip_suffix("px")
        && let Ok(val) = px.trim().parse::<f32>()
    {
        return Dimension::length(val);
    }

    if let Some(percent) = value.strip_suffix('%')
        && let Ok(val) = percent.trim().parse::<f32>()
    {
        return Dimension::percent(val / 100.0);
    }

    if let Ok(val) = value.parse::<f32>() {
        return Dimension::length(val);
    }

    Dimension::auto()
}

fn parse_length_percentage(value: &str) -> taffy::style::LengthPercentage {
    let value = value.trim();

    if let Some(px) = value.strip_suffix("px")
        && let Ok(val) = px.trim().parse::<f32>()
    {
        return taffy::style::LengthPercentage::length(val);
    }

    if let Some(percent) = value.strip_suffix('%')
        && let Ok(val) = percent.trim().parse::<f32>()
    {
        return taffy::style::LengthPercentage::percent(val / 100.0);
    }

    if let Ok(val) = value.parse::<f32>() {
        return taffy::style::LengthPercentage::length(val);
    }

    taffy::style::LengthPercentage::length(0.0)
}

fn parse_length_percentage_auto(value: &str) -> taffy::style::LengthPercentageAuto {
    let value = value.trim();

    if value == "auto" {
        return taffy::style::LengthPercentageAuto::auto();
    }

    if let Some(px) = value.strip_suffix("px")
        && let Ok(val) = px.trim().parse::<f32>()
    {
        return taffy::style::LengthPercentageAuto::length(val);
    }

    if let Some(percent) = value.strip_suffix('%')
        && let Ok(val) = percent.trim().parse::<f32>()
    {
        return taffy::style::LengthPercentageAuto::percent(val / 100.0);
    }

    if let Ok(val) = value.parse::<f32>() {
        return taffy::style::LengthPercentageAuto::length(val);
    }

    taffy::style::LengthPercentageAuto::auto()
}

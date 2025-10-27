use crate::dom::Document;
use std::collections::HashMap;
use taffy::prelude::*;

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub style: ComputedStyle,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutNode>,
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
            .compute_layout(taffy_root, taffy::Size { width: AvailableSpace::Definite(self.viewport_width), height: AvailableSpace::Definite(self.viewport_height) })
            .ok();

        // Параллельно строим нашу структурированную модель для дальнейшего рендера
        self.layout_tree = build_layout_node(document, root_id, styles, self, None);
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
    };

    compute_dimensions(&mut layout_node, engine);
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

fn compute_dimensions(node: &mut LayoutNode, engine: &LayoutEngine) {
    node.dimensions.width = match node.style.width {
        Size::Px(px) => px,
        Size::Percent(percent) => engine.viewport_width * percent / 100.0,
        Size::Auto => match node.style.display {
            Display::Block | Display::Flex => engine.viewport_width,
            Display::Inline => node
                .children
                .iter()
                .map(|child| child.dimensions.width)
                .sum::<f32>(),
            Display::None => 0.0,
        },
    };

    node.dimensions.height = match node.style.height {
        Size::Px(px) => px,
        Size::Percent(percent) => engine.viewport_height * percent / 100.0,
        Size::Auto => {
            let mut height = 0.0;
            for child in &node.children {
                height += child.dimensions.height;
            }
            height.max(node.style.font_size * 1.2)
        }
    };
}

fn build_taffy_tree(
    taffy: &mut TaffyTree<()>,
    document: &Document,
    node_id: usize,
    styles: &HashMap<usize, HashMap<String, String>>,
) -> NodeId {
    let node_styles = styles.get(&node_id).cloned().unwrap_or_default();

    let display = node_styles.get("display").map(|s| s.as_str());
    let is_flex = matches!(display, Some("flex"));

    let style = if is_flex {
        Style {
            display: taffy::prelude::Display::Flex,
            ..Default::default()
        }
    } else {
        Style {
            display: taffy::prelude::Display::Block,
            ..Default::default()
        }
    };

    // Дети
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


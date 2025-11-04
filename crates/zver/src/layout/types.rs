use std::collections::HashMap;
use taffy::style::{AlignItems, FlexDirection, JustifyContent};

/// Результат layout вычисления от Taffy
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutResult {
    /// ID DOM узла
    pub node_id: usize,
    /// Позиция X (включая padding/border)
    pub x: f32,
    /// Позиция Y (включая padding/border)
    pub y: f32,
    /// Полная ширина (включая padding/border)
    pub width: f32,
    /// Полная высота (включая padding/border)
    pub height: f32,
    /// Позиция контентной области X (без padding/border)
    pub content_x: f32,
    /// Позиция контентной области Y (без padding/border)
    pub content_y: f32,
    /// Ширина контентной области
    pub content_width: f32,
    /// Высота контентной области
    pub content_height: f32,
}

/// Визуальные свойства для рендеринга (отделены от layout свойств)
#[derive(Debug, Clone)]
pub struct VisualProperties {
    pub background_color: Option<String>,
    pub color: Option<String>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
}

/// Контекст для измерения текста в Taffy
#[derive(Debug, Clone)]
pub struct TextMeasureContext {
    pub content: String,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
}

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
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub list_style_type: ListStyleType,
    pub flex_direction: FlexDirection,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
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
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            list_style_type: ListStyleType::None,
            flex_direction: FlexDirection::Row,
            justify_content: None,
            align_items: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        }
    }
}

impl ComputedStyle {
    /// Создает ComputedStyle из CSS свойств
    pub fn from_css_properties(properties: &HashMap<String, String>) -> Self {
        let mut style = ComputedStyle::default();

        for (property, value) in properties {
            match property.as_str() {
                "display" => {
                    style.display = match value.as_str() {
                        "block" => Display::Block,
                        "inline" => Display::Inline,
                        "none" => Display::None,
                        "flex" => Display::Flex,
                        _ => style.display,
                    };
                    // Сбрасываем flex свойства если display не flex
                    if !matches!(style.display, Display::Flex) {
                        style.flex_direction = FlexDirection::Row;
                        style.justify_content = None;
                        style.align_items = None;
                        style.flex_grow = 0.0;
                        style.flex_shrink = 1.0;
                    }
                }
                "position" => {
                    style.position = match value.as_str() {
                        "static" => Position::Static,
                        "relative" => Position::Relative,
                        "absolute" => Position::Absolute,
                        _ => style.position,
                    };
                }
                "width" => {
                    style.width = Size::parse(value);
                }
                "height" => {
                    style.height = Size::parse(value);
                }
                "background-color" => {
                    style.background_color = Some(value.clone());
                }
                "color" => {
                    style.color = Some(value.clone());
                }
                "font-size" => {
                    if let Ok(size) = value.replace("px", "").parse::<f32>() {
                        style.font_size = size;
                    }
                }
                "font-weight" => {
                    style.font_weight = match value.as_str() {
                        "bold" | "700" | "800" | "900" => FontWeight::Bold,
                        _ => FontWeight::Normal,
                    };
                }
                "font-style" => {
                    style.font_style = match value.as_str() {
                        "italic" => FontStyle::Italic,
                        _ => FontStyle::Normal,
                    };
                }
                "flex-direction" => {
                    style.flex_direction = match value.as_str() {
                        "row" => FlexDirection::Row,
                        "row-reverse" => FlexDirection::RowReverse,
                        "column" => FlexDirection::Column,
                        "column-reverse" => FlexDirection::ColumnReverse,
                        _ => style.flex_direction,
                    };
                }
                "justify-content" => {
                    style.justify_content = Some(match value.as_str() {
                        "flex-start" => JustifyContent::Start,
                        "flex-end" => JustifyContent::End,
                        "center" => JustifyContent::Center,
                        "space-between" => JustifyContent::SpaceBetween,
                        "space-around" => JustifyContent::SpaceAround,
                        "space-evenly" => JustifyContent::SpaceEvenly,
                        _ => JustifyContent::Start,
                    });
                }
                "align-items" => {
                    style.align_items = Some(match value.as_str() {
                        "flex-start" => AlignItems::Start,
                        "flex-end" => AlignItems::End,
                        "center" => AlignItems::Center,
                        "stretch" => AlignItems::Stretch,
                        _ => AlignItems::Stretch,
                    });
                }
                "flex-grow" => {
                    if let Ok(grow) = value.parse::<f32>() {
                        style.flex_grow = grow;
                    }
                }
                "flex-shrink" => {
                    if let Ok(shrink) = value.parse::<f32>() {
                        style.flex_shrink = shrink;
                    }
                }
                _ => {} // остальные свойства игнорируем для layout
            }
        }

        style
    }

    /// Конвертирует в Taffy Style для layout-движка
    pub fn to_taffy_style(&self) -> taffy::Style {
        let mut style = taffy::Style {
            display: match self.display {
                Display::Block => taffy::Display::Block,
                Display::Flex => taffy::Display::Flex,
                Display::None => taffy::Display::None,
                Display::Inline => taffy::Display::Block, // Inline как Block в Taffy
            },
            ..Default::default()
        };

        // Устанавливаем flex свойства
        if matches!(self.display, Display::Flex) {
            // Это flex контейнер - используем flex свойства
            style.flex_direction = self.flex_direction;
            style.justify_content = self.justify_content;
            style.align_items = self.align_items;
            style.flex_grow = self.flex_grow;
            style.flex_shrink = self.flex_shrink;
        } else if matches!(self.display, Display::Block) {
            // Для блоковых элементов используем вертикальный layout
            style.flex_direction = FlexDirection::Column;
        }

        // Размеры
        style.size.width = self.width.to_taffy_dimension();
        style.size.height = self.height.to_taffy_dimension();

        style
    }

    /// Возвращает только визуальные свойства для рендеринга
    pub fn visual_properties(&self) -> VisualProperties {
        VisualProperties {
            background_color: self.background_color.clone(),
            color: self.color.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
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

impl Size {
    /// Парсит размер из CSS значения
    pub fn parse(value: &str) -> Self {
        if value == "auto" {
            Size::Auto
        } else if let Some(px_value) = value.strip_suffix("px") {
            if let Ok(pixels) = px_value.parse::<f32>() {
                Size::Px(pixels)
            } else {
                Size::Auto
            }
        } else if let Some(percent_value) = value.strip_suffix('%') {
            if let Ok(percent) = percent_value.parse::<f32>() {
                Size::Percent(percent / 100.0)
            } else {
                Size::Auto
            }
        } else {
            Size::Auto
        }
    }

    /// Конвертирует в Taffy Dimension
    pub fn to_taffy_dimension(&self) -> taffy::Dimension {
        match *self {
            Size::Auto => taffy::Dimension::auto(),
            Size::Px(val) => taffy::Dimension::length(val),
            Size::Percent(val) => taffy::Dimension::percent(val),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStyleType {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
}

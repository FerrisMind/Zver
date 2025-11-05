use std::collections::HashMap;
use taffy::geometry;
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
    pub align_self: Option<AlignItems>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_wrap: taffy::style::FlexWrap,
    pub gap: taffy::geometry::Size<taffy::style::LengthPercentage>,

    // Spacing properties
    pub margin: taffy::geometry::Rect<taffy::style::LengthPercentageAuto>,
    pub padding: taffy::geometry::Rect<taffy::style::LengthPercentage>,
    pub border: taffy::geometry::Rect<taffy::style::LengthPercentage>,

    // Grid properties - simplified for now, will be implemented later
    pub grid_template_rows: Vec<taffy::style::TrackSizingFunction>,
    pub grid_template_columns: Vec<taffy::style::TrackSizingFunction>,
    pub grid_row: geometry::Line<taffy::style::GridPlacement>,
    pub grid_column: geometry::Line<taffy::style::GridPlacement>,

    // Size constraints
    pub min_width: Size,
    pub min_height: Size,
    pub max_width: Size,
    pub max_height: Size,

    // Aspect ratio
    pub aspect_ratio: Option<f32>,
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
            align_self: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_wrap: taffy::style::FlexWrap::NoWrap,
            gap: taffy::geometry::Size::zero(),

            // Spacing properties
            margin: taffy::geometry::Rect::zero(),
            padding: taffy::geometry::Rect::zero(),
            border: taffy::geometry::Rect::zero(),

            // Grid properties
            grid_template_rows: Vec::new(),
            grid_template_columns: Vec::new(),
            grid_row: geometry::Line::default(),
            grid_column: geometry::Line::default(),

            // Size constraints
            min_width: Size::Auto,
            min_height: Size::Auto,
            max_width: Size::Auto,
            max_height: Size::Auto,

            // Aspect ratio
            aspect_ratio: None,
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
                        "grid" => Display::Grid,
                        _ => style.display,
                    };
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
                "min-width" => {
                    style.min_width = Size::parse(value);
                }
                "min-height" => {
                    style.min_height = Size::parse(value);
                }
                "max-width" => {
                    style.max_width = Size::parse(value);
                }
                "max-height" => {
                    style.max_height = Size::parse(value);
                }
                "aspect-ratio" => {
                    if let Ok(ratio) = value.parse::<f32>() {
                        style.aspect_ratio = Some(ratio);
                    }
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
                // Flex properties
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
                        "flex-start" | "start" => JustifyContent::Start,
                        "flex-end" | "end" => JustifyContent::End,
                        "center" => JustifyContent::Center,
                        "space-between" => JustifyContent::SpaceBetween,
                        "space-around" => JustifyContent::SpaceAround,
                        "space-evenly" => JustifyContent::SpaceEvenly,
                        _ => JustifyContent::Start,
                    });
                }
                "align-items" => {
                    style.align_items = Some(match value.as_str() {
                        "flex-start" | "start" => AlignItems::Start,
                        "flex-end" | "end" => AlignItems::End,
                        "center" => AlignItems::Center,
                        "stretch" => AlignItems::Stretch,
                        _ => AlignItems::Stretch,
                    });
                }
                "align-self" => {
                    style.align_self = Some(match value.as_str() {
                        "flex-start" | "start" => AlignItems::Start,
                        "flex-end" | "end" => AlignItems::End,
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
                "flex-wrap" => {
                    style.flex_wrap = match value.as_str() {
                        "wrap" => taffy::style::FlexWrap::Wrap,
                        "wrap-reverse" => taffy::style::FlexWrap::WrapReverse,
                        "nowrap" => taffy::style::FlexWrap::NoWrap,
                        _ => taffy::style::FlexWrap::NoWrap,
                    };
                }
                "gap" => {
                    let gap_val = parse_length_percentage(value);
                    style.gap = taffy::geometry::Size {
                        width: gap_val,
                        height: gap_val,
                    };
                }
                // Margin properties
                "margin" => {
                    let val = parse_length_percentage_auto(value);
                    style.margin = taffy::geometry::Rect {
                        left: val,
                        right: val,
                        top: val,
                        bottom: val,
                    };
                }
                "margin-left" => style.margin.left = parse_length_percentage_auto(value),
                "margin-right" => style.margin.right = parse_length_percentage_auto(value),
                "margin-top" => style.margin.top = parse_length_percentage_auto(value),
                "margin-bottom" => style.margin.bottom = parse_length_percentage_auto(value),

                // Padding properties
                "padding" => {
                    let val = parse_length_percentage(value);
                    style.padding = taffy::geometry::Rect {
                        left: val,
                        right: val,
                        top: val,
                        bottom: val,
                    };
                }
                "padding-left" => style.padding.left = parse_length_percentage(value),
                "padding-right" => style.padding.right = parse_length_percentage(value),
                "padding-top" => style.padding.top = parse_length_percentage(value),
                "padding-bottom" => style.padding.bottom = parse_length_percentage(value),

                // Border properties
                "border" => {
                    let val = parse_length_percentage(value);
                    style.border = taffy::geometry::Rect {
                        left: val,
                        right: val,
                        top: val,
                        bottom: val,
                    };
                }
                "border-left" | "border-left-width" => {
                    style.border.left = parse_length_percentage(value)
                }
                "border-right" | "border-right-width" => {
                    style.border.right = parse_length_percentage(value)
                }
                "border-top" | "border-top-width" => {
                    style.border.top = parse_length_percentage(value)
                }
                "border-bottom" | "border-bottom-width" => {
                    style.border.bottom = parse_length_percentage(value)
                }

                // Grid properties
                "grid-template-rows" => {
                    style.grid_template_rows = parse_grid_tracks(value);
                }
                "grid-template-columns" => {
                    style.grid_template_columns = parse_grid_tracks(value);
                }
                "grid-row" => {
                    style.grid_row = parse_grid_line(value);
                }
                "grid-column" => {
                    style.grid_column = parse_grid_line(value);
                }

                _ => {} // остальные свойства игнорируем для layout
            }
        }

        style
    }

    /// Конвертирует в Taffy Style для layout-движка
    pub fn to_taffy_style(&self) -> taffy::Style {
        use taffy::style::*;

        let mut style = taffy::Style {
            display: match self.display {
                crate::layout::types::Display::Block => taffy::style::Display::Block,
                crate::layout::types::Display::Flex => taffy::style::Display::Flex,
                crate::layout::types::Display::Grid => taffy::style::Display::Grid,
                crate::layout::types::Display::None => taffy::style::Display::None,
                crate::layout::types::Display::Inline => taffy::style::Display::Block, // Inline как Block в Taffy
            },
            ..Default::default()
        };

        // Размеры
        style.size.width = self.width.to_taffy_dimension();
        style.size.height = self.height.to_taffy_dimension();
        style.min_size.width = self.min_width.to_taffy_dimension();
        style.min_size.height = self.min_height.to_taffy_dimension();
        style.max_size.width = self.max_width.to_taffy_dimension();
        style.max_size.height = self.max_height.to_taffy_dimension();

        // Aspect ratio
        style.aspect_ratio = self.aspect_ratio;

        // Spacing
        style.margin = self.margin;
        style.padding = self.padding;
        style.border = self.border;

        // Flex properties
        if matches!(self.display, crate::layout::types::Display::Flex) {
            style.flex_direction = self.flex_direction;
            style.justify_content = self.justify_content;
            style.align_items = self.align_items;
            style.align_self = self.align_self;
            style.flex_grow = self.flex_grow;
            style.flex_shrink = self.flex_shrink;
            style.flex_wrap = self.flex_wrap;
            style.gap = self.gap;
        } else if matches!(self.display, crate::layout::types::Display::Block) {
            // Для блоковых элементов используем вертикальный layout
            style.flex_direction = FlexDirection::Column;
        }

        // Grid properties - TODO: implement proper grid support
        // if matches!(self.display, crate::layout::types::Display::Grid) {
        //     style.grid_template_rows = self.grid_template_rows.clone();
        //     style.grid_template_columns = self.grid_template_columns.clone();
        //     style.grid_row = self.grid_row;
        //     style.grid_column = self.grid_column;
        // }

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
    Grid,
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

/// Парсит значение в LengthPercentage
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

/// Парсит значение в LengthPercentageAuto
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

/// Парсит grid tracks (например: "1fr 200px auto")
fn parse_grid_tracks(value: &str) -> Vec<taffy::style::TrackSizingFunction> {
    use taffy::style_helpers::*;

    value
        .split_whitespace()
        .filter_map(|track| {
            if let Some(fr_val) = track.strip_suffix("fr") {
                if let Ok(val) = fr_val.parse::<f32>() {
                    Some(fr(val))
                } else {
                    None
                }
            } else if let Some(px_val) = track.strip_suffix("px") {
                if let Ok(val) = px_val.parse::<f32>() {
                    Some(length(val))
                } else {
                    None
                }
            } else if track == "auto" {
                Some(auto())
            } else if let Some(percent_val) = track.strip_suffix('%') {
                if let Ok(val) = percent_val.parse::<f32>() {
                    Some(percent(val / 100.0))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Парсит grid line (например: "1", "span 2", "3 / 5")
fn parse_grid_line(value: &str) -> geometry::Line<taffy::style::GridPlacement> {
    let value = value.trim();

    if let Some(span_part) = value.strip_prefix("span ")
        && let Ok(span) = span_part.trim().parse::<u16>()
    {
        return geometry::Line {
            start: taffy::style::GridPlacement::Span(span),
            end: taffy::style::GridPlacement::Auto,
        };
    }

    if let Ok(line) = value.parse::<i16>() {
        return geometry::Line {
            start: taffy::style::GridPlacement::Line(line.into()),
            end: taffy::style::GridPlacement::Auto,
        };
    }

    geometry::Line::default()
}

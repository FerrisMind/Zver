use crate::dom::Document;
use std::collections::HashMap;
use taffy::prelude::*;
use taffy::style::{AlignItems, Dimension, JustifyContent};

/// Строит Taffy дерево из DOM
pub fn build_taffy_tree(
    taffy: &mut TaffyTree<()>,
    document: &Document,
    node_id: usize,
    styles: &HashMap<usize, HashMap<String, String>>,
) -> NodeId {
    let node_styles = styles.get(&node_id).cloned().unwrap_or_default();

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

    apply_padding(&mut style, &node_styles);
    apply_margin(&mut style, &node_styles);
    apply_flexbox_properties(&mut style, &node_styles);

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

fn apply_padding(style: &mut Style, node_styles: &HashMap<String, String>) {
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
}

fn apply_margin(style: &mut Style, node_styles: &HashMap<String, String>) {
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
}

fn apply_flexbox_properties(style: &mut Style, node_styles: &HashMap<String, String>) {
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

    if let Some(gap) = node_styles.get("gap") {
        let dim = parse_length_percentage(gap);
        style.gap = taffy::geometry::Size {
            width: dim,
            height: dim,
        };
    }
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

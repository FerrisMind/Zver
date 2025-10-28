use super::types::*;
use std::collections::HashMap;

/// Применяет стили по умолчанию для HTML тегов
pub fn apply_default_tag_styles(style: &mut ComputedStyle, tag_name: &Option<String>) {
    if let Some(tag) = tag_name {
        match tag.as_str() {
            // Заголовки
            "h1" => {
                style.font_size = 32.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }
            "h2" => {
                style.font_size = 24.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }
            "h3" => {
                style.font_size = 19.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }
            "h4" => {
                style.font_size = 16.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }
            "h5" => {
                style.font_size = 13.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }
            "h6" => {
                style.font_size = 11.0;
                style.font_weight = FontWeight::Bold;
                style.display = Display::Block;
            }

            // Жирный текст
            "b" | "strong" => {
                style.font_weight = FontWeight::Bold;
                style.display = Display::Inline;
            }

            // Курсив
            "i" | "em" => {
                style.font_style = FontStyle::Italic;
                style.display = Display::Inline;
            }

            // Блочные элементы
            "div" | "p" | "section" | "article" | "header" | "footer" | "nav" | "main" => {
                style.display = Display::Block;
            }

            // Инлайн элементы
            "span" | "a" | "code" => {
                style.display = Display::Inline;
            }

            // Списки
            "ul" | "ol" => {
                style.display = Display::Block;
            }
            "li" => {
                style.display = Display::Block;
                style.list_style_type = ListStyleType::Disc;
            }

            // Скрытые элементы
            "script" | "style" | "meta" | "link" | "title" => {
                style.display = Display::None;
            }

            _ => {}
        }
    }
}

/// Применяет CSS стили к ComputedStyle
pub fn apply_css_styles(style: &mut ComputedStyle, css_styles: &HashMap<String, String>) {
    for (property, value) in css_styles {
        match property.as_str() {
            "display" => {
                style.display = match value.as_str() {
                    "block" => Display::Block,
                    "inline" => Display::Inline,
                    "none" => Display::None,
                    "flex" => Display::Flex,
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
                style.width = parse_size(value);
            }
            "height" => {
                style.height = parse_size(value);
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
            "list-style-type" => {
                style.list_style_type = match value.as_str() {
                    "disc" => ListStyleType::Disc,
                    "circle" => ListStyleType::Circle,
                    "square" => ListStyleType::Square,
                    "decimal" => ListStyleType::Decimal,
                    "none" => ListStyleType::None,
                    _ => style.list_style_type,
                };
            }
            _ => {}
        }
    }
}

/// Парсит размер из CSS значения
fn parse_size(value: &str) -> Size {
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

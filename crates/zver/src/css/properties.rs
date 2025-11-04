//! Парсинг и нормализация CSS-свойств.

use std::collections::HashMap;

use cssparser::ToCss;
use cssparser::{Parser, ParserInput, Token};
use thiserror::Error;

use super::color::{self, Color};

/// Нормализованная декларация CSS (один property/value).
#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub important: bool,
}

/// Представление свойства после применения каскада.
#[derive(Debug, Clone)]
pub struct AppliedProperty {
    pub value: String,
    pub important: bool,
    pub specificity: u32,
    pub order: u64,
}

/// Ошибки парсинга CSS-свойств.
#[derive(Debug, Error, Clone)]
pub enum PropertyParseError {
    #[error("property name is empty")]
    EmptyName,
    #[error("value for `{0}` is empty")]
    EmptyValue(String),
    #[error("too many components for `{0}` shorthand")]
    TooManyComponents(String),
    #[error("invalid value `{1}` for `{0}`")]
    InvalidValue(String, String),
    #[error("unsupported unit `{0}`")]
    UnsupportedUnit(String),
}

/// Парсит значение CSS-свойства и возвращает одно или несколько нормализованных деклараций.
pub fn parse_property(name: &str, raw_value: &str) -> Result<Vec<Property>, PropertyParseError> {
    let property_name = name.trim();
    if property_name.is_empty() {
        return Err(PropertyParseError::EmptyName);
    }

    let name_lower = property_name.to_ascii_lowercase();

    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Err(PropertyParseError::EmptyValue(name_lower));
    }

    let (value_part, important) = split_important(trimmed);
    if value_part.is_empty() {
        return Err(PropertyParseError::EmptyValue(name_lower));
    }

    let mut declarations = match name_lower.as_str() {
        "color" | "background-color" => vec![Property {
            name: name_lower.clone(),
            value: parse_color(value_part)?,
            important: false,
        }],
        "margin" => parse_box_shorthand("margin", value_part, true)?,
        "padding" => parse_box_shorthand("padding", value_part, false)?,
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => vec![Property {
            name: name_lower.clone(),
            value: parse_length_value(value_part, true)?,
            important: false,
        }],
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => vec![Property {
            name: name_lower.clone(),
            value: parse_length_value(value_part, false)?,
            important: false,
        }],
        "width" | "height" => vec![Property {
            name: name_lower.clone(),
            value: parse_length_or_keyword(value_part, true)?,
            important: false,
        }],
        "display" => vec![Property {
            name: name_lower.clone(),
            value: parse_display(value_part)?,
            important: false,
        }],
        _ => vec![Property {
            name: name_lower.clone(),
            value: value_part.to_string(),
            important: false,
        }],
    };

    for declaration in &mut declarations {
        declaration.important |= important;
    }

    Ok(declarations)
}

/// Добавляет свойство в карту каскада с учётом специфичности, порядка и `!important`.
pub fn merge_property(
    cascade: &mut HashMap<String, AppliedProperty>,
    property: &Property,
    specificity: u32,
    order: u64,
) {
    match cascade.get_mut(&property.name) {
        Some(existing) => {
            if should_override(existing, property.important, specificity, order) {
                *existing = AppliedProperty {
                    value: property.value.clone(),
                    important: property.important,
                    specificity,
                    order,
                };
            }
        }
        None => {
            cascade.insert(
                property.name.clone(),
                AppliedProperty {
                    value: property.value.clone(),
                    important: property.important,
                    specificity,
                    order,
                },
            );
        }
    }
}

fn should_override(
    current: &AppliedProperty,
    new_important: bool,
    new_specificity: u32,
    new_order: u64,
) -> bool {
    if current.important != new_important {
        return new_important;
    }

    if current.specificity != new_specificity {
        return new_specificity > current.specificity;
    }

    new_order >= current.order
}

fn split_important(value: &str) -> (&str, bool) {
    let trimmed = value.trim_end();
    if let Some(idx) = trimmed.rfind('!') {
        let (before, after) = trimmed.split_at(idx);
        if after[1..].trim().eq_ignore_ascii_case("important") {
            return (before.trim_end(), true);
        }
    }
    (trimmed, false)
}

fn parse_color(value: &str) -> Result<String, PropertyParseError> {
    match color::parse_css_color(value) {
        Some(color) => Ok(format_color(color)),
        None => Err(PropertyParseError::InvalidValue(
            "color".into(),
            value.into(),
        )),
    }
}

fn format_color(color: Color) -> String {
    let alpha = f32::from(color.a) / 255.0;
    format!(
        "rgba({}, {}, {}, {})",
        color.r,
        color.g,
        color.b,
        format_float(alpha)
    )
}

fn parse_box_shorthand(
    prefix: &str,
    value: &str,
    allow_auto: bool,
) -> Result<Vec<Property>, PropertyParseError> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);

    parser.skip_whitespace();
    if parser.is_exhausted() {
        return Err(PropertyParseError::EmptyValue(prefix.into()));
    }

    let mut parts = Vec::new();
    while !parser.is_exhausted() {
        let part = parse_length_component(&mut parser, allow_auto)?;
        parts.push(part);
        parser.skip_whitespace();
    }

    match parts.len() {
        1 => parts.resize(4, parts[0].clone()),
        2 => {
            parts.push(parts[0].clone());
            parts.push(parts[1].clone());
        }
        3 => parts.push(parts[1].clone()),
        4 => {}
        _ => {
            return Err(PropertyParseError::TooManyComponents(prefix.into()));
        }
    }

    let mut declarations = Vec::with_capacity(4);
    let suffixes = ["-top", "-right", "-bottom", "-left"];
    for (idx, suffix) in suffixes.iter().enumerate() {
        declarations.push(Property {
            name: format!("{prefix}{suffix}"),
            value: parts[idx].clone(),
            important: false,
        });
    }

    Ok(declarations)
}

fn parse_length_or_keyword(value: &str, allow_auto: bool) -> Result<String, PropertyParseError> {
    let lower = value.trim().to_ascii_lowercase();
    if matches!(lower.as_str(), "inherit" | "initial" | "unset") {
        return Ok(lower);
    }
    if allow_auto && lower == "auto" {
        return Ok(lower);
    }
    parse_length_value(value, allow_auto)
}

fn parse_length_value(value: &str, allow_auto: bool) -> Result<String, PropertyParseError> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let parsed = parse_length_component(&mut parser, allow_auto)?;
    parser.skip_whitespace();
    if parser.is_exhausted() {
        Ok(parsed)
    } else {
        Err(PropertyParseError::InvalidValue(
            "length".into(),
            value.into(),
        ))
    }
}

fn parse_length_component(
    parser: &mut Parser<'_, '_>,
    allow_auto: bool,
) -> Result<String, PropertyParseError> {
    match parser.next().cloned() {
        Ok(Token::Dimension { value, unit, .. }) => {
            let unit_lower = unit.as_ref().to_ascii_lowercase();
            match unit_lower.as_str() {
                "px" | "em" | "vh" | "vw" => Ok(format!("{}{}", format_float(value), unit_lower)),
                _ => Err(PropertyParseError::UnsupportedUnit(unit_lower)),
            }
        }
        Ok(Token::Percentage { unit_value, .. }) => {
            let percent = unit_value * 100.0;
            Ok(format!("{}%", format_float(percent)))
        }
        Ok(Token::Number { value, .. }) => {
            if value.abs() <= f32::EPSILON {
                Ok("0".to_string())
            } else {
                Err(PropertyParseError::InvalidValue(
                    "number".into(),
                    format_float(value),
                ))
            }
        }
        Ok(Token::Ident(ident)) => {
            let ident_lower = ident.as_ref().to_ascii_lowercase();
            match ident_lower.as_str() {
                "inherit" | "initial" | "unset" => Ok(ident_lower),
                "auto" if allow_auto => Ok(ident_lower),
                _ => Err(PropertyParseError::InvalidValue(
                    "identifier".into(),
                    ident_lower,
                )),
            }
        }
        Ok(other) => Err(PropertyParseError::InvalidValue(
            "token".into(),
            other.to_css_string(),
        )),
        Err(_) => Err(PropertyParseError::EmptyValue("length".into())),
    }
}

fn parse_display(value: &str) -> Result<String, PropertyParseError> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let ident = match parser.expect_ident() {
        Ok(i) => i.to_ascii_lowercase(),
        Err(_) => {
            return Err(PropertyParseError::InvalidValue(
                "display".into(),
                value.into(),
            ));
        }
    };

    let is_allowed = matches!(
        ident.as_str(),
        "block"
            | "inline"
            | "inline-block"
            | "flex"
            | "inline-flex"
            | "grid"
            | "inline-grid"
            | "contents"
            | "none"
            | "list-item"
            | "table"
            | "table-row"
            | "table-cell"
    );

    if !is_allowed {
        return Err(PropertyParseError::InvalidValue("display".into(), ident));
    }

    parser.skip_whitespace();
    if !parser.is_exhausted() {
        return Err(PropertyParseError::InvalidValue(
            "display".into(),
            value.into(),
        ));
    }

    Ok(ident)
}

fn format_float(value: f32) -> String {
    if (value - value.round()).abs() <= f32::EPSILON {
        (value.round() as i32).to_string()
    } else {
        let mut repr = format!("{value:.3}");
        while repr.contains('.') && repr.ends_with('0') {
            repr.pop();
        }
        if repr.ends_with('.') {
            repr.pop();
        }
        repr
    }
}

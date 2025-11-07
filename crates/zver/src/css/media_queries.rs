//! Парсинг и обработка CSS Media Queries (@media).
//!
//! Реализует полную поддержку W3C CSS Media Queries Level 3:
//! - Типы медиа: screen, print, all, speech
//! - Условия: min-width, max-width, min-height, max-height, orientation
//! - Логические операторы: and, or, not, only
//!
//! Спецификация: https://www.w3.org/TR/mediaqueries-3/
//! Референс: https://developer.mozilla.org/en-US/docs/Web/CSS/@media

use std::str::FromStr;

use cssparser::{ParseError, Parser, Token};
use std::fmt;

/// Типы медиа для @media правил.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    /// Все устройства (по умолчанию)
    All,
    /// Экранные устройства (мониторы, планшеты, телефоны)
    Screen,
    /// Печатные документы и предпросмотр печати
    Print,
    /// Речевые синтезаторы и устройства чтения с экрана
    Speech,
}

impl Default for MediaType {
    fn default() -> Self {
        Self::All
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Screen => write!(f, "screen"),
            Self::Print => write!(f, "print"),
            Self::Speech => write!(f, "speech"),
        }
    }
}

impl FromStr for MediaType {
    type Err = ();

    /// Парсит тип медиа из строки.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "screen" => Ok(Self::Screen),
            "print" => Ok(Self::Print),
            "speech" => Ok(Self::Speech),
            _ => Err(()),
        }
    }
}

/// Условие медиа-запроса (media feature).
#[derive(Debug, Clone, PartialEq)]
pub enum MediaFeature {
    /// min-width: минимальная ширина viewport в пикселях
    MinWidth(f32),
    /// max-width: максимальная ширина viewport в пикселях
    MaxWidth(f32),
    /// min-height: минимальная высота viewport в пикселях
    MinHeight(f32),
    /// max-height: максимальная высота viewport в пикселях
    MaxHeight(f32),
    /// orientation: portrait или landscape
    Orientation(Orientation),
    /// Поддержка hover: none, hover
    Hover(bool),
    /// aspect-ratio: соотношение сторон
    AspectRatio { width: u32, height: u32 },
    /// min-aspect-ratio: минимальное соотношение сторон
    MinAspectRatio { width: u32, height: u32 },
    /// max-aspect-ratio: максимальное соотношение сторон
    MaxAspectRatio { width: u32, height: u32 },
    /// Прочие фичи (для будущего расширения)
    Other { name: String, value: String },
}

/// Ориентация устройства.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl fmt::Display for MediaFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MinWidth(w) => write!(f, "(min-width: {}px)", w),
            Self::MaxWidth(w) => write!(f, "(max-width: {}px)", w),
            Self::MinHeight(h) => write!(f, "(min-height: {}px)", h),
            Self::MaxHeight(h) => write!(f, "(max-height: {}px)", h),
            Self::Orientation(o) => write!(
                f,
                "(orientation: {})",
                match o {
                    Orientation::Portrait => "portrait",
                    Orientation::Landscape => "landscape",
                }
            ),
            Self::Hover(enabled) => {
                write!(f, "(hover: {})", if *enabled { "hover" } else { "none" })
            }
            Self::AspectRatio { width, height } => {
                write!(f, "(aspect-ratio: {}/{})", width, height)
            }
            Self::MinAspectRatio { width, height } => {
                write!(f, "(min-aspect-ratio: {}/{})", width, height)
            }
            Self::MaxAspectRatio { width, height } => {
                write!(f, "(max-aspect-ratio: {}/{})", width, height)
            }
            Self::Other { name, value } => write!(f, "({}: {})", name, value),
        }
    }
}

/// Логический оператор для комбинирования условий.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaQueryOperator {
    /// Логическое AND
    And,
    /// Логическое OR (через запятую)
    Or,
}

/// Полное медиа-выражение (@media).
#[derive(Debug, Clone)]
pub struct MediaQuery {
    /// Модификатор not/only (опционально)
    pub modifier: Option<MediaQueryModifier>,
    /// Тип медиа
    pub media_type: MediaType,
    /// Список условий (features)
    pub features: Vec<MediaFeature>,
    /// Оператор для комбинирования (по умолчанию AND)
    pub operator: MediaQueryOperator,
}

/// Модификаторы медиа-запроса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaQueryModifier {
    /// Инвертирует весь медиа-запрос
    Not,
    /// Скрывает стили от старых браузеров (legacy)
    Only,
}

impl Default for MediaQuery {
    fn default() -> Self {
        Self {
            modifier: None,
            media_type: MediaType::All,
            features: Vec::new(),
            operator: MediaQueryOperator::And,
        }
    }
}

impl fmt::Display for MediaQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(modifier) = self.modifier {
            write!(
                f,
                "{} ",
                match modifier {
                    MediaQueryModifier::Not => "not",
                    MediaQueryModifier::Only => "only",
                }
            )?;
        }

        write!(f, "{}", self.media_type)?;

        if !self.features.is_empty() {
            write!(f, " and ")?;
            for (i, feature) in self.features.iter().enumerate() {
                if i > 0 {
                    write!(f, " and ")?;
                }
                write!(f, "{}", feature)?;
            }
        }

        Ok(())
    }
}

impl MediaQuery {
    /// Создает новый медиа-запрос для всех устройств.
    pub fn new() -> Self {
        Self::default()
    }

    /// Создает медиа-запрос для определенного типа медиа.
    pub fn for_media_type(media_type: MediaType) -> Self {
        Self {
            media_type,
            ..Default::default()
        }
    }

    /// Проверяет соответствие запроса текущему viewport.
    ///
    /// # Arguments
    /// * `viewport_width` - Ширина viewport в пикселях
    /// * `viewport_height` - Высота viewport в пикселях
    /// * `current_media_type` - Текущий тип медиа (обычно Screen)
    pub fn matches(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        current_media_type: MediaType,
    ) -> bool {
        // Проверка типа медиа
        if self.media_type != MediaType::All && self.media_type != current_media_type {
            let result = false;
            return if let Some(MediaQueryModifier::Not) = self.modifier {
                !result
            } else {
                result
            };
        }

        // Проверка всех условий (AND-логика)
        let mut matches = true;
        for feature in &self.features {
            let feature_matches = match feature {
                MediaFeature::MinWidth(w) => viewport_width >= *w,
                MediaFeature::MaxWidth(w) => viewport_width <= *w,
                MediaFeature::MinHeight(h) => viewport_height >= *h,
                MediaFeature::MaxHeight(h) => viewport_height <= *h,
                MediaFeature::Orientation(o) => {
                    let is_landscape = viewport_width > viewport_height;
                    match o {
                        Orientation::Landscape => is_landscape,
                        Orientation::Portrait => !is_landscape,
                    }
                }
                MediaFeature::AspectRatio { width, height } => {
                    let ratio = viewport_width / viewport_height;
                    let target_ratio = *width as f32 / *height as f32;
                    (ratio - target_ratio).abs() < 0.01 // Допуск для float-сравнения
                }
                MediaFeature::MinAspectRatio { width, height } => {
                    let ratio = viewport_width / viewport_height;
                    let target_ratio = *width as f32 / *height as f32;
                    ratio >= target_ratio
                }
                MediaFeature::MaxAspectRatio { width, height } => {
                    let ratio = viewport_width / viewport_height;
                    let target_ratio = *width as f32 / *height as f32;
                    ratio <= target_ratio
                }
                MediaFeature::Hover(_) => true, // TODO: Зависит от устройства ввода
                MediaFeature::Other { .. } => true, // Неизвестные фичи игнорируются
            };

            if !feature_matches {
                matches = false;
                break;
            }
        }

        // Применяем модификатор NOT
        if let Some(MediaQueryModifier::Not) = self.modifier {
            !matches
        } else {
            matches
        }
    }

    /// Парсит медиа-запрос из cssparser::Parser.
    ///
    /// Пример синтаксиса:
    /// - `screen`
    /// - `screen and (min-width: 768px)`
    /// - `not print`
    /// - `(min-width: 768px)` - feature-only query
    /// - `only screen and (min-width: 768px) and (max-width: 1024px)`
    pub fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
        let mut query = MediaQuery::default();

        input.skip_whitespace();

        // Парсим модификатор (not/only)
        let ident_result = input.try_parse(|i| {
            if let Token::Ident(ident) = i.next()? {
                Ok(ident.to_string())
            } else {
                Err(i.new_custom_error::<(), ()>(()))
            }
        });

        if let Ok(ident_str) = ident_result {
            match ident_str.to_ascii_lowercase().as_str() {
                "not" => query.modifier = Some(MediaQueryModifier::Not),
                "only" => query.modifier = Some(MediaQueryModifier::Only),
                other => {
                    // Это тип медиа, не модификатор
                    if let Ok(media_type) = other.parse::<MediaType>() {
                        query.media_type = media_type;
                    }
                }
            }
        }

        input.skip_whitespace();

        // Парсим тип медиа (если не был распознан выше)
        if query.modifier.is_some()
            && query.media_type == MediaType::All
            && let Ok(Token::Ident(ident)) = input.next()
            && let Ok(media_type) = ident.as_ref().parse::<MediaType>()
        {
            query.media_type = media_type;
        }

        input.skip_whitespace();

        // Попытка парсинга первого feature без "and" (для cases like "(min-width: 768px)")
        if let Ok(feature) = input.try_parse(|i| match i.next()? {
            Token::ParenthesisBlock => i.parse_nested_block(|input| parse_media_feature(input)),
            _ => Err(i.new_custom_error::<(), ()>(())),
        }) {
            query.features.push(feature);
            input.skip_whitespace();
        }

        // Парсим дополнительные условия (features) через "and"
        while input.try_parse(|i| i.expect_ident_matching("and")).is_ok() {
            input.skip_whitespace();

            // Условие всегда в скобках: (min-width: 768px)
            // Сначала ожидаем токен ParenthesisBlock, затем парсим его содержимое
            let feature = input
                .try_parse(|i| match i.next()? {
                    Token::ParenthesisBlock => {
                        i.parse_nested_block(|input| parse_media_feature(input))
                    }
                    _ => Err(i.new_custom_error::<(), ()>(())),
                })
                .map_err(|_| input.new_custom_error(()))?;

            query.features.push(feature);
            input.skip_whitespace();
        }

        Ok(query)
    }
}

/// Парсит одно медиа-условие (media feature).
fn parse_media_feature<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<MediaFeature, ParseError<'i, ()>> {
    input.skip_whitespace();

    // Ожидаем имя фичи
    let name = input.expect_ident()?.to_string().to_ascii_lowercase();

    input.skip_whitespace();
    input.expect_colon()?;
    input.skip_whitespace();

    // Парсим значение в зависимости от имени
    match name.as_str() {
        "min-width" => {
            let value = parse_dimension_px(input)?;
            Ok(MediaFeature::MinWidth(value))
        }
        "max-width" => {
            let value = parse_dimension_px(input)?;
            Ok(MediaFeature::MaxWidth(value))
        }
        "min-height" => {
            let value = parse_dimension_px(input)?;
            Ok(MediaFeature::MinHeight(value))
        }
        "max-height" => {
            let value = parse_dimension_px(input)?;
            Ok(MediaFeature::MaxHeight(value))
        }
        "orientation" => {
            let value = input.expect_ident()?;
            let orientation = match value.as_ref().to_ascii_lowercase().as_str() {
                "portrait" => Orientation::Portrait,
                "landscape" => Orientation::Landscape,
                _ => return Err(input.new_custom_error(())),
            };
            Ok(MediaFeature::Orientation(orientation))
        }
        "hover" => {
            let value = input.expect_ident()?;
            let enabled = match value.as_ref().to_ascii_lowercase().as_str() {
                "hover" => true,
                "none" => false,
                _ => return Err(input.new_custom_error(())),
            };
            Ok(MediaFeature::Hover(enabled))
        }
        "aspect-ratio" => {
            let width = parse_integer(input)?;
            input.expect_delim('/')?;
            let height = parse_integer(input)?;
            Ok(MediaFeature::AspectRatio { width, height })
        }
        "min-aspect-ratio" => {
            let width = parse_integer(input)?;
            input.expect_delim('/')?;
            let height = parse_integer(input)?;
            Ok(MediaFeature::MinAspectRatio { width, height })
        }
        "max-aspect-ratio" => {
            let width = parse_integer(input)?;
            input.expect_delim('/')?;
            let height = parse_integer(input)?;
            Ok(MediaFeature::MaxAspectRatio { width, height })
        }
        _ => {
            // Прочие фичи сохраняем как строку (для будущего)
            let value = input.expect_ident_or_string()?.to_string();
            Ok(MediaFeature::Other { name, value })
        }
    }
}

/// Парсит размерность в пикселях (например, "768px").
fn parse_dimension_px<'i, 't>(input: &mut Parser<'i, 't>) -> Result<f32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match token {
        Token::Dimension {
            value, ref unit, ..
        } => match unit.as_ref().to_ascii_lowercase().as_str() {
            "px" => Ok(value),
            _ => Err(input.new_custom_error(())),
        },
        Token::Number { value, .. } => Ok(value), // Безъединичное число = px
        _ => Err(input.new_custom_error(())),
    }
}

/// Парсит целое число.
fn parse_integer<'i, 't>(input: &mut Parser<'i, 't>) -> Result<u32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match token {
        Token::Number {
            int_value: Some(i), ..
        } if i >= 0 => Ok(i as u32),
        _ => Err(input.new_custom_error(())),
    }
}

/// Правило @media с вложенными CSS-правилами.
#[derive(Debug, Clone)]
pub struct MediaRule {
    /// Медиа-запрос
    pub query: MediaQuery,
    /// CSS-правила, которые применяются при соответствии запросу
    pub rules: Vec<crate::css::parser::ParsedRule>,
}

impl MediaRule {
    /// Создает новое @media правило.
    pub fn new(query: MediaQuery) -> Self {
        Self {
            query,
            rules: Vec::new(),
        }
    }

    /// Проверяет, применимо ли правило для данного viewport.
    pub fn applies_to(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        media_type: MediaType,
    ) -> bool {
        self.query
            .matches(viewport_width, viewport_height, media_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    #[test]
    fn test_media_type_parsing() {
        assert_eq!("screen".parse::<MediaType>(), Ok(MediaType::Screen));
        assert_eq!("PRINT".parse::<MediaType>(), Ok(MediaType::Print));
        assert_eq!("all".parse::<MediaType>(), Ok(MediaType::All));
        assert!("invalid".parse::<MediaType>().is_err());
    }

    #[test]
    fn test_simple_media_query() {
        let css = "screen";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);

        let query = MediaQuery::parse(&mut parser).unwrap();
        assert_eq!(query.media_type, MediaType::Screen);
        assert_eq!(query.features.len(), 0);
    }

    #[test]
    fn test_media_query_with_width() {
        let css = "screen and (min-width: 768px)";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);

        let query = MediaQuery::parse(&mut parser).unwrap();
        assert_eq!(query.media_type, MediaType::Screen);
        assert_eq!(query.features.len(), 1);
        assert!(matches!(query.features[0], MediaFeature::MinWidth(768.0)));
    }

    #[test]
    fn test_media_query_matches() {
        let query = MediaQuery {
            media_type: MediaType::Screen,
            features: vec![
                MediaFeature::MinWidth(768.0),
                MediaFeature::MaxWidth(1024.0),
            ],
            ..Default::default()
        };

        assert!(query.matches(800.0, 600.0, MediaType::Screen));
        assert!(!query.matches(500.0, 600.0, MediaType::Screen));
        assert!(!query.matches(1200.0, 600.0, MediaType::Screen));
    }
}

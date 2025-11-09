//! Парсинг и загрузка веб-шрифтов (@font-face).
//!
//! Реализует поддержку W3C CSS Fonts Module Level 3:
//! - @font-face объявления
//! - Загрузка TTF/WOFF/WOFF2 шрифтов
//! - Интеграция с fontdue для растеризации
//! - Кеширование загруженных шрифтов
//!
//! Спецификация: https://www.w3.org/TR/css-fonts-3/
//! Референс: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face

use std::str::FromStr;

use cssparser::{ParseError, Parser, Token};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Формат файла шрифта.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// TrueType Font (.ttf)
    TrueType,
    /// OpenType Font (.otf)
    OpenType,
    /// Web Open Font Format (.woff)
    Woff,
    /// Web Open Font Format 2.0 (.woff2)
    Woff2,
    /// Embedded OpenType (.eot) - legacy IE
    Eot,
    /// SVG Font (.svg) - deprecated
    Svg,
}

impl fmt::Display for FontFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrueType => write!(f, "truetype"),
            Self::OpenType => write!(f, "opentype"),
            Self::Woff => write!(f, "woff"),
            Self::Woff2 => write!(f, "woff2"),
            Self::Eot => write!(f, "embedded-opentype"),
            Self::Svg => write!(f, "svg"),
        }
    }
}

impl FontFormat {
    /// Определяет формат по расширению файла.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "ttf" => Some(Self::TrueType),
            "otf" => Some(Self::OpenType),
            "woff" => Some(Self::Woff),
            "woff2" => Some(Self::Woff2),
            "eot" => Some(Self::Eot),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    /// Проверяет, поддерживается ли формат fontdue.
    pub fn is_supported_by_fontdue(&self) -> bool {
        matches!(self, Self::TrueType | Self::OpenType)
    }
}

/// Источник шрифта (src дескриптор).
#[derive(Debug, Clone)]
pub enum FontSource {
    /// URL для загрузки шрифта
    Url {
        url: String,
        format: Option<FontFormat>,
    },
    /// Локальное имя шрифта (local("Arial"))
    Local { name: String },
}

impl fmt::Display for FontSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url { url, format } => {
                write!(f, "url('{}')", url)?;
                if let Some(fmt) = format {
                    write!(f, " format('{}')", fmt)?;
                }
                Ok(())
            }
            Self::Local { name } => write!(f, "local('{}')", name),
        }
    }
}

/// Вес шрифта (font-weight).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    /// Нормальный (400)
    Normal,
    /// Жирный (700)
    Bold,
    /// Конкретное числовое значение (100-900)
    Weight(u16),
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "400"),
            Self::Bold => write!(f, "700"),
            Self::Weight(w) => write!(f, "{}", w),
        }
    }
}

impl FontWeight {
    /// Конвертирует в числовое значение.
    pub fn to_numeric(&self) -> u16 {
        match self {
            Self::Normal => 400,
            Self::Bold => 700,
            Self::Weight(w) => *w,
        }
    }
}

impl FromStr for FontWeight {
    type Err = ();

    /// Парсит font-weight из строки.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            _ => s
                .parse::<u16>()
                .ok()
                .filter(|w| (100..=900).contains(w))
                .map(Self::Weight)
                .ok_or(()),
        }
    }
}

/// Стиль шрифта (font-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    /// Обычный стиль
    Normal,
    /// Курсив
    Italic,
    /// Наклонный (oblique)
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Italic => write!(f, "italic"),
            Self::Oblique => write!(f, "oblique"),
        }
    }
}

impl FromStr for FontStyle {
    type Err = ();

    /// Парсит font-style из строки.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Ok(Self::Normal),
            "italic" => Ok(Self::Italic),
            "oblique" => Ok(Self::Oblique),
            _ => Err(()),
        }
    }
}

/// Unicode-диапазон для subset шрифта.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnicodeRange {
    /// Начальная кодовая точка
    pub start: u32,
    /// Конечная кодовая точка
    pub end: u32,
}

impl UnicodeRange {
    /// Создает новый диапазон.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Проверяет, входит ли символ в диапазон.
    pub fn contains(&self, codepoint: u32) -> bool {
        codepoint >= self.start && codepoint <= self.end
    }
}

/// Определение @font-face.
///
/// Пример:
/// ```css
/// @font-face {
///   font-family: 'MyFont';
///   src: url('myfont.woff2') format('woff2'),
///        url('myfont.ttf') format('truetype');
///   font-weight: 400;
///   font-style: normal;
///   unicode-range: U+0000-00FF;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FontFace {
    /// Имя семейства шрифтов (font-family)
    pub family: String,
    /// Список источников (src) в порядке приоритета
    pub sources: Vec<FontSource>,
    /// Вес шрифта
    pub weight: FontWeight,
    /// Стиль шрифта
    pub style: FontStyle,
    /// Unicode-диапазоны (опционально)
    pub unicode_ranges: Vec<UnicodeRange>,
    /// Дескрипторы font-variant, font-stretch и т.д. (для будущего)
    pub descriptors: HashMap<String, String>,
}

impl FontFace {
    /// Создает новое определение @font-face.
    pub fn new(family: String) -> Self {
        Self {
            family,
            sources: Vec::new(),
            weight: FontWeight::default(),
            style: FontStyle::default(),
            unicode_ranges: Vec::new(),
            descriptors: HashMap::new(),
        }
    }

    /// Добавляет источник шрифта.
    pub fn add_source(&mut self, source: FontSource) {
        self.sources.push(source);
    }

    /// Проверяет, подходит ли шрифт для заданных параметров.
    pub fn matches(&self, family: &str, weight: FontWeight, style: FontStyle) -> bool {
        self.family.eq_ignore_ascii_case(family) && self.weight == weight && self.style == style
    }
    /// Парсит @font-face декларации из cssparser::Parser.
    pub fn parse_font_face_block<'i, 't>(
        input: &mut Parser<'i, 't>,
    ) -> Result<Self, ParseError<'i, ()>> {
        tracing::trace!("parse_font_face_block: начало парсинга");
        let mut font_face = None;
        let mut sources = Vec::new();
        let mut weight = FontWeight::default();
        let mut style = FontStyle::default();
        let mut unicode_ranges = Vec::new();
        let mut descriptors = HashMap::new();

        // Парсим декларации внутри @font-face
        while !input.is_exhausted() {
            input.skip_whitespace();

            // Парсим имя свойства
            tracing::trace!("parse_font_face_block: читаем следующий токен");
            let name = match input.next() {
                Ok(Token::Ident(ident)) => {
                    let n = ident.to_string().to_ascii_lowercase();
                    tracing::trace!("parse_font_face_block: нашли свойство '{}'", n);
                    n
                }
                Ok(other) => {
                    tracing::trace!("parse_font_face_block: пропускаем токен {:?}", other);
                    continue;
                }
                Err(e) => {
                    tracing::trace!("parse_font_face_block: ошибка чтения токена: {:?}", e);
                    break;
                }
            };

            input.skip_whitespace();

            // Ожидаем двоеточие
            if input.expect_colon().is_err() {
                continue;
            }

            input.skip_whitespace();

            // Парсим значение в зависимости от свойства
            match name.as_str() {
                "font-family" => {
                    // Парсим имя семейства (строка или идентификатор)
                    if let Ok(token) = input.next() {
                        let family_name = match token {
                            Token::QuotedString(s) => s.to_string(),
                            Token::Ident(s) => s.to_string(),
                            _ => continue,
                        };
                        font_face = Some(Self::new(family_name));
                    }
                }
                "src" => {
                    // Парсим список источников через запятую
                    tracing::trace!("parse_font_face_block: парсим src");
                    match parse_font_sources(input) {
                        Ok(s) => {
                            sources = s;
                            tracing::trace!(
                                "parse_font_face_block: src успешно распарсен, найдено {} источников",
                                sources.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("parse_font_face_block: ошибка парсинга src: {:?}", e);
                            return Err(e);
                        }
                    }
                }
                "font-weight" => {
                    if let Ok(token) = input.next() {
                        let weight_str = match token {
                            Token::Ident(s) => s.to_string(),
                            Token::Number {
                                int_value: Some(i), ..
                            } => i.to_string(),
                            _ => continue,
                        };
                        if let Ok(w) = weight_str.parse::<FontWeight>() {
                            weight = w;
                        }
                    }
                }
                "font-style" => {
                    if let Ok(Token::Ident(s)) = input.next()
                        && let Ok(st) = s.as_ref().parse::<FontStyle>()
                    {
                        style = st;
                    }
                }
                "unicode-range" => {
                    // Парсим Unicode-диапазоны
                    tracing::trace!("parse_font_face_block: парсим unicode-range");
                    match parse_unicode_ranges(input) {
                        Ok(ranges) => {
                            unicode_ranges = ranges;
                            tracing::trace!(
                                "parse_font_face_block: unicode-range успешно распарсен, найдено {} диапазонов",
                                unicode_ranges.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "parse_font_face_block: ошибка парсинга unicode-range: {:?}",
                                e
                            );
                            return Err(e);
                        }
                    }
                }
                other => {
                    // Прочие дескрипторы сохраняем как строки с правильной сериализацией
                    let value = super::serializer::serialize_value_tokens(input, true)
                        .unwrap_or_else(|_| String::new());
                    if !value.is_empty() {
                        descriptors.insert(other.to_string(), value);
                    }
                }
            }

            // Пропускаем точку с запятой
            let _ = input.try_parse(|i| i.expect_semicolon());
            input.skip_whitespace();
        }

        // Создаем FontFace, если font-family был указан
        if let Some(mut face) = font_face {
            face.sources = sources;
            face.weight = weight;
            face.style = style;
            face.unicode_ranges = unicode_ranges;
            face.descriptors = descriptors;
            Ok(face)
        } else {
            Err(input.new_custom_error(()))
        }
    }
}

/// Загруженный шрифт с данными fontdue.
#[derive(Clone)]
pub struct LoadedFont {
    /// Метаданные @font-face
    pub face: FontFace,
    /// Данные fontdue Font (обернутые в Arc для клонирования)
    pub fontdue_font: Option<Arc<fontdue::Font>>,
}

impl std::fmt::Debug for LoadedFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedFont")
            .field("face", &self.face)
            .field("fontdue_font", &self.fontdue_font.is_some())
            .finish()
    }
}

impl LoadedFont {
    /// Создает новый LoadedFont без данных (будет загружен позже).
    pub fn new(face: FontFace) -> Self {
        Self {
            face,
            fontdue_font: None,
        }
    }

    /// Загружает шрифт из байтов (TTF/OTF).
    pub fn load_from_bytes(&mut self, data: &[u8]) -> Result<(), String> {
        match fontdue::Font::from_bytes(data, fontdue::FontSettings::default()) {
            Ok(font) => {
                self.fontdue_font = Some(Arc::new(font));
                Ok(())
            }
            Err(e) => Err(format!("Failed to load font: {}", e)),
        }
    }

    /// Проверяет, загружен ли шрифт.
    pub fn is_loaded(&self) -> bool {
        self.fontdue_font.is_some()
    }
}

/// Парсит список источников шрифтов (src дескриптор).
fn parse_font_sources<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Vec<FontSource>, ParseError<'i, ()>> {
    let mut sources = Vec::new();

    loop {
        input.skip_whitespace();

        // Пробуем распарсить один источник
        let source: Result<FontSource, ParseError<'i, ()>> = input.try_parse(|input| {
            // Пробуем распарсить как url() или local()
            tracing::trace!("parse_font_sources: ожидаем url() или local()");
            let function_name = input.expect_function().map(|s| s.to_string())?;
            tracing::trace!("parse_font_sources: нашли функцию '{}'", function_name);

            let font_source =
                input.parse_nested_block(|input| -> Result<FontSource, ParseError<'i, ()>> {
                    match function_name.to_ascii_lowercase().as_str() {
                        "url" => {
                            let url = match input.next()? {
                                Token::QuotedString(s) => s.to_string(),
                                Token::UnquotedUrl(s) => s.to_string(),
                                _ => return Err(input.new_custom_error(())),
                            };

                            // НЕ парсим format() здесь - он снаружи url()
                            Ok(FontSource::Url { url, format: None })
                        }
                        "local" => {
                            let name = match input.next()? {
                                Token::QuotedString(s) => s.to_string(),
                                Token::Ident(s) => s.to_string(),
                                _ => return Err(input.new_custom_error(())),
                            };
                            Ok(FontSource::Local { name })
                        }
                        _ => Err(input.new_custom_error(())),
                    }
                })?;

            // Теперь парсим format() СНАРУЖИ url()
            input.skip_whitespace();
            let format = if input
                .try_parse(|i| i.expect_function_matching("format"))
                .is_ok()
            {
                input.parse_nested_block(
                    |input| -> Result<Option<FontFormat>, ParseError<'i, ()>> {
                        if let Ok(Token::QuotedString(s)) = input.next() {
                            Ok(match s.as_ref().to_ascii_lowercase().as_str() {
                                "truetype" | "ttf" => Some(FontFormat::TrueType),
                                "opentype" | "otf" => Some(FontFormat::OpenType),
                                "woff" => Some(FontFormat::Woff),
                                "woff2" => Some(FontFormat::Woff2),
                                "embedded-opentype" | "eot" => Some(FontFormat::Eot),
                                "svg" => Some(FontFormat::Svg),
                                _ => None,
                            })
                        } else {
                            Ok(None)
                        }
                    },
                )?
            } else {
                None
            };

            // Обновляем format в font_source если это Url
            let final_source = match font_source {
                FontSource::Url { url, .. } => {
                    tracing::trace!(
                        "parse_font_sources: создали Url источник с format={:?}",
                        format
                    );
                    FontSource::Url { url, format }
                }
                other => {
                    tracing::trace!("parse_font_sources: создали Local источник");
                    other
                }
            };

            tracing::trace!("parse_font_sources: успешно распарсили источник");
            Ok(final_source)
        });

        match source {
            Ok(src) => {
                tracing::trace!("parse_font_sources: добавили источник в список");
                sources.push(src);

                // Проверяем, есть ли запятая после этого источника
                input.skip_whitespace();
                if input.try_parse(|i| i.expect_comma()).is_err() {
                    tracing::trace!("parse_font_sources: запятой нет, заканчиваем");
                    // Нет запятой - заканчиваем
                    break;
                } else {
                    tracing::trace!("parse_font_sources: нашли запятую, продолжаем");
                }
            }
            Err(e) if sources.is_empty() => {
                // Не удалось распарсить ни одного источника
                tracing::error!(
                    "parse_font_sources: не удалось распарсить ни одного источника: {:?}",
                    e
                );
                return Err(input.new_custom_error(()));
            }
            Err(e) => {
                // Не удалось распарсить следующий источник, но уже есть предыдущие
                tracing::trace!(
                    "parse_font_sources: не удалось распарсить следующий источник (уже есть {}): {:?}",
                    sources.len(),
                    e
                );
                break;
            }
        }
    }

    if sources.is_empty() {
        tracing::error!("parse_font_sources: список источников пуст");
        Err(input.new_custom_error(()))
    } else {
        tracing::trace!(
            "parse_font_sources: успешно, всего источников: {}",
            sources.len()
        );
        Ok(sources)
    }
}

/// Парсит Unicode-диапазоны (unicode-range дескриптор).
fn parse_unicode_ranges<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Vec<UnicodeRange>, ParseError<'i, ()>> {
    let mut ranges = Vec::new();

    loop {
        input.skip_whitespace();

        // Try to parse one unicode-range
        let range: Result<UnicodeRange, ParseError<'i, ()>> = input.try_parse(|input| {
            // Read 'U' identifier
            let first_token = input.next();

            match first_token {
                Ok(Token::Ident(u)) if u.as_ref().eq_ignore_ascii_case("U") => {
                    // Found 'U' prefix
                }
                _ => {
                    return Err(input.new_custom_error::<(), ()>(()));
                }
            }

            // Read +XXXX (Number with + sign)
            let next_token = input.next();

            let start = match next_token {
                Ok(Token::Number {
                    has_sign: true,
                    int_value: Some(n),
                    ..
                }) if *n >= 0 => *n as u32,
                _ => {
                    return Err(input.new_custom_error::<(), ()>(()));
                }
            };

            // Check for range: '-YYYY'
            let dash_token = input.next();

            if let Ok(Token::Number {
                has_sign: true,
                int_value: Some(n),
                ..
            }) = dash_token
            {
                // Range: U+XXXX-YYYY (tokenized as U, +XXXX, -YYYY)
                let end: i32 = if *n < 0 {
                    (*n).unsigned_abs() as i32
                } else {
                    *n
                };
                Ok(UnicodeRange::new(start, end as u32))
            } else {
                // Single codepoint: U+XXXX
                Ok(UnicodeRange::new(start, start))
            }
        });

        match range {
            Ok(r) => {
                ranges.push(r);

                // Check for comma separator
                input.skip_whitespace();
                if input.try_parse(|i| i.expect_comma()).is_err() {
                    break;
                }
            }
            Err(_e) if ranges.is_empty() => {
                return Err(input.new_custom_error::<(), ()>(()));
            }
            Err(_) => {
                break;
            }
        }
    }

    if ranges.is_empty() {
        Err(input.new_custom_error::<(), ()>(()))
    } else {
        Ok(ranges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_from_extension() {
        assert_eq!(
            FontFormat::from_extension("ttf"),
            Some(FontFormat::TrueType)
        );
        assert_eq!(FontFormat::from_extension("WOFF2"), Some(FontFormat::Woff2));
        assert_eq!(FontFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_font_weight_parsing() {
        assert_eq!("normal".parse::<FontWeight>(), Ok(FontWeight::Normal));
        assert_eq!("bold".parse::<FontWeight>(), Ok(FontWeight::Bold));
        assert_eq!("400".parse::<FontWeight>(), Ok(FontWeight::Weight(400)));
        assert!("1000".parse::<FontWeight>().is_err()); // Out of range
    }

    #[test]
    fn test_font_style_parsing() {
        assert_eq!("normal".parse::<FontStyle>(), Ok(FontStyle::Normal));
        assert_eq!("ITALIC".parse::<FontStyle>(), Ok(FontStyle::Italic));
        assert_eq!("oblique".parse::<FontStyle>(), Ok(FontStyle::Oblique));
    }

    #[test]
    fn test_unicode_range() {
        let range = UnicodeRange::new(0x0000, 0x00FF);
        assert!(range.contains(0x0050));
        assert!(range.contains(0x0000));
        assert!(range.contains(0x00FF));
        assert!(!range.contains(0x0100));
    }
}

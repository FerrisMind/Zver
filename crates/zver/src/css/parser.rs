//! Инфраструктура строгого парсинга CSS.
//!
//! Модуль оборачивает `cssparser`, обеспечивая:
//! - Преобразование к типу [`ParsedRule`];
//! - Фоллбек на упрощённый синтаксис для обратной совместимости;
//! - Парсинг inline-стилей через те же функции;
//! - Поддержку @-правил: @media, @keyframes, @font-face (Фаза 2).

use std::fmt;

use cssparser::ToCss;
use cssparser::{CowRcStr, Parser, ParserInput, ParserState, StyleSheetParser, Token};
use thiserror::Error;

use super::animations::KeyframesDefinition;
use super::fonts::FontFace;
use super::media_queries::{MediaQuery, MediaRule};
use super::properties::{self, Property, PropertyParseError};
use super::selectors::{self, SelectorListHandle};

/// Настройки парсинга стилей.
#[derive(Debug, Clone, Copy)]
pub struct CssParseOptions {
    /// Разрешать ли продолжать парсинг после ошибок (по умолчанию — `true`).
    pub recover_from_errors: bool,
}

impl Default for CssParseOptions {
    fn default() -> Self {
        Self {
            recover_from_errors: true,
        }
    }
}

/// Высокоуровневая ошибка парсинга CSS.
#[derive(Debug, Error, Clone)]
pub enum CssParseError {
    #[error("CSS parse error: {0}")]
    Syntax(String),
}

impl CssParseError {
    fn from_messages(errors: Vec<String>) -> Self {
        CssParseError::Syntax(errors.join("\n"))
    }
}

/// Результат парсинга одного CSS-правила.
#[derive(Debug, Clone)]
pub struct ParsedRule {
    pub selector_text: String,
    pub selector_list: SelectorListHandle,
    pub declarations: Vec<Property>,
}

/// Распарсенное @-правило (Фаза 2).
#[derive(Debug, Clone)]
pub enum ParsedAtRule {
    /// @media правило с вложенными CSS-правилами
    Media(MediaRule),
    /// @keyframes определение анимации
    Keyframes(KeyframesDefinition),
    /// @font-face определение шрифта
    FontFace(FontFace),
}

/// Общий тип для обычных правил и @-правил.
#[derive(Debug, Clone)]
pub enum CssRule {
    /// Обычное CSS-правило (selector + declarations)
    Style(ParsedRule),
    /// @-правило
    AtRule(ParsedAtRule),
}

/// Результат парсинга таблицы стилей (Фаза 2).
#[derive(Debug, Clone, Default)]
pub struct ParsedStylesheet {
    /// Обычные CSS-правила
    pub rules: Vec<ParsedRule>,
    /// @media правила
    pub media_rules: Vec<MediaRule>,
    /// @keyframes определения
    pub keyframes: Vec<KeyframesDefinition>,
    /// @font-face определения
    pub font_faces: Vec<FontFace>,
}

/// Высокоуровневый обёртчик `cssparser::StyleSheetParser`.
/// Высокоуровневый обёртчик `cssparser::StyleSheetParser`.
#[derive(Debug, Clone)]
pub struct StylesheetParser {
    options: CssParseOptions,
}

impl StylesheetParser {
    pub fn new(options: CssParseOptions) -> Self {
        Self { options }
    }

    /// Парсит таблицу стилей, возвращая ParsedStylesheet с разделением на обычные правила и @-правила (Фаза 2).
    pub fn parse_stylesheet(&mut self, css: &str) -> Result<ParsedStylesheet, CssParseError> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let mut rule_parser = RuleCollector::new(self.options);

        let mut stylesheet = StyleSheetParser::new(&mut parser, &mut rule_parser);

        let mut result = ParsedStylesheet::default();
        let mut errors = Vec::new();

        for item_result in &mut stylesheet {
            match item_result {
                Ok(css_rule) => match css_rule {
                    CssRule::Style(rule) => result.rules.push(rule),
                    CssRule::AtRule(at_rule) => match at_rule {
                        ParsedAtRule::Media(media) => result.media_rules.push(media),
                        ParsedAtRule::Keyframes(keyframes) => result.keyframes.push(keyframes),
                        ParsedAtRule::FontFace(font_face) => result.font_faces.push(font_face),
                    },
                },
                Err((err, slice)) => {
                    let message = format!("{} (near `{}`)", err, slice.trim());
                    if self.options.recover_from_errors {
                        errors.push(message);
                    } else {
                        return Err(CssParseError::Syntax(message));
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(CssParseError::from_messages(errors));
        }

        Ok(result)
    }

    /// Фоллбек-парсер с упрощённым синтаксисом (историческое поведение движка).
    pub fn parse_with_fallback(&self, css: &str) -> Vec<ParsedRule> {
        css.split('}')
            .filter_map(|chunk| {
                let chunk = chunk.trim();
                if chunk.is_empty() {
                    return None;
                }

                let (selectors_part, declarations_part) = chunk.split_once('{')?;
                let selector_text = selectors_part.trim();
                if selector_text.is_empty() {
                    return None;
                }

                let selector_list = selectors::compile_selector_list(selector_text).ok()?;
                let declarations =
                    parse_declarations_from_str(declarations_part).unwrap_or_default();

                Some(ParsedRule {
                    selector_text: selector_text.to_string(),
                    selector_list,
                    declarations,
                })
            })
            .collect()
    }
}

/// Парсит inline-стили (значение атрибута `style`).
pub fn parse_inline_declarations(inline: &str) -> Result<Vec<Property>, CssParseError> {
    parse_declarations_from_str(inline)
        .map_err(|err| CssParseError::Syntax(format!("inline style error: {}", err)))
}

/// Вспомогательный коллекционер правил для `StyleSheetParser`.
struct RuleCollector {
    _options: CssParseOptions,
}

impl RuleCollector {
    fn new(options: CssParseOptions) -> Self {
        Self { _options: options }
    }
}

#[derive(Debug, Clone)]
enum RuleParseError {
    EmptySelector,
    InvalidSelector(String),
    InvalidDeclaration(String),
    UnsupportedAtRule(String),
}

impl fmt::Display for RuleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleParseError::EmptySelector => write!(f, "selector cannot be empty"),
            RuleParseError::InvalidSelector(reason) => write!(f, "{reason}"),
            RuleParseError::InvalidDeclaration(reason) => write!(f, "{reason}"),
            RuleParseError::UnsupportedAtRule(reason) => write!(f, "{reason}"),
        }
    }
}

impl<'i> cssparser::QualifiedRuleParser<'i> for RuleCollector {
    type Prelude = (String, SelectorListHandle);
    type QualifiedRule = CssRule;
    type Error = RuleParseError;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        let mut buffer = String::new();
        while let Ok(token) = input.next_including_whitespace_and_comments() {
            match token {
                Token::WhiteSpace(_) => {
                    if !buffer.ends_with(' ') && !buffer.is_empty() {
                        buffer.push(' ');
                    }
                }
                Token::Comment(_) => {}
                Token::Function(name) => {
                    // Функциональный токен: нужно парсить содержимое
                    buffer.push_str(name.as_ref());
                    buffer.push('(');

                    // Парсим содержимое функции
                    let content_result = input.parse_nested_block(|inner_input| {
                        let mut inner_buffer = String::new();
                        while let Ok(inner_token) =
                            inner_input.next_including_whitespace_and_comments()
                        {
                            match inner_token {
                                Token::WhiteSpace(_) => {
                                    if !inner_buffer.ends_with(' ') && !inner_buffer.is_empty() {
                                        inner_buffer.push(' ');
                                    }
                                }
                                Token::Comment(_) => {}
                                _ => {
                                    inner_buffer.push_str(&inner_token.to_css_string());
                                }
                            }
                        }
                        Ok::<_, cssparser::ParseError<'i, Self::Error>>(inner_buffer)
                    });

                    match content_result {
                        Ok(content) => {
                            buffer.push_str(&content);
                            buffer.push(')');
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                _ => {
                    buffer.push_str(&token.to_css_string());
                }
            }
        }

        let selector_text = buffer.trim().to_string();
        if selector_text.is_empty() {
            return Err(input.new_custom_error(RuleParseError::EmptySelector));
        }

        match selectors::compile_selector_list(&selector_text) {
            Ok(list) => Ok((selector_text, list)),
            Err(err) => {
                Err(input.new_custom_error(RuleParseError::InvalidSelector(err.to_string())))
            }
        }
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, cssparser::ParseError<'i, Self::Error>> {
        let (selector_text, selector_list) = prelude;

        match parse_declarations_from_parser(input) {
            Ok(declarations) => Ok(CssRule::Style(ParsedRule {
                selector_text,
                selector_list,
                declarations,
            })),
            Err(err) => {
                Err(input.new_custom_error(RuleParseError::InvalidDeclaration(err.to_string())))
            }
        }
    }
}

impl<'i> cssparser::AtRuleParser<'i> for RuleCollector {
    type Prelude = AtRulePrelude;
    type AtRule = CssRule;
    type Error = RuleParseError;

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        match name.as_ref().to_ascii_lowercase().as_str() {
            "media" => {
                let query = MediaQuery::parse(input)
                    .map_err(|_| at_rule_error(input, "invalid @media query"))?;
                Ok(AtRulePrelude::Media(query))
            }
            "keyframes" | "-webkit-keyframes" | "-moz-keyframes" => {
                input.skip_whitespace();
                // Парсим имя анимации: либо идентификатор, либо строка
                // Спецификация допускает <custom-ident> | <string>
                let animation_name = match input.next()? {
                    Token::Ident(ident) => ident.to_string(),
                    Token::QuotedString(s) => s.to_string(),
                    _ => return Err(at_rule_error(input, "@keyframes requires animation name")),
                };
                Ok(AtRulePrelude::Keyframes(animation_name))
            }
            "font-face" => Ok(AtRulePrelude::FontFace),
            _ => {
                let message = format!("unsupported at-rule @{}", name);
                Err(at_rule_error(input, &message))
            }
        }
    }

    fn rule_without_block(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        // Все наши @-правила требуют блока
        Err(())
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        match prelude {
            AtRulePrelude::Media(query) => {
                let mut nested_parser = RuleCollector::new(self._options);
                let mut nested_stylesheet = StyleSheetParser::new(input, &mut nested_parser);
                let mut rules = Vec::new();

                for result in &mut nested_stylesheet {
                    if let Ok(CssRule::Style(rule)) = result {
                        rules.push(rule);
                    }
                }

                let media_rule = MediaRule { query, rules };
                Ok(CssRule::AtRule(ParsedAtRule::Media(media_rule)))
            }
            AtRulePrelude::Keyframes(name) => {
                let keyframes = KeyframesDefinition::parse_keyframes_block(name, input)
                    .map_err(|_| at_rule_error(input, "invalid @keyframes block"))?;
                Ok(CssRule::AtRule(ParsedAtRule::Keyframes(keyframes)))
            }
            AtRulePrelude::FontFace => {
                let font_face = FontFace::parse_font_face_block(input)
                    .map_err(|_e| at_rule_error(input, "invalid @font-face block"))?;
                Ok(CssRule::AtRule(ParsedAtRule::FontFace(font_face)))
            }
        }
    }
}

/// Prelude для @-правил (промежуточный результат парсинга).
#[derive(Debug, Clone)]
enum AtRulePrelude {
    Media(MediaQuery),
    Keyframes(String),
    FontFace,
}

/// Helper для создания ошибки парсинга @-правила.
fn at_rule_error<'i>(
    input: &Parser<'i, '_>,
    message: &str,
) -> cssparser::ParseError<'i, RuleParseError> {
    input.new_custom_error(RuleParseError::UnsupportedAtRule(message.to_string()))
}

fn parse_declarations_from_parser(
    parser: &mut Parser<'_, '_>,
) -> Result<Vec<Property>, PropertyParseError> {
    let mut declarations = Vec::new();

    'declarations: while !parser.is_exhausted() {
        parser.skip_whitespace();
        if parser.is_exhausted() {
            break;
        }

        let name =
            match parser.try_parse(|input| input.expect_ident().map(|ident| ident.to_string())) {
                Ok(name) => name,
                Err(_) => {
                    skip_until_semicolon(parser);
                    continue;
                }
            };

        if parser.expect_colon().is_err() {
            skip_until_semicolon(parser);
            continue;
        }

        let value_start = parser.state();
        #[allow(unused_assignments)]
        let mut value_end: Option<ParserState> = None;

        loop {
            let before = parser.state();
            match parser.next_including_whitespace_and_comments() {
                Ok(Token::Semicolon) => {
                    value_end = Some(before);
                    break;
                }
                Ok(_) => {}
                Err(_) => {
                    value_end = Some(parser.state());
                    break;
                }
            }
        }

        let Some(end_state) = value_end else {
            continue 'declarations;
        };

        let raw_value = parser
            .slice(value_start.position()..end_state.position())
            .trim()
            .to_string();

        if raw_value.is_empty() {
            continue 'declarations;
        }

        let mut parsed = properties::parse_property(&name, raw_value.trim())?;
        declarations.append(&mut parsed);
    }

    Ok(declarations)
}

fn skip_until_semicolon(parser: &mut Parser<'_, '_>) {
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        if matches!(token, Token::Semicolon) {
            break;
        }
    }
}

fn parse_declarations_from_str(source: &str) -> Result<Vec<Property>, PropertyParseError> {
    let mut declarations = Vec::new();

    for candidate in source.split(';') {
        if let Some((name, value)) = candidate.split_once(':') {
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() || value.is_empty() {
                continue;
            }
            let mut parsed = properties::parse_property(name, value)?;
            declarations.append(&mut parsed);
        }
    }

    Ok(declarations)
}

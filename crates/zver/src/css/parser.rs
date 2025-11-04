//! Инфраструктура строгого парсинга CSS.
//!
//! Модуль оборачивает `cssparser`, обеспечивая:
//! - Преобразование к типу [`ParsedRule`];
//! - Фоллбек на упрощённый синтаксис для обратной совместимости;
//! - Парсинг inline-стилей через те же функции.

use std::fmt;

use cssparser::ToCss;
use cssparser::{CowRcStr, Parser, ParserInput, ParserState, StyleSheetParser, Token};
use thiserror::Error;

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

/// Высокоуровневый обёртчик `cssparser::StyleSheetParser`.
#[derive(Debug, Clone)]
pub struct StylesheetParser {
    options: CssParseOptions,
}

impl StylesheetParser {
    pub fn new(options: CssParseOptions) -> Self {
        Self { options }
    }

    /// Парсит таблицу стилей, выбрасывая первую же ошибку если
    /// `recover_from_errors` = `false`, иначе накапливает сообщения.
    pub fn parse_stylesheet(&mut self, css: &str) -> Result<Vec<ParsedRule>, CssParseError> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let mut rule_parser = RuleCollector::new(self.options);

        let mut stylesheet = StyleSheetParser::new(&mut parser, &mut rule_parser);
        let mut rules = Vec::new();
        let mut errors = Vec::new();

        for result in &mut stylesheet {
            match result {
                Ok(rule) => rules.push(rule),
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

        Ok(rules)
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
    type QualifiedRule = ParsedRule;
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
                _ => buffer.push_str(&token.to_css_string()),
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
            Ok(declarations) => Ok(ParsedRule {
                selector_text,
                selector_list,
                declarations,
            }),
            Err(err) => {
                Err(input.new_custom_error(RuleParseError::InvalidDeclaration(err.to_string())))
            }
        }
    }
}

impl<'i> cssparser::AtRuleParser<'i> for RuleCollector {
    type Prelude = ();
    type AtRule = ParsedRule;
    type Error = RuleParseError;

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        let message = format!("unsupported at-rule @{name}");
        Err(input.new_custom_error(RuleParseError::UnsupportedAtRule(message)))
    }

    fn rule_without_block(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        Err(())
    }

    fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        Err(input.new_custom_error(RuleParseError::UnsupportedAtRule(
            "unsupported at-rule".into(),
        )))
    }
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

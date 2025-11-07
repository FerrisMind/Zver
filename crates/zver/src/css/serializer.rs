//! CSS value serialization utilities.
//!
//! This module provides proper CSS serialization that converts tokens back to valid CSS text,
//! avoiding Debug formatting that produces invalid CSS like `Ident("red")` or `ParenthesisBlock([...])`.

use cssparser::{ParseError, Parser, Token};

/// Serializes CSS tokens into valid CSS text.
///
/// This function properly converts cssparser tokens back into their CSS representation,
/// handling all token types including functions, parentheses, and nested structures.
pub fn serialize_value_tokens<'i, 't>(
    input: &mut Parser<'i, 't>,
    until_semicolon: bool,
) -> Result<String, ParseError<'i, ()>> {
    let mut result = String::new();
    let mut needs_space = false;

    loop {
        if input.is_exhausted() {
            break;
        }

        let token = match input.next() {
            Ok(Token::Semicolon) if until_semicolon => break,
            Ok(token) => token.clone(), // Clone the token to avoid borrow checker issues
            Err(_) => break,
        };

        // Add spacing between tokens where needed
        if needs_space && needs_separator_before(&token) {
            result.push(' ');
        }

        serialize_token(&token, &mut result, input)?;
        needs_space = needs_separator_after(&token);
    }

    Ok(result.trim().to_string())
}

/// Serializes a single token to CSS text.
fn serialize_token<'i, 't>(
    token: &Token,
    result: &mut String,
    input: &mut Parser<'i, 't>,
) -> Result<(), ParseError<'i, ()>> {
    match token {
        Token::Ident(name) => result.push_str(name),
        Token::AtKeyword(name) => {
            result.push('@');
            result.push_str(name);
        }
        Token::Hash(name) | Token::IDHash(name) => {
            result.push('#');
            result.push_str(name);
        }
        Token::QuotedString(s) => {
            result.push('"');
            result.push_str(&escape_string(s));
            result.push('"');
        }
        Token::UnquotedUrl(url) => {
            result.push_str("url(");
            result.push_str(url);
            result.push(')');
        }
        Token::Delim(c) => result.push(*c),
        Token::Number { value, .. } => {
            result.push_str(&value.to_string());
        }
        Token::Percentage { unit_value, .. } => {
            result.push_str(&(unit_value * 100.0).to_string());
            result.push('%');
        }
        Token::Dimension { value, unit, .. } => {
            result.push_str(&value.to_string());
            result.push_str(unit);
        }
        Token::WhiteSpace(_) => result.push(' '),
        Token::Comment(_) => {} // Skip comments in serialized output
        Token::Colon => result.push(':'),
        Token::Semicolon => result.push(';'),
        Token::Comma => result.push(','),
        Token::IncludeMatch => result.push_str("~="),
        Token::DashMatch => result.push_str("|="),
        Token::PrefixMatch => result.push_str("^="),
        Token::SuffixMatch => result.push_str("$="),
        Token::SubstringMatch => result.push_str("*="),
        Token::CDO => result.push_str("<!--"),
        Token::CDC => result.push_str("-->"),
        Token::Function(name) => {
            result.push_str(name);
            result.push('(');
            // Parse the function contents in a nested block
            input.parse_nested_block(|nested_input| {
                let nested = serialize_value_tokens(nested_input, false)?;
                result.push_str(&nested);
                Ok(())
            })?;
            result.push(')');
        }
        Token::ParenthesisBlock => {
            result.push('(');
            input.parse_nested_block(|nested_input| {
                let nested = serialize_value_tokens(nested_input, false)?;
                result.push_str(&nested);
                Ok(())
            })?;
            result.push(')');
        }
        Token::SquareBracketBlock => {
            result.push('[');
            input.parse_nested_block(|nested_input| {
                let nested = serialize_value_tokens(nested_input, false)?;
                result.push_str(&nested);
                Ok(())
            })?;
            result.push(']');
        }
        Token::CurlyBracketBlock => {
            result.push('{');
            input.parse_nested_block(|nested_input| {
                let nested = serialize_value_tokens(nested_input, false)?;
                result.push_str(&nested);
                Ok(())
            })?;
            result.push('}');
        }
        Token::BadUrl(url) => {
            result.push_str("url(");
            result.push_str(url);
            result.push(')');
        }
        Token::BadString(s) => {
            result.push('"');
            result.push_str(s);
            result.push('"');
        }
        Token::CloseParenthesis => result.push(')'),
        Token::CloseSquareBracket => result.push(']'),
        Token::CloseCurlyBracket => result.push('}'),
    }
    Ok(())
}

/// Escapes special characters in CSS strings.
fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// Determines if whitespace is needed before this token.
fn needs_separator_before(token: &Token) -> bool {
    matches!(
        token,
        Token::Ident(_)
            | Token::AtKeyword(_)
            | Token::Hash(_)
            | Token::IDHash(_)
            | Token::Number { .. }
            | Token::Percentage { .. }
            | Token::Dimension { .. }
            | Token::Function(_)
    )
}

/// Determines if whitespace is needed after this token.
fn needs_separator_after(token: &Token) -> bool {
    matches!(
        token,
        Token::Ident(_)
            | Token::AtKeyword(_)
            | Token::Hash(_)
            | Token::IDHash(_)
            | Token::Number { .. }
            | Token::Percentage { .. }
            | Token::Dimension { .. }
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket
            | Token::Comma
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    #[test]
    fn test_serialize_simple_value() {
        let css = "red";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "red");
    }

    #[test]
    fn test_serialize_color_hex() {
        let css = "#ff0000";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "#ff0000");
    }

    #[test]
    fn test_serialize_dimension() {
        let css = "10px";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "10px");
    }

    #[test]
    fn test_serialize_function() {
        let css = "rgb(255, 0, 0)";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "rgb(255, 0, 0)");
    }

    #[test]
    fn test_serialize_url() {
        let css = "url(font.woff)";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "url(font.woff)");
    }

    #[test]
    fn test_serialize_quoted_string() {
        let css = r#""Helvetica Neue""#;
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, r#""Helvetica Neue""#);
    }

    #[test]
    fn test_serialize_multiple_values() {
        let css = "10px solid red";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "10px solid red");
    }

    #[test]
    fn test_serialize_until_semicolon() {
        let css = "10px; ignored";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let result = serialize_value_tokens(&mut parser, true).unwrap();
        assert_eq!(result, "10px");
    }
}

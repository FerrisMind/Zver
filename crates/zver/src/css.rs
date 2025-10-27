use cssparser::{ParseError, Parser, ToCss, Token};
use rayon::prelude::*;
use std::collections::HashMap;

use crate::dom::{Document, Node};

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub declarations: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StyleEngine {
    pub rules: Vec<StyleRule>,
    pub computed_styles: HashMap<usize, HashMap<String, String>>,
}

impl StyleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            computed_styles: HashMap::new(),
        }
    }

    // Параллельный парсинг CSS через rayon
    pub fn parse_css(&mut self, css: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.rules.clear();

        // Разбиваем CSS на отдельные правила
        let rule_strings: Vec<&str> = css.split('}').filter(|s| !s.trim().is_empty()).collect();

        // Параллельный парсинг правил через rayon
        let parsed_rules: Vec<Option<StyleRule>> = rule_strings
            .par_iter()
            .map(|rule_str| Self::parse_single_rule(rule_str))
            .collect();

        // Собираем успешно распарсенные правила
        for rule in parsed_rules.into_iter().flatten() {
            self.rules.push(rule);
        }

        // Фоллбек на простой парсинг если ничего не получилось
        if self.rules.is_empty() {
            self.parse_css_simple(css)?;
        }

        Ok(())
    }

    // Парсинг одного CSS правила
    fn parse_single_rule(rule_str: &str) -> Option<StyleRule> {
        let rule_str = rule_str.trim();
        if rule_str.is_empty() {
            return None;
        }

        let (selector_part, body_part) = rule_str.split_once('{')?;
        let selector = selector_part.trim().to_string();

        if selector.is_empty() {
            return None;
        }

        let mut declarations = HashMap::new();
        for declaration in body_part.split(';') {
            if let Some((property, value)) = declaration.split_once(':') {
                let property = property.trim();
                let value = value.trim();
                if !property.is_empty() && !value.is_empty() {
                    declarations.insert(property.to_string(), value.to_string());
                }
            }
        }

        if !declarations.is_empty() {
            Some(StyleRule {
                selector,
                declarations,
            })
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn parse_declarations<'i, 't>(
        parser: &mut Parser<'i, 't>,
    ) -> Result<HashMap<String, String>, ParseError<'i, ()>> {
        let mut declarations = HashMap::new();

        while !parser.is_exhausted() {
            // Читаем property
            let property = match parser.expect_ident() {
                Ok(ident) => ident.to_string(),
                Err(_) => {
                    // Пропускаем до следующей декларации
                    let _ = parser.expect_semicolon();
                    continue;
                }
            };

            // Ожидаем ':'
            if parser.expect_colon().is_err() {
                continue;
            }

            // Читаем значение до ';' или '}'
            let mut value_parts = Vec::new();
            loop {
                match parser.next() {
                    Ok(Token::Semicolon) | Err(_) => break,
                    Ok(token) => {
                        value_parts.push(token.to_css_string().to_string());
                    }
                }
            }

            let value = value_parts.join(" ").trim().to_string();
            if !property.is_empty() && !value.is_empty() {
                declarations.insert(property, value);
            }
        }

        Ok(declarations)
    }

    fn parse_css_simple(&mut self, css: &str) -> Result<(), Box<dyn std::error::Error>> {
        for block in css.split('}') {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            if let Some((selector_part, body_part)) = block.split_once('{') {
                let selector = selector_part.trim().to_string();
                if selector.is_empty() {
                    continue;
                }

                let mut declarations = HashMap::new();
                for declaration in body_part.split(';') {
                    if let Some((property, value)) = declaration.split_once(':') {
                        let property = property.trim();
                        let value = value.trim();
                        if !property.is_empty() && !value.is_empty() {
                            declarations.insert(property.to_string(), value.to_string());
                        }
                    }
                }

                if !declarations.is_empty() {
                    self.rules.push(StyleRule {
                        selector,
                        declarations,
                    });
                }
            }
        }
        Ok(())
    }

    // Параллельное применение стилей к нодам через rayon
    pub fn apply_styles(&mut self, document: &Document) -> Result<(), Box<dyn std::error::Error>> {
        self.computed_styles.clear();

        // Собираем все node_id в вектор для параллельной обработки
        let node_ids: Vec<usize> = document.nodes.keys().copied().collect();

        // Параллельно вычисляем стили для каждого узла
        let computed_styles: Vec<(usize, HashMap<String, String>)> = node_ids
            .par_iter()
            .filter_map(|node_id| {
                let node = document.nodes.get(node_id)?;
                let mut computed = HashMap::new();

                // Применяем правила CSS
                for rule in &self.rules {
                    if self.selector_matches(node, &rule.selector) {
                        for (property, value) in &rule.declarations {
                            computed.insert(property.clone(), value.clone());
                        }
                    }
                }

                // Применяем inline стили (перезаписывают CSS)
                if let Some(inline) = node.attributes.get("style") {
                    let _ = Self::parse_inline_styles_static(inline, &mut computed);
                }

                Some((*node_id, computed))
            })
            .collect();

        // Собираем результаты
        for (node_id, styles) in computed_styles {
            self.computed_styles.insert(node_id, styles);
        }

        Ok(())
    }

    fn selector_matches(&self, node: &Node, selector: &str) -> bool {
        match selector.chars().next() {
            Some('#') => node
                .attributes
                .get("id")
                .is_some_and(|id| id == &selector[1..]),
            Some('.') => node
                .attributes
                .get("class")
                .is_some_and(|class| class.split_whitespace().any(|c| c == &selector[1..])),
            _ => node.tag_name() == Some(selector),
        }
    }

    #[allow(dead_code)]
    fn parse_inline_styles(
        &self,
        inline: &str,
        computed: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::parse_inline_styles_static(inline, computed)
    }

    // Статическая версия для использования в параллельной обработке
    fn parse_inline_styles_static(
        inline: &str,
        computed: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for declaration in inline.split(';') {
            if let Some((property, value)) = declaration.split_once(':') {
                let property = property.trim();
                let value = value.trim();
                if !property.is_empty() && !value.is_empty() {
                    computed.insert(property.to_string(), value.to_string());
                }
            }
        }
        Ok(())
    }
}

impl Default for StyleEngine {
    fn default() -> Self {
        Self::new()
    }
}

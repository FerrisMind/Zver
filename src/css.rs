
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

    pub fn parse_css(&mut self, css: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.rules.clear();

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

    pub fn apply_styles(
        &mut self,
        document: &Document,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.computed_styles.clear();

        for (node_id, node) in &document.nodes {
            let mut computed = HashMap::new();

            for rule in &self.rules {
                if self.selector_matches(node, &rule.selector) {
                    for (property, value) in &rule.declarations {
                        computed.insert(property.clone(), value.clone());
                    }
                }
            }

            if let Some(inline) = node.attributes.get("style") {
                self.parse_inline_styles(inline, &mut computed)?;
            }

            self.computed_styles.insert(*node_id, computed);
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

    fn parse_inline_styles(
        &self,
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


//! Высокоуровневый движок CSS для Zver.
//!
//! Модуль объединяет три основных подсистемы:
//! - [`parser`] — строгий парсинг CSS с использованием `cssparser` и
//!   адаптированный фоллбек для упрощённого синтаксиса.
//! - [`selectors`] — интеграция с crate `selectors` и адаптер DOM-узлов.
//! - [`properties`] — нормализация и валидация CSS-свойств.
//!
//! Внешний API (`StyleEngine`) сохраняет обратную совместимость, но внутренняя
//! архитектура стала модульной, что упрощает расширение функциональности.

pub mod color;
pub mod parser;
pub mod properties;
pub mod selectors;

use rayon::prelude::*;
use std::collections::HashMap;

use crate::dom::{Document, Node};
use parser::{CssParseOptions, ParsedRule, StylesheetParser};
use properties::{AppliedProperty, Property};
use selectors::{CompiledSelector, NodeAdapter};

/// Представляет одно CSS-правило (selector + декларации) после нормализации.
#[derive(Debug, Clone)]
pub struct StyleRule {
    /// Оригинальное текстовое представление селектора.
    pub selector_text: String,
    /// Нормализованные декларации (без учёта каскада).
    pub declarations: Vec<Property>,
}

/// Хранилище стилей и механизм применения CSS к DOM.
#[derive(Debug, Default, Clone)]
pub struct StyleEngine {
    /// Сырые правила в порядке появления во входном CSS.
    pub rules: Vec<StyleRule>,
    /// Компилированные селекторы, синхронизированные по индексу с `rules`.
    pub parsed_selectors: Vec<selectors::SelectorListHandle>,
    /// Кэш готовых селекторов по текстовому ключу (используется для повторного парсинга).
    pub selector_cache: HashMap<String, CompiledSelector>,
    /// Вычисленные стили по node_id.
    pub computed_styles: HashMap<usize, HashMap<String, String>>,
}

impl StyleEngine {
    /// Создаёт пустой движок стилей.
    pub fn new() -> Self {
        Self::default()
    }

    /// Парсит CSS-строку и обновляет внутренний набор правил.
    pub fn parse_css(&mut self, css: &str) -> Result<(), parser::CssParseError> {
        self.rules.clear();
        self.parsed_selectors.clear();

        let mut stylesheet = StylesheetParser::new(CssParseOptions::default());
        let mut parsed_rules = stylesheet.parse_stylesheet(css)?;

        if parsed_rules.is_empty() {
            parsed_rules = stylesheet.parse_with_fallback(css);
        }

        for ParsedRule {
            selector_text,
            selector_list,
            declarations,
        } in parsed_rules
        {
            self.parsed_selectors.push(selector_list.clone());

            let compiled = self
                .selector_cache
                .entry(selector_text.clone())
                .or_insert_with(|| CompiledSelector::new(selector_list.clone()));
            compiled.mark_used();

            self.rules.push(StyleRule {
                selector_text,
                declarations,
            });
        }

        // Удаляем записи кэша, которые не использовались на текущем проходе.
        self.selector_cache
            .retain(|_, compiled| compiled.is_marked());
        for compiled in self.selector_cache.values_mut() {
            compiled.reset_usage_flag();
        }

        Ok(())
    }

    /// Применяет каскад CSS к DOM-дереву и формирует карту вычисленных стилей.
    pub fn apply_styles(&mut self, document: &Document) -> Result<(), parser::CssParseError> {
        self.computed_styles.clear();

        let node_ids: Vec<usize> = document.nodes.keys().copied().collect();

        let cascaded: Vec<(usize, HashMap<String, AppliedProperty>)> = node_ids
            .par_iter()
            .filter_map(|node_id| {
                document
                    .nodes
                    .get(node_id)
                    .filter(|node| node.is_element())
                    .map(|node| (*node_id, self.compute_styles_for_node(document, node)))
            })
            .collect();

        for (node_id, properties) in cascaded {
            let mut normalized = HashMap::with_capacity(properties.len());
            for (name, applied) in properties {
                normalized.insert(name, applied.value);
            }
            self.computed_styles.insert(node_id, normalized);
        }

        Ok(())
    }

    fn compute_styles_for_node(
        &self,
        document: &Document,
        node: &Node,
    ) -> HashMap<String, AppliedProperty> {
        let mut cascade: HashMap<String, AppliedProperty> = HashMap::new();
        let adapter = NodeAdapter::new(document, node.id);

        if adapter.is_none() {
            return cascade;
        }
        let adapter = adapter.unwrap();

        let mut cascade_order: u64 = 0;

        for (rule_index, rule) in self.rules.iter().enumerate() {
            if let Some(compiled) = self.selector_cache.get(&rule.selector_text)
                && let Some(specificity) = compiled.matches(&adapter)
            {
                selectors::apply_rule(
                    &mut cascade,
                    rule,
                    specificity,
                    rule_index as u64,
                    &mut cascade_order,
                );
            }
        }

        // Inline-стили имеют максимальную специфичность и обрабатываются в конце.
        if let Some(inline) = node.attributes.get("style")
            && let Ok(properties) = parser::parse_inline_declarations(inline)
        {
            selectors::apply_inline(&mut cascade, &properties, &mut cascade_order);
        }

        cascade
    }
}

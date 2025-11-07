//! Высокоуровневый движок CSS для Zver.
//!
//! Модуль объединяет основные подсистемы CSS:
//! - [`parser`] — строгий парсинг CSS с использованием `cssparser` и
//!   адаптированный фоллбек для упрощённого синтаксиса.
//! - [`selectors`] — интеграция с crate `selectors` и адаптер DOM-узлов.
//! - [`properties`] — нормализация и валидация CSS-свойств.
//! - [`media_queries`] — поддержка @media queries (Фаза 2).
//! - [`animations`] — поддержка @keyframes анимаций (Фаза 2).
//! - [`fonts`] — поддержка @font-face и загрузка шрифтов (Фаза 2).
//!
//! Внешний API (`StyleEngine`) сохраняет обратную совместимость, но внутренняя
//! архитектура стала модульной, что упрощает расширение функциональности.

pub mod animations;
pub mod color;
pub mod fonts;
pub mod media_queries;
pub mod parser;
pub mod properties;
pub mod selectors;
pub mod serializer;

use rayon::prelude::*;
use std::collections::HashMap;

use crate::dom::Document;
use parser::{CssParseOptions, ParsedRule, StylesheetParser};
use properties::{AppliedProperty, Property};
use selectors::{CompiledSelector, NodeAdapter, PseudoElement};

/// Представляет одно CSS-правило (selector + декларации) после нормализации.
#[derive(Debug, Clone)]
pub struct StyleRule {
    /// Оригинальное текстовое представление селектора.
    pub selector_text: String,
    /// Нормализованные декларации (без учёта каскада).
    pub declarations: Vec<Property>,
}

#[derive(Debug, Clone)]
pub struct PseudoStyle {
    pub properties: HashMap<String, String>,
    pub content: String,
}

/// Хранилище стилей и механизм применения CSS к DOM.
#[derive(Debug, Clone, Default)]
pub struct StyleEngine {
    /// Сырые правила в порядке появления во входном CSS.
    pub rules: Vec<StyleRule>,
    /// Компилированные селекторы, синхронизированные по индексу с `rules`.
    pub parsed_selectors: Vec<selectors::SelectorListHandle>,
    /// Кэш готовых селекторов по текстовому ключу (используется для повторного парсинга).
    pub selector_cache: HashMap<String, CompiledSelector>,
    /// Вычисленные стили по node_id.
    pub computed_styles: HashMap<usize, HashMap<String, String>>,
    pub pseudo_element_styles: HashMap<usize, HashMap<PseudoElement, PseudoStyle>>,

    // === Фаза 2: @-правила ===
    /// @media правила (Фаза 2)
    pub media_rules: Vec<media_queries::MediaRule>,
    /// @keyframes определения по имени (Фаза 2)
    pub keyframes: HashMap<String, animations::KeyframesDefinition>,
    /// @font-face определения (Фаза 2)
    pub fonts: Vec<fonts::LoadedFont>,
    
    // === Viewport для media queries ===
    /// Ширина viewport для @media queries (по умолчанию 1024px)
    pub viewport_width: f32,
    /// Высота viewport для @media queries (по умолчанию 768px)
    pub viewport_height: f32,
    /// Тип медиа (по умолчанию Screen)
    pub media_type: media_queries::MediaType,
}

impl StyleEngine {
    /// Создаёт пустой движок стилей.
    pub fn new() -> Self {
        Self {
            viewport_width: 1024.0,
            viewport_height: 768.0,
            media_type: media_queries::MediaType::Screen,
            ..Default::default()
        }
    }

    /// Устанавливает размеры viewport для @media queries.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Устанавливает тип медиа для @media queries.
    pub fn set_media_type(&mut self, media_type: media_queries::MediaType) {
        self.media_type = media_type;
    }

    /// Парсит CSS-строку и обновляет внутренний набор правил (Фаза 2: с поддержкой @-rules).
    pub fn parse_css(&mut self, css: &str) -> Result<(), parser::CssParseError> {
        self.rules.clear();
        self.parsed_selectors.clear();
        self.media_rules.clear();
        self.keyframes.clear();
        self.fonts.clear();
        self.pseudo_element_styles.clear();

        let mut stylesheet = StylesheetParser::new(CssParseOptions::default());
        let mut parsed_stylesheet = stylesheet.parse_stylesheet(css)?;

        // Fallback если нет обычных правил
        if parsed_stylesheet.rules.is_empty()
            && parsed_stylesheet.media_rules.is_empty()
            && parsed_stylesheet.keyframes.is_empty()
            && parsed_stylesheet.font_faces.is_empty()
        {
            let fallback_rules = stylesheet.parse_with_fallback(css);
            parsed_stylesheet.rules = fallback_rules;
        }

        // Обрабатываем обычные CSS-правила
        for ParsedRule {
            selector_text,
            selector_list,
            declarations,
        } in parsed_stylesheet.rules
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

        // Сохраняем @-правила (Фаза 2)
        self.media_rules = parsed_stylesheet.media_rules;

        // Преобразуем keyframes в HashMap
        for kf in parsed_stylesheet.keyframes {
            self.keyframes.insert(kf.name.clone(), kf);
        }

        // Загружаем шрифты (TODO: Фаза 3 - реализовать загрузку)
        for font_face in parsed_stylesheet.font_faces {
            // Пока сохраняем определения, загрузка будет в Фазе 3
            let loaded_font = fonts::LoadedFont::new(font_face);
            self.fonts.push(loaded_font);
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
        self.pseudo_element_styles.clear();

        let element_ids: Vec<usize> = document
            .nodes
            .iter()
            .filter(|(_, node)| node.is_element())
            .map(|(id, _)| *id)
            .collect();

        type CascadedStyles = (
            usize,
            HashMap<String, String>,
            HashMap<PseudoElement, HashMap<String, String>>,
        );
        let cascaded: Vec<CascadedStyles> = element_ids
            .par_iter()
            .map(|&node_id| {
                let (cascade, pseudo) = self.compute_styles_for_node(document, node_id);
                let normalized = Self::normalize_cascade(cascade);
                let normalized_pseudo = pseudo
                    .into_iter()
                    .map(|(pseudo, properties)| (pseudo, Self::normalize_cascade(properties)))
                    .collect();
                (node_id, normalized, normalized_pseudo)
            })
            .collect();

        for (node_id, styles, pseudo_styles) in cascaded {
            if !styles.is_empty() {
                self.computed_styles.insert(node_id, styles);
            }

            let pseudo = Self::build_pseudo_styles(pseudo_styles);
            if !pseudo.is_empty() {
                self.pseudo_element_styles.insert(node_id, pseudo);
            }
        }
        Ok(())
    }

    /// Преобразует каскад AppliedProperty в финальные строковые значения.
    fn normalize_cascade(properties: HashMap<String, AppliedProperty>) -> HashMap<String, String> {
        properties
            .into_iter()
            .map(|(name, applied)| (name, applied.value))
            .collect()
    }

    fn build_pseudo_styles(
        pseudo: HashMap<PseudoElement, HashMap<String, String>>,
    ) -> HashMap<PseudoElement, PseudoStyle> {
        pseudo
            .into_iter()
            .filter_map(|(pseudo, properties)| {
                Self::parse_pseudo_content(&properties).map(|content| {
                    (
                        pseudo,
                        PseudoStyle {
                            properties,
                            content,
                        },
                    )
                })
            })
            .collect()
    }

    fn parse_pseudo_content(properties: &HashMap<String, String>) -> Option<String> {
        let value = properties.get("content")?;
        Self::parse_content_value(value)
    }

    fn parse_content_value(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("normal") {
            return None;
        }

        let quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''));
        if quoted && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Some(Self::unescape_css_string(inner));
        }

        None
    }

    fn unescape_css_string(value: &str) -> String {
        let mut result = String::with_capacity(value.len());
        let mut chars = value.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else {
                result.push(ch);
            }
        }
        result
    }

    pub fn pseudo_element_contents(&self) -> HashMap<usize, HashMap<PseudoElement, String>> {
        self.pseudo_element_styles
            .iter()
            .map(|(&node_id, styles)| {
                let contents = styles
                    .iter()
                    .map(|(&pseudo, style)| (pseudo, style.content.clone()))
                    .collect();
                (node_id, contents)
            })
            .collect()
    }

    fn compute_styles_for_node(
        &self,
        document: &Document,
        node_id: usize,
    ) -> (
        HashMap<String, AppliedProperty>,
        HashMap<PseudoElement, HashMap<String, AppliedProperty>>,
    ) {
        let mut cascade: HashMap<String, AppliedProperty> = HashMap::new();
        let mut pseudo_cascades: HashMap<PseudoElement, HashMap<String, AppliedProperty>> =
            HashMap::new();
        let mut pseudo_orders: HashMap<PseudoElement, u64> = HashMap::new();

        let Some(node) = document.nodes.get(&node_id) else {
            return (cascade, pseudo_cascades);
        };

        let Some(adapter) = NodeAdapter::new(document, node_id) else {
            return (cascade, pseudo_cascades);
        };

        let mut cascade_order: u64 = 0;

        // Применяем правила из таблицы стилей
        for (rule_index, rule) in self.rules.iter().enumerate() {
            if let Some(compiled) = self.selector_cache.get(&rule.selector_text) {
                if let Some(specificity) = compiled.matches(&adapter) {
                    selectors::apply_rule(
                        &mut cascade,
                        rule,
                        specificity,
                        rule_index as u64,
                        &mut cascade_order,
                    );
                }

                let mut pseudo_targets = Vec::new();
                for selector in compiled.selector_list().slice().iter() {
                    if let Some(pseudo) = selector.pseudo_element().copied()
                        && !pseudo_targets.contains(&pseudo)
                    {
                        pseudo_targets.push(pseudo);
                    }
                }

                for pseudo in pseudo_targets {
                    if let Some(pseudo_specificity) = compiled.matches_pseudo(&adapter, pseudo) {
                        let entry = pseudo_cascades.entry(pseudo).or_default();
                        let order = pseudo_orders.entry(pseudo).or_insert(0);
                        selectors::apply_rule(
                            entry,
                            rule,
                            pseudo_specificity,
                            rule_index as u64,
                            order,
                        );
                    }
                }
            }
        }

        // Применяем правила из @media queries
        for media_rule in &self.media_rules {
            if media_rule.applies_to(self.viewport_width, self.viewport_height, self.media_type.clone()) {
                for rule in &media_rule.rules {
                    if let Some(compiled) = self.selector_cache.get(&rule.selector_text)
                        && let Some(specificity) = compiled.matches(&adapter)
                    {
                        // Преобразуем ParsedRule в StyleRule для apply_rule
                        let style_rule = StyleRule {
                            selector_text: rule.selector_text.clone(),
                            declarations: rule.declarations.clone(),
                        };
                        selectors::apply_rule(
                            &mut cascade,
                            &style_rule,
                            specificity,
                            0, // Media rules имеют порядок после обычных правил
                            &mut cascade_order,
                        );
                    }
                }
            }
        }

        // Применяем inline-стили с максимальной специфичностью
        if let Some(inline) = node.attributes.get("style")
            && let Ok(properties) = parser::parse_inline_declarations(inline)
        {
            selectors::apply_inline(&mut cascade, &properties, &mut cascade_order);
        }

        (cascade, pseudo_cascades)
    }
}

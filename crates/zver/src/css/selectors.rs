//! Интеграция с crate `selectors` и адаптация DOM-узлов проекта.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use cssparser::ToCss;
use cssparser::{Parser as CssParser, ParserInput, serialize_string};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::matching::{
    self, MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};
use selectors::parser::{self, SelectorList, SelectorParseErrorKind};
use selectors::{Element, OpaqueElement};
use thiserror::Error;

use super::StyleRule;
use super::properties::{self, AppliedProperty, Property};
use crate::dom::{Document, ElementState, Node};

/// Тип-ссылка на список селекторов, с которым работает движок.
pub type SelectorListHandle = SelectorList<Simple>;

/// Компилированный селектор вместе с кэшем для быстрого сопоставления.
pub struct CompiledSelector {
    selector_list: SelectorListHandle,
    caches: Mutex<SelectorCaches>,
    used_in_last_pass: bool,
}

impl CompiledSelector {
    pub fn new(selector_list: SelectorListHandle) -> Self {
        Self {
            selector_list,
            caches: Mutex::new(SelectorCaches::default()),
            used_in_last_pass: false,
        }
    }

    pub fn mark_used(&mut self) {
        self.used_in_last_pass = true;
    }

    pub fn is_marked(&self) -> bool {
        self.used_in_last_pass
    }

    pub fn reset_usage_flag(&mut self) {
        self.used_in_last_pass = false;
    }

    /// Проверяет соответствие DOM-узла селектору и возвращает специфику (если найдено).
    pub fn matches(&self, element: &NodeAdapter<'_>) -> Option<u32> {
        let mut caches = match self.caches.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                #[cfg(debug_assertions)]
                eprintln!("Warning: Selector cache was poisoned, recovering with existing data");
                poisoned.into_inner()
            }
        };

        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );

        self.selector_list
            .slice()
            .iter()
            .filter(|selector| matching::matches_selector(selector, 0, None, element, &mut context))
            .map(|selector| selector.specificity())
            .max()
    }

    pub fn selector_list(&self) -> &SelectorListHandle {
        &self.selector_list
    }

    pub fn matches_pseudo(&self, element: &NodeAdapter<'_>, pseudo: PseudoElement) -> Option<u32> {
        let mut caches = match self.caches.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                #[cfg(debug_assertions)]
                eprintln!("Warning: Selector cache was poisoned, recovering with existing data");
                poisoned.into_inner()
            }
        };

        let filter = Self::pseudo_filter(pseudo);
        let mut context = MatchingContext::new(
            MatchingMode::ForStatelessPseudoElement,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        context.pseudo_element_matching_fn = Some(&filter);

        self.selector_list
            .slice()
            .iter()
            .filter(|selector| selector.pseudo_element().is_some_and(&filter))
            .filter(|selector| matching::matches_selector(selector, 0, None, element, &mut context))
            .map(|selector| selector.specificity())
            .max()
    }
}

impl Clone for CompiledSelector {
    fn clone(&self) -> Self {
        Self {
            selector_list: self.selector_list.clone(),
            caches: Mutex::new(SelectorCaches::default()),
            used_in_last_pass: self.used_in_last_pass,
        }
    }
}

impl fmt::Debug for CompiledSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledSelector")
            .field("selector", &self.selector_list.to_css_string())
            .field("used", &self.used_in_last_pass)
            .finish()
    }
}

impl CompiledSelector {
    fn pseudo_filter(pseudo: PseudoElement) -> fn(&PseudoElement) -> bool {
        match pseudo {
            PseudoElement::Before => Self::is_before,
            PseudoElement::After => Self::is_after,
            PseudoElement::FirstLine => Self::is_first_line,
            PseudoElement::FirstLetter => Self::is_first_letter,
        }
    }

    fn is_before(candidate: &PseudoElement) -> bool {
        matches!(candidate, PseudoElement::Before)
    }

    fn is_after(candidate: &PseudoElement) -> bool {
        matches!(candidate, PseudoElement::After)
    }

    fn is_first_line(candidate: &PseudoElement) -> bool {
        matches!(candidate, PseudoElement::FirstLine)
    }

    fn is_first_letter(candidate: &PseudoElement) -> bool {
        matches!(candidate, PseudoElement::FirstLetter)
    }
}

/// Ошибка компиляции CSS-селектора.
#[derive(Debug, Error, Clone)]
#[error("selector parse error: {0}")]
pub struct SelectorCompileError(String);

/// Компилирует текст селектора в `SelectorListHandle`.
pub fn compile_selector_list(
    selector_text: &str,
) -> Result<SelectorListHandle, SelectorCompileError> {
    let mut input = ParserInput::new(selector_text);
    let mut parser = CssParser::new(&mut input);
    SelectorList::parse(&SelectorParser, &mut parser, parser::ParseRelative::No)
        .map_err(|err| SelectorCompileError(format!("{err:?}")))
}

/// Применяет CSS-правило к карте каскада с учётом порядка и специфичности.
pub fn apply_rule(
    cascade: &mut HashMap<String, AppliedProperty>,
    rule: &StyleRule,
    specificity: u32,
    rule_index: u64,
    cascade_order: &mut u64,
) {
    for property in &rule.declarations {
        *cascade_order = cascade_order.saturating_add(1);
        let order = ((*cascade_order) << 32) | rule_index;
        properties::merge_property(cascade, property, specificity, order);
    }
}

/// Применяет inline-стили (атрибут `style`) с максимальной специфичностью.
pub fn apply_inline(
    cascade: &mut HashMap<String, AppliedProperty>,
    properties: &[Property],
    cascade_order: &mut u64,
) {
    for property in properties {
        *cascade_order = cascade_order.saturating_add(1);
        let order = ((*cascade_order) << 32) | 0xFFFF_FFFF;
        properties::merge_property(cascade, property, u32::MAX, order);
    }
}

/// Адаптер DOM-узла для использования с crate `selectors`.
#[derive(Clone)]
pub struct NodeAdapter<'a> {
    document: &'a Document,
    node_id: usize,
}

impl<'a> NodeAdapter<'a> {
    pub fn new(document: &'a Document, node_id: usize) -> Option<Self> {
        if document.nodes.get(&node_id).is_some_and(Node::is_element) {
            Some(Self { document, node_id })
        } else {
            None
        }
    }

    fn node(&self) -> Option<&Node> {
        self.document.nodes.get(&self.node_id)
    }

    fn child_element_from(&self, iter: impl Iterator<Item = usize>) -> Option<Self> {
        for child_id in iter {
            if let Some(adapter) = NodeAdapter::new(self.document, child_id) {
                return Some(adapter);
            }
        }
        None
    }

    fn sibling_position<F>(&self, predicate: F) -> Option<(usize, usize)>
    where
        F: Fn(&Node) -> bool,
    {
        let node = self.node()?;
        let parent_id = node.parent?;
        let parent = self.document.nodes.get(&parent_id)?;

        let mut current_index: Option<usize> = None;
        let mut total = 0usize;

        for &child_id in &parent.children {
            let child = self.document.nodes.get(&child_id)?;
            if !child.is_element() || !predicate(child) {
                continue;
            }

            total += 1;
            if child_id == self.node_id {
                current_index = Some(total);
            }
        }

        current_index.map(|index| (index, total))
    }

    fn structural_position(&self) -> Option<(usize, usize)> {
        self.sibling_position(|_| true)
    }

    fn type_position(&self) -> Option<(usize, usize)> {
        let tag = {
            let node = self.node()?;
            let tag = node.tag_name.as_ref()?;
            tag.to_ascii_lowercase()
        };

        self.sibling_position(|sibling| {
            sibling
                .tag_name
                .as_ref()
                .map(|name| name.eq_ignore_ascii_case(&tag))
                .unwrap_or(false)
        })
    }

    fn has_attribute(node: &Node, name: &str) -> bool {
        Self::attribute_value(node, name).is_some()
    }

    fn attribute_value<'n>(node: &'n Node, name: &str) -> Option<&'n str> {
        node.attributes
            .iter()
            .find(|(attr_name, _)| attr_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn is_form_control(node: &Node) -> bool {
        node.tag_name
            .as_deref()
            .map(|tag| {
                ["input", "textarea", "select", "button", "option"]
                    .iter()
                    .any(|candidate| tag.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(false)
    }

    fn is_checkable(node: &Node) -> bool {
        if let Some(tag) = node.tag_name.as_deref() {
            if tag.eq_ignore_ascii_case("option") {
                return true;
            }
            if tag.eq_ignore_ascii_case("input")
                && let Some(input_type) = Self::attribute_value(node, "type")
            {
                return input_type.eq_ignore_ascii_case("checkbox")
                    || input_type.eq_ignore_ascii_case("radio");
            }
        }
        false
    }

    fn is_disabled(node: &Node) -> bool {
        node.element_state.contains(ElementState::DISABLED) || Self::has_attribute(node, "disabled")
    }

    fn is_checked(node: &Node) -> bool {
        if node
            .tag_name
            .as_deref()
            .is_some_and(|tag| tag.eq_ignore_ascii_case("option"))
        {
            return Self::has_attribute(node, "selected");
        }

        Self::is_checkable(node)
            && (node.element_state.contains(ElementState::CHECKED)
                || Self::has_attribute(node, "checked"))
    }

    fn is_link_element(node: &Node) -> bool {
        if !Self::has_attribute(node, "href") {
            return false;
        }

        node.tag_name
            .as_deref()
            .map(|tag| {
                tag.eq_ignore_ascii_case("a")
                    || tag.eq_ignore_ascii_case("area")
                    || tag.eq_ignore_ascii_case("link")
            })
            .unwrap_or(false)
    }

    fn is_placeholder_shown(node: &Node) -> bool {
        Self::is_form_control(node) && Self::has_attribute(node, "placeholder")
    }

    fn is_required(node: &Node) -> bool {
        Self::is_form_control(node) && Self::has_attribute(node, "required")
    }

    fn is_read_only(node: &Node) -> bool {
        !Self::is_form_control(node)
            || Self::has_attribute(node, "readonly")
            || Self::is_disabled(node)
    }

    fn is_read_write(node: &Node) -> bool {
        Self::is_form_control(node)
            && !Self::has_attribute(node, "readonly")
            && !Self::is_disabled(node)
    }

    fn is_aria_invalid(node: &Node) -> bool {
        Self::attribute_value(node, "aria-invalid")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }
}

impl fmt::Debug for NodeAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeAdapter")
            .field("node_id", &self.node_id)
            .finish()
    }
}

impl<'a> Element for NodeAdapter<'a> {
    type Impl = Simple;

    fn opaque(&self) -> OpaqueElement {
        self.node()
            .map(OpaqueElement::new)
            .unwrap_or_else(|| OpaqueElement::new(&self.node_id))
    }

    fn parent_element(&self) -> Option<Self> {
        let node = self.node()?;
        let parent_id = node.parent?;
        NodeAdapter::new(self.document, parent_id)
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let node = self.node()?;
        let parent_id = node.parent?;
        let parent = self.document.nodes.get(&parent_id)?;
        let index = parent.children.iter().position(|&id| id == self.node_id)?;
        self.child_element_from(parent.children[..index].iter().rev().copied())
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let node = self.node()?;
        let parent_id = node.parent?;
        let parent = self.document.nodes.get(&parent_id)?;
        let index = parent.children.iter().position(|&id| id == self.node_id)?;
        self.child_element_from(parent.children[(index + 1)..].iter().copied())
    }

    fn first_element_child(&self) -> Option<Self> {
        let node = self.node()?;
        self.child_element_from(node.children.iter().copied())
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.node().is_some()
    }

    fn has_local_name(&self, local_name: &str) -> bool {
        self.node()
            .and_then(|node| node.tag_name.as_ref())
            .map(|tag| tag.eq_ignore_ascii_case(local_name))
            .unwrap_or(false)
    }

    fn has_namespace(&self, ns: &str) -> bool {
        ns.is_empty() || ns.eq_ignore_ascii_case("http://www.w3.org/1999/xhtml")
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.node().and_then(|node| node.tag_name.as_ref())
            == other.node().and_then(|node| node.tag_name.as_ref())
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&CssNamespace>,
        local_name: &CssLocalName,
        operation: &AttrSelectorOperation<&CssString>,
    ) -> bool {
        if !namespace_matches(ns) {
            return false;
        }

        let node = match self.node() {
            Some(node) => node,
            None => return false,
        };

        node.attributes
            .get(local_name.as_ref())
            .map(|value| operation.eval_str(value))
            .unwrap_or(false)
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &NonTSPseudoClass,
        _context: &mut MatchingContext<'_, Self::Impl>,
    ) -> bool {
        use NonTSPseudoClass::*;

        let Some(node) = self.node() else {
            return false;
        };

        match pc {
            FirstChild => self.structural_position().is_some_and(|(pos, _)| pos == 1),
            LastChild => self
                .structural_position()
                .is_some_and(|(pos, total)| pos == total),
            OnlyChild => self
                .structural_position()
                .is_some_and(|(_, total)| total == 1),
            FirstOfType => self.type_position().is_some_and(|(pos, _)| pos == 1),
            LastOfType => self
                .type_position()
                .is_some_and(|(pos, total)| pos == total),
            // Note: nth-child, nth-last-child, nth-of-type, nth-last-of-type
            // обрабатываются через Component::Nth в selectors crate
            Hover => node.element_state.contains(ElementState::HOVER),
            Focus => node.element_state.contains(ElementState::FOCUS),
            Active => node.element_state.contains(ElementState::ACTIVE),
            Disabled => Self::is_disabled(node),
            Enabled => Self::is_form_control(node) && !Self::is_disabled(node),
            Checked => Self::is_checked(node),
            Indeterminate => node.element_state.contains(ElementState::INDETERMINATE),
            Link | AnyLink => Self::is_link_element(node),
            Visited => false,
            ReadOnly => Self::is_read_only(node),
            ReadWrite => Self::is_read_write(node),
            PlaceholderShown => Self::is_placeholder_shown(node),
            Valid => !Self::is_aria_invalid(node),
            Invalid => Self::is_aria_invalid(node),
            InRange => false,
            OutOfRange => false,
            Required => Self::is_required(node),
            Optional => Self::is_form_control(node) && !Self::is_required(node),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: matching::ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        self.node()
            .and_then(|node| node.tag_name.as_ref())
            .is_some_and(|tag| tag.eq_ignore_ascii_case("link"))
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(&self, id: &CssString, case_sensitivity: CaseSensitivity) -> bool {
        self.node()
            .and_then(|node| node.attributes.get("id"))
            .map(|value| case_sensitivity.eq(value.as_bytes(), id.as_ref().as_bytes()))
            .unwrap_or(false)
    }

    fn has_class(&self, name: &CssString, case_sensitivity: CaseSensitivity) -> bool {
        self.node()
            .and_then(|node| node.attributes.get("class"))
            .map(|classes| {
                classes
                    .split_whitespace()
                    .any(|class| case_sensitivity.eq(class.as_bytes(), name.as_ref().as_bytes()))
            })
            .unwrap_or(false)
    }

    fn has_custom_state(&self, _name: &CssString) -> bool {
        false
    }

    fn imported_part(&self, _name: &CssString) -> Option<CssString> {
        None
    }

    fn is_part(&self, _name: &CssString) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        let node = match self.node() {
            Some(node) => node,
            None => return true,
        };

        node.children.iter().all(|child_id| {
            if let Some(child) = self.document.nodes.get(child_id) {
                if child.is_element() {
                    return false;
                }
                if let Some(text) = &child.text_content {
                    return text.trim().is_empty();
                }
            }
            true
        })
    }

    fn is_root(&self) -> bool {
        self.document.root == Some(self.node_id)
    }

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }
}

/// Простая реализация `SelectorImpl` без поддержки псевдо-элементов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Simple;

impl parser::SelectorImpl for Simple {
    type AttrValue = CssString;
    type Identifier = CssString;
    type LocalName = CssLocalName;
    type NamespacePrefix = CssLocalName;
    type NamespaceUrl = CssNamespace;
    type BorrowedNamespaceUrl = str;
    type BorrowedLocalName = str;
    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;
    type ExtraMatchingData<'a> = ();
}

/// Псевдоклассы, поддерживаемые в Zver (Фаза 3)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonTSPseudoClass {
    // Структурные псевдоклассы (простые, без параметров)
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    // Note: nth-child, nth-last-child, nth-of-type, nth-last-of-type
    // обрабатываются selectors crate через Component::Nth автоматически

    // Псевдоклассы состояния
    Hover,
    Focus,
    Active,
    Disabled,
    Enabled,
    Checked,
    Indeterminate,

    // Псевдоклассы типа ссылок
    Link,
    Visited,
    AnyLink,

    // Псевдоклассы формы
    ReadOnly,
    ReadWrite,
    PlaceholderShown,
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    Required,
    Optional,
}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = Simple;

    fn is_active_or_hover(&self) -> bool {
        matches!(self, Self::Active | Self::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(self, Self::Active | Self::Hover | Self::Focus)
    }
}

impl cssparser::ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::FirstChild => dest.write_str(":first-child"),
            Self::LastChild => dest.write_str(":last-child"),
            Self::OnlyChild => dest.write_str(":only-child"),
            Self::FirstOfType => dest.write_str(":first-of-type"),
            Self::LastOfType => dest.write_str(":last-of-type"),
            Self::Hover => dest.write_str(":hover"),
            Self::Focus => dest.write_str(":focus"),
            Self::Active => dest.write_str(":active"),
            Self::Disabled => dest.write_str(":disabled"),
            Self::Enabled => dest.write_str(":enabled"),
            Self::Checked => dest.write_str(":checked"),
            Self::Indeterminate => dest.write_str(":indeterminate"),
            Self::Link => dest.write_str(":link"),
            Self::Visited => dest.write_str(":visited"),
            Self::AnyLink => dest.write_str(":any-link"),
            Self::ReadOnly => dest.write_str(":read-only"),
            Self::ReadWrite => dest.write_str(":read-write"),
            Self::PlaceholderShown => dest.write_str(":placeholder-shown"),
            Self::Valid => dest.write_str(":valid"),
            Self::Invalid => dest.write_str(":invalid"),
            Self::InRange => dest.write_str(":in-range"),
            Self::OutOfRange => dest.write_str(":out-of-range"),
            Self::Required => dest.write_str(":required"),
            Self::Optional => dest.write_str(":optional"),
        }
    }
}

/// Псевдоэлементы, поддерживаемые в Zver (Фаза 3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    Before,
    After,
    FirstLine,
    FirstLetter,
}

impl parser::PseudoElement for PseudoElement {
    type Impl = Simple;
}

impl cssparser::ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::Before => dest.write_str("::before"),
            Self::After => dest.write_str("::after"),
            Self::FirstLine => dest.write_str("::first-line"),
            Self::FirstLetter => dest.write_str("::first-letter"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SelectorParser;

impl<'i> parser::Parser<'i> for SelectorParser {
    type Impl = Simple;
    type Error = SelectorParseErrorKind<'i>;

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }

    fn parse_nth_child_of(&self) -> bool {
        false // Пока не поддерживаем селекторы "of" в nth-child
    }

    fn parse_non_ts_pseudo_class(
        &self,
        _location: cssparser::SourceLocation,
        name: cssparser::CowRcStr<'i>,
    ) -> Result<NonTSPseudoClass, cssparser::ParseError<'i, Self::Error>> {
        use NonTSPseudoClass::*;

        match &*name {
            // Структурные псевдоклассы
            "first-child" => Ok(FirstChild),
            "last-child" => Ok(LastChild),
            "only-child" => Ok(OnlyChild),
            "first-of-type" => Ok(FirstOfType),
            "last-of-type" => Ok(LastOfType),

            // Псевдоклассы состояния
            "hover" => Ok(Hover),
            "focus" => Ok(Focus),
            "active" => Ok(Active),
            "disabled" => Ok(Disabled),
            "enabled" => Ok(Enabled),
            "checked" => Ok(Checked),
            "indeterminate" => Ok(Indeterminate),

            // Псевдоклассы ссылок
            "link" => Ok(Link),
            "visited" => Ok(Visited),
            "any-link" => Ok(AnyLink),

            // Псевдоклассы формы
            "read-only" => Ok(ReadOnly),
            "read-write" => Ok(ReadWrite),
            "placeholder-shown" => Ok(PlaceholderShown),
            "valid" => Ok(Valid),
            "invalid" => Ok(Invalid),
            "in-range" => Ok(InRange),
            "out-of-range" => Ok(OutOfRange),
            "required" => Ok(Required),
            "optional" => Ok(Optional),

            _ => Err(cssparser::ParseError {
                kind: cssparser::ParseErrorKind::Custom(
                    SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name),
                ),
                location: _location,
            }),
        }
    }

    fn parse_pseudo_element(
        &self,
        _location: cssparser::SourceLocation,
        name: cssparser::CowRcStr<'i>,
    ) -> Result<PseudoElement, cssparser::ParseError<'i, Self::Error>> {
        use PseudoElement::*;

        match &*name {
            "before" => Ok(Before),
            "after" => Ok(After),
            "first-line" => Ok(FirstLine),
            "first-letter" => Ok(FirstLetter),
            _ => Err(cssparser::ParseError {
                kind: cssparser::ParseErrorKind::Custom(
                    SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name),
                ),
                location: _location,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CssString(pub String);

impl<'a> From<&'a str> for CssString {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for CssString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for CssString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CssString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl cssparser::ToCss for CssString {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        serialize_string(&self.0, dest)
    }
}

impl PrecomputedHash for CssString {
    fn precomputed_hash(&self) -> u32 {
        hash_bytes(self.0.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CssLocalName(pub String);

impl<'a> From<&'a str> for CssLocalName {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for CssLocalName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for CssLocalName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl cssparser::ToCss for CssLocalName {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl PrecomputedHash for CssLocalName {
    fn precomputed_hash(&self) -> u32 {
        hash_bytes(self.0.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CssNamespace(pub String);

impl<'a> From<&'a str> for CssNamespace {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for CssNamespace {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for CssNamespace {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl cssparser::ToCss for CssNamespace {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl PrecomputedHash for CssNamespace {
    fn precomputed_hash(&self) -> u32 {
        hash_bytes(self.0.as_bytes())
    }
}

fn hash_bytes(bytes: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 0x811C9DC5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn namespace_matches(ns: &NamespaceConstraint<&CssNamespace>) -> bool {
    match ns {
        NamespaceConstraint::Any => true,
        NamespaceConstraint::Specific(namespace) => {
            let ns = namespace.as_ref();
            ns.is_empty() || ns.eq_ignore_ascii_case("http://www.w3.org/1999/xhtml")
        }
    }
}

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
use crate::dom::{Document, Node};

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
        let mut caches = self.caches.lock().expect("selector cache poisoned");
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
        _pc: &NonTSPseudoClass,
        _context: &mut MatchingContext<'_, Self::Impl>,
    ) -> bool {
        false
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonTSPseudoClass {}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = Simple;

    fn is_active_or_hover(&self) -> bool {
        false
    }

    fn is_user_action_state(&self) -> bool {
        false
    }
}

impl cssparser::ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoElement {}

impl parser::PseudoElement for PseudoElement {
    type Impl = Simple;
}

impl cssparser::ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        Ok(())
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

use crate::css::selectors::PseudoElement;
use bitflags::bitflags;
use std::collections::HashMap;

bitflags! {
    /// UI-состояния элемента, используемые в псевдоклассах.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ElementState: u16 {
        const HOVER = 1 << 0;
        const FOCUS = 1 << 1;
        const ACTIVE = 1 << 2;
        const DISABLED = 1 << 3;
        const CHECKED = 1 << 4;
        const INDETERMINATE = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub element_state: ElementState,
    pub pseudo_element: Option<PseudoElement>,
}

impl Node {
    pub fn is_element(&self) -> bool {
        self.tag_name.is_some()
    }

    pub fn tag_name(&self) -> Option<&str> {
        self.tag_name.as_deref()
    }

    pub fn new_element(id: usize, tag_name: String, parent: Option<usize>) -> Self {
        Self {
            id,
            tag_name: Some(tag_name),
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            parent,
            element_state: ElementState::default(),
            pseudo_element: None,
        }
    }

    pub fn new_text(id: usize, text: String, parent: Option<usize>) -> Self {
        Self {
            id,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: Some(text),
            children: Vec::new(),
            parent,
            element_state: ElementState::default(),
            pseudo_element: None,
        }
    }
}

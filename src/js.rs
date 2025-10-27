use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum JSValue {
    Undefined,
    Number(f64),
    String(String),
    Boolean(bool),
    Object(HashMap<String, JSValue>),
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct JSEngine {
    variables: HashMap<String, JSValue>,
}

impl JSEngine {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn execute(&mut self, _code: &str) -> Result<JSValue, Box<dyn std::error::Error>> {
        Ok(JSValue::Undefined)
    }
}


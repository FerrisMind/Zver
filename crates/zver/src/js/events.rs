use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Типы DOM событий
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Click,
    Input,
    Change,
    KeyDown,
    KeyUp,
    MouseOver,
    MouseOut,
    Focus,
    Blur,
    Submit,
    Load,
    Custom(String),
}

impl EventType {
    pub fn parse(s: &str) -> Self {
        match s {
            "click" => EventType::Click,
            "input" => EventType::Input,
            "change" => EventType::Change,
            "keydown" => EventType::KeyDown,
            "keyup" => EventType::KeyUp,
            "mouseover" => EventType::MouseOver,
            "mouseout" => EventType::MouseOut,
            "focus" => EventType::Focus,
            "blur" => EventType::Blur,
            "submit" => EventType::Submit,
            "load" => EventType::Load,
            other => EventType::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            EventType::Click => "click",
            EventType::Input => "input",
            EventType::Change => "change",
            EventType::KeyDown => "keydown",
            EventType::KeyUp => "keyup",
            EventType::MouseOver => "mouseover",
            EventType::MouseOut => "mouseout",
            EventType::Focus => "focus",
            EventType::Blur => "blur",
            EventType::Submit => "submit",
            EventType::Load => "load",
            EventType::Custom(s) => s.as_str(),
        }
    }
}

/// Event listener - хранит код callback'а
#[derive(Debug, Clone)]
pub struct EventListener {
    pub callback_code: String,
    pub once: bool,
}

/// Event target - узел DOM с привязанными слушателями
#[derive(Debug, Clone, Default)]
pub struct EventTarget {
    pub listeners: HashMap<EventType, Vec<EventListener>>,
}

impl EventTarget {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn add_listener(&mut self, event_type: EventType, callback_code: String, once: bool) {
        let listener = EventListener {
            callback_code,
            once,
        };

        self.listeners.entry(event_type).or_default().push(listener);
    }

    pub fn remove_listener(&mut self, event_type: &EventType, callback_code: &str) {
        if let Some(listeners) = self.listeners.get_mut(event_type) {
            listeners.retain(|l| l.callback_code != callback_code);
        }
    }

    pub fn get_listeners(&self, event_type: &EventType) -> Option<&Vec<EventListener>> {
        self.listeners.get(event_type)
    }

    pub fn remove_once_listeners(&mut self, event_type: &EventType) {
        if let Some(listeners) = self.listeners.get_mut(event_type) {
            listeners.retain(|l| !l.once);
        }
    }
}

/// Event registry - глобальное хранилище слушателей для всех узлов
#[derive(Debug, Clone, Default)]
pub struct EventRegistry {
    targets: Arc<Mutex<HashMap<usize, EventTarget>>>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            targets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_listener(
        &self,
        node_id: usize,
        event_type: EventType,
        callback_code: String,
        once: bool,
    ) {
        if let Ok(mut targets) = self.targets.lock() {
            let target = targets.entry(node_id).or_insert_with(EventTarget::new);
            target.add_listener(event_type, callback_code, once);
        }
    }

    pub fn remove_listener(&self, node_id: usize, event_type: &EventType, callback_code: &str) {
        if let Ok(mut targets) = self.targets.lock()
            && let Some(target) = targets.get_mut(&node_id)
        {
            target.remove_listener(event_type, callback_code);
        }
    }

    pub fn get_listeners(&self, node_id: usize, event_type: &EventType) -> Vec<EventListener> {
        if let Ok(targets) = self.targets.lock()
            && let Some(target) = targets.get(&node_id)
            && let Some(listeners) = target.get_listeners(event_type)
        {
            return listeners.clone();
        }
        Vec::new()
    }

    pub fn remove_once_listeners(&self, node_id: usize, event_type: &EventType) {
        if let Ok(mut targets) = self.targets.lock()
            && let Some(target) = targets.get_mut(&node_id)
        {
            target.remove_once_listeners(event_type);
        }
    }

    pub fn clear_node(&self, node_id: usize) {
        if let Ok(mut targets) = self.targets.lock() {
            targets.remove(&node_id);
        }
    }
}

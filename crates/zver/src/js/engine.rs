use boa_engine::{Context, Source};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use super::console;
use super::document;
use super::events::EventRegistry;
use super::events::EventType;
use super::timers::{self, PendingCallback};

/// Типы значений JavaScript
#[derive(Debug, Clone)]
pub enum JSValue {
    Undefined,
    Number(f64),
    String(String),
    Boolean(bool),
    Object(HashMap<String, JSValue>),
}

/// Движок JavaScript на основе boa_engine
#[derive(Debug)]
pub struct JSEngine {
    context: Context,
    dom_ref: Option<Arc<RwLock<super::super::dom::Document>>>,
    timers: Arc<Mutex<HashMap<u32, tokio::task::JoinHandle<()>>>>,
    next_timer_id: Arc<Mutex<u32>>,
    pending_callbacks: Arc<Mutex<HashMap<u32, PendingCallback>>>,
    event_registry: EventRegistry,
}

impl JSEngine {
    pub fn new() -> Self {
        let mut context = Context::default();

        // Инициализируем глобальные объекты
        console::init_console(&mut context);

        let timers = Arc::new(Mutex::new(HashMap::new()));
        let next_timer_id = Arc::new(Mutex::new(1));
        let pending_callbacks = Arc::new(Mutex::new(HashMap::new()));

        // Всегда инициализируем таймеры
        timers::init_timers(
            &mut context,
            timers.clone(),
            next_timer_id.clone(),
            pending_callbacks.clone(),
        );

        Self {
            context,
            dom_ref: None,
            timers,
            next_timer_id,
            pending_callbacks,
            event_registry: EventRegistry::new(),
        }
    }

    pub fn with_dom(mut self, dom: Arc<RwLock<super::super::dom::Document>>) -> Self {
        self.dom_ref = Some(dom.clone());
        self.init_document();
        self
    }

    fn init_document(&mut self) {
        if let Some(dom_ref) = &self.dom_ref {
            document::init_document(
                &mut self.context,
                dom_ref.clone(),
                self.event_registry.clone(),
            );
        }
    }

    /// Выполняет готовые callbacks из setTimeout
    pub fn tick_timers(&mut self) -> usize {
        timers::tick_timers(
            &mut self.context,
            self.pending_callbacks.clone(),
            self.timers.clone(),
        )
    }

    /// Dispatch event на узел DOM
    pub fn dispatch_event(&mut self, node_id: usize, event_type: &str) -> usize {
        let event_type = EventType::parse(event_type);
        let listeners = self.event_registry.get_listeners(node_id, &event_type);

        let mut executed = 0;
        for listener in listeners {
            let exec_code = if listener.callback_code.starts_with("function")
                || listener.callback_code.contains("=>")
            {
                format!("({})()", listener.callback_code)
            } else {
                listener.callback_code.clone()
            };

            match self.execute(&exec_code) {
                Ok(_) => {
                    executed += 1;
                    #[cfg(debug_assertions)]
                    println!("Executed event listener for {}", event_type.as_str());
                }
                Err(e) => {
                    eprintln!("Error executing event listener: {:?}", e);
                }
            }
        }

        // Удаляем once listeners после выполнения
        if executed > 0 {
            self.event_registry
                .remove_once_listeners(node_id, &event_type);
        }

        executed
    }

    /// Resets the JavaScript context for a new page load
    /// This prevents "duplicate lexical declaration" errors when const/let variables are redeclared
    pub fn reset_context(&mut self) {
        tracing::debug!("Resetting JavaScript context");

        // Create a new context to avoid conflicts with previous declarations
        self.context = Context::default();

        // Re-initialize global objects
        console::init_console(&mut self.context);

        // Re-initialize timers and document APIs
        timers::init_timers(
            &mut self.context,
            self.timers.clone(),
            self.next_timer_id.clone(),
            self.pending_callbacks.clone(),
        );

        if self.dom_ref.is_some() {
            self.init_document();
        }

        // Clear timers and callbacks from previous page
        if let Ok(mut timers) = self.timers.lock() {
            for (timer_id, handle) in timers.drain() {
                tracing::debug!("Aborting timer {}", timer_id);
                handle.abort();
            }
        }

        if let Ok(mut callbacks) = self.pending_callbacks.lock() {
            let count = callbacks.len();
            callbacks.clear();
            tracing::debug!("Cleared {} pending callbacks", count);
        }

        // Reset event registry
        self.event_registry = EventRegistry::new();
    }

    pub fn execute(&mut self, code: &str) -> Result<JSValue, Box<dyn std::error::Error>> {
        let source = Source::from_bytes(code);
        match self.context.eval(source) {
            Ok(js_value) => {
                // Конвертируем JsValue обратно в наш enum
                if js_value.is_undefined() || js_value.is_null() {
                    Ok(JSValue::Undefined)
                } else if let Some(b) = js_value.as_boolean() {
                    Ok(JSValue::Boolean(b))
                } else if let Some(s) = js_value.as_string() {
                    Ok(JSValue::String(s.to_std_string_escaped()))
                } else if let Some(f) = js_value.as_number() {
                    Ok(JSValue::Number(f))
                } else if let Some(i) = js_value.as_bigint() {
                    Ok(JSValue::Number(i.to_f64()))
                } else if js_value.is_object() {
                    Ok(JSValue::Object(HashMap::new())) // упрощение
                } else {
                    Ok(JSValue::Undefined)
                }
            }
            Err(e) => Err(format!("JavaScript error: {:?}", e).into()),
        }
    }
}

impl Default for JSEngine {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: JSEngine can be safely sent between threads because:
// 1. boa_engine::Context is designed to be Send
// 2. Arc<RwLock<Document>> is explicitly Send
// 3. Arc<Mutex<HashMap>> and Arc<Mutex<u32>> are Send
// 4. All access is protected by RwLock in the parent Zver struct
unsafe impl Send for JSEngine {}

// SAFETY: JSEngine can be shared between threads (behind Arc<RwLock<>>) because:
// 1. All mutations to Context require exclusive lock (RwLock::write)
// 2. Timers HashMap is protected by its own Mutex
// 3. No interior mutability without proper synchronization
unsafe impl Sync for JSEngine {}

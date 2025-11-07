pub mod events;

use boa_engine::{
    Context, JsValue, NativeFunction, js_string, object::ObjectInitializer, property::Attribute,
};
use events::{EventRegistry, EventType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct PendingCallback {
    pub code: String,
    pub fire_time: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum JSValue {
    Undefined,
    Number(f64),
    String(String),
    Boolean(bool),
    Object(HashMap<String, JSValue>),
}

#[derive(Debug)]
pub struct JSEngine {
    context: Context,
    dom_ref: Option<Arc<RwLock<super::dom::Document>>>,
    timers: Arc<Mutex<HashMap<u32, tokio::task::JoinHandle<()>>>>,
    next_timer_id: Arc<Mutex<u32>>,
    pending_callbacks: Arc<Mutex<HashMap<u32, PendingCallback>>>,
    event_registry: EventRegistry,
}

impl JSEngine {
    pub fn new() -> Self {
        let mut context = Context::default();

        // Инициализируем глобальные объекты
        Self::init_console(&mut context);

        let mut engine = Self {
            context,
            dom_ref: None,
            timers: Arc::new(Mutex::new(HashMap::new())),
            next_timer_id: Arc::new(Mutex::new(1)),
            pending_callbacks: Arc::new(Mutex::new(HashMap::new())),
            event_registry: EventRegistry::new(),
        };

        // Всегда инициализируем таймеры
        engine.init_timers();
        engine
    }

    pub fn with_dom(mut self, dom: Arc<RwLock<super::dom::Document>>) -> Self {
        self.dom_ref = Some(dom.clone());
        self.init_document();
        self
    }

    fn init_console(context: &mut Context) {
        // console.log
        let console_log = NativeFunction::from_fn_ptr(|_this, args, _context| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.display());
            }
            println!();
            Ok(JsValue::undefined())
        });

        // console.error
        let console_error = NativeFunction::from_fn_ptr(|_this, args, _context| {
            eprint!("ERROR: ");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    eprint!(" ");
                }
                eprint!("{}", arg.display());
            }
            eprintln!();
            Ok(JsValue::undefined())
        });

        // console.warn
        let console_warn = NativeFunction::from_fn_ptr(|_this, args, _context| {
            eprint!("WARN: ");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    eprint!(" ");
                }
                eprint!("{}", arg.display());
            }
            eprintln!();
            Ok(JsValue::undefined())
        });

        let console = ObjectInitializer::new(context)
            .function(console_log, js_string!("log"), 0)
            .function(console_error, js_string!("error"), 0)
            .function(console_warn, js_string!("warn"), 0)
            .build();

        let _ = context.register_global_property(
            js_string!("console"),
            console,
            Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
        );
    }

    fn init_document(&mut self) {
        if let Some(dom_ref) = &self.dom_ref {
            let event_registry = self.event_registry.clone();

            // document.querySelector
            // SAFETY: NativeFunction::from_closure is unsafe because:
            // 1. The closure captures dom_ref_clone which is Arc<RwLock<Document>>
            // 2. Arc is thread-safe and the closure doesn't outlive the JSEngine
            // 3. try_read() ensures we never hold a lock across JS execution boundaries
            // 4. The closure doesn't contain any 'static references that could be invalidated
            let query_selector_fn = {
                let dom_ref = dom_ref.clone();
                let event_registry = event_registry.clone();
                unsafe {
                    NativeFunction::from_closure(move |_this, args, context| {
                        if let Some(selector_js) = args.first()
                            && let Some(selector_str) = selector_js.as_string()
                            && let Ok(dom) = dom_ref.try_read()
                        {
                            let ids =
                                dom.query_selector(selector_str.to_std_string_escaped().as_str());
                            if let Some(&first_id) = ids.first() {
                                drop(dom); // Release read lock before creating object
                                let element = Self::create_element_object(
                                    context,
                                    first_id,
                                    dom_ref.clone(),
                                    event_registry.clone(),
                                );
                                return Ok(JsValue::from(element));
                            }
                        }
                        Ok(JsValue::null())
                    })
                }
            };

            // document.getElementById
            // SAFETY: Same reasoning as query_selector_fn above
            // The closure is self-contained and doesn't leak references
            let get_element_by_id_fn = {
                let dom_ref = dom_ref.clone();
                let event_registry = event_registry.clone();
                unsafe {
                    NativeFunction::from_closure(move |_this, args, context| {
                        if let Some(id_js) = args.first()
                            && let Some(id_str) = id_js.as_string()
                            && let Ok(dom) = dom_ref.try_read()
                            && let Some(node_id) =
                                dom.get_element_by_id(id_str.to_std_string_escaped().as_str())
                        {
                            drop(dom); // Release read lock
                            let element = Self::create_element_object(
                                context,
                                node_id,
                                dom_ref.clone(),
                                event_registry.clone(),
                            );
                            return Ok(JsValue::from(element));
                        }
                        Ok(JsValue::null())
                    })
                }
            };

            // document.createElement
            let create_elem_fn = {
                let dom_ref = dom_ref.clone();
                let event_registry = event_registry.clone();
                unsafe {
                    NativeFunction::from_closure(move |_this, args, context| {
                        if let Some(tag_js) = args.first()
                            && let Some(tag_str) = tag_js.as_string()
                            && let Ok(mut dom) = dom_ref.try_write()
                        {
                            match dom.create_element(tag_str.to_std_string_escaped().as_str()) {
                                Ok(node_id) => {
                                    drop(dom); // Release write lock
                                    let element = Self::create_element_object(
                                        context,
                                        node_id,
                                        dom_ref.clone(),
                                        event_registry.clone(),
                                    );
                                    return Ok(JsValue::from(element));
                                }
                                Err(e) => {
                                    eprintln!("Failed to create element: {}", e);
                                    return Ok(JsValue::null());
                                }
                            }
                        }
                        Ok(JsValue::null())
                    })
                }
            };

            let document = ObjectInitializer::new(&mut self.context)
                .function(query_selector_fn, js_string!("querySelector"), 0)
                .function(get_element_by_id_fn, js_string!("getElementById"), 0)
                .function(create_elem_fn, js_string!("createElement"), 1)
                .build();

            let _ = self.context.register_global_property(
                js_string!("document"),
                document,
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            );
        }
    }

    fn create_element_object(
        context: &mut Context,
        node_id: usize,
        dom_ref: Arc<RwLock<super::dom::Document>>,
        event_registry: EventRegistry,
    ) -> boa_engine::object::JsObject {
        let dom_ref_clone = dom_ref.clone();

        // appendChild
        let append_child_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(parent_obj) = this.as_object()
                        && let Ok(parent_id_val) = parent_obj.get(js_string!("nodeId"), context)
                        && let Some(parent_id) = parent_id_val.as_number()
                        && let Some(child_arg) = args.first()
                        && let Some(child_obj) = child_arg.as_object()
                        && let Ok(child_id_val) = child_obj.get(js_string!("nodeId"), context)
                        && let Some(child_id) = child_id_val.as_number()
                        && let Ok(mut dom) = dom_ref.try_write()
                    {
                        match dom.append_child(parent_id as usize, child_id as usize) {
                            Ok(_) => return Ok(child_arg.clone()),
                            Err(e) => {
                                return Ok({
                                    eprintln!("appendChild failed: {}", e);
                                    JsValue::null()
                                });
                            }
                        }
                    }
                    Ok({
                        eprintln!("Invalid appendChild call");
                        JsValue::null()
                    })
                })
            }
        };

        // removeChild
        let remove_child_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(parent_obj) = this.as_object()
                        && let Ok(parent_id_val) = parent_obj.get(js_string!("nodeId"), context)
                        && let Some(parent_id) = parent_id_val.as_number()
                        && let Some(child_arg) = args.first()
                        && let Some(child_obj) = child_arg.as_object()
                        && let Ok(child_id_val) = child_obj.get(js_string!("nodeId"), context)
                        && let Some(child_id) = child_id_val.as_number()
                        && let Ok(mut dom) = dom_ref.try_write()
                    {
                        match dom.remove_child(parent_id as usize, child_id as usize) {
                            Ok(_) => return Ok(child_arg.clone()),
                            Err(e) => {
                                return Ok({
                                    eprintln!("removeChild failed: {}", e);
                                    JsValue::null()
                                });
                            }
                        }
                    }
                    Ok({
                        eprintln!("Invalid removeChild call");
                        JsValue::null()
                    })
                })
            }
        };

        // setAttribute
        let set_attr_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Some(name_js) = args.first()
                        && let Some(name_str) = name_js.as_string()
                        && let Some(value_js) = args.get(1)
                        && let Some(value_str) = value_js.as_string()
                        && let Ok(mut dom) = dom_ref.try_write()
                    {
                        let name = name_str.to_std_string_escaped();
                        let value = value_str.to_std_string_escaped();
                        match dom.set_attribute(node_id as usize, &name, &value) {
                            Ok(_) => return Ok(JsValue::undefined()),
                            Err(e) => {
                                return Ok({
                                    eprintln!("setAttribute failed: {}", e);
                                    JsValue::null()
                                });
                            }
                        }
                    }
                    Ok({
                        eprintln!("Invalid setAttribute call");
                        JsValue::null()
                    })
                })
            }
        };

        // getAttribute
        let get_attr_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Some(name_js) = args.first()
                        && let Some(name_str) = name_js.as_string()
                        && let Ok(dom) = dom_ref.try_read()
                    {
                        let name = name_str.to_std_string_escaped();
                        if let Some(value) = dom.get_attribute(node_id as usize, &name) {
                            return Ok(JsValue::from(js_string!(value)));
                        }
                        return Ok(JsValue::null());
                    }
                    Ok({
                        eprintln!("Invalid getAttribute call");
                        JsValue::null()
                    })
                })
            }
        };

        // textContent getter
        let get_text_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, _args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Ok(dom) = dom_ref.try_read()
                    {
                        let text = dom.get_text_content(node_id as usize);
                        return Ok(JsValue::from(js_string!(text)));
                    }
                    Ok(JsValue::from(js_string!("")))
                })
            }
        };

        // textContent setter
        let set_text_fn = {
            let dom_ref = dom_ref.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Some(text_js) = args.first()
                    {
                        let text = text_js
                            .to_string(context)
                            .unwrap_or_default()
                            .to_std_string_escaped();
                        if let Ok(mut dom) = dom_ref.try_write() {
                            match dom.set_text_content(node_id as usize, &text) {
                                Ok(_) => return Ok(JsValue::undefined()),
                                Err(e) => {
                                    return Ok({
                                        eprintln!("set textContent failed: {}", e);
                                        JsValue::null()
                                    });
                                }
                            }
                        }
                    }
                    Ok({
                        eprintln!("Invalid textContent setter call");
                        JsValue::null()
                    })
                })
            }
        };

        // tagName getter
        let tag_name_fn = {
            unsafe {
                NativeFunction::from_closure(move |this, _args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Ok(dom) = dom_ref_clone.try_read()
                        && let Some(tag) = dom.get_tag_name(node_id as usize)
                    {
                        return Ok(JsValue::from(js_string!(tag.to_uppercase())));
                    }
                    Ok(JsValue::null())
                })
            }
        };

        // addEventListener
        let add_event_listener_fn = {
            let event_registry = event_registry.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Some(event_type_js) = args.first()
                        && let Some(event_type_str) = event_type_js.as_string()
                        && let Some(callback_val) = args.get(1)
                    {
                        let event_type = EventType::parse(&event_type_str.to_std_string_escaped());

                        // Конвертируем callback в строку кода
                        let callback_code = if let Some(s) = callback_val.as_string() {
                            s.to_std_string_escaped()
                        } else if callback_val.is_callable() {
                            match callback_val.to_string(context) {
                                Ok(s) => s.to_std_string_escaped(),
                                Err(_) => {
                                    eprintln!("Failed to convert event callback to string");
                                    return Ok(JsValue::undefined());
                                }
                            }
                        } else {
                            return Ok(JsValue::undefined());
                        };

                        // Опции (пока поддерживаем только once)
                        let once = args
                            .get(2)
                            .and_then(|v| v.as_object())
                            .and_then(|obj| obj.get(js_string!("once"), context).ok())
                            .and_then(|v| v.as_boolean())
                            .unwrap_or(false);

                        event_registry.add_listener(
                            node_id as usize,
                            event_type,
                            callback_code,
                            once,
                        );

                        return Ok(JsValue::undefined());
                    }
                    Ok({
                        eprintln!("Invalid addEventListener call");
                        JsValue::undefined()
                    })
                })
            }
        };

        // removeEventListener
        let remove_event_listener_fn = {
            let event_registry = event_registry.clone();
            unsafe {
                NativeFunction::from_closure(move |this, args, context| {
                    if let Some(elem_obj) = this.as_object()
                        && let Ok(node_id_val) = elem_obj.get(js_string!("nodeId"), context)
                        && let Some(node_id) = node_id_val.as_number()
                        && let Some(event_type_js) = args.first()
                        && let Some(event_type_str) = event_type_js.as_string()
                        && let Some(callback_val) = args.get(1)
                    {
                        let event_type = EventType::parse(&event_type_str.to_std_string_escaped());

                        let callback_code = if let Some(s) = callback_val.as_string() {
                            s.to_std_string_escaped()
                        } else if callback_val.is_callable() {
                            match callback_val.to_string(context) {
                                Ok(s) => s.to_std_string_escaped(),
                                Err(_) => return Ok(JsValue::undefined()),
                            }
                        } else {
                            return Ok(JsValue::undefined());
                        };

                        event_registry.remove_listener(
                            node_id as usize,
                            &event_type,
                            &callback_code,
                        );

                        return Ok(JsValue::undefined());
                    }
                    Ok(JsValue::undefined())
                })
            }
        };

        ObjectInitializer::new(context)
            .property(
                js_string!("nodeId"),
                JsValue::from(node_id as f64),
                Attribute::READONLY,
            )
            .function(append_child_fn, js_string!("appendChild"), 1)
            .function(remove_child_fn, js_string!("removeChild"), 1)
            .function(set_attr_fn, js_string!("setAttribute"), 2)
            .function(get_attr_fn, js_string!("getAttribute"), 1)
            .function(get_text_fn, js_string!("getTextContent"), 0)
            .function(set_text_fn, js_string!("setTextContent"), 1)
            .function(tag_name_fn, js_string!("getTagName"), 0)
            .function(add_event_listener_fn, js_string!("addEventListener"), 2)
            .function(
                remove_event_listener_fn,
                js_string!("removeEventListener"),
                2,
            )
            .build()
    }

    fn init_timers(&mut self) {
        let timers = self.timers.clone();
        let next_id = self.next_timer_id.clone();
        let pending_callbacks = self.pending_callbacks.clone();

        // setTimeout
        // SAFETY: The closure captures Arc<Mutex<>> which are thread-safe
        // The spawned tokio task is independent and doesn't access JSEngine state
        // Timer handles are stored separately and can be safely accessed
        let set_timeout_fn = unsafe {
            NativeFunction::from_closure(move |_this, args, context| {
                // Поддерживаем две формы: setTimeout("code", delay) и setTimeout(function, delay)
                if let Some(callback_val) = args.first() {
                    let delay_ms = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;

                    // Конвертируем callback в строку кода
                    let callback_code = if let Some(s) = callback_val.as_string() {
                        // Строка - это уже код
                        s.to_std_string_escaped()
                    } else if callback_val.is_callable() {
                        // Функция - вызываем её toString()
                        match callback_val.to_string(context) {
                            Ok(s) => s.to_std_string_escaped(),
                            Err(_) => {
                                eprintln!("Failed to convert callback to string");
                                return Ok(JsValue::undefined());
                            }
                        }
                    } else {
                        return Ok(JsValue::undefined());
                    };

                    let timer_id = {
                        let mut timer_id_lock = match next_id.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => {
                                #[cfg(debug_assertions)]
                                eprintln!("Warning: Timer ID mutex was poisoned, recovering");
                                poisoned.into_inner()
                            }
                        };
                        let id = *timer_id_lock;
                        *timer_id_lock += 1;
                        id
                    };

                    // Сохраняем callback для последующего выполнения
                    let fire_time =
                        std::time::Instant::now() + std::time::Duration::from_millis(delay_ms);
                    if let Ok(mut pending) = pending_callbacks.lock() {
                        pending.insert(
                            timer_id,
                            PendingCallback {
                                code: callback_code,
                                fire_time,
                            },
                        );
                    }

                    // Планируем таймер
                    let pending_clone = pending_callbacks.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        // Таймер сработал - callback будет выполнен в следующем tick_timers()
                        #[cfg(debug_assertions)]
                        {
                            if let Ok(pending) = pending_clone.lock()
                                && pending.contains_key(&timer_id)
                            {
                                println!("Timer {} ready to fire after {}ms", timer_id, delay_ms);
                            }
                        }
                    });

                    if let Ok(mut timers_lock) = timers.lock() {
                        timers_lock.insert(timer_id, handle);
                    } else {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "Warning: Failed to acquire timers lock, timer {} may not be tracked",
                            timer_id
                        );
                    }

                    return Ok(JsValue::from(timer_id as f64));
                }
                Ok(JsValue::undefined())
            })
        };

        let _ = self.context.register_global_builtin_callable(
            js_string!("setTimeout"),
            0,
            set_timeout_fn,
        );
    }

    /// Выполняет готовые callbacks из setTimeout
    pub fn tick_timers(&mut self) -> usize {
        let now = std::time::Instant::now();
        let mut executed = 0;

        // Собираем callbacks, которые нужно выполнить
        let ready_callbacks: Vec<(u32, String)> = {
            let pending = match self.pending_callbacks.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    eprintln!("Warning: pending_callbacks mutex poisoned");
                    poisoned.into_inner()
                }
            };

            pending
                .iter()
                .filter(|(_, cb)| cb.fire_time <= now)
                .map(|(id, cb)| (*id, cb.code.clone()))
                .collect()
        };

        // Выполняем callbacks и удаляем их из pending
        for (timer_id, code) in ready_callbacks {
            // Пытаемся выполнить как код или как вызов функции
            let exec_code = if code.starts_with("function") || code.contains("=>") {
                format!("({})()", code)
            } else {
                code.clone()
            };

            match self.execute(&exec_code) {
                Ok(_) => {
                    executed += 1;
                    #[cfg(debug_assertions)]
                    println!("Executed timer {} callback", timer_id);
                }
                Err(e) => {
                    eprintln!("Error executing timer {} callback: {:?}", timer_id, e);
                }
            }

            // Удаляем выполненный callback
            if let Ok(mut pending) = self.pending_callbacks.lock() {
                pending.remove(&timer_id);
            }

            // Удаляем завершённый таймер
            if let Ok(mut timers) = self.timers.lock() {
                timers.remove(&timer_id);
            }
        }

        executed
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

    pub fn execute(&mut self, code: &str) -> Result<JSValue, Box<dyn std::error::Error>> {
        use boa_engine::Source;

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

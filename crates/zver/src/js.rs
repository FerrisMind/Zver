use boa_engine::{
    js_string, object::ObjectInitializer, property::Attribute, Context, JsValue, NativeFunction,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    timers: Arc<RwLock<HashMap<u32, tokio::task::JoinHandle<()>>>>,
    next_timer_id: Arc<RwLock<u32>>,
}

impl JSEngine {
    pub fn new() -> Self {
        let mut context = Context::default();

        // Инициализируем глобальные объекты
        Self::init_console(&mut context);

        Self {
            context,
            dom_ref: None,
            timers: Arc::new(RwLock::new(HashMap::new())),
            next_timer_id: Arc::new(RwLock::new(1)),
        }
    }

    pub fn with_dom(mut self, dom: Arc<RwLock<super::dom::Document>>) -> Self {
        self.dom_ref = Some(dom.clone());
        self.init_document();
        self.init_timers();
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
            let dom_ref_clone = dom_ref.clone();
            
            // document.querySelector
            let query_selector_fn = unsafe { NativeFunction::from_closure(move |_this, args, _context| {
                if let Some(selector_js) = args.first()
                    && let Some(selector_str) = selector_js.as_string()
                    && let Ok(dom) = dom_ref_clone.try_read() {
                        let ids = dom.query_selector(selector_str.to_std_string_escaped().as_str());
                        if let Some(first_id) = ids.first() {
                            // Возвращаем объект-элемент с id и методами
                            let element = ObjectInitializer::new(_context)
                                .property(
                                    js_string!("id"),
                                    JsValue::from(*first_id as f64),
                                    Attribute::READONLY,
                                )
                                .build();
                            return Ok(JsValue::from(element));
                        }
                    }
                Ok(JsValue::null())
            }) };
            
            // document.getElementById
            let dom_ref_clone2 = dom_ref.clone();
            let get_element_by_id_fn = unsafe { NativeFunction::from_closure(move |_this, args, _context| {
                if let Some(id_js) = args.first()
                    && let Some(id_str) = id_js.as_string()
                    && let Ok(dom) = dom_ref_clone2.try_read()
                    && let Some(node_id) = dom.get_element_by_id(id_str.to_std_string_escaped().as_str()) {
                        let element = ObjectInitializer::new(_context)
                            .property(
                                js_string!("id"),
                                JsValue::from(node_id as f64),
                                Attribute::READONLY,
                            )
                            .build();
                        return Ok(JsValue::from(element));
                    }
                Ok(JsValue::null())
            }) };

            let document = ObjectInitializer::new(&mut self.context)
                .function(query_selector_fn, js_string!("querySelector"), 0)
                .function(get_element_by_id_fn, js_string!("getElementById"), 0)
                .build();

            let _ = self.context.register_global_property(
                js_string!("document"),
                document,
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            );
        }
    }
    
    fn init_timers(&mut self) {
        let timers = self.timers.clone();
        let next_id = self.next_timer_id.clone();
        
        // setTimeout
        let set_timeout_fn = unsafe { NativeFunction::from_closure(move |_this, args, _context| {
            if let Some(callback) = args.first()
                && callback.is_callable() {
                    let delay_ms = args.get(1)
                        .and_then(|v| v.as_number())
                        .unwrap_or(0.0) as u64;
                    
                    let mut timer_id_lock = tokio::runtime::Handle::current().block_on(next_id.write());
                    let timer_id = *timer_id_lock;
                    *timer_id_lock += 1;
                    drop(timer_id_lock);
                    
                    // Планируем выполнение (в реальности нужен более сложный механизм)
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        // Здесь должен быть вызов callback в контексте JS
                        println!("Timer {} fired after {}ms", timer_id, delay_ms);
                    });
                    
                    let mut timers_lock = tokio::runtime::Handle::current().block_on(timers.write());
                    timers_lock.insert(timer_id, handle);
                    
                    return Ok(JsValue::from(timer_id as f64));
                }
            Ok(JsValue::undefined())
        }) };
        
        let _ = self.context.register_global_builtin_callable(
            js_string!("setTimeout"),
            0,
            set_timeout_fn,
        );
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

unsafe impl Send for JSEngine {}
unsafe impl Sync for JSEngine {}


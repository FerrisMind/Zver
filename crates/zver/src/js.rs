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
}

impl JSEngine {
    pub fn new() -> Self {
        let mut context = Context::default();

        // Инициализируем глобальные объекты
        Self::init_console(&mut context);

        Self {
            context,
            dom_ref: None,
        }
    }

    pub fn with_dom(mut self, dom: Arc<RwLock<super::dom::Document>>) -> Self {
        self.dom_ref = Some(dom);
        self.init_document();
        self
    }

    fn init_console(context: &mut Context) {
        let console_log = NativeFunction::from_fn_ptr(|_this, args, _context| {
            // console.log
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.display());
            }
            println!();
            Ok(JsValue::undefined())
        });

        let console = ObjectInitializer::new(context)
            .function(console_log, js_string!("log"), 0)
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
            let dom_ref_clone_inner = dom_ref_clone.clone();
            let query_selector_fn = unsafe { NativeFunction::from_closure(move |_this, args, _context| {
                // document.querySelector
                if let Some(selector_js) = args.first()
                    && let Some(selector_str) = selector_js.as_string()
                    && let Ok(dom) = dom_ref_clone_inner.try_read() {
                        let ids = dom.query_selector(selector_str.to_std_string_escaped().as_str());
                        if let Some(first_id) = ids.first() {
                            // Возвращаем простой объект с id
                            let obj = ObjectInitializer::new(_context)
                                .property(
                                    js_string!("id"),
                                    JsValue::from(*first_id as f64),
                                    Attribute::READONLY,
                                )
                                .build();
                            return Ok(JsValue::from(obj));
                        }
                }
                Ok(JsValue::null())
            }) };

            let document = ObjectInitializer::new(&mut self.context)
                .function(query_selector_fn, js_string!("querySelector"), 0)
                .build();

            let _ = self.context.register_global_property(
                js_string!("document"),
                document,
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            );
        }
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


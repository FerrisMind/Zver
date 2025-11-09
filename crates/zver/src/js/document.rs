use boa_engine::{
    Context, JsValue, NativeFunction, js_string, object::ObjectInitializer, property::Attribute,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::element::create_element_object;
use super::events::EventRegistry;

/// Инициализирует глобальный объект document с методами querySelector, getElementById, createElement
pub fn init_document(
    context: &mut Context,
    dom_ref: Arc<RwLock<super::super::dom::Document>>,
    event_registry: EventRegistry,
) {
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
                    let ids = dom.query_selector(selector_str.to_std_string_escaped().as_str());
                    if let Some(&first_id) = ids.first() {
                        drop(dom); // Release read lock before creating object
                        let element = create_element_object(
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
                    let element = create_element_object(
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
                            let element = create_element_object(
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

    let document = ObjectInitializer::new(context)
        .function(query_selector_fn, js_string!("querySelector"), 0)
        .function(get_element_by_id_fn, js_string!("getElementById"), 0)
        .function(create_elem_fn, js_string!("createElement"), 1)
        .build();

    let _ = context.register_global_property(
        js_string!("document"),
        document,
        Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
    );
}

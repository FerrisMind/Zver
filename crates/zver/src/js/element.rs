use boa_engine::{
    Context, JsValue, NativeFunction, js_string, object::ObjectInitializer, property::Attribute,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::events::{EventRegistry, EventType};

/// Создает JS объект Element с методами для работы с DOM
pub fn create_element_object(
    context: &mut Context,
    node_id: usize,
    dom_ref: Arc<RwLock<super::super::dom::Document>>,
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

                    event_registry.add_listener(node_id as usize, event_type, callback_code, once);

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

                    event_registry.remove_listener(node_id as usize, &event_type, &callback_code);

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

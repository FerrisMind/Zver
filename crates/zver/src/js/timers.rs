use boa_engine::{Context, JsValue, NativeFunction, js_string};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Pending callback для setTimeout
#[derive(Debug, Clone)]
pub struct PendingCallback {
    pub code: String,
    pub fire_time: std::time::Instant,
}

/// Инициализирует глобальную функцию setTimeout
pub fn init_timers(
    context: &mut Context,
    timers: Arc<Mutex<HashMap<u32, tokio::task::JoinHandle<()>>>>,
    next_id: Arc<Mutex<u32>>,
    pending_callbacks: Arc<Mutex<HashMap<u32, PendingCallback>>>,
) {
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
                #[allow(unused_variables)]
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

    let _ = context.register_global_builtin_callable(js_string!("setTimeout"), 0, set_timeout_fn);
}

/// Выполняет готовые callbacks из setTimeout
pub fn tick_timers(
    context: &mut Context,
    pending_callbacks: Arc<Mutex<HashMap<u32, PendingCallback>>>,
    timers: Arc<Mutex<HashMap<u32, tokio::task::JoinHandle<()>>>>,
) -> usize {
    let now = std::time::Instant::now();
    let mut executed = 0;

    // Собираем callbacks, которые нужно выполнить
    let ready_callbacks: Vec<(u32, String)> = {
        let pending = match pending_callbacks.lock() {
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

        match execute_code(context, &exec_code) {
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
        if let Ok(mut pending) = pending_callbacks.lock() {
            pending.remove(&timer_id);
        }

        // Удаляем завершённый таймер
        if let Ok(mut timers_lock) = timers.lock() {
            timers_lock.remove(&timer_id);
        }
    }

    executed
}

/// Вспомогательная функция для выполнения JS кода
fn execute_code(context: &mut Context, code: &str) -> Result<JsValue, boa_engine::JsError> {
    use boa_engine::Source;
    let source = Source::from_bytes(code);
    context.eval(source)
}

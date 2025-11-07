// Модули JavaScript движка
mod console;
mod document;
mod element;
mod engine;
pub mod events;
mod timers;

// Публичные экспорты
pub use engine::{JSEngine, JSValue};

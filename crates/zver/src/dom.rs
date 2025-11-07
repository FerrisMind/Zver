// Модули DOM
mod document;
mod manipulation;
mod node;
mod parser;
mod pseudo;
mod selectors;
pub mod serialization;

// Публичные экспорты
pub use document::Document;
pub use node::{ElementState, Node};

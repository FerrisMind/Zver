pub mod address_bar;
pub mod devtools;
pub mod render_view;
/// Browser module exports
///
/// Implements TRIZ principle of "Drobleniye" (Segmentation) by breaking
/// the monolithic application into independent modular components
pub mod tab;

pub use address_bar::AddressBar;
pub use devtools::{ConsoleEntry, DevTools};
pub use render_view::RenderView;
pub use tab::TabManager;

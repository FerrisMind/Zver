/// Browser module exports
/// 
/// Implements TRIZ principle of "Drobleniye" (Segmentation) by breaking
/// the monolithic application into independent modular components
pub mod tab;
pub mod address_bar;
pub mod devtools;
pub mod render_view;

pub use tab::TabManager;
pub use address_bar::AddressBar;
pub use devtools::DevTools;
pub use render_view::RenderView;

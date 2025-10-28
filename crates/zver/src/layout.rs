pub mod compute;
pub mod render;
pub mod styles;
pub mod taffy_integration;
pub mod types;

pub use render::*;
pub use types::*;

use crate::dom::Document;
use std::collections::HashMap;
use taffy::prelude::*;

pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
    pub layout_tree: Option<LayoutNode>,
    taffy: TaffyTree<()>,
}

unsafe impl Send for LayoutEngine {}
unsafe impl Sync for LayoutEngine {}

impl LayoutEngine {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            layout_tree: None,
            taffy: TaffyTree::new(),
        }
    }

    pub fn compute(
        &mut self,
        document: &Document,
        styles: &HashMap<usize, HashMap<String, String>>,
    ) -> Option<&LayoutNode> {
        let root_id = document.root?;

        // Строим Taffy-дерево из DOM
        let taffy_root =
            taffy_integration::build_taffy_tree(&mut self.taffy, document, root_id, styles);

        // Вычисляем лейаут
        let _ = self
            .taffy
            .compute_layout(
                taffy_root,
                taffy::Size {
                    width: AvailableSpace::Definite(self.viewport_width),
                    height: AvailableSpace::Definite(self.viewport_height),
                },
            )
            .ok();

        // Строим нашу структурированную модель для дальнейшего рендера
        self.layout_tree = compute::build_layout_node(
            document,
            root_id,
            styles,
            self.viewport_width,
            self.viewport_height,
            None,
        );

        // Устанавливаем позиции всех узлов
        if let Some(tree) = &mut self.layout_tree {
            compute::compute_positions(tree, 0.0, 0.0);
        }

        self.layout_tree.as_ref()
    }

    pub fn layout_tree(&self) -> Option<&LayoutNode> {
        self.layout_tree.as_ref()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

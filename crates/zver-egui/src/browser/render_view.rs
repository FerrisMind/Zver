use crate::egui_integration::render_clean_layout_from_results;
/// Render view component for clean page rendering
///
/// Implements TRIZ principle of "Obedinenie" (Merging) where layout results
/// are combined with rendering logic in a unified, scrollable viewport
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;
use zver::layout::RenderInfo;

/// Render view for displaying page content
pub struct RenderView;

impl RenderView {
    /// Renders the page content in a clean viewport
    ///
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `engine` - The Zver engine to render from
    /// * `runtime` - Tokio runtime for async operations
    /// * `show_debug_overlays` - Whether to show debug information
    pub fn render(
        ui: &mut egui::Ui,
        engine: &Arc<Zver>,
        runtime: &Arc<Runtime>,
        _show_debug_overlays: bool,
        highlighted_node: Option<usize>,
    ) {
        runtime.block_on(async {
            let layout = engine.layout.read().await;
            let dom = engine.dom.read().await;
            let resolved_styles = layout.resolved_styles().clone();

            let render_info = layout.collect_render_info(&dom);

            if render_info.is_empty() {
                drop(layout);
                drop(dom);
                ui.centered_and_justified(|ui| {
                    ui.label("No content loaded. Enter a URL and press Load.");
                });
                return;
            }

            // Calculate content dimensions
            let (canvas_width, canvas_height) = Self::calculate_content_size(&render_info);

            drop(layout);
            drop(dom);

            // Render in scrollable area with white background
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(canvas_width, canvas_height),
                    egui::Sense::hover(),
                );

                // White background for clean render
                painter.rect_filled(response.rect, 0.0, egui::Color32::WHITE);

                // Render layout results
                render_clean_layout_from_results(
                    &painter,
                    response.rect.min,
                    &render_info,
                    &resolved_styles,
                    highlighted_node,
                );
            });
        });
    }

    /// Calculates the content size from render info
    ///
    /// # Arguments
    /// * `render_info` - Layout results to measure
    ///
    /// # Returns
    /// Tuple of (width, height) for the content canvas
    fn calculate_content_size(render_info: &[RenderInfo]) -> (f32, f32) {
        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;

        for info in render_info {
            let right = info.layout.x + info.layout.width;
            let bottom = info.layout.y + info.layout.height;
            max_x = max_x.max(right);
            max_y = max_y.max(bottom);
        }

        // Add padding and ensure minimum size
        let canvas_width = (max_x + 20.0).max(800.0);
        let canvas_height = (max_y + 20.0).max(600.0);

        (canvas_width, canvas_height)
    }

    /// Renders with debug overlays
    ///
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `engine` - The Zver engine to render from
    /// * `runtime` - Tokio runtime for async operations
    #[allow(dead_code)]
    pub fn render_with_debug(
        ui: &mut egui::Ui,
        engine: &Arc<Zver>,
        runtime: &Arc<Runtime>,
        highlighted_node: Option<usize>,
    ) {
        use crate::egui_integration::render_layout_results_in_painter;

        runtime.block_on(async {
            let layout = engine.layout.read().await;
            let dom = engine.dom.read().await;
            let resolved_styles = layout.resolved_styles().clone();

            let render_info = layout.collect_render_info(&dom);

            if render_info.is_empty() {
                drop(layout);
                drop(dom);
                ui.centered_and_justified(|ui| {
                    ui.label("No content loaded.");
                });
                return;
            }

            // Display statistics
            ui.horizontal(|ui| {
                ui.label(format!("Elements: {}", render_info.len()));

                let mut text_nodes = 0;
                for node in dom.nodes.values() {
                    if let Some(text) = &node.text_content
                        && !text.trim().is_empty()
                    {
                        text_nodes += 1;
                    }
                }
                ui.label(format!("Text nodes: {}", text_nodes));
            });

            ui.separator();

            // Calculate content dimensions
            let (canvas_width, canvas_height) = Self::calculate_content_size(&render_info);

            drop(layout);
            drop(dom);

            // Render with debug overlays
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(canvas_width, canvas_height),
                    egui::Sense::hover(),
                );

                render_layout_results_in_painter(
                    &painter,
                    response.rect.min,
                    &render_info,
                    &resolved_styles,
                    true, // show_debug = true
                    highlighted_node,
                );
            });
        });
    }
}

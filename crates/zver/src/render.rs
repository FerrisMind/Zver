pub mod initialization;
pub mod operations;
pub mod types;
pub mod utils;

pub use types::*;
pub use utils::*;

use std::collections::HashMap;
use wgpu::{Device, Queue, Surface, Texture, TextureView};
use wgpu_text::TextBrush;

pub struct RenderEngine {
    device: Option<Device>,
    queue: Option<Queue>,
    surface: Option<Surface<'static>>,
    pub state: Option<RenderState>,

    // Текстовый рендерер (wgpu-text)
    text_brush: Option<TextBrush<wgpu_text::glyph_brush::ab_glyph::FontArc>>,

    // Пайплайны для рендеринга
    rect_pipeline: Option<wgpu::RenderPipeline>,
    image_pipeline: Option<wgpu::RenderPipeline>,

    // MSAA ресурсы
    msaa_texture: Option<Texture>,
    msaa_view: Option<TextureView>,

    // Bind group layouts
    image_bind_group_layout: Option<wgpu::BindGroupLayout>,

    // Кэш загруженных изображений
    image_cache: HashMap<String, std::sync::Arc<ImageTexture>>,

    // Буферы для батчинга
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,

    // Инкрементальный рендеринг
    #[allow(dead_code)]
    dirty_regions: Vec<types::Rect>,
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderEngine {
    pub fn new() -> Self {
        Self {
            device: None,
            queue: None,
            surface: None,
            state: None,
            text_brush: None,
            rect_pipeline: None,
            image_pipeline: None,
            msaa_texture: None,
            msaa_view: None,
            image_bind_group_layout: None,
            image_cache: HashMap::new(),
            vertex_buffer: None,
            index_buffer: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            dirty_regions: Vec::new(),
        }
    }
}

impl RenderEngine {
    /// Простой метод рендеринга для совместимости
    pub async fn paint(
        &mut self,
        layout: &crate::layout::LayoutEngine,
        document: &crate::dom::Document,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Получаем информацию для рендеринга с использованием нового API
        let _render_info = layout.collect_render_info(document);
        // Заглушка - основная логика рендеринга будет в других методах
        Ok(())
    }
}

impl std::fmt::Debug for RenderEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RenderEngine {{ initialized: {} }}",
            self.device.is_some()
        )
    }
}

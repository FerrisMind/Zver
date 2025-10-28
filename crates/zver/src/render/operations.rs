use super::RenderEngine;
use super::types::*;
use wgpu_text::glyph_brush::{Section, Text};

impl RenderEngine {
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if let (Some(device), Some(surface), Some(state)) =
            (&self.device, &self.surface, &mut self.state)
        {
            state.config.width = new_width.max(1);
            state.config.height = new_height.max(1);
            surface.configure(device, &state.config);

            // Пересоздаем MSAA текстуру
            let (msaa_texture, msaa_view) = create_msaa_texture(device, &state.config);
            self.msaa_texture = Some(msaa_texture);
            self.msaa_view = Some(msaa_view);

            // Обновляем размер текстового браша
            if let Some(text_brush) = &mut self.text_brush
                && let Some(queue) = &self.queue
            {
                text_brush.resize_view(new_width as f32, new_height as f32, queue);
            }
        }
    }

    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (device, queue, surface, msaa_view) =
            match (&self.device, &self.queue, &self.surface, &self.msaa_view) {
                (Some(d), Some(q), Some(s), Some(mv)) => (d, q, s, mv),
                _ => return Ok(()),
            };

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Основной render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Рендерим прямоугольники
            if !self.vertices.is_empty() {
                self.render_rectangles(&mut render_pass, queue);
            }

            // Рендерим изображения
            self.render_images(&mut render_pass);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn render_rectangles(&self, render_pass: &mut wgpu::RenderPass, queue: &wgpu::Queue) {
        if let (Some(vertex_buffer), Some(index_buffer), Some(rect_pipeline)) =
            (&self.vertex_buffer, &self.index_buffer, &self.rect_pipeline)
        {
            // Обновляем буферы
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(&self.indices));

            render_pass.set_pipeline(rect_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }
    }

    fn render_images(&self, render_pass: &mut wgpu::RenderPass) {
        if let Some(image_pipeline) = &self.image_pipeline {
            render_pass.set_pipeline(image_pipeline);

            for image in self.image_cache.values() {
                render_pass.set_bind_group(0, Some(&*image.bind_group), &[]);
                // Здесь должна быть логика рендеринга конкретного изображения
                // render_pass.draw_indexed(...);
            }
        }
    }

    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let start_index = self.vertices.len() as u16;

        // Добавляем вершины прямоугольника
        self.vertices.extend_from_slice(&[
            Vertex {
                position: [x, y],
                tex_coords: [0.0, 0.0],
                color,
            },
            Vertex {
                position: [x + width, y],
                tex_coords: [1.0, 0.0],
                color,
            },
            Vertex {
                position: [x + width, y + height],
                tex_coords: [1.0, 1.0],
                color,
            },
            Vertex {
                position: [x, y + height],
                tex_coords: [0.0, 1.0],
                color,
            },
        ]);

        // Добавляем индексы для двух треугольников
        self.indices.extend_from_slice(&[
            start_index,
            start_index + 1,
            start_index + 2,
            start_index,
            start_index + 2,
            start_index + 3,
        ]);
    }

    pub fn add_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        if let (Some(text_brush), Some(device), Some(queue)) =
            (&mut self.text_brush, &self.device, &self.queue)
        {
            let section = Section::default()
                .add_text(Text::new(text).with_color(color).with_scale(size))
                .with_screen_position((x, y));

            let _ = text_brush.queue(device, queue, vec![&section]);
        }
    }

    pub fn clear_frame(&mut self) {
        self.vertices.clear();
        self.indices.clear();

        // Очищаем очередь текста (это делается автоматически после рендеринга)
    }
}

fn create_msaa_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (wgpu::Texture, wgpu::TextureView) {
    let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MSAA Texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
    (msaa_texture, msaa_view)
}

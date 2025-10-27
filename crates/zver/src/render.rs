use wgpu::{Device, Queue, Surface, SurfaceConfiguration, Texture, TextureView};
use winit::window::Window;
use wgpu_text::{BrushBuilder, TextBrush, glyph_brush::{Section, Text, ab_glyph::FontArc}};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RenderState {
    pub config: SurfaceConfiguration,
}

// Структура для хранения текстур изображений
pub struct ImageTexture {
    pub texture: Arc<Texture>,
    pub view: Arc<TextureView>,
    pub bind_group: Arc<wgpu::BindGroup>,
    pub width: u32,
    pub height: u32,
}

// Вершина для рендеринга изображений и прямоугольников
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct RenderEngine {
    device: Option<Device>,
    queue: Option<Queue>,
    surface: Option<Surface<'static>>,
    pub state: Option<RenderState>,
    
    // Текстовый рендерер (wgpu-text)
    text_brush: Option<TextBrush<FontArc>>,
    
    // Пайплайны для рендеринга
    rect_pipeline: Option<wgpu::RenderPipeline>,
    image_pipeline: Option<wgpu::RenderPipeline>,
    
    // MSAA ресурсы
    msaa_texture: Option<Texture>,
    msaa_view: Option<TextureView>,
    
    // Bind group layouts
    image_bind_group_layout: Option<wgpu::BindGroupLayout>,
    
    // Кэш загруженных изображений
    image_cache: HashMap<String, Arc<ImageTexture>>,
    
    // Буферы для батчинга
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    
    // Инкрементальный рендеринг
    dirty_regions: Vec<Rect>,
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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

    pub async fn initialize(
        &mut self,
        window: &'static Window,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        #[allow(unused_unsafe)]
        let surface = unsafe { instance.create_surface(window)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter {
            Ok(adapter) => adapter,
            Err(_) => return Err("Failed to find suitable adapter".into()),
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: Some("Zver Device"),
                    memory_hints: Default::default(),
                    trace: Default::default(),
                    experimental_features: Default::default(),
                },
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Инициализация текстового брашa (wgpu-text)
        let font_data = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
        let font = FontArc::try_from_slice(font_data).expect("Failed to load font");
        let text_brush = BrushBuilder::using_font(font)
            .build(&device, config.width, config.height, config.format);

        // Создаём MSAA текстуру (4x)
        let msaa_sample_count = 4;
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa_sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Создаём bind group layout для изображений
        let image_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Image Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Создаём пайплайны
        let rect_pipeline = self.create_rect_pipeline(&device, &config, msaa_sample_count);
        let image_pipeline = self.create_image_pipeline(&device, &config, &image_bind_group_layout, msaa_sample_count);

        // Создаём начальные буферы
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 65536, // 64KB начальный размер
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 32768, // 32KB начальный размер
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.state = Some(RenderState { config });
        self.text_brush = Some(text_brush);
        self.rect_pipeline = Some(rect_pipeline);
        self.image_pipeline = Some(image_pipeline);
        self.msaa_texture = Some(msaa_texture);
        self.msaa_view = Some(msaa_view);
        self.image_bind_group_layout = Some(image_bind_group_layout);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);

        Ok(())
    }

    // Создание пайплайна для рендеринга прямоугольников
    fn create_rect_pipeline(&self, device: &Device, config: &SurfaceConfiguration, sample_count: u32) -> wgpu::RenderPipeline {
        let shader_src = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    // Создание пайплайна для рендеринга изображений
    fn create_image_pipeline(
        &self,
        device: &Device,
        config: &SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        sample_count: u32,
    ) -> wgpu::RenderPipeline {
        let shader_src = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture, texture_sampler, input.tex_coords);
    return tex_color * input.color;
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Image Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Image Pipeline Layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Image Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
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

impl RenderEngine {
    // Загрузка изображения из байтов
    pub async fn load_image_from_bytes(&mut self, url: &str, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let (device, queue, layout) = match (
            self.device.as_ref(),
            self.queue.as_ref(),
            self.image_bind_group_layout.as_ref(),
        ) {
            (Some(d), Some(q), Some(l)) => (d, q, l),
            _ => return Err("Render engine not initialized".into()),
        };

        // Декодируем изображение
        let img = image::load_from_memory(bytes)?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        // Создаём текстуру
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Image Texture: {}", url)),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Создаём сэмплер
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Создаём bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Image Bind Group: {}", url)),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let image_texture = ImageTexture {
            texture: Arc::new(texture),
            view: Arc::new(view),
            bind_group: Arc::new(bind_group),
            width: dimensions.0,
            height: dimensions.1,
        };

        self.image_cache.insert(url.to_string(), Arc::new(image_texture));
        Ok(())
    }

    // Добавление изображения в батч для рендеринга
    pub fn queue_image(&mut self, url: &str, x: f32, y: f32, width: f32, height: f32) -> bool {
        if let Some(_img_tex) = self.image_cache.get(url) {
            let state = match self.state.as_ref() {
                Some(s) => s,
                None => return false, // Не инициализирован
            };
            
            let base_idx = self.vertices.len() as u16;

            // Конвертируем координаты в NDC
            let ndc_x = (x / state.config.width as f32) * 2.0 - 1.0;
            let ndc_y = 1.0 - (y / state.config.height as f32) * 2.0;
            let ndc_w = (width / state.config.width as f32) * 2.0;
            let ndc_h = (height / state.config.height as f32) * 2.0;

            // Добавляем вершины для изображения
            self.vertices.push(Vertex {
                position: [ndc_x, ndc_y],
                tex_coords: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            self.vertices.push(Vertex {
                position: [ndc_x + ndc_w, ndc_y],
                tex_coords: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            self.vertices.push(Vertex {
                position: [ndc_x + ndc_w, ndc_y - ndc_h],
                tex_coords: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            self.vertices.push(Vertex {
                position: [ndc_x, ndc_y - ndc_h],
                tex_coords: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });

            self.indices.extend_from_slice(&[
                base_idx, base_idx + 1, base_idx + 2,
                base_idx + 2, base_idx + 3, base_idx,
            ]);

            true
        } else {
            false
        }
    }

    // Добавление прямоугольника в батч
    pub fn queue_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let state = match self.state.as_ref() {
            Some(s) => s,
            None => return, // Не инициализирован, пропускаем
        };
        
        let base_idx = self.vertices.len() as u16;

        // Конвертируем координаты в NDC (Normalized Device Coordinates)
        let ndc_x = (x / state.config.width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y / state.config.height as f32) * 2.0;
        let ndc_w = (width / state.config.width as f32) * 2.0;
        let ndc_h = (height / state.config.height as f32) * 2.0;

        // Добавляем 4 вершины для прямоугольника
        self.vertices.push(Vertex {
            position: [ndc_x, ndc_y],
            tex_coords: [0.0, 0.0],
            color,
        });
        self.vertices.push(Vertex {
            position: [ndc_x + ndc_w, ndc_y],
            tex_coords: [1.0, 0.0],
            color,
        });
        self.vertices.push(Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h],
            tex_coords: [1.0, 1.0],
            color,
        });
        self.vertices.push(Vertex {
            position: [ndc_x, ndc_y - ndc_h],
            tex_coords: [0.0, 1.0],
            color,
        });

        // Добавляем 6 индексов для 2 треугольников
        self.indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx + 2, base_idx + 3, base_idx,
        ]);
    }

    // Добавление текста в очередь
    pub fn queue_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: [f32; 4]) {
        if let Some(brush) = &mut self.text_brush
            && let Some(queue) = self.queue.as_ref()
            && let Some(device) = self.device.as_ref()
        {
            let section = Section::default()
                .add_text(
                    Text::new(text)
                        .with_scale(font_size)
                        .with_color(color)
                )
                .with_screen_position((x, y));
            
            let _ = brush.queue(device, queue, vec![&section]);
        }
    }

    // Полноценный метод рендеринга
    pub async fn paint(
        &mut self,
        layout: &crate::layout::LayoutEngine,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Рендерим layout дерево
        if let Some(layout_tree) = &layout.layout_tree {
            self.vertices.clear();
            self.indices.clear();
            self.render_layout_node(layout_tree);
        }

        let (device, queue, surface, _state, msaa_view) = match (
            self.device.as_ref(),
            self.queue.as_ref(),
            self.surface.as_ref(),
            self.state.as_ref(),
            self.msaa_view.as_ref(),
        ) {
            (Some(d), Some(q), Some(s), Some(st), Some(mv)) => (d, q, s, st, mv),
            _ => return Ok(()),
        };

        // Получаем текущий кадр
        let frame = surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Zver Render Encoder"),
        });

        // Обновляем буферы если нужно
        if !self.vertices.is_empty() {
            let vertex_data = bytemuck::cast_slice(&self.vertices);
            if vertex_data.len() > self.vertex_buffer.as_ref().unwrap().size() as usize {
                // Пересоздаём буфер если не хватает места
                self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Vertex Buffer"),
                    size: (vertex_data.len() * 2) as u64, // Удваиваем размер
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            queue.write_buffer(self.vertex_buffer.as_ref().unwrap(), 0, vertex_data);
        }

        if !self.indices.is_empty() {
            let index_data = bytemuck::cast_slice(&self.indices);
            if index_data.len() > self.index_buffer.as_ref().unwrap().size() as usize {
                self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Index Buffer"),
                    size: (index_data.len() * 2) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            queue.write_buffer(self.index_buffer.as_ref().unwrap(), 0, index_data);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Zver Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Рендерим прямоугольники
            if !self.indices.is_empty()
                && let (Some(pipeline), Some(vb), Some(ib)) = (
                    self.rect_pipeline.as_ref(),
                    self.vertex_buffer.as_ref(),
                    self.index_buffer.as_ref(),
                )
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(0, vb.slice(..));
                render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            }

            // Рендерим текст
            if let Some(brush) = &mut self.text_brush {
                brush.draw(&mut render_pass);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    // Рендеринг узла layout дерева
    fn render_layout_node(&mut self, node: &crate::layout::LayoutNode) {
        let dims = &node.dimensions;

        // Рендерим фон если есть
        if let Some(bg_color) = &node.style.background_color {
            let color = parse_color(bg_color);
            self.queue_rect(dims.x, dims.y, dims.width, dims.height, color);
        }

        // Рекурсивно рендерим детей
        for child in &node.children {
            self.render_layout_node(child);
        }
    }

    // Инкрементальный рендеринг - методы для управления dirty regions
    pub fn mark_region_dirty(&mut self, rect: Rect) {
        self.dirty_regions.push(rect);
    }

    pub fn clear_dirty_regions(&mut self) {
        self.dirty_regions.clear();
    }

    pub fn get_dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    // Проверяем, нужно ли перерисовывать весь экран
    pub fn needs_full_redraw(&self) -> bool {
        self.dirty_regions.is_empty() || self.dirty_regions.len() > 10 // порог для полного перерисовывания
    }

    // Ресайз окна - пересоздание MSAA текстуры
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if let (Some(state), Some(surface), Some(device), Some(brush)) = (
            self.state.as_mut(),
            self.surface.as_ref(),
            self.device.as_ref(),
            self.text_brush.as_mut(),
        ) {
            state.config.width = new_width.max(1);
            state.config.height = new_height.max(1);
            surface.configure(device, &state.config);

            // Пересоздаём MSAA текстуру
            let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: state.config.width,
                    height: state.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: state.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa_view = Some(msaa_texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.msaa_texture = Some(msaa_texture);

            // Обновляем размеры у текстового брашa
            if let Some(queue) = self.queue.as_ref() {
                brush.resize_view(state.config.width as f32, state.config.height as f32, queue);
            }
        }
    }
}

// Вспомогательная функция для парсинга цвета
fn parse_color(color_str: &str) -> [f32; 4] {
    if let Some(hex) = color_str.strip_prefix('#')
        && hex.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        )
    {
        return [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        ];
    }
    
    // Цвета по умолчанию
    match color_str {
        "red" => [1.0, 0.0, 0.0, 1.0],
        "green" => [0.0, 1.0, 0.0, 1.0],
        "blue" => [0.0, 0.0, 1.0, 1.0],
        "white" => [1.0, 1.0, 1.0, 1.0],
        "black" => [0.0, 0.0, 0.0, 1.0],
        _ => [0.5, 0.5, 0.5, 1.0],
    }
}

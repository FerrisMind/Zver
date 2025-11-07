use super::RenderEngine;
use super::types::*;
use wgpu::{Device, SurfaceConfiguration};
use wgpu_text::{BrushBuilder, glyph_brush::ab_glyph::FontArc};
use winit::window::Window;

impl RenderEngine {
    pub async fn initialize(
        &mut self,
        window: &'static Window,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();

        // SAFETY: Creating a surface from a window is inherently unsafe because:
        // 1. The window handle must remain valid for the lifetime of the surface
        // 2. winit::Window is guaranteed to outlive the surface in our architecture
        // 3. The surface is stored alongside the window in RenderEngine
        // 4. Both are dropped together, preventing use-after-free
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
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                label: Some("Zver Device"),
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            })
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

        // Инициализация текстового браша
        let font_data = include_bytes!("../../../../assets/fonts/Roboto-Regular.ttf");
        let font = FontArc::try_from_slice(font_data).expect("Failed to load font");
        let text_brush = BrushBuilder::using_font(font).build(
            &device,
            config.width,
            config.height,
            config.format,
        );

        // Создание пайплайнов
        let (rect_pipeline, image_pipeline, image_bind_group_layout) =
            create_pipelines(&device, &config);

        // Создание MSAA текстуры
        let (msaa_texture, msaa_view) = create_msaa_texture(&device, &config);

        // Создание буферов
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * 1024) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (std::mem::size_of::<u16>() * 2048) as u64,
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
}

fn create_pipelines(
    device: &Device,
    config: &SurfaceConfiguration,
) -> (
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::BindGroupLayout,
) {
    // Создание шейдеров для прямоугольников
    let rect_shader_src = r#"
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

    let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Rect Shader"),
        source: wgpu::ShaderSource::Wgsl(rect_shader_src.into()),
    });

    // Создание шейдеров для изображений
    let image_shader_src = r#"
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

    let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Image Shader"),
        source: wgpu::ShaderSource::Wgsl(image_shader_src.into()),
    });

    // Bind group layout для изображений
    let image_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("Image Bind Group Layout"),
        });

    // Pipeline layout для прямоугольников
    let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Rect Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    // Pipeline layout для изображений
    let image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Image Pipeline Layout"),
        bind_group_layouts: &[&image_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Render pipeline для прямоугольников
    let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Rect Pipeline"),
        layout: Some(&rect_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &rect_shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &rect_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    // Render pipeline для изображений
    let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Image Pipeline"),
        layout: Some(&image_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &image_shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &image_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    (rect_pipeline, image_pipeline, image_bind_group_layout)
}

fn create_msaa_texture(
    device: &Device,
    config: &SurfaceConfiguration,
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

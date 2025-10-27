use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

#[derive(Debug, Clone)]
pub struct RenderState {
    pub config: SurfaceConfiguration,
}

pub struct RenderEngine {
    device: Option<Device>,
    queue: Option<Queue>,
    surface: Option<Surface<'static>>,
    pub state: Option<RenderState>,
    pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
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
            pipeline: None,
            vertex_buffer: None,
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
            .await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: None,
                    memory_hints: Default::default(),
                    experimental_features: Default::default(),
                    trace: wgpu::Trace::default(),
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

        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.state = Some(RenderState { config });

        // Создаем простой пайплайн и вершины для треугольника
        self.create_pipeline_and_geometry();

        Ok(())
    }

    pub async fn paint(
        &self,
        _layout: &crate::layout::LayoutEngine,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (device, queue, surface, _) = match (
            self.device.as_ref(),
            self.queue.as_ref(),
            self.surface.as_ref(),
            self.state.as_ref(),
        ) {
            (Some(device), Some(queue), Some(surface), Some(state)) => (device, queue, surface, state),
            _ => return Ok(()),
        };

        let frame = surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Zver Render Encoder"),
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Zver Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
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

        if let (Some(pipeline), Some(vb)) = (self.pipeline.as_ref(), self.vertex_buffer.as_ref()) {
            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.draw(0..3, 0..1);
        }
        drop(render_pass);

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        let _ = _layout;
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

impl RenderEngine {
    fn create_pipeline_and_geometry(&mut self) {
        let (device, state) = match (self.device.as_ref(), self.state.as_ref()) {
            (Some(d), Some(s)) => (d, s),
            _ => return,
        };

        // WGSL шейдер (позиция + заливка цветом)
        let shader_src = r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index : u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>( 0.0,  0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5)
    );
    let pos = positions[vertex_index];
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.10, 0.45, 0.85, 1.0);
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zver_inline_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zver_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zver_triangle_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Пустой вершинный буфер (используем vertex_index)
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("zver_dummy_vb"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        self.pipeline = Some(pipeline);
        self.vertex_buffer = Some(vb);
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
}


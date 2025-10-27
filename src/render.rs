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

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        drop(_render_pass);

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


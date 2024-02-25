use crate::core::graphics::gpu_selection_utils::{
    eliminate_gpu_on_unsupported_feats, select_gpu_on_backend, select_gpu_on_type,
};
use crate::core::window::components::Window;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use log::info;
use wgpu::{Backends, PresentMode};

pub struct GraphicsState<'window> {
    // Global Objects
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,

    // Per-Window Objects
    pub surface_states: HashMap<winit::window::WindowId, SurfaceState<'window>>,

    _not_send_sync: PhantomData<*const ()>,
}

impl<'window> GraphicsState<'window> {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();

        let mut adapters = instance.enumerate_adapters(Backends::all());
        assert!(!adapters.is_empty(), "No GPU!");

        adapters = eliminate_gpu_on_unsupported_feats(adapters);
        adapters = select_gpu_on_type(adapters);
        adapters = select_gpu_on_backend(adapters);

        // Simply choose the first one
        let adapter = adapters.remove(0);
        
        info!("Selected Adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        Self {
            instance,
            device,
            queue,
            adapter,
            surface_states: HashMap::new(),
            _not_send_sync: PhantomData,
        }
    }

    pub async fn create_surface(
        &mut self,
        window: Arc<winit::window::Window>,
        window_component: &Window,
    ) {
        let surface = self.instance.create_surface(window.clone()).unwrap();

        let surface_caps = surface.get_capabilities(&self.adapter);
        // Gets the first surface format that is sRGB, otherwise use the first surface format returned
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: if window_component.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&self.device, &config);

        self.surface_states.insert(
            window.id(),
            SurfaceState {
                surface,
                config,
                size: window.inner_size(),
            },
        );
    }

    pub fn destroy_surface(&mut self, window_id: winit::window::WindowId) {
        self.surface_states.remove(&window_id);
    }
}

pub struct SurfaceState<'window> {
    pub surface: wgpu::Surface<'window>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl SurfaceState<'_> {
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(device, &self.config);
        }
    }
}

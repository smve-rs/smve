//! Bevy resources for the graphics module.
//!
//! This module contains the resources used by the graphics module such as the [`GraphicsState`] struct.

use crate::core::graphics::adapter_selection_utils::get_best_adapter;
use crate::core::window::components::{RawHandleWrapper, Window};
use log::info;
use std::collections::HashMap;
use wgpu::{Backends, InstanceDescriptor, PresentMode};

/// Contains the global and per-window objects needed for rendering.
///
/// # Notes
/// This owns the wgpu instance, device, queue, adapter and all the surfaces.
pub struct GraphicsState<'window> {
    // Global Objects
    /// The wgpu instance.
    pub instance: wgpu::Instance,
    /// The wgpu device.
    pub device: wgpu::Device,
    /// The wgpu queue.
    pub queue: wgpu::Queue,
    /// The wgpu adapter.
    pub adapter: wgpu::Adapter,

    // Per-Window Objects
    /// Contains a mapping from the window id to the surface state.
    pub surface_states: HashMap<winit::window::WindowId, SurfaceState<'window>>,
    //_not_send_sync: PhantomData<*const ()>,
}

impl<'window> GraphicsState<'window> {
    /// Asynchronously creates a new instance of the graphics state.
    ///
    /// Initializes the instance, selects the best adapter, creates the device and queue and creates an empty surface state map.
    ///
    /// # Returns
    /// An instance of [`GraphicsState`] containing the created instances and an empty surface state map.
    pub async fn new() -> Self {
        // Create instance with all backends
        let instance = wgpu::Instance::default();

        // Get the backend of the best adapter
        let adapters = instance.enumerate_adapters(Backends::all());
        assert!(!adapters.is_empty(), "No adapters found!");

        let adapter = get_best_adapter(adapters);

        info!("Selected Backend: {:?}", adapter.get_info().backend);

        // Recreate the instance based on the backend chosen (fix DX12 problem on windows)
        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: adapter.get_info().backend.into(),
            ..Default::default()
        });

        // Find the best adapter again
        let adapters = instance.enumerate_adapters(Backends::all());

        let adapter = get_best_adapter(adapters);

        info!("Selected Adapter: {:?}", adapter.get_info());

        // Create device
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
            //_not_send_sync: PhantomData,
        }
    }

    /// Creates a new surface for a window.
    ///
    /// This function creates a new surface for the window and configures it with the given parameters specified in the [`Window`] component.
    ///
    /// # Arguments
    /// * `window` - The winit window to create the surface for.
    /// * `window_component` - The corresponding window component of the window.
    /// * `raw_handle_wrapper` - The raw handle wrapper component containing the raw handle of the window.
    pub fn create_surface(
        &mut self,
        window: &winit::window::Window,
        window_component: &Window,
        raw_handle_wrapper: &RawHandleWrapper,
    ) {
        let handle = unsafe { raw_handle_wrapper.get_handle() };
        let surface = self.instance.create_surface(handle).unwrap();

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

    /// Destroys the surface for a window.
    ///
    /// # Arguments
    /// * `window_id` - The id of the window to destroy the surface for.
    pub fn destroy_surface(&mut self, window_id: winit::window::WindowId) {
        self.surface_states.remove(&window_id);
    }
}

/// Contains various values associated with a surface.
///
/// This will be stored in the [`GraphicsState`] struct for each window with a surface.
pub struct SurfaceState<'window> {
    /// The wgpu surface.
    pub surface: wgpu::Surface<'window>,
    /// The surface configuration.
    pub config: wgpu::SurfaceConfiguration,
    /// The size of the surface.
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl SurfaceState<'_> {
    #[allow(dead_code)]
    /// Resizes the surface to the new size.
    ///
    /// # Arguments
    /// * `new_size` - The new size to resize the surface to.
    /// * `device` - The wgpu device to configure the surface with.
    ///
    /// # Notes
    /// Use this when the window is resized, moved between monitors or when the DPI changes.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(device, &self.config);
        }
    }
}

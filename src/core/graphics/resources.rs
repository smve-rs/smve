//! Bevy resources for the graphics module.
//!
//! This module contains the resources used by the graphics module such as the [`GraphicsState`] struct.

use crate::core::graphics::adapter_selection_utils::get_best_adapter;
use crate::core::graphics::extract::window::ExtractedWindow;
use crate::core::window::components::RawHandleWrapper;
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use log::info;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use wgpu::{Backends, CreateSurfaceError, InstanceDescriptor, PresentMode};
use winit::dpi::PhysicalSize;

/// Contains the global and per-window objects needed for rendering.
///
/// # Notes
/// This owns the wgpu instance, device, queue, adapter and all the surfaces.
#[derive(Resource)]
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
    pub surface_states: HashMap<Entity, SurfaceState<'window>>,
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

        // * Fun Fact: This "DX12" problem was extremely hard to debug.
        // *           Turns out it is something to do with the fact that wgpu creates an instance for all backends when using the default constructor.
        // *           This causes the DX12 backend to fail as it could not share the instance with the Vulkan backend.
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
            .unwrap_or_else(|err| {
                panic!("Failed to create device: {err}");
            });

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
    /// - `window` - The winit window to create the surface for.
    /// - `window_component` - The corresponding window component of the window.
    /// - `raw_handle_wrapper` - The raw handle wrapper component containing the raw handle of the window.
    ///
    /// # Returns
    /// An empty result if the surface was created successfully, otherwise a [`CreateSurfaceError`] is returned.
    pub fn create_surface(
        &mut self,
        window_component: &ExtractedWindow,
        entity: Entity,
        raw_handle_wrapper: &RawHandleWrapper,
        // * Fun Fact: I used to not return a Result here because I was simply panicking if the surface creation failed.
        // *           That was a horrible idea.
    ) -> Result<(), CreateSurfaceError> {
        // * Fun Fact: I saw tutorials online suggesting to pass the window straight in to the create_surface function.
        // *           But the create_surface function needs to take ownership of whatever was passed in.
        // *           But x2, all windows are owned by WinitWindows.
        // *           So I "solved" the problem by making WinitWindows own Arcs of the windows, and cloning the Arc here.
        // *           That was a horrible idea though as an Arc means ownership is shared. This means when we drop the window
        // *           in WinitWindows, it might not drop the window as somebody else might still own it.
        // *           I finally solved the problem by looking at bevy's code and realizing that I could've just passed in a struct
        // *           containing the handles (which could be cloned)
        let handle = unsafe { raw_handle_wrapper.get_handle() };
        let surface = self.instance.create_surface(handle)?;

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
            width: window_component.physical_width,
            height: window_component.physical_height,
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
            entity,
            SurfaceState {
                surface,
                config,
                size: PhysicalSize::new(
                    window_component.physical_width,
                    window_component.physical_height,
                ),
            },
        );

        Ok(())
    }

    /// Destroys the surface for a window.
    ///
    /// # Arguments
    /// - `entity` - The entity corresponding to the surface to be destroyed
    pub fn destroy_surface(&mut self, entity: Entity) {
        self.surface_states.remove(&entity);
        info!("Surface destroyed for entity {:?}", entity);
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
    pub size: PhysicalSize<u32>,
}

impl SurfaceState<'_> {
    #[allow(dead_code)]
    /// Resizes the surface to the new size.
    ///
    /// # Arguments
    /// - `new_size` - The new size to resize the surface to.
    /// - `device` - The wgpu device to configure the surface with.
    ///
    /// # Notes
    /// Use this when the window is resized, moved between monitors or when the DPI changes.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>, device: &wgpu::Device) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(device, &self.config);
        }
    }
}

/// A blank world to swap the actual world with during extraction to avoid constantly making new worlds
#[derive(Default, Resource)]
pub struct ScratchMainWorld(pub World);

/// A resource for the render app to access the main app for extraction
#[derive(Resource)]
pub struct MainWorld(pub World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A dummy type that is [`!Send](Send) to force systems to run on the main thread.
#[derive(Default)]
pub struct NonSendMarker(PhantomData<*mut ()>);

/// A resource on the render app that contains all the extracted windows
#[derive(Default, Resource)]
pub struct ExtractedWindows {
    /// The primary window
    pub primary: Option<Entity>,
    /// Map from entities to their corresponding windows
    pub windows: EntityHashMap<ExtractedWindow>,
}

//! Components for the window system

use crate::client::core::window::icon;
use bevy_ecs::prelude::Component;
use macros::ExtractComponent;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
use winit::dpi::{LogicalSize, PhysicalSize, Pixel};

/// A marker component for the primary window.
/// There should be only one primary window at any one time.
#[derive(Component)]
pub struct PrimaryWindow;

/// Component description of the window
///
/// This contains various parameters of the window.
#[derive(Component, Clone)]
pub struct Window {
    /// The dimensions of the window
    // * Fun Fact: This used to be just a width and a height. Then I ran into a few problems with the scale factor.
    // *           So I decided to also store the scale factor and make functions that would convert from physical size to logical size.
    pub resolution: WindowResolution,
    /// The title of the window
    pub title: String,
    /// 0 if there is no icon
    pub icon_width: u32,
    /// 0 if there is no icon
    pub icon_height: u32,
    /// A flat vector of RGBA data of the icon
    /// `None` if there is no icon
    pub icon_data: Option<Vec<u8>>,
    /// Whether vsync is enabled
    pub vsync: bool,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            resolution: Default::default(),
            title: "Ruxel".to_string(),
            icon_width: icon::IMAGE_WIDTH as u32,
            icon_height: icon::IMAGE_HEIGHT as u32,
            icon_data: Some(icon::IMAGE_DATA.to_vec()),
            vsync: true,
        }
    }
}

/// A structure representing the resolution of the window
#[derive(Clone, PartialEq, Debug)]
pub struct WindowResolution {
    /// The physical width (pixels) of the window
    physical_width: u32,
    /// The physical height (pixels) of the window
    physical_height: u32,
    /// The scale factor of the window
    scale_factor: f64,
}

impl Default for WindowResolution {
    fn default() -> Self {
        WindowResolution {
            physical_width: 800,
            physical_height: 600,
            scale_factor: 1.0,
        }
    }
}

#[allow(dead_code)]
impl WindowResolution {
    /// Creates a new window resolution with the given logical size
    pub fn new<P: Pixel>(logical_size: LogicalSize<P>) -> Self {
        // Assume the scale factor is 1 since it should be updated when the window is created
        let physical_size = logical_size.to_physical(1.0);
        WindowResolution {
            physical_width: physical_size.width,
            physical_height: physical_size.height,
            scale_factor: 1.0,
        }
    }

    /// Creates a new window resolution with the given physical size and scale factor
    pub fn new_physical<P: Pixel>(physical_size: PhysicalSize<P>, scale_factor: f64) -> Self {
        let physical_size = physical_size.cast::<u32>();
        WindowResolution {
            physical_width: physical_size.width,
            physical_height: physical_size.height,
            scale_factor,
        }
    }

    /// Returns the physical width of the window
    pub fn physical_width(&self) -> u32 {
        self.physical_width
    }

    /// Returns the physical height of the window
    pub fn physical_height(&self) -> u32 {
        self.physical_height
    }

    /// Returns the scale factor of the window
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Returns the logical width of the window
    pub fn width(&self) -> f64 {
        self.physical_width as f64 / self.scale_factor
    }

    /// Returns the logical height of the window
    pub fn height(&self) -> f64 {
        self.physical_height as f64 / self.scale_factor
    }

    /// Returns the logical size of the window
    pub fn size(&self) -> LogicalSize<f64> {
        LogicalSize::new(self.width(), self.height())
    }

    /// Returns the physical size of the window
    pub fn physical_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.physical_width, self.physical_height)
    }

    /// Sets the physical width of the window
    pub fn set_physical_size<P: Pixel>(&mut self, physical_size: PhysicalSize<P>) {
        let physical_size = physical_size.cast::<u32>();
        self.physical_width = physical_size.width;
        self.physical_height = physical_size.height;
    }

    /// Sets the logical size of the window
    pub fn set_logical_size<P: Pixel>(&mut self, logical_size: LogicalSize<P>) {
        let physical_size: PhysicalSize<u32> = logical_size.to_physical(self.scale_factor);
        self.set_physical_size(physical_size);
    }

    /// Sets the scale factor of the window
    /// To ensure the logical size does not change, the physical size is adjusted based on the new scale factor
    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        let old_scale_factor = self.scale_factor;
        self.scale_factor = scale_factor;
        self.physical_width = (self.physical_width as f64 / old_scale_factor * scale_factor) as u32;
        self.physical_height =
            (self.physical_height as f64 / old_scale_factor * scale_factor) as u32;
    }
}

/// A cached state of the window so that we can check if it has changed
#[derive(Component)]
pub struct CachedWindow(pub Window);

/// A wrapper for the raw handle of the window
///
/// [`create_surface`](wgpu::Instance::create_surface) requires ownership to an object that implements [`HasWindowHandle`] and [`HasDisplayHandle`].
/// The winit [`Window`](winit::window::Window) is owned by [`WinitWindows`](crate::core::window::WinitWindows) and cannot be cloned.
/// This wrapper allows the raw handle to be cloned and passed to the [`create_surface`](wgpu::Instance::create_surface) function.
#[derive(Clone, Component, ExtractComponent)]
pub struct RawHandleWrapper {
    /// The raw display handle of the window
    pub display_handle: RawDisplayHandle,
    /// The raw window handle of the window
    pub window_handle: RawWindowHandle,
}

// SAFETY:
// The wrapper forces the user to call an unsafe function to get the underlying handles.
// https://github.com/rust-windowing/raw-window-handle/pull/152
unsafe impl Send for RawHandleWrapper {}
unsafe impl Sync for RawHandleWrapper {}

impl RawHandleWrapper {
    /// Gets a thread-locked version of the raw handle wrapper
    ///
    /// This exists because the functions in [`HasDisplayHandle`] and [`HasWindowHandle`] are safe.
    ///
    /// # Safety
    /// This function should be called in a correct context (some platforms do not support doing windowing operations on different threads).
    pub unsafe fn get_handle(&self) -> ThreadLockedRawHandleWrapper {
        ThreadLockedRawHandleWrapper(self.clone())
    }
}

/// A thread-locked version of the raw handle wrapper
///
/// This is a wrapper around the raw handle wrapper and can ONLY be constructed by calling [`RawHandleWrapper::get_handle`].
/// This ensures that the raw handle wrapper is only used in a correct context.
// * Fun Fact: This struct is not actually thread-locked. I'm not sure why but this is exactly how bevy handles this situation in their code.
pub struct ThreadLockedRawHandleWrapper(RawHandleWrapper);

impl HasDisplayHandle for ThreadLockedRawHandleWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: The caller has validated that this is a valid context to get the raw handles.
        unsafe { Ok(DisplayHandle::borrow_raw(self.0.display_handle)) }
    }
}

impl HasWindowHandle for ThreadLockedRawHandleWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: The caller has validated that this is a valid context to get the raw handles.
        unsafe { Ok(WindowHandle::borrow_raw(self.0.window_handle)) }
    }
}

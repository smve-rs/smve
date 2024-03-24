//! Components for the window system

use crate::core::window::icon;
use bevy_ecs::prelude::Component;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

/// A marker component for the primary window.
/// There should be only one primary window at any one time.
#[derive(Component)]
pub struct PrimaryWindow;

/// Component description of the window
///
/// This contains various parameters of the window.
#[derive(Component, Clone)]
pub struct Window {
    /// The width of the window
    pub width: u32,
    /// The height of the window
    pub height: u32,
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
            width: 800,
            height: 600,
            title: "Ruxel".to_string(),
            icon_width: icon::IMAGE_WIDTH as u32,
            icon_height: icon::IMAGE_HEIGHT as u32,
            icon_data: Some(icon::IMAGE_DATA.to_vec()),
            vsync: true,
        }
    }
}

/// A wrapper for the raw handle of the window
///
/// [`create_surface`](wgpu::Instance::create_surface) requires ownership to an object that implements [`HasWindowHandle`] and [`HasDisplayHandle`].
/// The winit [`Window`](winit::window::Window) is owned by [`WinitWindows`](crate::core::window::WinitWindows) and cannot be cloned.
/// This wrapper allows the raw handle to be cloned and passed to the [`create_surface`](wgpu::Instance::create_surface) function.
#[derive(Clone, Component)]
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

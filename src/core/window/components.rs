use crate::core::window::icon;
use bevy_ecs::prelude::Component;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

/// A marker for the primary window.
/// There should be only one primary window at any one time.
#[derive(Component)]
pub struct PrimaryWindow;

/// Component description of the window
#[derive(Component, Clone)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub title: String,
    /// 0 if there is no icon
    pub icon_width: u32,
    /// 0 if there is no icon
    pub icon_height: u32,
    /// A flat vector of RGBA data of the icon
    /// `None` if there is no icon
    pub icon_data: Option<Vec<u8>>,
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

#[derive(Clone, Component)]
pub struct RawHandleWrapper {
    pub display_handle: RawDisplayHandle,
    pub window_handle: RawWindowHandle,
}

// SAFETY:
// https://github.com/rust-windowing/raw-window-handle/pull/152
unsafe impl Send for RawHandleWrapper {}
unsafe impl Sync for RawHandleWrapper {}

impl RawHandleWrapper {
    pub unsafe fn get_handle(&self) -> ThreadLockedRawHandleWrapper {
        ThreadLockedRawHandleWrapper(self.clone())
    }
}

pub struct ThreadLockedRawHandleWrapper(RawHandleWrapper);

impl HasDisplayHandle for ThreadLockedRawHandleWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe { Ok(DisplayHandle::borrow_raw(self.0.display_handle)) }
    }
}

impl HasWindowHandle for ThreadLockedRawHandleWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe { Ok(WindowHandle::borrow_raw(self.0.window_handle)) }
    }
}

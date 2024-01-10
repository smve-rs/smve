use crate::core::window::icon;
use bevy_ecs::prelude::Component;

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
        }
    }
}

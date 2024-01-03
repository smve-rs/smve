use crate::core::window::icon;
use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct PrimaryWindow;

#[derive(Component, Clone)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub icon_width: u32,
    pub icon_height: u32,
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

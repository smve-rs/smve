/*
 * RustyCraft: a voxel engine written in Rust
 * Copyright (C)  2023  SunnyMonster
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use bevy_ecs::prelude::Component;
use crate::core::window::icon;

#[derive(Component)]
pub struct PrimaryWindow;

#[derive(Component, Clone)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub icon_width: u32,
    pub icon_height: u32,
    pub icon_data: Option<Vec<u8>>
}

impl Default for Window {
    fn default() -> Self {
        Window {
            width: 800,
            height: 600,
            title: "RustyCraft".to_string(),
            icon_width: icon::IMAGE_WIDTH as u32,
            icon_height: icon::IMAGE_HEIGHT as u32,
            icon_data: Some(icon::IMAGE_DATA.to_vec()),
        }
    }
}
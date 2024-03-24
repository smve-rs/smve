#![deny(missing_docs)]

#![doc(html_favicon_url = "https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/icon.png")]
#![doc(html_logo_url = "https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/icon.png")]

//! <picture>
//!     <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_dark.svg">
//!     <source media="(prefers-color-scheme: light)" srcset="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_light.svg">
//!     <img alt="Ruxel" width="200" src="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_dark.svg">
//! </picture>
//! 
//! A voxel engine written in Rust.

pub mod core;

use crate::core::window::WindowPlugin;
use bevy_app::prelude::*;
use env_logger::Env;

/// The main entry point for the application.
///
/// Initializes the logger and runs the [bevy application](https://docs.rs/bevy_app/latest/bevy_app/).
pub fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    App::new().add_plugins(WindowPlugin::default()).run();
}

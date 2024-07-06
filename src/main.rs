#![cfg_attr(feature = "windowed", windows_subsystem = "windows")]
#![deny(missing_docs)]
#![doc(html_favicon_url = "https://cdn.jsdelivr.net/gh/smve-rs/smve/images/icon.png")]
#![doc(html_logo_url = "https://cdn.jsdelivr.net/gh/smve-rs/smve/images/icon.png")]

//! <picture>
//!     <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/smve-rs/smve/images/title_logo_dark.svg">
//!     <source media="(prefers-color-scheme: light)" srcset="https://cdn.jsdelivr.net/gh/smve-rs/smve/images/title_logo_light.svg">
//!     <img alt="smve" width="200" src="https://cdn.jsdelivr.net/gh/smve-rs/smve/images/title_logo_dark.svg">
//! </picture>
//!
//! A voxel engine written in Rust.

mod client;
mod common;
mod plugins;

use crate::plugins::client::ClientPlugins;
use bevy_app::prelude::*;

/// The main entry point for the application.
///
/// Initializes the logger and runs the [bevy application](https://docs.rs/bevy_app/latest/bevy_app/).
pub fn main() -> AppExit {
    App::new().add_plugins(ClientPlugins).run()
}

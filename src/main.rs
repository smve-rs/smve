mod core;

use crate::core::window::WindowPlugin;
use bevy_app::prelude::*;
use env_logger::Env;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    App::new().add_plugins(WindowPlugin::default()).run();
}

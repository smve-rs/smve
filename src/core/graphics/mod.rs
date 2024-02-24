use crate::core::graphics::resources::GraphicsState;
use bevy_app::{App, Plugin, Update};
use crate::core::graphics::systems::{u_create_surface, u_destroy_surface, u_resize};

pub mod resources;
mod gpu_selection_utils;
mod systems;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        // TODO: Perhaps do this asynchronously instead of blocking?
        // By implementing ready() to check if the async process is done.
        app.insert_non_send_resource(pollster::block_on(GraphicsState::new()));
        app.add_systems(Update, u_create_surface);
        app.add_systems(Update, u_resize);
        app.add_systems(Update, u_destroy_surface);
    }
}

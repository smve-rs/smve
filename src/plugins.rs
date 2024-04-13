//! Module containing plugin groups

use crate::core::graphics::GraphicsPlugin;
use crate::game::GamePlugin;
use bevy_app::{PluginGroup, PluginGroupBuilder};

/// Default plugins for Ruxel
pub struct RuxelPlugins;

impl PluginGroup for RuxelPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        group = group
            .add(GraphicsPlugin)
            .add_after::<GraphicsPlugin, _>(GamePlugin);

        group
    }
}

//! Module containing plugin groups

use crate::client::core::graphics::GraphicsPlugin;
use crate::client::GamePlugin;
use crate::common::trace::TracePlugin;
use bevy_app::{PluginGroup, PluginGroupBuilder};
use crate::client::core::graphics::pipelined_rendering::PipelinedRenderingPlugin;
use crate::common::core::CorePlugin;

/// Default plugins for Ruxel
pub struct ClientPlugins;

impl PluginGroup for ClientPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        group = group
            .add(TracePlugin)
            .add(CorePlugin)
            .add(GraphicsPlugin)
            .add_after::<GraphicsPlugin, _>(GamePlugin)
            .add_after::<GraphicsPlugin, _>(PipelinedRenderingPlugin);

        group
    }
}

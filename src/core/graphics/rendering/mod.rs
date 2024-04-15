mod resources;
mod systems;
mod utils;
mod components;

use crate::core::graphics::rendering::systems::{rpq_begin_render_passes, rp_create_command_encoder, rfq_finish_queue, rr_render, rc_clear_entities};
use crate::core::graphics::{Render, RenderSet};
use crate::core::graphics::RenderSet::{Queue, Prepare, PreQueue, FinishQueue, CleanUp};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::IntoSystemConfigs;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Render,
            (
                rp_create_command_encoder.in_set(Prepare),
                rp_create_command_encoder.in_set(PreQueue),
                rpq_begin_render_passes.in_set(Queue),
                rfq_finish_queue.in_set(FinishQueue),
                rr_render.in_set(RenderSet::Render),
                rc_clear_entities.in_set(CleanUp)
            ),
        );
    }
}

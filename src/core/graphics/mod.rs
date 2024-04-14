//! Rendering code for the engine.
//!
//! This module contains the [`GraphicsPlugin`] which is responsible for initializing rendering with [`wgpu`](https://docs.rs/wgpu/latest/wgpu/index.html).

use crate::core::graphics::extract::camera::CameraExtractPlugin;
use crate::core::graphics::extract::window::WindowExtractPlugin;
use crate::core::graphics::resources::{GraphicsState, MainWorld, ScratchMainWorld};
use crate::core::graphics::systems::{rp_create_surface, rp_resize};
use crate::core::window::WindowPlugin;
use bevy_app::{App, AppLabel, Plugin, SubApp};
use bevy_ecs::prelude::{Schedule, SystemSet, World};
use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs, ScheduleLabel};

mod adapter_selection_utils;
pub mod camera;
pub mod extract;
pub mod resources;
mod systems;

/// Responsible for initializing rendering with wgpu.
///
/// This plugin initializes the graphics state and adds the necessary systems to create and destroy surfaces.
///
/// # Examples
///
/// * Creates a primary window with default settings, initializes the graphics state and creates a surface for the primary window.
/// ```rust
/// App::new().add_plugin(GraphicsPlugin).run();
/// ```
/// * Applies custom parameters to the WindowPlugin.
/// ```rust
/// App::new()
///     .add_plugins((
///         WindowPlugin {
///             primary_window: Some(Window {
///                 title: "New Title".to_string(),
///                 ..Default::default()
///             }),
///             ..Default::default()
///         },
///         GraphicsPlugin,
///     ))
///     .run();
/// ```
///
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WindowPlugin>() {
            app.add_plugins(WindowPlugin::default());
        }

        app.init_resource::<ScratchMainWorld>();

        let mut render_app_inner = App::empty();

        render_app_inner.add_schedule(Render::schedule());
        render_app_inner.add_schedule(Schedule::new(ExtractSchedule));
        render_app_inner.main_schedule_label = Render.intern();

        render_app_inner.insert_resource(pollster::block_on(GraphicsState::new()));
        render_app_inner.add_systems(
            Render,
            (
                (rp_create_surface, rp_resize).in_set(RenderSet::Prepare),
                World::clear_entities.in_set(RenderSet::CleanUp),
            ),
        );
        //render_app_inner.add_systems(
        //    Render,
        //    (rp_create_surface, rp_destroy_surface, rp_resize).in_set(RenderSet::Prepare),
        //);

        let render_app = SubApp::new(render_app_inner, extract);
        app.insert_sub_app(RenderSubApp, render_app);

        app.add_plugins(CameraExtractPlugin);
        app.add_plugins(WindowExtractPlugin);
    }
}

fn extract(world: &mut World, app: &mut App) {
    // Move app world into render app and replace app world with empty world
    let scratch_world = world
        .remove_resource::<ScratchMainWorld>()
        .expect("ScratchMainWorld should exist");
    let inserted_world = std::mem::replace(world, scratch_world.0);
    app.world.insert_resource(MainWorld(inserted_world));
    app.world.run_schedule(ExtractSchedule);

    // Move app world back and replace scratch world with empty world.
    let inserted_world = app
        .world
        .remove_resource::<MainWorld>()
        .expect("MainWorld should exist");
    let scratch_world = std::mem::replace(world, inserted_world.0);
    world.insert_resource(ScratchMainWorld(scratch_world));
}

/// System sets for the Render schedule
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSet {
    /// Prepare resources and entities needed for rendering
    Prepare,
    /// Rendering happens here
    Render,
    /// Clean up the ECS world after rendering
    CleanUp,
}

/// Schedule label of the Render schedule
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Render;

impl Render {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::new(Render);

        schedule
            .configure_sets((RenderSet::Prepare, RenderSet::Render, RenderSet::CleanUp).chain());

        schedule
    }
}

/// App label for the Render sub app
#[derive(AppLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct RenderSubApp;

/// Schedule label of the Extract schedule
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ExtractSchedule;

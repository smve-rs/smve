//! Used to extract Windows into the render world

use crate::client::core::graphics::extract::utils::extract_param::Extract;
use crate::client::core::graphics::resources::{ExtractedWindows, GraphicsState};
use crate::client::core::graphics::{ExtractSchedule, RenderSubApp};
use crate::client::core::window::components::{PrimaryWindow, RawHandleWrapper, Window};
use crate::client::core::window::events::CloseRequestedEvent;
use bevy_app::{App, Plugin};
use bevy_ecs::entity::Entity;
use bevy_ecs::event::EventReader;
use bevy_ecs::system::{Query, ResMut};

/// Responsible for extracting the windows into the render world
pub struct WindowExtractPlugin;

impl Plugin for WindowExtractPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            .get_sub_app_mut(RenderSubApp)
            .expect("RenderSubApp should exist");
        render_app.init_resource::<ExtractedWindows>();
        render_app.add_systems(ExtractSchedule, e_extract_windows);
    }
}

/// A representation of the window in the render world
pub struct ExtractedWindow {
    /// Physical width in pixels of the window
    pub physical_width: u32,
    /// Physical height in pixels of the window
    pub physical_height: u32,
    /// Whether V-Sync is enabled for the window
    pub vsync: bool,
    /// Raw handles of the window
    pub raw_handles: RawHandleWrapper,
    /// Whether the window size has changed since last frame
    pub size_changed: bool,
    /// Whether the vsync value was changed since last frame
    pub present_mode_changed: bool,
}

/// System added to the extract schedule to extract windows
fn e_extract_windows(
    mut extracted_windows: ResMut<ExtractedWindows>,
    main_world_query: Extract<Query<(Entity, &Window, &RawHandleWrapper, Option<&PrimaryWindow>)>>,
    mut closed_windows: Extract<EventReader<CloseRequestedEvent>>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
) {
    for (entity, window, handle, primary) in main_world_query.iter() {
        if primary.is_some() {
            extracted_windows.primary = Some(entity);
        }

        let (new_width, new_height) = (
            // Make sure the window size isn't 0x0
            window.resolution.physical_width().max(1),
            window.resolution.physical_height().max(1),
        );

        let extracted_window = extracted_windows.entry(entity).or_insert(ExtractedWindow {
            physical_width: new_width,
            physical_height: new_height,
            vsync: window.vsync,
            raw_handles: handle.clone(),
            size_changed: false,
            present_mode_changed: false,
        });

        // This relies on the fact that `extracted_window` will reflect the old values if it already exists
        extracted_window.size_changed = new_width != extracted_window.physical_width
            || new_height != extracted_window.physical_height;
        extracted_window.present_mode_changed = window.vsync != extracted_window.vsync;

        if extracted_window.size_changed {
            extracted_window.physical_width = new_width;
            extracted_window.physical_height = new_height;
        }

        if extracted_window.vsync {
            extracted_window.vsync = window.vsync;
        }
    }

    for closed_window in closed_windows.read() {
        extracted_windows.remove(&closed_window.entity);
        graphics_state.destroy_surface(closed_window.entity);
    }
}

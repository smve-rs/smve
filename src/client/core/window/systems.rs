//! Systems managing winit windows and window components.

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use log::{info, warn};
use winit::dpi::LogicalSize;

use crate::client::core::window::components::{CachedWindow, PrimaryWindow, Window};
use crate::client::core::window::events::{CloseRequestedEvent, WindowResizedEvent};
use crate::client::core::window::resources::WinitWindows;

/// System to update the physical window when a value is changed on the [`Window`] component
///
/// Called on `Late` to update the winit window when the window component changes
pub fn l_update_windows(
    mut query: Query<(Entity, &mut Window, &mut CachedWindow), Changed<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut window_resized: EventWriter<WindowResizedEvent>,
) {
    for (entity, mut window, mut cache) in query.iter_mut() {
        let Some(winit_window) = winit_windows.get_window(entity) else {
            continue;
        };

        if window.resolution != cache.0.resolution {
            //info!("Update window resolution: {}, {}, {}", window.resolution.physical_width(), window.resolution.physical_height(), window.resolution.scale_factor());
            if let Some(size_now) = winit_window.request_inner_size(window.resolution.size()) {
                window.resolution.set_physical_size(size_now);

                window_resized.send(WindowResizedEvent {
                    entity,
                    new_width: window.resolution.width(),
                    new_height: window.resolution.height(),
                });
            }
        }

        if window.title != cache.0.title {
            winit_window.set_title(&window.title);
        }

        if window.icon_data != cache.0.icon_data {
            if let Some(icon_data) = window.icon_data.clone() {
                winit_window.set_window_icon(Some(
                    winit::window::Icon::from_rgba(
                        icon_data,
                        window.icon_width,
                        window.icon_height,
                    )
                    .expect("Failed to create window icon"),
                ));
            } else {
                winit_window.set_window_icon(None);
            }
        }

        cache.0 = window.clone();
    }
}

/// System to update window component when winit windows get resized
///
/// Called on `Late`
pub fn l_react_to_resize(
    mut window_resized: EventReader<WindowResizedEvent>,
    mut query: Query<&mut Window>,
) {
    for event in window_resized.read() {
        let mut window = query
            .get_mut(event.entity)
            .expect("Window component should exist");
        window
            .resolution
            .set_logical_size(LogicalSize::new(event.new_width, event.new_height));

        //info!("React to resize: {}, {}, {}", window.resolution.physical_width(), window.resolution.physical_height(), window.resolution.scale_factor());
    }
}

/// System to make sure there is only ever one primary window and every primary window has a window component
/// Called on Update and will remove the primary window component from any duplicates found and any primary windows without a window component
pub fn u_primary_window_check(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&Window>), Added<PrimaryWindow>>,
    mut primary_window_count: Local<u32>,
) {
    for (entity, window) in query.iter_mut() {
        if window.is_none() {
            warn!(
                "Entity {:?} has a PrimaryWindow component but no Window component, removing PrimaryWindow",
                entity
            );
            commands.entity(entity).remove::<PrimaryWindow>();
            continue;
        }

        let window = window.expect("Window component should exist");

        *primary_window_count += 1;
        if *primary_window_count > 1 {
            warn!(
                "A primary window already exists, removing PrimaryWindow component from entity {:?} with window titled {}",
                entity, window.title
            );
            commands.entity(entity).remove::<PrimaryWindow>();
            *primary_window_count -= 1;
        }
    }
}

/// System to despawn a Window entity when a close event is received
///
/// Called on Update when a [`CloseRequestedEvent`] is received.
pub fn u_despawn_windows(
    mut commands: Commands,
    mut close_requested_event: EventReader<CloseRequestedEvent>,
) {
    for event in close_requested_event.read() {
        commands.entity(event.entity).despawn();
    }
}

/// System to close the winit window when a Window entity is despawned
///
/// Called on PostUpdate (after [`u_despawn_windows`]) when a Window entity is despawned.
pub fn pu_close_windows(
    mut removed_windows: RemovedComponents<Window>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for entity in removed_windows.read() {
        winit_windows
            .destroy_window(entity)
            .expect("Entity should have a winit-window");
    }
}

/// Exits the app when the primary window is closed
///
/// Called on PostUpdate when the primary window is closed.
/// Emits an [`AppExit`] event when the primary window is closed.
pub fn pu_exit_on_primary_closed(
    mut app_exit_event: EventWriter<AppExit>,
    windows: Query<(), (With<Window>, With<PrimaryWindow>)>,
) {
    if windows.is_empty() {
        info!("Primary window closed, exiting");
        app_exit_event.send(AppExit);
    }
}

/// Exits the app when all windows are closed
///
/// Called on PostUpdate when all windows are closed.
/// Emits an [`AppExit`] event when all windows are closed.
pub fn pu_exit_on_all_closed(
    mut app_exit_event: EventWriter<AppExit>,
    windows: Query<(), With<Window>>,
) {
    if windows.is_empty() {
        info!("All windows closed, exiting");
        app_exit_event.send(AppExit);
    }
}

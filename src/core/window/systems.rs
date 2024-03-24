use crate::core::window::components::{PrimaryWindow, Window};
use crate::core::window::events::CloseRequestedEvent;
use crate::core::window::resources::{PrimaryWindowCount, WinitWindows};
use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use log::{info, warn};

/// System to make sure there is only ever one primary window
/// Called on Update and will remove the primary window component from any duplicates found
pub fn u_primary_window_check(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&Window>), Added<PrimaryWindow>>,
    mut primary_window_count: ResMut<PrimaryWindowCount>,
) {
    for (entity, window) in query.iter_mut() {
        primary_window_count.0 += 1;
        if primary_window_count.0 > 1 {
            let with_window_titled = if let Some(window) = window {
                format!("with Window titled \"{}\"", window.title)
            } else {
                "with no Window component".to_string()
            };
            warn!(
                "A primary window already exists, removing PrimaryWindow component from entity {:?} {}",
                entity, with_window_titled
            );
            commands.entity(entity).remove::<PrimaryWindow>();
            primary_window_count.0 -= 1;
        }
    }
}

/// System to despawn a Window entity when a close event is received
///
/// Called on Update when a [`CloseRequestedEvent`] is received.
pub fn u_despawn_windows(
    mut commands: Commands,
    mut close_requested_event: EventReader<CloseRequestedEvent>,
    winit_windows: NonSendMut<WinitWindows>,
) {
    for event in close_requested_event.read() {
        let entity = winit_windows.window_to_entity[&event.window_id];
        commands.entity(entity).despawn();
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
        winit_windows.destroy_window(entity);
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

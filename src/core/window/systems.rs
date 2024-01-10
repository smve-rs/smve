use crate::core::window::components::{PrimaryWindow, Window};
use crate::core::window::events::CloseRequestedEvent;
use crate::core::window::resources::{PrimaryWindowCount, WinitWindows};
use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use log::{info, warn};

/// System to make sure there is ever one primary window
/// It will remove the primary window component from any duplicates found
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

/// This closes and despawns a window when a close requested event is emitted
pub fn u_close_windows(
    mut commands: Commands,
    mut close_requested_event: EventReader<CloseRequestedEvent>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for event in close_requested_event.read() {
        winit_windows.windows.remove(&event.window_id);
        let entity = winit_windows
            .window_to_entity
            .remove(&event.window_id)
            .unwrap();
        commands.entity(entity).despawn();
        winit_windows.window_to_entity.remove(&event.window_id);
        winit_windows.entity_to_window.remove(&entity);
    }
}

/// Exits the app when the primary window is closed
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
pub fn pu_exit_on_all_closed(
    mut app_exit_event: EventWriter<AppExit>,
    windows: Query<(), With<Window>>,
) {
    if windows.is_empty() {
        info!("All windows closed, exiting");
        app_exit_event.send(AppExit);
    }
}

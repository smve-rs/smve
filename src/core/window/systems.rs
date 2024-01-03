/*
 * Ruxel: a voxel engine written in Rust
 * Copyright (C)  2023  SunnyMonster
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::core::window::components::{PrimaryWindow, Window};
use crate::core::window::events::CloseRequestedEvent;
use crate::core::window::resources::{PrimaryWindowCount, WinitWindows};
use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use log::{info, warn};

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

pub fn pu_exit_on_primary_closed(
    mut app_exit_event: EventWriter<AppExit>,
    windows: Query<(), (With<Window>, With<PrimaryWindow>)>,
) {
    if windows.is_empty() {
        info!("Primary window closed, exiting");
        app_exit_event.send(AppExit);
    }
}

pub fn pu_exit_on_all_closed(
    mut app_exit_event: EventWriter<AppExit>,
    windows: Query<(), With<Window>>,
) {
    if windows.is_empty() {
        info!("All windows closed, exiting");
        app_exit_event.send(AppExit);
    }
}

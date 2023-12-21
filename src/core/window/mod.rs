/*
 * RustyCraft: a voxel engine written in Rust
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


mod resources;

mod icon;

use bevy_app::{App, Plugin};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop};
use winit::window::{Icon, WindowBuilder};
use crate::core::window::resources::Window;

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.set_runner(runner);
    }
}

fn runner(mut app: App) {
    info!("Opening window...");
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("RustyCraft")
        .with_window_icon(Some(Icon::from_rgba(icon::IMAGE_DATA.to_vec(), icon::IMAGE_WIDTH as u32, icon::IMAGE_HEIGHT as u32).expect("Bad Icon")))
        .build(&event_loop).unwrap();

    app.insert_resource(Window(window));

    info!("Entered event loop");

    let mut should_update = true;
    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("Window closed, exiting...");
                should_update = false;
                window_target.exit();
            },
            Event::AboutToWait => {
                if should_update {
                    app.update();
                }
            },
            _ => {}
        };
    }).expect("Event Loop Error");
}
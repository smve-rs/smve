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

use crate::core::ecs::Ecs;
use bevy_ecs::prelude::*;

mod core;

#[derive(Component)]
struct Name {
    name: String,
}

fn print_name(mut query: Query<&mut Name>) {
    for mut name in &mut query {
        println!("{}", name.name);
        name.name = "Hello".into();
    }
}

fn main() {
    let mut ecs = Ecs::new();

    ecs.world.spawn(Name {
        name: "Hello World".into(),
    });

    ecs.schedule.add_systems(print_name);

    ecs.run();
}

// Ideal main function:
// fn main() {
//     let mut ecs = Ecs::new();
//
//     let scene = Scene::deserialize("example.rcscene");
//
//     ecs.load_scene(&scene);
//
//     loop {
//         ecs.run();
//     }
// }

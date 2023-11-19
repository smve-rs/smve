/*
 * RustyCraft - A voxel engine written in Rust
 * Copyright (C) 2023 SunnyMonster
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

use bevy_ecs::prelude::*;

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
    let mut world = World::new();

    world.spawn(
        Name { name: "Hello World".into() }
    );

    let mut schedule = Schedule::default();

    schedule.add_systems(print_name);

    schedule.run(&mut world);
    schedule.run(&mut world);
}

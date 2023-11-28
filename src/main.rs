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

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
                        Name 
        
        
        {
        name: "Hello World".into(),
    });

    let mut schedule = Schedule::default();

    schedule.add_systems(print_name);

    schedule.run(&mut world);
    schedule.run(&mut world);
}

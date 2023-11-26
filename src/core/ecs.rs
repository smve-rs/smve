use bevy_ecs::prelude::*;

pub struct Ecs {
    pub world: World,
    pub schedule: Schedule,
}

impl Ecs {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            schedule: Schedule::default(),
        }
    }

    pub fn run(&mut self) {
        self.schedule.run(&mut self.world);
    }
}

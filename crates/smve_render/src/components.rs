use bevy_ecs::component::Component;
use bevy_render::extract_component::ExtractComponent;

#[derive(Clone, Component, ExtractComponent)]
pub struct Triangle;

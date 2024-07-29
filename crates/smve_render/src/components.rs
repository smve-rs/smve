//! Components used by the renderer

use bevy_ecs::component::Component;
use bevy_render::extract_component::ExtractComponent;

/// Represents a triangle in the ECS
#[derive(Clone, Component, ExtractComponent)]
pub struct Triangle;

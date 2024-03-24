//! Bevy resources for the windowing module.

use crate::core::window::components::Window;
use bevy_ecs::prelude::{Entity, Resource};
use log::{info, warn};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;
use winit::dpi::LogicalSize;
use winit::window::{Icon, WindowBuilder};

/// Resource to keep track of the number of primary windows
/// Used in a system to make sure there is only ever one primary window
#[derive(Resource, Default)]
pub struct PrimaryWindowCount(pub u32);

/// Resource used to keep track of all the windows
///
/// This creates an association between the entity and the winit window associated with it
pub struct WinitWindows {
    /// Maps from ID (which can be cloned, moved, etc.) to the winit window (which this resource exclusively owns)
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    /// Maps from entity to window ID
    pub entity_to_window: HashMap<Entity, winit::window::WindowId>,
    /// Maps from window ID to entity
    pub window_to_entity: HashMap<winit::window::WindowId, Entity>,
    // Many winit functions are not Send or Sync, so this resource is not Send or Sync
    _not_send_sync: PhantomData<*const ()>,
}

impl Default for WinitWindows {
    fn default() -> Self {
        WinitWindows {
            windows: HashMap::new(),
            entity_to_window: HashMap::new(),
            window_to_entity: HashMap::new(),
            _not_send_sync: PhantomData,
        }
    }
}

impl WinitWindows {
    /// Creates a winit window, configures it and associates it with an entity.
    pub fn create_window(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        entity: Entity,
        window: &Window,
    ) -> &winit::window::Window {
        info!("Opening window {} on {:?}", window.title, entity);
        let mut window_builder = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(window.width, window.height))
            .with_title(&window.title);
        if let Some(icon_data) = window.icon_data.clone() {
            window_builder = window_builder.with_window_icon(Some(
                Icon::from_rgba(icon_data, window.icon_width, window.icon_height)
                    .expect("Bad Icon"),
            ));
        }
        let winit_window = window_builder.build(event_loop).unwrap();
        self.entity_to_window.insert(entity, winit_window.id());
        self.window_to_entity.insert(winit_window.id(), entity);

        match self.windows.entry(winit_window.id()) {
            Entry::Occupied(e) => {
                warn!("I'm not sure what happened but a Window with the same ID already exists");
                &*e.into_mut()
            }
            Entry::Vacant(e) => &*e.insert(winit_window),
        }
    }

    /// Destroys a window and removes it from the resource.
    pub fn destroy_window(&mut self, entity: Entity) {
        let window = self.entity_to_window.remove(&entity).unwrap();
        self.window_to_entity.remove(&window);
        self.windows.remove(&window);
    }
}

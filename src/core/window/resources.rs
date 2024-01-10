use crate::core::window::components::Window;
use bevy_ecs::prelude::{Entity, Resource};
use log::info;
use std::collections::HashMap;
use std::marker::PhantomData;
use winit::dpi::LogicalSize;
use winit::window::{Icon, WindowBuilder};

/// Resource to keep track of the number of primary windows
/// Used in a system to make sure there is only ever one primary window
#[derive(Resource, Default)]
pub struct PrimaryWindowCount(pub u32);

/// Contains a map from the entity to the window and vice versa
pub struct WinitWindows {
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    pub entity_to_window: HashMap<Entity, winit::window::WindowId>,
    pub window_to_entity: HashMap<winit::window::WindowId, Entity>,
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
    /// Only called from a system to open any windows based on their Window component
    pub fn create_window(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
        entity: Entity,
        window: &Window,
    ) {
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
        self.windows.insert(winit_window.id(), winit_window);
    }
}

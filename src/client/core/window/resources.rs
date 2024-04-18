//! Bevy resources for the windowing module.

use crate::client::core::window::components::Window;
use bevy_ecs::prelude::Entity;
use log::{info, warn};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use winit::window::{BadIcon, Icon, WindowBuilder, WindowId};

/// Resource used to keep track of all the windows
///
/// This creates an association between the entity and the winit window associated with it
pub struct WinitWindows {
    /// Maps from ID (which can be cloned, moved, etc.) to the winit window (which this resource exclusively owns)
    pub windows: HashMap<WindowId, winit::window::Window>,
    /// Maps from entity to window ID
    pub entity_to_window: HashMap<Entity, WindowId>,
    /// Maps from window ID to entity
    pub window_to_entity: HashMap<WindowId, Entity>,
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
    ) -> Result<&winit::window::Window, WindowError> {
        info!("Opening window {} on {:?}", window.title, entity);

        let mut window_builder = WindowBuilder::new()
            .with_inner_size(window.resolution.size())
            .with_title(&window.title);
        if let Some(icon_data) = window.icon_data.clone() {
            window_builder = window_builder.with_window_icon(Some(
                Icon::from_rgba(icon_data, window.icon_width, window.icon_height)
                    .map_err(WindowError::IconError)?,
            ));
        }

        let winit_window = window_builder
            .build(event_loop)
            .map_err(WindowError::WindowCreationError)?;

        self.entity_to_window.insert(entity, winit_window.id());
        self.window_to_entity.insert(winit_window.id(), entity);

        match self.windows.entry(winit_window.id()) {
            Entry::Occupied(e) => {
                warn!("I'm not sure what happened but a Window with the same ID already exists");
                Ok(e.into_mut())
            }
            Entry::Vacant(e) => Ok(e.insert(winit_window)),
        }
    }

    /// Gets the winit window associated with an entity.
    pub fn get_window(&self, entity: Entity) -> Option<&winit::window::Window> {
        self.entity_to_window
            .get(&entity)
            .and_then(|window_id| self.windows.get(window_id))
    }

    /// Destroys a window and removes it from the resource.
    pub fn destroy_window(&mut self, entity: Entity) -> Result<(), WindowError> {
        let window = self.entity_to_window.remove(&entity);
        if let Some(window) = window {
            self.windows.remove(&window);
            self.window_to_entity.remove(&window);
            Ok(())
        } else {
            Err(WindowError::WindowEntityError(entity))
        }
    }

    /// Gets the entity associated with a window.
    pub fn get_window_entity(&self, window_id: WindowId) -> Option<Entity> {
        self.window_to_entity.get(&window_id).cloned()
    }
}

/// Handling various errors related to windowing
pub enum WindowError {
    /// Error when an entity does not have a window associated with it
    WindowEntityError(Entity),
    /// Error on failure to load an icon
    IconError(BadIcon),
    /// Error on failure to create a window
    WindowCreationError(winit::error::OsError),
}

impl Debug for WindowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::WindowEntityError(entity) => {
                write!(
                    f,
                    "Entity {:?} does not have a window associated with it",
                    entity
                )
            }
            WindowError::IconError(bad_icon) => {
                write!(f, "Failed to load icon: {:?}", bad_icon)
            }
            WindowError::WindowCreationError(os_error) => {
                write!(f, "Failed to create window: {:?}", os_error)
            }
        }
    }
}

impl Display for WindowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::WindowEntityError(entity) => {
                write!(
                    f,
                    "Entity {:?} does not have a window associated with it",
                    entity
                )
            }
            WindowError::IconError(bad_icon) => {
                write!(f, "Failed to load icon: {bad_icon}")
            }
            WindowError::WindowCreationError(os_error) => {
                write!(f, "Failed to create window: {os_error}")
            }
        }
    }
}

impl Error for WindowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WindowError::WindowEntityError(_) => None,
            WindowError::IconError(bad_icon) => Some(bad_icon),
            WindowError::WindowCreationError(os_error) => Some(os_error),
        }
    }
}

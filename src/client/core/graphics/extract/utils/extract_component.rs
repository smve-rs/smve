//! Utility trait to easily extract components into the render world by cloning

use crate::client::core::graphics::extract::utils::extract_param::Extract;
use crate::client::core::graphics::{ExtractSchedule, RenderSubApp};
use bevy_app::{App, Plugin};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Component, Local};
use bevy_ecs::query::{QueryFilter, QueryItem, ReadOnlyQueryData};
use bevy_ecs::system::{Commands, Query};
use std::marker::PhantomData;

/// A trait representing the extraction from the main world to the render world
pub trait ExtractComponent: Component {
    /// The data part of the query
    ///
    /// # Example
    /// For this query:
    /// ```rust
    /// Query<(Entity, Camera)>
    /// ```
    /// the query data should be
    /// ```rust
    /// QueryData = (Entity, Camera)
    /// ```
    type QueryData: ReadOnlyQueryData;
    /// The filter part of the query
    ///
    /// # Example
    /// For this query:
    /// ```rust
    /// Query<(Entity, Camera), With<PrimaryWindow>>
    /// ```
    /// the query filter should be
    /// ```rust
    /// QueryFilter = With<PrimaryWindow>
    /// ```
    type QueryFilter: QueryFilter;

    /// This bundle will be added to the render world after the extraction
    type Out: Bundle;

    /// Defines how the component is transferred to the render world
    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out>;
}

/// Add this plugin to the main app to extract the component
///
/// # Generics
/// - `C`: The component implementing the [`ExtractComponent`] trait.
pub struct ExtractComponentPlugin<C> {
    /// Marks the type of the plugin.
    /// It contains a function pointer so that `C` does not need to implement [`Default`]
    marker: PhantomData<fn() -> C>,
}

impl<C> Default for ExtractComponentPlugin<C> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<C: ExtractComponent> Plugin for ExtractComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderSubApp) {
            render_app.add_systems(ExtractSchedule, e_extract_components::<C>);
        }
    }
}

/// A system that runs the [`extract_component`][1] function for all the components found
///
/// Runs on `Extract`.
///
/// [1]: ExtractComponent::extract_component()
fn e_extract_components<C: ExtractComponent>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, query_item) in &query {
        if let Some(component) = C::extract_component(query_item) {
            values.push((entity, component));
        }
    }

    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

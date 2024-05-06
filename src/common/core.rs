//! Contains core engine code

use bevy_app::{App, Plugin};
use bevy_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, TaskPool};

/// Contains core engine code (Initialize task pools for now)
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {
        ComputeTaskPool::get_or_init(TaskPool::default);
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        IoTaskPool::get_or_init(TaskPool::default);
    }
}

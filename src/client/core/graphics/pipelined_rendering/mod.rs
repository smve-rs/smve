//! Contains the [`PipelinedRenderingPlugin`].

use crate::client::core::graphics::RenderSubApp;
use async_channel::{Receiver, Sender};
use bevy_app::{App, AppExit, AppLabel, Plugin, SubApp};
use bevy_ecs::change_detection::Mut;
use bevy_ecs::prelude::World;
use bevy_ecs::schedule::MainThreadExecutor;
use bevy_ecs::system::Resource;
use bevy_tasks::ComputeTaskPool;
use tracing::debug;

/// This plugin manages the pipelined rendering.
///
/// It removes the render sub app from the main app, spawns a separate thread for rendering
/// and manages the render sub app (calling extract and update, etc.)
///
/// Order of execution:
/// ```text
/// |--------------------------------------------------------------------|
/// |         | RenderExtractApp schedule | winit events | main schedule |
/// | extract |----------------------------------------------------------|
/// |         | extract commands | rendering schedule                    |
/// |--------------------------------------------------------------------|
/// ```
pub struct PipelinedRenderingPlugin;

impl Plugin for PipelinedRenderingPlugin {
    fn build(&self, app: &mut App) {
        // If render app doesn't exist, don't do anything with pipelined rendering
        if app.get_sub_app(RenderSubApp).is_err() {
            return;
        }

        // This is used in the extract to receive the render app onto the main thread.
        app.insert_resource(MainThreadExecutor::new());

        let sub_app_inner = App::new();
        let sub_app = SubApp::new(sub_app_inner, renderer_extract);
        app.insert_sub_app(PipelinedRenderingApp, sub_app);
    }

    fn cleanup(&self, app: &mut App) {
        // Don't continue if render app doesn't exist
        if app.get_sub_app(RenderSubApp).is_err() {
            return;
        }

        // Create the channels for sending and receiving the render app between threads
        let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
        let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);

        let mut render_app = app
            .remove_sub_app(RenderSubApp)
            .expect("This function is expected to return if render sub app doesn't exist.");

        // Give the render app a copy of the executor for running some systems on the main thread
        let executor = app
            .world
            .get_resource::<MainThreadExecutor>()
            .expect("Executor is added in build().");
        render_app.app.insert_resource(executor.clone());

        // Somewhat unintuitively, we are sending the render app from the main thread to the main thread.
        // This is because when the extract runs, it expects the render sub app from the render thread,
        // but obviously the render thread won't have run yet. So we are using this sender to fool it
        // into thinking that it came from the render thread.
        render_to_app_sender
            .send_blocking(render_app)
            .expect("Channel should not be closed.");

        // Add the app sender and receivers to the main world
        app.insert_resource(RenderAppChannels::new(
            app_to_render_sender,
            render_to_app_receiver,
        ));

        // Start the render thread
        std::thread::Builder::new()
            .name("Render Thread".to_string())
            .spawn(move || {
                #[cfg(feature = "trace")]
                let _span = tracing::info_span!("render thread").entered();

                let compute_task_pool = ComputeTaskPool::get();
                loop {
                    // Wait until main thread is done with the render app (and sends it over)
                    let sent_app = compute_task_pool
                        .scope(|s| {
                            s.spawn(async { app_to_render_receiver.recv().await });
                        })
                        .pop();

                    let Some(Ok(mut render_app)) = sent_app else {
                        break;
                    };

                    // Runs the render schedules
                    {
                        #[cfg(feature = "trace")]
                        let _sub_app_span =
                            tracing::info_span!("sub app", name = ?RenderSubApp).entered();
                        render_app.app.update();
                    }

                    // Send it back to the main thread once we have finished rendering
                    if render_to_app_sender.send_blocking(render_app).is_err() {
                        break;
                    }
                }

                debug!("Exiting render thread");
            })
            .unwrap_or_else(|e| {
                panic!("Unable to create render thread: {e}");
            });
    }
}

/// The extract for the pipelined render sub app.
///
/// It receives the render app from the render thread, runs its extract and sends it back to
/// the render thread.
fn renderer_extract(world: &mut World, _app: &mut App) {
    // Get both the executor and the channels from the main world
    world.resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
        world.resource_scope(|world, mut render_channels: Mut<RenderAppChannels>| {
            // Receive the render app from the render thread
            if let Some(mut render_app) = ComputeTaskPool::get()
                .scope_with_executor(true, Some(&*main_thread_executor.0), |s| {
                    s.spawn(async { render_channels.recv().await });
                })
                .pop()
                .expect("Render app should exist")
            {
                // Extract objects from main world to render world
                render_app.extract(world);

                // Send render app back to render world after extraction.
                render_channels.send_blocking(render_app);
            } else {
                // Render thread has panicked.
                world.send_event(AppExit);
            }
        });
    });
}

/// Sub app label for the pipelined rendering app
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct PipelinedRenderingApp;

/// The resource containing the sender and receivers the main thread manages.
///
/// This is used to send the render app to the render thread and receiving it from the render thread.
#[derive(Resource)]
pub struct RenderAppChannels {
    /// Sender used to send the render app to the render thread
    app_to_render_sender: Sender<SubApp>,
    /// Receiver used to receive the render app from the render thread
    render_to_app_receiver: Receiver<SubApp>,
    /// Used on [`Drop`] to receive the render app back before dropping it
    render_app_in_render_thread: bool,
}

impl RenderAppChannels {
    /// Create a `RenderAppChannels` from a [`Receiver`] and [`Sender`]
    fn new(app_to_render_sender: Sender<SubApp>, render_to_app_receiver: Receiver<SubApp>) -> Self {
        RenderAppChannels {
            app_to_render_sender,
            render_to_app_receiver,
            render_app_in_render_thread: false,
        }
    }

    /// Blocks while sending the render app back to the render thread
    fn send_blocking(&mut self, render_app: SubApp) {
        self.app_to_render_sender
            .send_blocking(render_app)
            .expect("Channel should not be closed.");
        self.render_app_in_render_thread = true;
    }

    /// Asynchronously receives the render app back from the render thread
    async fn recv(&mut self) -> Option<SubApp> {
        let render_app = self.render_to_app_receiver.recv().await.ok()?;
        self.render_app_in_render_thread = false;
        Some(render_app)
    }
}

impl Drop for RenderAppChannels {
    fn drop(&mut self) {
        if self.render_app_in_render_thread {
            // Non-send data in the render world was initialized on the main thread
            // So when the app ends, we receive it back so that the drop methods runs on the right
            // thread. (From bevy)
            self.render_to_app_receiver.recv_blocking().ok();
        }
    }
}

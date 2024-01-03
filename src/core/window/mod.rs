pub mod resources;

pub mod components;
pub mod events;
pub mod icon;
pub mod systems;

use crate::core::window::components::{PrimaryWindow, Window};
use crate::core::window::events::CloseRequestedEvent;
use crate::core::window::resources::{PrimaryWindowCount, WinitWindows};
use crate::core::window::systems::{
    pu_exit_on_all_closed, pu_exit_on_primary_closed, u_close_windows, u_primary_window_check,
};
use bevy_app::prelude::*;
use bevy_app::{AppExit, PluginsState};
use bevy_ecs::event::ManualEventReader;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use log::{error, info};
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};

pub struct WindowPlugin {
    pub primary_window: Option<Window>,
    pub exit_condition: ExitCondition,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(Window::default()),
            exit_condition: ExitCondition::default(),
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CloseRequestedEvent>();

        if let Some(primary_window) = &self.primary_window {
            app.world
                .spawn(primary_window.clone())
                .insert(PrimaryWindow);
        }

        match self.exit_condition {
            ExitCondition::OnPrimaryClosed => {
                app.add_systems(PostUpdate, pu_exit_on_primary_closed);
            }
            ExitCondition::OnAllClosed => {
                app.add_systems(PostUpdate, pu_exit_on_all_closed);
            }
            ExitCondition::DontExit => {}
        }

        app.insert_non_send_resource(EventLoop::new().unwrap());
        app.insert_non_send_resource(WinitWindows::default());
        app.insert_resource(PrimaryWindowCount::default());
        app.add_systems(Update, u_primary_window_check);
        app.add_systems(Update, u_close_windows);
        app.set_runner(runner);
    }
}

fn runner(mut app: App) {
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }

    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<()>>()
        .unwrap();

    let mut create_windows_system_state: SystemState<(
        Query<(Entity, &Window), Added<Window>>,
        NonSendMut<WinitWindows>,
    )> = SystemState::from_world(&mut app.world);

    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();

    // ! Temporary fix of extra AboutToWait events on windows
    let mut exited = false;

    let event_handler = move |event: Event<()>, window_target: &EventLoopWindowTarget<()>| {
        if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
            if app_exit_event_reader.read(app_exit_events).last().is_some() {
                window_target.exit();
                exited = true;
                return;
            }
        }

        match event {
            Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init => {
                    let (query, winit_windows) =
                        create_windows_system_state.get_mut(&mut app.world);
                    create_windows(query, winit_windows, window_target);
                    create_windows_system_state.apply(&mut app.world);
                }
                _ => {}
            },
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } => {
                // Close window
                app.world.send_event(CloseRequestedEvent { window_id })
            }
            Event::AboutToWait => {
                if app.plugins_state() == PluginsState::Cleaned && !exited {
                    app.update();

                    if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                        if app_exit_event_reader.read(app_exit_events).last().is_some() {
                            window_target.exit();
                            exited = true;
                            return;
                        }
                    }
                }
            }
            _ => {}
        };

        let (query, winit_windows) = create_windows_system_state.get_mut(&mut app.world);
        create_windows(query, winit_windows, window_target);
        create_windows_system_state.apply(&mut app.world);
    };

    event_loop.set_control_flow(ControlFlow::Poll);
    info!("Entered event loop");
    if let Err(err) = event_loop.run(event_handler) {
        error!("winit event loop error: {err}");
    }
}

fn create_windows(
    query: Query<(Entity, &Window), Added<Window>>,
    mut winit_windows: NonSendMut<WinitWindows>,
    event_loop: &EventLoopWindowTarget<()>,
) {
    for (entity, window) in query.iter() {
        if winit_windows.entity_to_window.contains_key(&entity) {
            continue;
        }

        winit_windows.create_window(event_loop, entity, window);
    }
}

#[allow(dead_code)]
pub enum ExitCondition {
    OnPrimaryClosed,
    OnAllClosed,
    DontExit,
}

impl Default for ExitCondition {
    fn default() -> Self {
        ExitCondition::OnAllClosed
    }
}

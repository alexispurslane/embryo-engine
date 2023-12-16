use std::{
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use crate::{
    entity::{camera_component::CameraComponent, transform_component::TransformComponent},
    events,
    render_gl::resources::ResourceManager,
    scene::{self, GameState, RenderCameraState, RenderStateEvent},
    systems, CONFIG,
};

pub fn updater(
    mut game_state: GameState,
    resource_manager: &ResourceManager,
    render_state_sender: Sender<RenderStateEvent>,
    event_receiver: Receiver<scene::Event>,

    window: &sdl2::video::Window,

    running: Arc<AtomicBool>,
) {
    let (width, height) = window.size();
    let core_ids = core_affinity::get_core_ids().unwrap();
    let running = running.clone();
    let UPDATE_INTERVAL = CONFIG.performance.update_interval as u128;
    std::thread::spawn(move || {
        let res = core_affinity::set_for_current(core_ids[0]);
        if res {
            let time = std::time::Instant::now();
            let mut last_time = time.elapsed().as_millis();
            let mut dt: u128;
            let mut lag = 0;
            while game_state.running {
                let current_time = time.elapsed().as_millis();
                dt = current_time - last_time;
                lag += dt;
                last_time = current_time;

                let total_lag = lag;
                // Catch up with things that require a maximum step size to be stable
                while lag > UPDATE_INTERVAL {
                    let delta_time = lag.min(UPDATE_INTERVAL);
                    systems::physics(&mut game_state, delta_time);
                    lag -= UPDATE_INTERVAL;
                }

                if total_lag > UPDATE_INTERVAL {
                    // Catch up with events
                    while let Some(event) = event_receiver.try_iter().next() {
                        if let scene::Event::SDLEvent(sdl2::event::Event::Quit { timestamp }) =
                            event
                        {
                            running.store(false, std::sync::atomic::Ordering::SeqCst);
                        } else {
                            events::handle_event(&mut game_state, event, lag);
                        }
                    }
                    let cam = {
                        let camera = game_state.camera.expect("Must have camera");
                        let cc = game_state
                            .entities
                            .get_component::<CameraComponent>(camera)
                            .expect("Camera must still exist and have camera component!");
                        let ct = game_state
                            .entities
                            .get_component::<TransformComponent>(camera)
                            .expect("Camera must still exist and have transform component!");

                        RenderCameraState {
                            view: ct.point_of_view(),
                            proj: cc.project(width, height),
                        }
                    };
                    let _ = render_state_sender.send(RenderStateEvent {
                        camera: Some(cam),
                        entity_generations: game_state.entities.current_entity_generations.clone(),
                        entity_transforms: Box::new(
                            game_state
                                .entities
                                .get_component_vec_mut::<TransformComponent>()
                                .iter_mut()
                                .map(|opt_tc| opt_tc.as_mut().map(|tc| tc.get_matrix()))
                                .collect(),
                        ),
                    });
                    if CONFIG.performance.cap_update_fps {
                        let sleep_time = UPDATE_INTERVAL.checked_sub(dt).unwrap_or(0);
                        if sleep_time > 0 {
                            std::thread::sleep(Duration::from_millis(sleep_time as u64));
                        }
                    }
                }
            }
            running.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });
}

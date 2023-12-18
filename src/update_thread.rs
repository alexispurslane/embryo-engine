use std::{
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use crate::{
    entity::{
        camera_component::CameraComponent, light_component::LightComponent,
        transform_component::TransformComponent,
    },
    events,
    render_thread::{light_component_to_shader_light, RenderCameraState, RenderStateEvent},
    resource_manager::ResourceManager,
    systems, CONFIG,
};

use crate::entity::{Entity, EntitySystem};

pub type Direction = glam::Vec3;
pub type PitchYawRoll = glam::Vec3;
pub enum SceneCommand {
    MoveCameraInDirection(Direction),
    RotateCamera(PitchYawRoll),
    DisplaceEntity(Entity, glam::Vec3),
    Exit(),
}

pub enum GameStateEvent {
    SDLEvent(sdl2::event::Event),
    FrameEvent(
        Vec<(sdl2::keyboard::Scancode, bool)>,
        sdl2::mouse::RelativeMouseState,
    ),
}

struct GameStateDiff {
    camera_changed: bool,
    lights_changed: bool,
    command_queue_changed: bool,
    entities_changed: bool,
}

impl GameStateDiff {
    pub fn any_changed(&self) -> bool {
        self.camera_changed
            || self.lights_changed
            || self.command_queue_changed
            || self.entities_changed
    }
}

pub struct GameState {
    camera: Option<Entity>,
    lights: Vec<Entity>,
    light_count: usize,
    command_queue: Vec<SceneCommand>,
    entities: EntitySystem,
    running: bool,
    changed_diff: GameStateDiff,
}
// NOTE: Same logic as for the Send implementation for Model: I will never be
// sending this anywhere where OpenGL functions will be called, since the
// rendering happens only on the main thread, and the update thread only
// receives this object as immutable, and accessing scene properties can never
// call any OpenGL functions because accessing is completely passive. So the one
// thing that makes it thread-unsafe, the Vertex Buffer Objects having OpenGL
// code in their methods, is fine.
unsafe impl Send for GameState {}
unsafe impl Sync for GameState {}

impl GameState {
    pub fn new() -> Self {
        Self {
            camera: None,
            command_queue: vec![],
            entities: EntitySystem::new(),
            running: true,
            lights: Vec::with_capacity(CONFIG.performance.max_lights),
            light_count: 0,
            changed_diff: GameStateDiff {
                camera_changed: true,
                lights_changed: true,
                command_queue_changed: true,
                entities_changed: true,
            },
        }
    }

    pub fn entities(&self) -> &EntitySystem {
        &self.entities
    }

    pub fn entities_mut(&mut self) -> &mut EntitySystem {
        self.changed_diff.entities_changed = true;
        &mut self.entities
    }

    pub fn camera(&self) -> &Option<Entity> {
        &self.camera
    }

    pub fn lights(&self) -> &Vec<Entity> {
        &self.lights
    }

    /// Adds an entity to the list of entities we're treating as active light sources. If this would overflow the list of entities, it just rotates them.
    pub fn register_light(&mut self, e: Entity) {
        self.changed_diff.lights_changed = true;
        if self.lights.len() < CONFIG.performance.max_lights {
            self.lights.push(e);
        } else {
            self.lights[self.light_count] = e;
        }
        self.light_count = (self.light_count + 1) % CONFIG.performance.max_lights;
    }

    pub fn register_camera(&mut self, e: Entity) {
        self.changed_diff.camera_changed = true;
        self.camera = Some(e);
    }

    /// Queue world state changes
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    pub fn move_camera_by_vector(&mut self, d: Direction, dt: u128) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities_mut()
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.displace_by(d * CONFIG.controls.motion_speed * (dt as f32 / 1000.0));
    }

    pub fn rotate_camera(&mut self, pyr: PitchYawRoll, dt: u128) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities_mut()
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.rotate(pyr * CONFIG.controls.mouse_sensitivity * dt as f32 / 1000.0);
    }

    pub fn displace_entity(&mut self, entity: Entity, rel_vec: glam::Vec3) {
        self.entities_mut()
            .get_component_mut::<TransformComponent>(entity)
            .expect("Displaced entity must have transform component")
            .displace_by(rel_vec);
    }

    /// Apply queued world state changes to the world state
    pub fn update(&mut self, dt: u128) {
        // Update world state
        while let Some(command) = self.command_queue.pop() {
            match command {
                SceneCommand::MoveCameraInDirection(d) => {
                    self.move_camera_by_vector(d, dt);
                }
                SceneCommand::RotateCamera(pyr) => self.rotate_camera(pyr, dt),
                SceneCommand::DisplaceEntity(entity, rel_vec) => {
                    self.displace_entity(entity, rel_vec)
                }
                SceneCommand::Exit() => self.running = false,
            }
        }
    }
}

pub fn spawn_update_loop(
    mut game_state: GameState,
    resource_manager: &ResourceManager,
    render_state_sender: Sender<RenderStateEvent>,
    event_receiver: Receiver<GameStateEvent>,

    window: &sdl2::video::Window,

    running: Arc<AtomicBool>,
) {
    let (width, height) = window.size();
    let core_ids = core_affinity::get_core_ids().unwrap();
    let running = running.clone();
    let interval = CONFIG.performance.update_interval as u128;
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
                while lag > interval {
                    let delta_time = lag.min(interval);
                    systems::physics(&mut game_state, delta_time);
                    lag -= interval;
                }

                if total_lag > interval {
                    // Catch up with events
                    while let Some(event) = event_receiver.try_iter().next() {
                        if let GameStateEvent::SDLEvent(sdl2::event::Event::Quit { timestamp }) =
                            event
                        {
                            running.store(false, std::sync::atomic::Ordering::SeqCst);
                        } else {
                            events::handle_event(&mut game_state, event, lag);
                        }
                    }

                    if game_state.changed_diff.any_changed() {
                        let cam = {
                            if game_state.changed_diff.camera_changed {
                                let camera = game_state.camera.expect("Must have camera");
                                let cc = game_state
                                    .entities
                                    .get_component::<CameraComponent>(camera)
                                    .expect("Camera must still exist and have camera component!");
                                let ct = game_state
                                    .entities
                                    .get_component::<TransformComponent>(camera)
                                    .expect(
                                        "Camera must still exist and have transform component!",
                                    );

                                Some(RenderCameraState {
                                    view: ct.point_of_view(),
                                    proj: cc.project(width, height),
                                })
                            } else {
                                None
                            }
                        };
                        let matrices = {
                            if game_state.changed_diff.entities_changed {
                                // We DON'T use the entities_mut() command here
                                // because if we did, it would lead to a degenerate
                                // loop of stuff being marked changed every frame
                                // once the first change happens
                                Some(
                                    game_state
                                        .entities
                                        .get_component_vec_mut::<TransformComponent>()
                                        .iter_mut()
                                        .map(|opt_tc| {
                                            opt_tc.as_mut().map(|tc| tc.get_matrix().clone())
                                        })
                                        .collect(),
                                )
                            } else {
                                None
                            }
                        };
                        let lights = {
                            if game_state.changed_diff.lights_changed {
                                Some(
                                    game_state
                                        .lights
                                        .iter()
                                        .map(|e| {
                                            let lc = game_state
                                                .entities
                                                .get_component::<LightComponent>(*e)
                                                .unwrap();
                                            let tc = game_state
                                                .entities
                                                .get_component::<TransformComponent>(*e)
                                                .unwrap();
                                            light_component_to_shader_light(&lc, &tc)
                                        })
                                        .collect(),
                                )
                            } else {
                                None
                            }
                        };
                        let _ = render_state_sender.send(RenderStateEvent {
                            camera: cam,
                            entity_generations: Some(
                                game_state.entities.current_entity_generations.clone(),
                            ),
                            entity_transforms: matrices.map(|x| Box::new(x)),
                            lights: lights.map(|x| Box::new(x)),
                        });
                    }
                    if CONFIG.performance.cap_update_fps {
                        let sleep_time = interval.checked_sub(dt).unwrap_or(0);
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

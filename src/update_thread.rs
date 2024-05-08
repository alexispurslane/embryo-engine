/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crossbeam_channel::{unbounded, Receiver, Sender};
use gltf::scene::Transform;
use rayon::slice::ParallelSlice;
use std::{
    collections::{BinaryHeap, HashMap},
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc, RwLock},
    thread::panicking,
    time::Duration,
};

use crate::{
    dead_drop::DeadDrop,
    entity::{
        camera_component::CameraComponent, light_component::LightComponent,
        transform_component::TransformComponent, EntityID,
    },
    events,
    render_thread::{light_component_to_shader_light, RenderCameraState, RenderWorldState},
    resource_manager::ResourceManager,
    systems, utils, CONFIG,
};

use crate::entity::{Entity, EntitySystem};

pub struct EntityTransformationUpdate {
    eid: usize,
    depth: usize,
    matrix: glam::Mat4,
    parent_matrix: Option<glam::Mat4>,
}

impl Ord for EntityTransformationUpdate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.depth < other.depth {
            std::cmp::Ordering::Greater
        } else if self.depth == other.depth {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

impl PartialOrd for EntityTransformationUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for EntityTransformationUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth && self.matrix == other.matrix && self.eid == other.eid
    }
}

impl Eq for EntityTransformationUpdate {}

pub type Direction = glam::Vec3;
pub type PitchYawRoll = glam::Vec3;
pub enum SceneCommand {
    MoveCameraInDirection(Direction),
    RotateCamera(PitchYawRoll),
    DisplaceEntity(Entity, glam::Vec3),
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
    pub resource_manager: ResourceManager,
    camera: Option<Entity>,
    lights: Vec<Entity>,
    light_count: usize,
    command_queue: Vec<SceneCommand>,
    entities: EntitySystem,
    changed_diff: GameStateDiff,
    entity_transforms: HashMap<EntityID, glam::Mat4>,
}

impl GameState {
    pub fn new(resource_manager: ResourceManager) -> Self {
        Self {
            resource_manager,
            entity_transforms: HashMap::new(),
            camera: None,
            command_queue: vec![],
            entities: EntitySystem::new(),
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

    pub fn move_camera_by_vector(&mut self, d: Direction, dt: f32) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities_mut()
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.displace_by(d * CONFIG.controls.motion_speed * (dt as f32 / 1000.0));
    }

    pub fn rotate_camera(&mut self, pyr: PitchYawRoll, dt: f32) {
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
    pub fn update(&mut self, dt: f32) {
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
            }
        }
    }

    pub fn load_initial_entities(&mut self) {
        let new_entities = systems::load_entities(self);
        systems::load_entity_models(self, &new_entities);
    }

    pub fn update_loop(
        mut self,
        rws_sender: DeadDrop<RenderWorldState>,
        event_receiver: Receiver<GameStateEvent>,

        (width, height): (u32, u32),

        running: Arc<AtomicBool>,
    ) {
        let interval = CONFIG.performance.update_interval as f32;

        let time = std::time::Instant::now();
        let start_time = time.elapsed().as_millis();
        let mut last_time = time.elapsed().as_millis();
        let mut dt: f32;
        let mut lag = 0.0;
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            let current_time = time.elapsed().as_millis();
            dt = (current_time - last_time) as f32;
            lag += dt;
            last_time = current_time;

            let total_lag = lag;
            // Catch up with things that require a maximum step size to be stable
            while lag > interval {
                let delta_time = lag.min(interval);
                systems::physics(&mut self, delta_time, current_time - start_time);
                lag -= interval;
            }

            if self.changed_diff.entities_changed {
                let mut tcs = self.entities.get_component_vec_mut::<TransformComponent>();
                let mut new_trans = BinaryHeap::new();
                for eid in 0..tcs.len() {
                    let (a, b) = tcs.split_at_mut(eid);
                    let (item, c) = b.split_at_mut(1);
                    if let Some(tc) = &mut item[0] {
                        if let Some(parent) = tc.parent.and_then(|p_ref| {
                            a.get_mut(p_ref.id).or(c.get_mut(p_ref.id)).and_then(|ptc| {
                                ptc.as_mut().filter(|_| {
                                    self.entities
                                        .entity_generations
                                        .get(&p_ref.id)
                                        .map(|gen| *gen == p_ref.generation)
                                        .unwrap_or(false)
                                })
                            })
                        }) {
                            if tc.dirty_flag || parent.dirty_flag {
                                new_trans.push(EntityTransformationUpdate {
                                    depth: tc.depth,
                                    eid,
                                    matrix: tc.transform.to_matrix(),
                                    parent_matrix: Some(parent.transform.to_matrix()),
                                });
                            }
                        } else {
                            if tc.dirty_flag {
                                new_trans.push(EntityTransformationUpdate {
                                    depth: tc.depth,
                                    eid,
                                    matrix: tc.transform.to_matrix(),
                                    parent_matrix: None,
                                });
                            }
                        }
                    }
                }
                for update in new_trans.iter() {
                    self.entity_transforms.insert(
                        update.eid,
                        if let Some(pm) = update.parent_matrix {
                            update.matrix * pm
                        } else {
                            update.matrix
                        },
                    );
                }
            }

            if total_lag > interval {
                // Catch up with events
                while let Some(event) = event_receiver.try_iter().next() {
                    events::handle_event(&mut self, event, lag);
                }

                if self.changed_diff.any_changed() {
                    let camera = self.camera.expect("Must have camera");
                    let cc = self
                        .entities
                        .get_component::<CameraComponent>(camera)
                        .expect("Camera must still exist and have camera component!");
                    let ct = self
                        .entities
                        .get_component::<TransformComponent>(camera)
                        .expect("Camera must still exist and have transform component!");

                    rws_sender.send(RenderWorldState {
                        lights: self
                            .lights
                            .iter()
                            .map(|e| {
                                let lc = self.entities.get_component::<LightComponent>(*e).unwrap();
                                let tc = self
                                    .entities
                                    .get_component::<TransformComponent>(*e)
                                    .unwrap();
                                light_component_to_shader_light(&lc, &tc)
                            })
                            .collect(),
                        active_camera: Some(RenderCameraState {
                            view: ct.point_of_view(),
                            proj: cc.project(width, height),
                        }),
                        entity_generations: self.entities.entity_generations.clone(),
                        entity_transforms: self.entity_transforms.clone(),
                    });
                    self.changed_diff.camera_changed = false;
                    self.changed_diff.entities_changed = false;
                    self.changed_diff.lights_changed = false;
                }
                if CONFIG.performance.cap_update_fps {
                    let sleep_time = interval - dt;
                    if sleep_time > 0.0 {
                        std::thread::sleep(Duration::from_millis(sleep_time as u64));
                    }
                }
            }
        }
    }
}

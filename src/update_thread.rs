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
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicBool, Arc, RwLock},
    thread::panicking,
    time::Duration,
};

use crate::{
    dead_drop::DeadDrop,
    entity::{
        camera_component::CameraComponent, hierarchy_component::HierarchyComponent,
        light_component::LightComponent, mesh_component::ModelComponent,
        transform_component::TransformComponent, Component, EntityID,
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

pub struct Accessor<T> {
    dirty_flag: bool,
    inner: T,
}

impl<T> Accessor<T> {
    pub fn new(val: T) -> Self {
        Self {
            dirty_flag: true,
            inner: val,
        }
    }
}

impl<T> Deref for Accessor<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Accessor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty_flag = true;
        &mut self.inner
    }
}

pub struct GameState {
    resource_manager: ResourceManager,
    pub camera: Accessor<Option<Entity>>,
    pub lights: Accessor<Vec<Entity>>,
    pub command_queue: Accessor<Vec<SceneCommand>>,
    pub entities: EntitySystem,
    transform_update_queue: BinaryHeap<EntityTransformationUpdate>,
    entity_transforms: HashMap<EntityID, glam::Mat4>,
}

impl GameState {
    pub fn new(resource_manager: ResourceManager) -> Self {
        Self {
            resource_manager,
            entity_transforms: HashMap::new(),
            camera: Accessor::new(None),
            command_queue: Accessor::new(vec![]),
            entities: EntitySystem::new(),
            lights: Accessor::new(vec![]),
            transform_update_queue: BinaryHeap::new(),
        }
    }

    pub fn gen_entity(&mut self) -> Entity {
        self.entities.gen_entity()
    }

    pub fn add_component<T: Component + 'static>(&mut self, e: Entity, mut c: T) {
        c.add_hook(e, self);
        self.entities.add_component(e, c);
    }

    /// Adds an entity to the list of entities we're treating as active light
    /// sources.
    pub fn register_light(&mut self, e: Entity) {
        self.lights.push(e);
    }

    /// Sets the current camera to the provided entity (assumes it has a camera and transform component)
    pub fn register_camera(&mut self, e: Entity) {
        self.camera.replace(e);
    }

    // Sends a request to load whatever model the given entity has
    pub fn load_model_for(&mut self, e: Entity, c: &ModelComponent) {
        self.resource_manager
            .request_models(vec![(c.path.clone(), e)]);
    }

    /// Queue world state changes
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    pub fn move_camera_by_vector(&mut self, d: Direction, dt: f32) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.displace_by(d * CONFIG.controls.motion_speed * (dt as f32 / 1000.0));
    }

    pub fn rotate_camera(&mut self, pyr: PitchYawRoll, dt: f32) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.rotate(pyr * CONFIG.controls.mouse_sensitivity * dt as f32 / 1000.0);
    }

    pub fn displace_entity(&mut self, entity: Entity, rel_vec: glam::Vec3) {
        self.entities
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
        systems::load_entities(self);
    }

    pub fn any_changed(&self) -> bool {
        self.camera.dirty_flag
            || self.lights.dirty_flag
            || self.entities.dirty()
            || self.command_queue.dirty_flag
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

            let missed_frames = (lag / interval).round() as usize;
            let events = event_receiver.try_iter().collect::<Vec<_>>();
            // Catch up with things that require a maximum step size to be stable
            while lag > interval {
                let delta_time = lag.min(interval);
                systems::physics(&mut self, delta_time, current_time - start_time);

                lag -= interval;
            }
            for event in events.into_iter().rev().take(missed_frames).rev() {
                match event {
                    GameStateEvent::FrameEvent(scancodes, mouse_state) => {
                        events::handle_keyboard(&mut self, scancodes, dt);
                        events::handle_mouse(&mut self, &mouse_state, dt);
                    }
                    _ => events::handle_event(&mut self, event, dt),
                }
            }

            if self.entities.dirty() {
                let mut tcs = self
                    .entities
                    .get_component_vec_mut::<TransformComponent>()
                    .unwrap();
                let hcs = self.entities.get_component_vec::<HierarchyComponent>();
                for eid in 0..tcs.len() {
                    let (a, b) = tcs.split_at_mut(eid);
                    let (item, c) = b.split_at_mut(1);
                    if let Some(tc) = &mut item[0] {
                        // We can't use a normal get_component::<T> because we
                        // need to access the parent from the split transform
                        // arrays above, so we have to do a little
                        // monad-juggling
                        let hc = hcs.as_ref().and_then(|hcs| hcs[eid].as_ref());
                        if let Some(parent) = hc.and_then(|hc| {
                            // If there is a hierarchy component, get the parent on it
                            let p_ref = hc.parent;
                            // Look up the parent's ID in the left slice, and if it isn't there, in the right slice.
                            a.get_mut(p_ref.id).or(c.get_mut(p_ref.id)).and_then(|ptc| {
                                // If there's a row with that ID in one of those
                                // slices, only return Some if that row actually
                                // had Some in it *and* the generation matches
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
                                self.transform_update_queue
                                    .push(EntityTransformationUpdate {
                                        depth: hc.map_or(0, |x| x.depth),
                                        eid,
                                        matrix: tc.transform.to_matrix(),
                                        parent_matrix: Some(parent.transform.to_matrix()),
                                    });
                            }
                        } else {
                            if tc.dirty_flag {
                                self.transform_update_queue
                                    .push(EntityTransformationUpdate {
                                        depth: 0,
                                        eid,
                                        matrix: tc.transform.to_matrix(),
                                        parent_matrix: None,
                                    });
                            }
                        }
                    }
                    for update in self.transform_update_queue.drain() {
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
            }

            // Catch up with events

            if self.any_changed() {
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
                self.lights.dirty_flag = false;
                self.camera.dirty_flag = false;
                self.entities.clear_dirty();
            }

            // Cap update FPS (ignores option not to because it really doesn't even function right if you do that)
            let sleep_time = interval - dt;
            if sleep_time > 0.0 {
                std::thread::sleep(Duration::from_millis(sleep_time as u64));
            }
        }
    }
}

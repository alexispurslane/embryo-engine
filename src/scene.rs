use std::cell::RefMut;
use std::collections::HashMap;

use gl::Gl;
use rayon::prelude::{ParallelBridge, ParallelIterator};

use crate::entity::camera_component::CameraComponent;
use crate::entity::mesh_component::Model;
use crate::entity::transform_component::TransformComponent;
use crate::entity::{Entity, EntityID, EntitySystem};
use crate::render_gl::resources::ResourceManager;
use crate::render_gl::shaders::Program;
use crate::CONFIG;

pub type Direction = glam::Vec3;
pub type PitchYawRoll = glam::Vec3;
pub enum SceneCommand {
    MoveCameraInDirection(Direction),
    RotateCamera(PitchYawRoll),
    DisplaceEntity(Entity, glam::Vec3),
    Exit(),
}

pub enum Event {
    SDLEvent(sdl2::event::Event),
    FrameEvent(
        Vec<(sdl2::keyboard::Scancode, bool)>,
        sdl2::mouse::RelativeMouseState,
    ),
}

pub struct RenderStateEvent {
    pub camera: Option<RenderCameraState>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,
}

pub struct RenderCameraState {
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

pub struct RenderState {
    pub camera: Option<RenderCameraState>,
    pub models: HashMap<String, Model>,
    pub shader_programs: Vec<Program>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,
}

impl RenderState {
    pub fn get_entity_transform<'a>(
        entity_generations: &'a HashMap<EntityID, usize>,
        entity_transforms: &'a Vec<Option<glam::Mat4>>,
        e: Entity,
    ) -> Option<&'a glam::Mat4> {
        if entity_generations
            .get(&e.id)
            .filter(|gen| **gen == e.generation)
            .is_some()
        {
            entity_transforms.get(e.id).and_then(|x| x.as_ref())
        } else {
            entity_transforms.get(e.id).and_then(|x| x.as_ref())
        }
    }
}

pub struct GameState {
    pub camera: Option<Entity>,
    pub command_queue: Vec<SceneCommand>,
    pub entities: EntitySystem,
    pub running: bool,
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
    /// Queue world state changes
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    pub fn move_camera_by_vector(&mut self, d: Direction, dt: u128) {
        let camera_entity = self.camera.expect("No camera found");
        let mut camera_transform = self
            .entities
            .get_component_mut::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");

        camera_transform.displace_by(d * CONFIG.controls.motion_speed * (dt as f32 / 1000.0));
    }

    pub fn rotate_camera(&mut self, pyr: PitchYawRoll, dt: u128) {
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

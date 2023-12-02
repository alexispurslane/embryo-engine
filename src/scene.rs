use std::cell::RefMut;
use std::collections::HashMap;

use rayon::prelude::{ParallelBridge, ParallelIterator};

use crate::entity::camera_component::CameraComponent;
use crate::entity::mesh_component::Model;
use crate::entity::transform_component::TransformComponent;
use crate::entity::{Entity, EntitySystem};
use crate::render_gl::resources::ResourceManager;
use crate::render_gl::shaders::Program;
use crate::UPDATE_INTERVAL;

const MOUSE_SENSITIVITY: f32 = 10.0;
const MOTION_SPEED: f32 = 10.0;

pub type Direction = glam::Vec3;
pub type PitchYawRoll = glam::Vec3;
pub enum SceneCommand {
    MoveCameraInDirection(Direction),
    RotateCamera(PitchYawRoll),
    DisplaceEntity(usize, glam::Vec3),
    Exit(),
}

pub struct Scene {
    pub camera: Option<Entity>,
    pub command_queue: Vec<SceneCommand>,
    pub running: bool,
    pub entities: EntitySystem,
    pub shader_programs: Vec<Program>,
    pub resource_manager: ResourceManager,
}
// NOTE: Same logic as for the Send implementation for Model: I will never be
// sending this anywhere where OpenGL functions will be called, since the
// rendering happens only on the main thread, and the update thread only
// receives this object as immutable, and accessing scene properties can never
// call any OpenGL functions because accessing is completely passive. So the one
// thing that makes it thread-unsafe, the Vertex Buffer Objects having OpenGL
// code in their methods, is fine.
unsafe impl Send for Scene {}
unsafe impl Sync for Scene {}

impl Scene {
    /// Queue world state changes
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    /// Apply queued world state changes to the world state
    pub fn update(&mut self) {
        // Update world state
        while let Some(command) = self.command_queue.pop() {
            match command {
                SceneCommand::MoveCameraInDirection(d) => {
                    let camera_entity = self.camera.expect("No camera found");
                    let mut camera_transform = self
                        .entities
                        .get_component_mut::<TransformComponent>(camera_entity)
                        .expect("Camera needs to have TransformComponent");

                    camera_transform
                        .displace_by(d * MOTION_SPEED * (UPDATE_INTERVAL as f32 / 1000.0));
                }
                SceneCommand::RotateCamera(pyr) => {
                    let camera_entity = self.camera.expect("No camera found");
                    let mut camera_transform = self
                        .entities
                        .get_component_mut::<TransformComponent>(camera_entity)
                        .expect("Camera needs to have TransformComponent");

                    camera_transform
                        .rotate(pyr * MOUSE_SENSITIVITY * UPDATE_INTERVAL as f32 / 1000.0);
                }
                SceneCommand::DisplaceEntity(eid, rel_vec) => {
                    self.entities.get_component_vec_mut::<TransformComponent>()[eid]
                        .as_mut()
                        .expect("Displaced entity must have transform component")
                        .displace_by(rel_vec);
                }
                SceneCommand::Exit() => self.running = false,
            }
        }
    }
}

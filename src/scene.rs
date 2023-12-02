use std::collections::HashMap;

use rayon::prelude::{ParallelBridge, ParallelIterator};

use crate::entity::camera_component::CameraComponent;
use crate::entity::mesh_component::Model;
use crate::entity::transform_component::TransformComponent;
use crate::entity::{EntityID, EntitySystem};
use crate::render_gl::shaders::Program;

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
    pub camera: Option<EntityID>,
    pub command_queue: Vec<SceneCommand>,
    pub running: bool,
    pub entities: EntitySystem,
    pub shader_programs: Vec<Program>,
    pub models: HashMap<String, Model>,
}

impl Scene {
    /// Queue world state changes
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    /// Apply queued world state changes to the world state
    pub fn update(&mut self, dt: f32) {
        // Update world state
        while let Some(command) = self.command_queue.pop() {
            match command {
                SceneCommand::MoveCameraInDirection(d) => {
                    let camera_eid = self.camera.expect("No camera found");
                    let ct = &mut self.entities.get_component_vec_mut::<TransformComponent>()
                        [camera_eid];
                    let camera_transform = ct
                        .as_mut()
                        .expect("Camera needs to have TransformComponent");

                    camera_transform.displace_by(d * MOTION_SPEED * (dt / 1000.0));
                }
                SceneCommand::RotateCamera(pyr) => {
                    let camera_eid = self.camera.expect("No camera found");
                    let ct = &mut self.entities.get_component_vec_mut::<TransformComponent>()
                        [camera_eid];
                    let camera_transform = ct
                        .as_mut()
                        .expect("Camera needs to have TransformComponent");

                    camera_transform.rotate(pyr * MOUSE_SENSITIVITY * dt / 1000.0);
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

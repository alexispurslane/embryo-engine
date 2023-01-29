use crate::camera::{Camera, PitchYawRoll};

pub type Direction = glam::Vec3;
pub enum SceneCommand {
    MoveCameraInDirection(Direction),
    RotateCamera(PitchYawRoll),
    Exit(),
}

pub struct Scene {
    pub camera: Box<dyn Camera>,
    pub command_queue: Vec<SceneCommand>,
    pub running: bool,
}

impl Scene {
    pub fn queue_commands(&mut self, cs: Vec<SceneCommand>) {
        self.command_queue.extend(cs);
    }

    pub fn update(&mut self, dt: u128) {
        while let Some(command) = self.command_queue.pop() {
            match command {
                SceneCommand::MoveCameraInDirection(d) => self.camera.displace(d, dt),
                SceneCommand::RotateCamera(pyr) => self.camera.rotate(pyr),
                SceneCommand::Exit() => self.running = false,
            }
        }
    }
}

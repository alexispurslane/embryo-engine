use crate::entity::{Component, ComponentID};
use render_gl_derive::ComponentId;

use crate::utils::Degrees;

#[derive(ComponentId)]
pub struct CameraComponent {
    pub fov: Degrees,
}

impl CameraComponent {
    pub fn project(&self, width: u32, height: u32) -> glam::Mat4 {
        glam::Mat4::perspective_rh_gl(
            self.fov.to_radians(),
            width as f32 / height as f32,
            0.1,
            100.0,
        )
    }
}

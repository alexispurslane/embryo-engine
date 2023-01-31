use crate::render_gl::data::InstanceTransformVertex;

use super::*;
use render_gl_derive::ComponentId;

#[derive(Copy, Clone, Debug)]
pub struct PitchYawRoll {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}
impl PitchYawRoll {
    pub fn new(pitch: f32, yaw: f32, roll: f32) -> Self {
        PitchYawRoll { pitch, yaw, roll }
    }
}

#[derive(ComponentId)]
pub struct TransformComponent {
    pub rotations: Vec<PitchYawRoll>,
    pub positions: Vec<glam::Vec3>,
    pub scales: Vec<f32>,
    pub up: glam::Vec3,
    pub position_change_flag: gl::types::GLenum,
}

impl TransformComponent {
    /// Apply the given vector along the forward direction indicated by the yaw
    /// (left-right turning) without using pitch (up down turning) to modify movement
    pub fn displace_along_absolute_plane(&mut self, idx: usize, rel_vec: glam::Vec3) {
        let rot = self.rotations[idx];
        let front = glam::Vec3::X
            * glam::vec3(rot.yaw.to_radians().cos(), 1.0, rot.yaw.to_radians().sin()).normalize();
        self.positions[idx] += (rel_vec.x * front)
            + (rel_vec.y * front.cross(self.up).normalize())
            + (rel_vec.z * self.up);
    }

    /// Fly in the direction pointed using the vector
    pub fn displace_along_own_plane(&mut self, idx: usize, rel_vec: glam::Vec3) {
        let rot = self.rotations[idx];
        let front = glam::vec3(
            rot.yaw.to_radians().cos() * rot.pitch.to_radians().cos(),
            rot.pitch.to_radians().sin(),
            rot.yaw.to_radians().sin() * rot.pitch.to_radians().cos(),
        )
        .normalize();
        self.positions[idx] += (rel_vec.x * front)
            + (rel_vec.y * front.cross(self.up).normalize())
            + (rel_vec.z * self.up);
    }

    pub fn matrix_transforms(&self) -> Vec<InstanceTransformVertex> {
        self.positions
            .iter()
            .zip(self.rotations.iter())
            .map(|(pos, rot)| {
                let model = glam::Mat4::from_rotation_translation(
                    glam::Quat::from_euler(glam::EulerRot::YXZ, rot.yaw, rot.pitch, rot.roll),
                    *pos,
                );
                InstanceTransformVertex::new(model.to_cols_array())
            })
            .collect()
    }

    pub fn points_of_view(&self) -> Vec<glam::Mat4> {
        self.positions
            .iter()
            .zip(self.rotations.iter())
            .map(|(pos, rot)| {
                let front = glam::vec3(
                    rot.yaw.to_radians().cos() * rot.pitch.to_radians().cos(),
                    rot.pitch.to_radians().sin(),
                    rot.yaw.to_radians().sin() * rot.pitch.to_radians().cos(),
                )
                .normalize();
                glam::Mat4::look_at_rh(*pos, *pos + front, self.up)
            })
            .collect()
    }
}

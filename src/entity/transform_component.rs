use std::collections::HashSet;

use crate::render_gl::{data, objects};

use super::*;
use crate::render_gl::objects::Buffer;
use glam::Vec4Swizzles;
use render_gl_derive::ComponentId;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    pub trans: glam::Vec3,
    pub rot: glam::Vec3,
}

impl Transform {
    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(
            glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                self.rot.x.to_radians(),
                self.rot.y.to_radians(),
                self.rot.z.to_radians(),
            ),
            self.trans,
        )
    }
}

#[derive(ComponentId)]
pub struct TransformComponent {
    pub transform: Transform,
    pub dirty_flag: bool,
}

impl TransformComponent {
    pub fn new_from_rot_trans(rot: glam::Vec3, trans: glam::Vec3, pcf: gl::types::GLenum) -> Self {
        let transform = Transform { trans, rot };
        Self {
            transform,
            dirty_flag: true,
        }
    }

    /// Displaces object by the given relative vector *rotated by the direction
    /// the object is pointing*
    pub fn displace_by(&mut self, rel_vec: glam::Vec3) {
        let rot = self.transform.rot;
        let direction = glam::vec3(
            (rot.y.to_radians()).cos() * (rot.x.to_radians()).cos(),
            (rot.x.to_radians()).sin(),
            (rot.y.to_radians()).sin() * (rot.x.to_radians()).cos(),
        )
        .normalize();
        self.transform.trans += rel_vec.z * direction
            + rel_vec.x * direction.cross(glam::Vec3::Y).normalize()
            + rel_vec.y * glam::Vec3::Y;
        self.dirty_flag = true;
    }

    pub fn rotate(&mut self, pyr: glam::Vec3) {
        // Pitch
        self.transform.rot.x = (self.transform.rot.x + pyr.x).clamp(-89.0, 89.0);

        // Yaw
        self.transform.rot.y = (self.transform.rot.y + pyr.y) % 360.0;

        // Roll
        self.transform.rot.z = (self.transform.rot.z + pyr.z) % 360.0;

        self.dirty_flag = true;
    }

    pub fn point_of_view(&self) -> glam::Mat4 {
        let Transform { trans: pos, rot } = self.transform;
        let direction = glam::vec3(
            (rot.y.to_radians()).cos() * (rot.x.to_radians()).cos(),
            (rot.x.to_radians()).sin(),
            (rot.y.to_radians()).sin() * (rot.x.to_radians()).cos(),
        )
        .normalize();
        // We are *always* right side up, so we don't get the up vector
        glam::Mat4::look_at_rh(pos, pos + direction, glam::Vec3::Y)
    }
}

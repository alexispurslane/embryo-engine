use std::collections::HashSet;

use crate::render_gl::{data, objects};

use super::*;
use crate::render_gl::objects::Buffer;
use glam::Vec4Swizzles;
use render_gl_derive::ComponentId;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    pub trans: glam::Vec3,
    pub rot: glam::Quat,
}

impl Transform {
    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(self.rot, self.trans)
    }
}

#[derive(ComponentId)]
pub struct TransformComponent {
    pub transform: Transform,
    /// Whether the rotating object behaves as if it is attached to the "ground"
    /// (original XZ plane) so horizontal rotations are always relative to that
    /// original XZ plane while vertical rotations are relative, or if all
    /// rotations are relative. Useful for cameras.
    matrix: glam::Mat4,
    pub grounded: bool,
    pub dirty_flag: bool,
}

impl TransformComponent {
    pub fn new_from_rot_trans(rot: glam::Vec3, trans: glam::Vec3, grounded: bool) -> Self {
        let rot = glam::Quat::from_euler(glam::EulerRot::XYZ, rot.x, rot.y, rot.z).normalize();
        let transform = Transform { trans, rot };
        Self {
            transform,
            grounded,
            matrix: transform.to_matrix(),
            dirty_flag: false,
        }
    }

    /// Displaces object by the given relative vector *rotated by the direction
    /// the object is pointing*
    pub fn displace_by(&mut self, rel_vec: glam::Vec3) {
        self.transform.trans += self.transform.rot * rel_vec;
        self.dirty_flag = true;
    }

    pub fn rotate(&mut self, pyr: glam::Vec3) {
        if self.grounded {
            let rot_xz = glam::Quat::from_euler(glam::EulerRot::XYZ, pyr.x, 0.0, pyr.z).normalize();
            let rot_h = glam::Quat::from_euler(glam::EulerRot::XYZ, 0.0, pyr.y, 0.0).normalize();
            // Horizontal rotations should be on an absolute axis, so we apply the
            // original rotation first, and then premultiply the new rotation, so
            // the new rotation is not *transformed* by the original rotation and
            // thus in its coordinate system.
            self.transform.rot = rot_h * self.transform.rot;
            // All others are relative to the object's own axes, so we apply them
            // first and then let the original transform put them in its coordinate
            // system
            self.transform.rot = self.transform.rot * rot_xz;
        } else {
            let rot = glam::Quat::from_euler(glam::EulerRot::XYZ, pyr.x, pyr.y, pyr.z).normalize();
            self.transform.rot = self.transform.rot * rot;
        }

        self.dirty_flag = true;
    }

    pub fn point_of_view(&self) -> glam::Mat4 {
        let Transform { trans: pos, rot } = self.transform;
        let direction = rot * glam::Vec3::Z;
        glam::Mat4::look_at_rh(pos, pos + direction, rot * glam::Vec3::Y)
    }

    pub fn get_matrix(&mut self) -> glam::Mat4 {
        if self.dirty_flag {
            self.matrix = self.transform.to_matrix();
        }
        self.matrix
    }
}

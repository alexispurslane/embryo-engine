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
    fn to_matrix(&self) -> glam::Mat4 {
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
    instance_transforms: Vec<Transform>,
    pub instance_matrices: Vec<glam::Mat4>,
    pub ibo: objects::VertexBufferObject<data::InstanceTransformVertex>,
    pub instances: u32,
    pub position_change_flag: gl::types::GLenum,
    dirty_matrices: HashSet<usize>,
}

impl TransformComponent {
    pub fn new_from_rot_trans(rot: glam::Vec3, trans: glam::Vec3, pcf: gl::types::GLenum) -> Self {
        let transform = Transform { trans, rot };
        let matrix = transform.to_matrix();
        let ibo = objects::VertexBufferObject::new_with_vec(
            gl::ARRAY_BUFFER,
            &[data::InstanceTransformVertex::new(matrix.to_cols_array())],
        );
        Self {
            instance_transforms: vec![transform],
            instance_matrices: vec![matrix],
            instances: 1,
            dirty_matrices: HashSet::with_capacity(1),
            position_change_flag: pcf,
            ibo,
        }
    }

    pub fn new_from_rot_trans_instances(
        instances: Vec<(glam::Vec3, glam::Vec3)>,
        pcf: gl::types::GLenum,
    ) -> Self {
        let instance_transforms: Vec<_> = instances
            .into_iter()
            .map(|(rot, trans)| Transform { trans, rot })
            .collect();

        let instance_matrices = instance_transforms
            .iter()
            .map(|transform| transform.to_matrix())
            .collect::<Vec<_>>();

        let ibo = objects::VertexBufferObject::new_with_vec(
            gl::ARRAY_BUFFER,
            &instance_matrices
                .iter()
                .map(|mat| data::InstanceTransformVertex::new(mat.to_cols_array()))
                .collect::<Vec<_>>()[..],
        );
        let instances = instance_transforms.len() as u32;
        Self {
            instance_transforms,
            instance_matrices,
            instances,
            dirty_matrices: HashSet::with_capacity(instances as usize),
            position_change_flag: pcf,
            ibo,
        }
    }

    /// Displaces object by the given relative vector *rotated by the direction
    /// the object is pointing*
    pub fn displace_by(&mut self, idx: usize, rel_vec: glam::Vec3) {
        let rot = self.instance_transforms[idx].rot;
        let direction = glam::vec3(
            (rot.y.to_radians()).cos() * (rot.x.to_radians()).cos(),
            (rot.x.to_radians()).sin(),
            (rot.y.to_radians()).sin() * (rot.x.to_radians()).cos(),
        )
        .normalize();
        self.instance_transforms[idx].trans += rel_vec.z * direction
            + rel_vec.x * direction.cross(glam::Vec3::Y).normalize()
            + rel_vec.y * glam::Vec3::Y;
        self.dirty_matrices.insert(idx);
    }

    pub fn rotate(&mut self, idx: usize, pyr: glam::Vec3) {
        // Pitch
        self.instance_transforms[idx].rot.x =
            (self.instance_transforms[idx].rot.x + pyr.x).clamp(-89.0, 89.0);
        // Yaw
        self.instance_transforms[idx].rot.y = (self.instance_transforms[idx].rot.y + pyr.y) % 360.0;
        self.dirty_matrices.insert(idx);
    }

    /// If the list of dirty matrices is not empty, update those matrices from
    /// the transforms and then update the corrisponding parts of the IBO.
    pub fn flush_matrices(&mut self) {
        for i in self.dirty_matrices.drain() {
            let mat = self.instance_transforms[i].to_matrix();
            self.instance_matrices[i] = mat;
            self.ibo.bind();
            self.ibo.update_data(
                &[data::InstanceTransformVertex::new(mat.to_cols_array())],
                i * std::mem::size_of::<data::InstanceTransformVertex>(),
            );
            self.ibo.unbind();
        }
    }

    pub fn point_of_view(&self, idx: usize) -> glam::Mat4 {
        let Transform { trans: pos, rot } = self.instance_transforms[idx];
        let direction = glam::vec3(
            (-rot.y.to_radians()).cos() * (-rot.x.to_radians()).cos(),
            (-rot.x.to_radians()).sin(),
            (-rot.y.to_radians()).sin() * (-rot.x.to_radians()).cos(),
        )
        .normalize();
        // We are *always* right side up, so we don't get the up vector
        glam::Mat4::look_at_rh(-pos, -pos + direction, glam::Vec3::Y)
    }
}

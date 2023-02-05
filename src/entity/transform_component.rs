use crate::render_gl::{data, objects};

use super::*;
use glam::Vec4Swizzles;
use render_gl_derive::ComponentId;

#[derive(ComponentId)]
pub struct TransformComponent {
    pub instance_transforms: Vec<glam::Mat4>,
    pub ibo: objects::VertexBufferObject<data::InstanceTransformVertex>,
    pub instances: u32,
    pub dirty_flag: bool,
    pub position_change_flag: gl::types::GLenum,
    look_angle: f32,
}

impl TransformComponent {
    pub fn new_from_rot_trans(rot: glam::Vec3, trans: glam::Vec3, pcf: gl::types::GLenum) -> Self {
        let quat = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            rot.x.to_radians(),
            rot.y.to_radians(),
            rot.z.to_radians(),
        );
        let matrix = glam::Mat4::from_rotation_translation(quat, trans);
        let ibo = objects::VertexBufferObject::new_with_vec(
            gl::ARRAY_BUFFER,
            &[data::InstanceTransformVertex::new(matrix.to_cols_array())],
        );
        Self {
            instance_transforms: vec![matrix],
            instances: 1,
            dirty_flag: false,
            position_change_flag: pcf,
            ibo,
            look_angle: 0.0,
        }
    }

    pub fn new_from_rot_trans_instances(
        instances: Vec<(glam::Vec3, glam::Vec3)>,
        pcf: gl::types::GLenum,
    ) -> Self {
        let instance_transforms: Vec<_> = instances
            .iter()
            .map(|(rot, trans)| {
                let quat = glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    rot.x.to_radians(),
                    rot.y.to_radians(),
                    rot.z.to_radians(),
                );
                glam::Mat4::from_rotation_translation(quat, *trans)
            })
            .collect();

        let ibo = objects::VertexBufferObject::new_with_vec(
            gl::ARRAY_BUFFER,
            &instance_transforms
                .iter()
                .map(|mat| data::InstanceTransformVertex::new(mat.to_cols_array()))
                .collect::<Vec<_>>()[..],
        );
        let instances = instance_transforms.len() as u32;
        Self {
            instance_transforms,
            instances,
            dirty_flag: false,
            position_change_flag: pcf,
            ibo,
            look_angle: 0.0,
        }
    }

    pub fn displace_by(&mut self, idx: usize, rel_vec: glam::Vec3) {
        self.instance_transforms[idx] *= glam::Mat4::from_translation(rel_vec);
        self.dirty_flag = true;
    }

    pub fn rotate(&mut self, idx: usize, pyr: glam::Vec3) {
        let new_matrix = self.instance_transforms[idx]
            * glam::Mat4::from_rotation_translation(
                glam::Quat::from_euler(glam::EulerRot::XYZ, pyr.x, pyr.y, pyr.z),
                glam::Vec3::ZERO,
            );
        if pyr.x + self.look_angle > -89.0 && pyr.x + self.look_angle < 89.0 {
            self.instance_transforms[idx] = new_matrix;
            self.look_angle += pyr.x;
        }
        self.dirty_flag = true;
    }

    /// Upload the new transform matrices stored on this component into the
    /// instance buffer object on video memory for the renderer to use. Only use
    /// if the dirty flag is true
    pub fn update_ibo(&mut self) {
        self.ibo.upload_data(
            &self
                .instance_transforms
                .iter()
                .map(|mat| data::InstanceTransformVertex::new(mat.to_cols_array()))
                .collect::<Vec<_>>()[..],
            self.position_change_flag,
        );
        self.dirty_flag = false;
    }

    pub fn point_of_view(&self, idx: usize) -> glam::Mat4 {
        let mat = self.instance_transforms[idx];
        // By default, all objects face in the Z direction (right), so all rotations are relative to that
        let front = mat.transform_vector3(glam::Vec3::Z).normalize();
        let pos = mat.col(3).xyz();
        // We are *always* right side up, so we don't get the up vector
        glam::Mat4::look_at_rh(pos, pos + front, glam::Vec3::Y)
    }
}

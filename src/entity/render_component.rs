use std::ffi::CString;

use crate::render_gl::{
    objects::{self, Buffer},
    shaders,
    textures::{self, IntoTextureUnit},
};

use super::*;

type TextureID = &'static str;

pub struct MeshComponent {
    pub vao: objects::VertexArrayObject,
    pub vbo: Box<dyn objects::Buffer>,
    pub ebo: Option<objects::ElementBufferObject>,
    pub textures: Vec<(TextureID, Box<dyn textures::AbstractTexture>)>,
    pub program: shaders::Program,
}

impl Component for MeshComponent {
    fn get_id() -> ComponentID {
        "MeshComponent"
    }
}

impl MeshComponent {
    pub fn new(
        shaders: &[shaders::Shader],
        vbo: Box<dyn objects::Buffer>,
        ebo: Option<objects::ElementBufferObject>,
        textures: Vec<(TextureID, Box<dyn textures::AbstractTexture>)>,
    ) -> Self {
        Self {
            vao: objects::VertexArrayObject::new(),
            vbo,
            ebo,
            textures,
            program: shaders::Program::from_shaders(shaders).unwrap(),
        }
    }

    pub fn render(&self, instances: u32, point_of_view: glam::Mat4, projection: glam::Mat4) {
        self.program.set_used();

        self.program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &point_of_view.to_cols_array(),
        );
        self.program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &projection.to_cols_array(),
        );

        // Render boxes
        self.vao.bind();
        for (i, (uniform_name, tex)) in self.textures.iter().enumerate() {
            self.program
                .set_uniform_1i(&CString::new(*uniform_name).unwrap(), i as i32);
            tex.bind_to_texture_unit(i.to_texture_unit());
        }

        if let Some(ebo) = &self.ebo {
            self.vao.draw_elements_instanced(
                gl::TRIANGLES,
                ebo.count() as gl::types::GLint,
                gl::UNSIGNED_INT,
                0,
                instances as gl::types::GLint,
            );
        } else {
            self.vao.draw_arrays_instanced(
                gl::TRIANGLES,
                0,
                self.vbo.count() as gl::types::GLint,
                instances as gl::types::GLint,
            )
        }
        self.vao.unbind();
    }
}

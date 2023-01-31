use std::ffi::CString;

use crate::render_gl::{data, objects, shaders, textures};
use crate::scene::Scene;

use super::transform_component::*;
use super::*;

type TextureID = &'static str;

trait VBOTag {}
impl<V: data::Vertex> VBOTag for objects::VertexBufferObject<V> {}

trait TexTag {}
impl<T: textures::ColorDepth> TexTag for textures::Texture<T> {}

pub struct RenderComponent<'a> {
    pub vao: objects::VertexArrayObject,
    pub vbo: Box<dyn VBOTag>,
    pub ebo: objects::ElementBufferObject,
    pub ibo: objects::VertexBufferObject<data::InstanceTransformVertex>,
    pub textures: &'a [(TextureID, Box<dyn TexTag>)],
    pub program: shaders::Program,
}

impl Component for RenderComponent<'_> {
    fn get_id() -> ComponentID {
        "RenderComponent"
    }
}

impl<'a> RenderComponent<'a> {
    pub fn new(
        shaders: &[shaders::Shader],
        vbo: Box<dyn VBOTag>,
        ebo: objects::ElementBufferObject,
        textures: &'a [(TextureID, Box<dyn TexTag>)],
    ) -> Self {
        Self {
            vao: objects::VertexArrayObject::new(),
            vbo,
            ebo,
            ibo: objects::VertexBufferObject::new(gl::ARRAY_BUFFER),
            textures,
            program: shaders::Program::from_shaders(shaders).unwrap(),
        }
    }
}

trait IntoTextureUnit {
    fn to_texture_unit(&self) -> gl::types::GLenum;
}

impl IntoTextureUnit for usize {
    fn to_texture_unit(&self) -> gl::types::GLenum {
        match &self {
            0 => gl::TEXTURE0,
            1 => gl::TEXTURE1,
            2 => gl::TEXTURE2,
            3 => gl::TEXTURE3,
            4 => gl::TEXTURE4,
            5 => gl::TEXTURE5,
            6 => gl::TEXTURE6,
            7 => gl::TEXTURE7,
            8 => gl::TEXTURE8,
            9 => gl::TEXTURE9,
            10 => gl::TEXTURE10,
            11 => gl::TEXTURE11,
            12 => gl::TEXTURE12,
            13 => gl::TEXTURE13,
            14 => gl::TEXTURE14,
            15 => gl::TEXTURE15,
            16 => gl::TEXTURE16,
            17 => gl::TEXTURE17,
            18 => gl::TEXTURE18,
            19 => gl::TEXTURE19,
            20 => gl::TEXTURE20,
            21 => gl::TEXTURE21,
            22 => gl::TEXTURE22,
            23 => gl::TEXTURE23,
            24 => gl::TEXTURE24,
            25 => gl::TEXTURE25,
            26 => gl::TEXTURE26,
            27 => gl::TEXTURE27,
            28 => gl::TEXTURE28,
            29 => gl::TEXTURE29,
            30 => gl::TEXTURE30,
            _ => panic!("Too many textures!"),
        }
    }
}

pub fn setup_render_components_system(entities: &mut EntitySystem) {
    let mut has_renderable = entities.get_component_vec_mut::<RenderComponent>();
    let mut has_transform = entities.get_component_vec_mut::<TransformComponent>();
    for (eid, rc, tc) in entities.get_with_components_mut(&mut has_renderable, &mut has_transform) {
        // Set up the vertex array object we'll be using to render
        rc.vao.bind();

        // Add in the vertex info
        rc.vbo.bind();
        rc.vbo.setup_vertex_attrib_pointers();

        // Add in the index info
        rc.ebo.bind();

        // Add in the instance info
        rc.ibo.bind();
        rc.ibo
            .upload_data(&tc.matrix_transforms(), tc.position_change_flag);
        rc.ibo.setup_vertex_attrib_pointers();
        rc.vao.unbind();
    }
}

pub fn render_system(scene: &Scene) {
    let has_renderable = scene.entities.get_component_vec::<RenderComponent>();
    let has_transform = scene.entities.get_component_vec::<TransformComponent>();

    for (eid, rc, tc) in scene
        .entities
        .get_with_components(&has_renderable, &has_transform)
    {
        // Update the matrix transforms of all the instances of this entity from its own data
        let tfms = tc.matrix_transforms();
        rc.ibo.upload_data(&tfms, tc.position_change_flag);
        // Update box uniforms
        rc.program.set_used();

        rc.program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &scene.camera.view().to_cols_array(),
        );
        rc.program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &scene.camera.project(1024, 768).to_cols_array(),
        );

        // Render boxes
        rc.vao.bind();
        for (i, (uniform_name, tex)) in rc.textures.iter().enumerate() {
            rc.program
                .set_uniform_1i(&CString::new(*uniform_name).unwrap(), i as i32);
            tex.bind_to_texture_unit(i.to_texture_unit());
        }

        rc.vao.draw_elements_instanced(
            gl::TRIANGLES,
            rc.ebo.count as gl::types::GLint,
            gl::UNSIGNED_INT,
            0,
            tfms.len() as gl::types::GLint,
        );
        rc.vao.unbind();
    }
}

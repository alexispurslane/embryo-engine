use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;

use russimp::node::Node;

use crate::entity::{Component, ComponentID};
use crate::render_gl::data::{self, Cvec4, InstanceTransformVertex, VertexNormTex};
use crate::render_gl::objects::VertexBufferObject;
use crate::render_gl::textures::{AbstractTexture, Texture, RGB8};
use crate::render_gl::{
    objects::{self, Buffer},
    shaders::Program,
    textures::{self, IntoTextureUnit, TextureParameters},
};
use crate::utils;

use super::EntityID;

type TextureID = usize;

#[derive(Debug)]
enum FactorOrTexture {
    Factor(f32),
    VecFactor(Cvec4),
    Texture(TextureID),
}

pub struct Material {
    name: String,
    base_color: FactorOrTexture,
    normal: Option<TextureID>,
    metalness: FactorOrTexture,
    roughness: FactorOrTexture,
    ambient_occlusion: Option<TextureID>,
}

impl Material {
    pub fn activate(&self, model: &Model, shader_program: &Program) {
        use FactorOrTexture::*;

        match self.base_color {
            Texture(tex) => {
                let texture = &model.textures[tex];
                texture.activate((0 as usize).to_texture_unit());
                texture.bind();
                shader_program
                    .set_uniform_1i(&CString::new("material.baseColorTexture").unwrap(), 0);
                shader_program.set_uniform_1b(&CString::new("material.hasTexture").unwrap(), true);
            }
            VecFactor(vec) => {
                shader_program.set_uniform_4f(
                    &CString::new("material.baseColorFactor").unwrap(),
                    vec.d0,
                    vec.d1,
                    vec.d2,
                    vec.d3,
                );
                shader_program.set_uniform_1b(&CString::new("material.hasTexture").unwrap(), false);
            }
            _ => {
                println!("Should never get base color value: {:?}", self.base_color);
                unreachable!()
            }
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Box<dyn AbstractTexture>>,
    pub materials: Vec<Material>,
    pub entities: Vec<EntityID>,
    pub ibo: VertexBufferObject<InstanceTransformVertex>,
    pub entities_dirty_flag: bool,
}

impl Model {
    pub fn from_ai_scene(ai_scene: russimp::scene::Scene) -> Result<Self, String> {
        let ibo = VertexBufferObject::new(gl::ARRAY_BUFFER);

        let meshes = ai_scene
            .meshes
            .iter()
            .map(|mesh| Self::process_mesh(&ai_scene, &ibo, mesh))
            .collect::<Vec<Mesh>>();

        let mut texture_ids = HashMap::new();
        let mut textures: Vec<Box<dyn AbstractTexture>> = vec![];
        let mut materials = vec![];

        for material in ai_scene.materials {
            let mut material_textures = HashMap::new();
            for (texture_type, texture) in &material.textures {
                let texture = texture.borrow();
                if !texture_ids.contains_key(&texture.filename) {
                    textures.push(Box::new(Self::process_texture(&texture)));
                    texture_ids.insert(texture.filename.clone(), textures.len() - 1);
                }
                material_textures.insert(texture_type, texture_ids[&texture.filename]);
            }

            let base_color = material_textures
                .get(&russimp::material::TextureType::BaseColor)
                .map(|x| FactorOrTexture::Texture(*x))
                .or(
                    utils::material_get_property(&material, "$clr.base").and_then(|t| match t {
                        russimp::material::PropertyTypeInfo::FloatArray(array) => {
                            if array.len() == 4 {
                                Some(FactorOrTexture::VecFactor(Cvec4::new(
                                    array[0], array[1], array[2], array[3],
                                )))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }),
                )
                .unwrap_or(FactorOrTexture::VecFactor(Cvec4::new(0.75, 0.0, 1.0, 1.0)));

            let normal = material_textures
                .get(&russimp::material::TextureType::Normals)
                .map(|x| *x);

            let metalness = material_textures
                .get(&russimp::material::TextureType::Metalness)
                .map(|x| FactorOrTexture::Texture(*x))
                .or(
                    utils::material_get_property(&material, "$mat.metallicFactor").and_then(|t| {
                        match t {
                            russimp::material::PropertyTypeInfo::FloatArray(array) => {
                                if array.len() == 1 {
                                    Some(FactorOrTexture::Factor(array[0]))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }),
                )
                .unwrap_or(FactorOrTexture::Factor(0.5));

            let roughness = material_textures
                .get(&russimp::material::TextureType::Roughness)
                .map(|x| FactorOrTexture::Texture(*x))
                .or(
                    utils::material_get_property(&material, "$mat.roughnessFactor").and_then(|t| {
                        match t {
                            russimp::material::PropertyTypeInfo::FloatArray(array) => {
                                if array.len() == 1 {
                                    Some(FactorOrTexture::Factor(array[0]))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }),
                )
                .unwrap_or(FactorOrTexture::Factor(0.5));

            let ambient_occlusion = material_textures
                .get(&russimp::material::TextureType::AmbientOcclusion)
                .map(|x| *x);

            materials.push(Material {
                name: "unknown".to_string(),
                base_color,
                normal,
                metalness,
                roughness,
                ambient_occlusion,
            })
        }

        Ok(Self {
            meshes,
            textures,
            materials,
            ibo: ibo,
            entities: vec![],
            entities_dirty_flag: true,
        })
    }

    fn process_texture(texture: &russimp::material::Texture) -> Texture<RGB8> {
        let (width, height, bytes) =
            utils::load_image_u8(&format!("assets/textures/{}.jpg", texture.filename));
        Texture::new_with_bytes(TextureParameters::default(), &bytes, width, height)
    }

    fn process_mesh(
        ai_scene: &russimp::scene::Scene,
        ibo: &VertexBufferObject<InstanceTransformVertex>,
        mesh: &russimp::mesh::Mesh,
    ) -> Mesh {
        let mut vertices = Vec::with_capacity(mesh.vertices.len());

        // Set up vertices
        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            let n = mesh.normals[i];
            let t = mesh.texture_coords[0].as_ref().map(|x| x[i]).unwrap();
            vertices.push(data::VertexNormTex {
                pos: (v.x, v.y, v.z).into(),
                norm: (n.x, n.y, n.z).into(),
                tex: (t.x, t.y).into(),
            });
        }

        let indices: Vec<u32> = mesh.faces.iter().flat_map(|face| face.0.clone()).collect();

        println!(
            "    Mesh Vertices: {}, Indices: {}",
            vertices.len(),
            indices.len()
        );

        Mesh::new(&vertices, &indices, &ibo, mesh.material_index as usize)
    }
}

pub struct Mesh {
    pub vao: objects::VertexArrayObject,
    pub vbo: Box<dyn objects::Buffer>,
    pub ebo: objects::ElementBufferObject,
    pub material_index: usize,
}

impl Mesh {
    pub fn new(
        vertices: &Vec<VertexNormTex>,
        indices: &Vec<u32>,
        ibo: &VertexBufferObject<InstanceTransformVertex>,
        material_index: usize,
    ) -> Self {
        let vao = objects::VertexArrayObject::new();
        let vbo = Box::new(objects::VertexBufferObject::new_with_vec(
            gl::ARRAY_BUFFER,
            vertices,
        ));
        let ebo = objects::ElementBufferObject::new_with_vec(&indices);

        vao.bind();

        vbo.bind();
        vbo.setup_vertex_attrib_pointers();

        ebo.bind();

        ibo.bind();
        ibo.setup_vertex_attrib_pointers();

        vao.unbind();

        Mesh {
            vao,
            vbo,
            ebo,
            material_index,
        }
    }
}

#[derive(ComponentId)]
pub struct ModelComponent {
    pub path: String,
}

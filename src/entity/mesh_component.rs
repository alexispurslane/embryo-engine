use std::cell::Ref;
use std::ffi::CString;

use russimp::node::Node;
use russimp::scene::PostProcess;

use crate::entity::{Component, ComponentID};
use crate::render_gl::data;
use crate::render_gl::textures::AbstractTexture;
use crate::render_gl::{
    objects::{self, Buffer},
    shaders::Program,
    textures::{self, IntoTextureUnit, TextureParameters},
};
use crate::utils;

type TextureID = String;

#[derive(ComponentId)]
pub struct ModelComponent {
    pub meshes: Vec<Mesh>,
    pub material_textures: Vec<Vec<Box<dyn AbstractTexture>>>,
}

impl ModelComponent {
    pub fn from_file(path: String) -> Result<Self, String> {
        println!("Loading model from file '{}'", path);
        let ai_scene = russimp::scene::Scene::from_file(
            &path,
            vec![
                PostProcess::Triangulate,
                PostProcess::ValidateDataStructure,
                PostProcess::ImproveCacheLocality,
                PostProcess::GenerateUVCoords,
                PostProcess::OptimizeMeshes,
                PostProcess::FlipUVs,
            ],
        )
        .map_err(|x| format!("{:?}", x))?;

        let root_node = ai_scene
            .root
            .as_ref()
            .ok_or("No root node in loaded scene")?;

        println!("Loading meshes from nodes:");
        let meshes = Self::process_node(&ai_scene, root_node.borrow());
        println!("    Meshes: {}", meshes.len());

        let textures_by_material = ai_scene
            .materials
            .iter()
            .enumerate()
            .map(|(i, material)| {
                println!(
                    "Loading material {}, {:?}",
                    i,
                    material
                        .properties
                        .iter()
                        .find_map(|prop| if prop.key == "?mat.name" {
                            Some(&prop.data)
                        } else {
                            None
                        })
                );
                material
                    .textures
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (_tex_ty, tex))| {
                        let tex = tex.clone();
                        let tex = tex.borrow();
                        let (width, height, bytes) =
                            utils::load_image_u8(&format!("assets/textures/{}.jpg", tex.filename));
                        println!(
                            "    Texture {}: {}",
                            i,
                            format!("assets/textures/{}.jpg", tex.filename)
                        );
                        let result = textures::Texture::new_with_bytes(
                            TextureParameters::default(),
                            &bytes,
                            width,
                            height,
                        );
                        Some(Box::new(result) as Box<dyn AbstractTexture>)
                    })
                    .collect()
            })
            .collect();

        Ok(Self {
            meshes,
            material_textures: textures_by_material,
        })
    }

    fn process_node(ai_scene: &russimp::scene::Scene, node: Ref<Node>) -> Vec<Mesh> {
        let mut node_meshes = node
            .meshes
            .iter()
            .filter_map(|id| Self::process_mesh(ai_scene, &ai_scene.meshes[*id as usize]))
            .collect::<Vec<_>>();
        for child in node.children.iter() {
            let child = child.clone();
            node_meshes.extend(Self::process_node(ai_scene, child.borrow()));
        }
        node_meshes
    }

    fn process_mesh(ai_scene: &russimp::scene::Scene, mesh: &russimp::mesh::Mesh) -> Option<Mesh> {
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
        let vbo = objects::VertexBufferObject::new_with_vec(gl::ARRAY_BUFFER, &vertices);
        let ebo = objects::ElementBufferObject::new_with_vec(&indices);
        Some(Mesh::new(
            Box::new(vbo),
            Some(ebo),
            (0..ai_scene.materials[mesh.material_index as usize]
                .textures
                .len())
                .map(|i| format!("texture{}", i))
                .collect(),
            mesh.material_index as usize,
        ))
    }

    pub fn setup_mesh_components(
        &self,
        ibo: &objects::VertexBufferObject<data::InstanceTransformVertex>,
    ) {
        for mesh in self.meshes.iter() {
            // Set up the vertex array object we'll be using to render
            mesh.vao.bind();

            // Add in the vertex info
            mesh.vbo.bind();
            mesh.vbo.setup_vertex_attrib_pointers();

            if let Some(ebo) = &mesh.ebo {
                // Add in the index info
                ebo.bind();
            }

            // Add in the instance info
            ibo.bind();
            ibo.setup_vertex_attrib_pointers();
            mesh.vao.unbind();
        }
    }

    pub fn render(&self, instances: u32, program: &Program) {
        for mesh in self.meshes.iter() {
            mesh.render(instances, program, &self.material_textures);
        }
    }
}

pub struct Mesh {
    pub vao: objects::VertexArrayObject,
    pub vbo: Box<dyn objects::Buffer>,
    pub ebo: Option<objects::ElementBufferObject>,
    pub textures: Vec<TextureID>,
    pub material_index: usize,
}

impl Mesh {
    pub fn new(
        vbo: Box<dyn objects::Buffer>,
        ebo: Option<objects::ElementBufferObject>,
        textures: Vec<TextureID>,
        material_index: usize,
    ) -> Self {
        println!("Mesh textures: {:?}", textures);
        Self {
            vao: objects::VertexArrayObject::new(),
            vbo,
            ebo,
            material_index,
            textures,
        }
    }

    pub fn render(
        &self,
        instances: u32,
        program: &Program,
        textures: &Vec<Vec<Box<dyn AbstractTexture>>>,
    ) {
        self.vao.bind();
        for (texture_unit_for_material, texture_name) in self.textures.iter().enumerate() {
            let texture = &textures[self.material_index][texture_unit_for_material];
            texture.activate(texture_unit_for_material.to_texture_unit());
            program.set_uniform_1i(
                &CString::new(texture_name.to_owned()).unwrap(),
                texture_unit_for_material as i32,
            );
            texture.bind();
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

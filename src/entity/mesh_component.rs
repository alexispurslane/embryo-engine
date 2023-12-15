use crate::rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;
use std::thread::{self, Thread};

use gl::Gl;
use gltf::Gltf;
use image::GenericImageView;
use rayon::prelude::ParallelBridge;
use ritelinked::LinkedHashSet;

use crate::entity::{Component, ComponentID};
use crate::render_gl::data::{
    self, Cvec2, Cvec3, Cvec4, InstanceTransformVertex, VertexNormTex, VertexNormTexTan,
};
use crate::render_gl::objects::VertexBufferObject;
use crate::render_gl::textures::{AbstractTexture, Texture, RGB8};
use crate::render_gl::{
    objects::{self, Buffer},
    shaders::Program,
    textures::{self, IntoTextureUnit, TextureParameters},
};
use crate::utils::zip;

use super::Entity;

type TextureID = usize;

#[derive(Debug)]
enum FactorOrTexture {
    Factor(f32),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Texture(TextureID),
}

pub struct Material {
    name: String,
    diffuse: FactorOrTexture,
}

impl Material {
    pub fn activate(&self, model: &Model, shader_program: &Program) {
        use FactorOrTexture::*;

        match self.diffuse {
            Texture(tex) => {
                let texture = &model.textures.as_ref().expect("Cannot activate a material in the shader if that material and associated model have not had their OpenGL things set up.")[tex];
                texture.activate((0 as usize).to_texture_unit());
                texture.bind();
                shader_program.set_uniform_1i(&CString::new("material.diffuseTexture").unwrap(), 0);
                shader_program
                    .set_uniform_1b(&CString::new("material.diffuseIsTexture").unwrap(), true);
            }
            Vec4(vec) => {
                shader_program.set_uniform_4f(
                    &CString::new("material.diffuseFactor").unwrap(),
                    vec[0],
                    vec[1],
                    vec[2],
                    vec[3],
                );
                shader_program
                    .set_uniform_1b(&CString::new("material.diffuseIsTexture").unwrap(), false);
            }
            _ => {
                println!("Should never get base color value: {:?}", self.diffuse);
                unreachable!()
            }
        }
    }
}

pub struct MeshNode {
    pub name: String,
    pub primitives: Vec<Mesh>,
    pub children: Vec<Box<MeshNode>>,
}

impl MeshNode {
    pub fn setup_mesh_gl(&mut self, gl: &Gl, ibo: &VertexBufferObject<InstanceTransformVertex>) {
        for primitive in self.primitives.iter_mut() {
            primitive.gl_mesh = Some(MeshGl::setup_mesh_gl(gl, &primitive, &ibo));
        }
        for child in self.children.iter_mut() {
            child.setup_mesh_gl(gl, ibo);
        }
    }
}

pub struct Model {
    pub meshes: Vec<MeshNode>,
    pub textures_raw: Vec<(Vec<u8>, u32, u32)>,
    pub materials: Vec<Material>,

    pub entities: LinkedHashSet<Entity>,

    pub entities_dirty_flag: bool,
    pub shader_program: usize,

    pub textures: Option<Vec<Box<dyn AbstractTexture>>>,
    pub ibo: Option<VertexBufferObject<InstanceTransformVertex>>,
}
/// NOTE: Textures and Buffers aren't safe to Send usually, because they require
/// OpenGL calls to construct/manipulate, but I won't actually be constructing
/// or manipulating those properties on another thread, they'll be set to None
/// when I'm on another thread, and then populated when we get home. We
/// dynamically check this at runtime to at least have some guarantees.
unsafe impl Send for Model {}

impl Model {
    pub fn from_gltf(
        (document, buffers, images): (
            gltf::Document,
            Vec<gltf::buffer::Data>,
            Vec<gltf::image::Data>,
        ),
    ) -> Option<Self> {
        let time = std::time::Instant::now();

        let mat_start = time.elapsed().as_millis();
        let materials = document
            .materials()
            .map(|m| Self::process_material(m))
            .collect::<Vec<Material>>();
        let mat_end = time.elapsed().as_millis();

        let mesh_start = time.elapsed().as_millis();
        let meshes = document
            .nodes()
            .filter_map(|n| Self::process_node(n, &buffers))
            .collect::<Vec<MeshNode>>();
        let mesh_end = time.elapsed().as_millis();

        let textures_start = time.elapsed().as_millis();
        let textures_raw = document
            .textures()
            .map(|t| Self::process_texture(t, &images))
            .collect::<Vec<(Vec<u8>, u32, u32)>>();
        let textures_end = time.elapsed().as_millis();

        println!("Model processing times: ");
        println!("    material processing done in {}ms", mat_end - mat_start);
        println!("    mesh processing done in {}ms", mesh_end - mesh_start);
        println!(
            "    texture processing done in {}ms",
            textures_end - textures_start
        );

        Some(Model {
            meshes,
            textures_raw,
            materials,

            entities: LinkedHashSet::new(),

            entities_dirty_flag: true,
            shader_program: 0,

            textures: None,
            ibo: None,
        })
    }

    fn process_node(n: gltf::Node, buffers: &Vec<gltf::buffer::Data>) -> Option<MeshNode> {
        let time = std::time::Instant::now();

        let prim_start = time.elapsed().as_millis();
        let primitives = n
            .mesh()?
            .primitives()
            .map(|prim| {
                let reader = prim.reader(|b| buffers.get(b.index()).map(|x| &*x.0));

                let vertices = {
                    let positions = reader.read_positions();
                    let normals = reader.read_normals();
                    let tangents = reader.read_tangents();
                    let texcoords = reader.read_tex_coords(0).unwrap().into_f32();
                    zip!(
                        positions.expect(&format!(
                            "Vertices in node {} are missing positions!",
                            n.name().unwrap()
                        )),
                        normals.expect(&format!(
                            "Vertices in node {} are missing normals!",
                            n.name().unwrap()
                        )),
                        tangents.expect(&format!(
                            "Vertices in node {} are missing tangents!",
                            n.name().unwrap()
                        )),
                        texcoords
                    )
                    .map(|(pos, (norm, (tan, tex)))| VertexNormTexTan {
                        pos: Cvec3::new(pos[0], pos[1], pos[2]),
                        norm: Cvec3::new(norm[0], norm[1], norm[2]),
                        tex: Cvec2::new(tex[0], tex[1]),
                        tan: Cvec4::new(tan[0], tan[1], tan[2], tan[3]),
                    })
                    .collect::<Vec<VertexNormTexTan>>()
                };
                let indices = reader.read_indices().unwrap().into_u32().collect();
                let material_index = prim.material().index().unwrap();
                let bounding_box = prim.bounding_box();
                Mesh {
                    vertices,
                    indices,
                    material_index,
                    bounding_box,
                    gl_mesh: None,
                }
            })
            .collect();
        let prim_end = time.elapsed().as_millis();

        println!(
            "    node {}'s primitives took {}ms to process",
            n.name().unwrap(),
            prim_end - prim_start
        );

        let child_start = time.elapsed().as_millis();
        let children = n
            .children()
            .filter_map(|n| Self::process_node(n, &buffers).map(|x| Box::new(x)))
            .collect();
        let child_end = time.elapsed().as_millis();

        println!(
            "    node {}'s children took {}ms to process",
            n.name().unwrap(),
            child_end - child_start
        );
        println!("");

        Some(MeshNode {
            name: n.name().unwrap_or("UnknownMesh").to_string(),
            primitives,
            children,
        })
    }

    fn process_material(m: gltf::Material) -> Material {
        let pbr = m.pbr_metallic_roughness();
        Material {
            name: m.name().unwrap_or("UnknownMaterial").to_string(),
            diffuse: pbr
                .base_color_texture()
                .map_or(FactorOrTexture::Vec4(pbr.base_color_factor()), |info| {
                    FactorOrTexture::Texture(info.texture().index())
                }),
        }
    }

    pub fn process_texture(
        t: gltf::Texture,
        images: &Vec<gltf::image::Data>,
    ) -> (Vec<u8>, u32, u32) {
        let image = images
            .get(t.source().index())
            .expect(&format!("No image for texture: {:?}", t.name()));
        (image.pixels.clone(), image.width, image.height)
    }

    pub fn setup_model_gl(&mut self, gl: &Gl) {
        if !thread::current().name().is_some_and(|x| x.contains("main")) {
            panic!("Called OpenGL setup function on model while not on main thread: this is undefined behavior!");
        }
        self.ibo = Some(VertexBufferObject::new(gl, gl::ARRAY_BUFFER));
        self.textures = Some(
            self.textures_raw
                .iter()
                .map(|(bytes, width, height)| {
                    Box::new(Texture::new_with_bytes(
                        gl,
                        TextureParameters::default(),
                        bytes,
                        *width,
                        *height,
                    )) as Box<dyn AbstractTexture>
                })
                .collect::<Vec<Box<dyn AbstractTexture>>>(),
        );
        for mesh_node in self.meshes.iter_mut() {
            mesh_node.setup_mesh_gl(gl, self.ibo.as_ref().unwrap());
        }
    }
}

pub struct MeshGl {
    pub vao: objects::VertexArrayObject,
    pub vbo: Box<dyn objects::Buffer>,
    pub ebo: objects::ElementBufferObject,
}

impl MeshGl {
    pub fn setup_mesh_gl(
        gl: &Gl,
        mesh: &Mesh,
        ibo: &VertexBufferObject<InstanceTransformVertex>,
    ) -> Self {
        let vao = objects::VertexArrayObject::new(gl);
        let vbo = Box::new(objects::VertexBufferObject::new_with_vec(
            gl,
            gl::ARRAY_BUFFER,
            &mesh.vertices,
        ));
        println!("indices: {}", mesh.indices.len());
        let ebo = objects::ElementBufferObject::new_with_vec(gl, &mesh.indices);

        vao.bind();

        vbo.bind();
        vbo.setup_vertex_attrib_pointers();

        ebo.bind();

        ibo.bind();
        ibo.setup_vertex_attrib_pointers();

        vao.unbind();

        MeshGl { vao, vbo, ebo }
    }
}

pub struct Mesh {
    vertices: Vec<VertexNormTexTan>,
    indices: Vec<u32>,
    pub gl_mesh: Option<MeshGl>,
    pub material_index: usize,
    pub bounding_box: gltf::mesh::BoundingBox,
}
// NOTE: same reasoning as for Model above.
unsafe impl Send for Mesh {}

impl Mesh {
    pub fn new(
        vertices: Vec<VertexNormTexTan>,
        indices: Vec<u32>,
        material_index: usize,
        bounding_box: gltf::mesh::BoundingBox,
    ) -> Self {
        Self {
            vertices,
            indices,
            material_index,
            bounding_box,
            gl_mesh: None,
        }
    }
}

#[derive(ComponentId)]
pub struct ModelComponent {
    pub path: String,
    pub shader_program: usize,
}

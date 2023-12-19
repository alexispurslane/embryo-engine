use crate::rayon::iter::ParallelIterator;
use crate::CONFIG;
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::rc::Rc;
use std::thread::{self, Thread};

use bytes::BytesMut;
use gl::Gl;
use gltf::image::Format;
use gltf::Gltf;
use rayon::prelude::ParallelBridge;

use crate::entity::{Component, ComponentID};
use crate::render_gl::data::{
    self, Cvec2, Cvec3, Cvec4, InstanceTransformVertex, VertexNormTex, VertexNormTexTan,
};
use crate::render_gl::objects::{BufferObject, VertexArray};
use crate::render_gl::textures::{AbstractTexture, Texture, RGB8};
use crate::render_gl::{
    objects::{self, Buffer},
    shaders::Program,
    textures::{self, TextureParameters},
};
use crate::utils::zip;

use super::Entity;

type TextureID = usize;

#[derive(Debug, Clone, Copy)]
enum FactorOrTexture {
    Factor(f32),
    Vec3(Cvec3),
    Vec4(Cvec4),
    Texture(TextureID),
}

pub struct Material {
    name: String,

    diffuse: FactorOrTexture,
    specular: FactorOrTexture,
    normal_map: Option<TextureID>,

    shininess: f32,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            name: "TestMaterial".to_string(),
            diffuse: FactorOrTexture::Vec4([0.4, 0.4, 0.4, 1.0].into()),
            specular: FactorOrTexture::Vec3([1.0, 1.0, 1.0].into()),
            normal_map: None,
            shininess: 2.0,
        }
    }
}

impl Material {
    pub fn activate(&self, model: &Model, shader_program: &Program) {
        Self::send_factor_or_texture(model, shader_program, &self.diffuse, "diffuse", 0);
        Self::send_factor_or_texture(model, shader_program, &self.specular, "specular", 1);
        shader_program.set_uniform_1f(&CString::new("shininess").unwrap(), self.shininess);
        if let Some(nm) = self.normal_map {
            let texture = &model.textures.as_ref().expect("Cannot activate a material in the shader if that material and associated model have not had their OpenGL things set up.")[nm];
            texture.bind(2);
            shader_program.set_uniform_1ui(&CString::new("material.normalMap").unwrap(), 2);
        }
    }

    fn send_factor_or_texture(
        model: &Model,
        shader_program: &Program,
        val: &FactorOrTexture,
        uniform_name: &str,
        texture_bind: usize,
    ) {
        use FactorOrTexture::*;
        match val {
            Texture(tex) => {
                let texture = &model.textures.as_ref().expect("Cannot activate a material in the shader if that material and associated model have not had their OpenGL things set up.")[*tex];
                texture.bind(texture_bind);
                shader_program.set_uniform_1ui(
                    &CString::new(format!("{}Texture", uniform_name)).unwrap(),
                    texture_bind as u32,
                );
                shader_program.set_uniform_1b(
                    &CString::new(format!("{}IsTexture", uniform_name)).unwrap(),
                    true,
                );
            }
            Vec3(vec) => {
                shader_program.set_uniform_3f(
                    &CString::new(format!("{}Factor", uniform_name)).unwrap(),
                    *vec,
                );
                shader_program.set_uniform_1b(
                    &CString::new(format!("{}IsTexture", uniform_name)).unwrap(),
                    false,
                );
            }
            Vec4(vec) => {
                shader_program.set_uniform_4f(
                    &CString::new(format!("{}Factor", uniform_name)).unwrap(),
                    *vec,
                );
                shader_program.set_uniform_1b(
                    &CString::new(format!("{}IsTexture", uniform_name)).unwrap(),
                    false,
                );
            }
            _ => {
                unreachable!()
            }
        }
    }
}

pub struct MeshNode {
    pub name: String,
    pub primitives: Vec<Mesh>,
}

impl MeshNode {
    pub fn setup_mesh_gl(&mut self, gl: &Gl, ibo: &BufferObject<InstanceTransformVertex>) {
        for primitive in self.primitives.iter_mut() {
            primitive.gl_mesh = Some(MeshGl::setup_mesh_gl(gl, &primitive, &ibo));
        }
    }
}

pub struct Model {
    pub meshes: Vec<MeshNode>,
    pub textures_raw: Vec<(Vec<u8>, u32, u32)>,
    pub materials: Vec<Material>,

    pub entities: HashSet<Entity>,

    pub entities_dirty_flag: bool,
    pub shader_program: usize,

    pub textures: Option<Vec<Box<dyn AbstractTexture>>>,
    pub ibo: Option<BufferObject<InstanceTransformVertex>>,
}
/// NOTE: Textures and Buffers aren't safe to Send usually, because they require
/// OpenGL calls to construct/manipulate, but I won't actually be constructing
/// or manipulating those properties on another thread, they'll be set to None
/// when I'm on another thread, and then populated when we get home. We
/// dynamically check this at runtime to at least have some guarantees.
unsafe impl Send for Model {}

impl Default for Model {
    fn default() -> Self {
        Self {
            meshes: vec![],
            textures_raw: vec![],
            materials: vec![],
            entities: HashSet::new(),
            entities_dirty_flag: true,
            shader_program: 0,
            textures: None,
            ibo: None,
        }
    }
}

impl Model {
    pub fn from_gltf(
        (document, buffers, mut images): (
            gltf::Document,
            Vec<gltf::buffer::Data>,
            Vec<gltf::image::Data>,
        ),
    ) -> Option<Self> {
        let time = std::time::Instant::now();

        let mat_start = time.elapsed().as_millis();
        let materials = document
            .materials()
            .map(|m| Self::process_material(m, &mut images))
            .collect::<Vec<Material>>();
        let mat_end = time.elapsed().as_millis();

        let mesh_start = time.elapsed().as_millis();
        let meshes = document
            .meshes()
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

            entities: HashSet::new(),

            entities_dirty_flag: true,
            shader_program: 0,

            textures: None,
            ibo: None,
        })
    }

    fn process_node(n: gltf::Mesh, buffers: &Vec<gltf::buffer::Data>) -> Option<MeshNode> {
        let time = std::time::Instant::now();

        let prim_start = time.elapsed().as_millis();
        let primitives = n
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

        Some(MeshNode {
            name: n.name().unwrap_or("UnknownMesh").to_string(),
            primitives,
        })
    }

    // Curve control points: (108, 0), (185, 49), (208, 112), figured out using
    // the two given control points and the grid in the graph from the Valve
    // wiki page. Using some internet sleuthing, I figured out that this is
    // supposed to be a natural interpolated cubic spline curve.
    fn magic_specular_curve(x: f32) -> f32 {
        if x <= 108.0 {
            0.0
        } else if x > 108.0 && x <= 169.0 {
            2.7 * (10_f32).powi(-5) * x.powi(3) - 4.66 * (10_f32).powi(-3) * x.powi(2)
                + 5.04 * (10_f32).powi(-1) * x
                - 3.43 * 10.0
        } else {
            9.1 * (10_f32).powi(-3) * x.powi(2) - 1.83 * x + 9.71 * 10.0
        }
    }

    // Curve control points: (127, 0), (191, 16), (255, 64)
    fn magic_diffuse_curve(x: f32) -> f32 {
        if x <= 127.0 {
            0.0
        } else if x > 127.0 && x <= 191.0 {
            2.38 * 10_f32.powi(-5) * x.powi(3) - 8.06 * 10_f32.powi(-3) * x.powi(2)
                + 9.8 * 10_f32.powi(-1) * x
                - 4.33 * 10.0
        } else {
            -2.92 * 10_f32.powi(-5) * x.powi(3) + 2.23 * 10_f32.powi(-2) * x.powi(2) - 4.82 * x
                + 3.26 * 10_f32.powi(2)
        }
    }

    /// Takes roughness and uses [the equations on the Valve Source Engine
    /// Wiki](https://developer.valvesoftware.com/wiki/Adapting_PBR_Textures_to_Source#Non-Metal)
    /// to convert it to (specular, diffuse)
    fn convert_roughness(roughness: f32, metalness: f32) -> (f32, f32) {
        // in glTF, values are from 0.0 to 1.0, but the equations on the wiki
        // *appear* (and I stress, *appear*) to be using values up to 255.0?? So
        // let's scale it.
        let r = 1.0 - roughness;
        let mut specular = Self::magic_specular_curve(r * 255.0) / 255.0;
        let fucked = (56.0 + metalness * 71.0) / 255.0;
        if metalness > 0.5 {
            specular = 1.0 - ((1.0 - specular) * (1.0 - fucked));
        }
        (
            specular,
            if specular == 0.0 {
                1.0 / (1.0 - 0.25 * (Self::magic_diffuse_curve(roughness * 255.0) / 64.0))
            } else if metalness > 0.8 {
                1.0 - metalness
            } else {
                1.0
            },
        )
    }

    fn byte_size_from_format(f: Format) -> usize {
        use Format::*;
        match f {
            R8G8B8 => 3,
            R8G8B8A8 => 4,
            R16G16B16 => 6,
            R16G16B16A16 => 8,
            R32G32B32FLOAT => 12,
            R32G32B32A32FLOAT => 16,
            _ => panic!(
                "Metallic roughness texture should have green and blue components in glTF 2.0!"
            ),
        }
    }

    unsafe fn convert_value(components: &[u8], format: Format) -> (f32, f32, f32, f32) {
        use Format::*;
        match format {
            R8G8B8A8 | R8G8B8 => (
                components[0] as f32,
                components[1] as f32,
                components[2] as f32,
                1.0,
            ),
            R16G16B16 => (
                std::mem::transmute::<[u8; 2], u16>(components[0..2].try_into().unwrap()).to_le()
                    as f32,
                std::mem::transmute::<[u8; 2], u16>(components[2..4].try_into().unwrap()).to_le()
                    as f32,
                std::mem::transmute::<[u8; 2], u16>(components[4..6].try_into().unwrap()).to_le()
                    as f32,
                1.0,
            ),
            R16G16B16A16 => (
                std::mem::transmute::<[u8; 2], u16>(components[0..2].try_into().unwrap()).to_le()
                    as f32,
                std::mem::transmute::<[u8; 2], u16>(components[2..4].try_into().unwrap()).to_le()
                    as f32,
                std::mem::transmute::<[u8; 2], u16>(components[4..6].try_into().unwrap()).to_le()
                    as f32,
                std::mem::transmute::<[u8; 2], u16>(components[6..8].try_into().unwrap()).to_le()
                    as f32,
            ),
            R32G32B32FLOAT => (
                std::mem::transmute::<[u8; 4], f32>(components[0..4].try_into().unwrap()),
                std::mem::transmute::<[u8; 4], f32>(components[4..8].try_into().unwrap()),
                std::mem::transmute::<[u8; 4], f32>(components[8..12].try_into().unwrap()),
                1.0,
            ),
            R32G32B32A32FLOAT => (
                std::mem::transmute::<[u8; 4], f32>(components[0..4].try_into().unwrap()),
                std::mem::transmute::<[u8; 4], f32>(components[4..8].try_into().unwrap()),
                std::mem::transmute::<[u8; 4], f32>(components[8..12].try_into().unwrap()),
                std::mem::transmute::<[u8; 4], f32>(components[12..16].try_into().unwrap()),
            ),
            _ => panic!(
                "Metallic roughness texture should have green and blue components in glTF 2.0!"
            ),
        }
    }

    fn process_material(m: gltf::Material, images: &mut Vec<gltf::image::Data>) -> Material {
        let pbr = m.pbr_metallic_roughness();

        // Diffuse can stay unchanged
        let mut diffuse_map = pbr
            .base_color_texture()
            .and_then(|x| images.get(x.texture().source().index()).cloned());
        let mut diffuse_factor = pbr.base_color_factor();

        // We need to turn metallicroughness into specular factor, an adjustment
        // to the diffuse factor, and shininess.

        // If we have a metallicroughness *factor*, then we need to adjust the
        // diffuse image and build a specular map. :horror:
        if let Some(img) = pbr.metallic_roughness_texture() {
            let mut shininess = 0.5;
            let image = &images[img.texture().source().index()];
            let mut specular_map_buffer =
                BytesMut::with_capacity((image.width * image.height * 12) as usize);
            let bytes_per_pixel = Self::byte_size_from_format(image.format);
            for current_pixel in 0..image.width * image.height {
                let current_pixel = current_pixel as usize;
                let current_byte = current_pixel * bytes_per_pixel;
                let components = &image.pixels[current_byte..(current_byte + bytes_per_pixel)];
                use Format::*;

                unsafe {
                    let (_, roughness, metalness, _) =
                        Self::convert_value(components, image.format);

                    let (specular, diffuse_adj) = Self::convert_roughness(roughness, metalness);

                    // Shininess is average across shininesses at each roughness patch
                    shininess = (shininess + (1.0 - roughness).sqrt() + 0.25) / 2.0;

                    // Adjust the diffuse color

                    if let Some(map) = diffuse_map.as_mut() {
                        let diffuse_stride = Self::byte_size_from_format(image.format);
                        let diffuse_color = Self::convert_value(
                            &image.pixels[(current_pixel * diffuse_stride)
                                ..(current_pixel * diffuse_stride + diffuse_stride)],
                            image.format,
                        );
                        let diffuse_color = &[
                            diffuse_color.0 * diffuse_adj,
                            diffuse_color.1 * diffuse_adj,
                            diffuse_color.2 * diffuse_adj,
                            diffuse_color.3,
                        ];
                        let bytes = Self::value_to_bytes(diffuse_color, map.format);
                        for j in 0..(diffuse_stride) {
                            map.pixels[current_pixel * diffuse_stride + j] = bytes[j];
                        }
                    } else {
                        // Make the diffuse factor the average of the adjustments
                        diffuse_factor = [
                            (diffuse_factor[0] + diffuse_factor[0] * diffuse_adj) / 2.0,
                            (diffuse_factor[1] + diffuse_factor[1] * diffuse_adj) / 2.0,
                            (diffuse_factor[2] + diffuse_factor[2] * diffuse_adj) / 2.0,
                            diffuse_factor[3],
                        ];
                    }

                    // Write to the specular map
                    let bytes = Self::value_to_bytes(&[specular], Format::R32G32B32FLOAT);
                    for j in 0..12 {
                        specular_map_buffer[current_byte / bytes_per_pixel * 12 + j] = bytes[j];
                    }
                }
            }
            images.push(gltf::image::Data {
                pixels: specular_map_buffer.to_vec(),
                format: Format::R32G32B32FLOAT,
                width: image.width,
                height: image.height,
            });
            let specular_id = images.len() - 1;
            if let Some(map) = diffuse_map {
                images.push(map);
            }
            Material {
                name: m.name().unwrap_or("UnknownMaterial").to_string(),
                diffuse: pbr
                    .base_color_texture()
                    .map_or(FactorOrTexture::Vec4(diffuse_factor.into()), |_| {
                        FactorOrTexture::Texture(images.len() - 1)
                    }),
                specular: FactorOrTexture::Texture(specular_id),
                normal_map: None,
                shininess,
            }
        } else {
            let (specular_factor, diffuse_adj_factor) =
                Self::convert_roughness(pbr.roughness_factor(), pbr.metallic_factor());
            let shininess = (1.0 - pbr.roughness_factor()).sqrt() + 0.25;

            println!(
                "Metalness factor: {}, roughness factor: {}",
                pbr.metallic_factor(),
                pbr.roughness_factor()
            );
            println!(
                "Specular factor: {}, diffuse factor: {:?}, shininess: {}",
                specular_factor, diffuse_adj_factor, shininess
            );

            if let Some(mut image) = diffuse_map {
                let diffuse_stride = Self::byte_size_from_format(image.format);
                for current_pixel in 0..image.width * image.height {
                    let current_byte = current_pixel as usize * diffuse_stride;
                    unsafe {
                        let diffuse_color = Self::convert_value(
                            &image.pixels[current_byte..(current_byte + diffuse_stride)],
                            image.format,
                        );

                        let diffuse_color = &[
                            diffuse_color.0 * diffuse_adj_factor,
                            diffuse_color.1 * diffuse_adj_factor,
                            diffuse_color.2 * diffuse_adj_factor,
                            diffuse_color.3,
                        ];
                        let bytes = Self::value_to_bytes(diffuse_color, image.format);
                        for j in 0..(diffuse_stride) {
                            image.pixels[current_byte + j] = bytes[j];
                        }
                    }
                }
                images[pbr.base_color_texture().unwrap().texture().source().index()] = image;
            }

            let diffuse_color = pbr.base_color_factor();
            let diffuse_color = [
                diffuse_color[0] * diffuse_adj_factor,
                diffuse_color[1] * diffuse_adj_factor,
                diffuse_color[2] * diffuse_adj_factor,
                diffuse_color[3],
            ];
            Material {
                name: m.name().unwrap_or("UnknownMaterial").to_string(),
                diffuse: pbr
                    .base_color_texture()
                    .map_or(FactorOrTexture::Vec4(diffuse_color.into()), |info| {
                        FactorOrTexture::Texture(info.texture().source().index())
                    }),
                specular: FactorOrTexture::Vec3(
                    [specular_factor, specular_factor, specular_factor].into(),
                ),
                normal_map: None,
                shininess,
            }
        }
    }

    fn value_to_bytes(value: &[f32], format: Format) -> Vec<u8> {
        use Format::*;
        value
            .iter()
            .map(|x| match format {
                R8G8B8 | R8G8B8A8 => (*x as u8).to_le_bytes().to_vec(),
                R16G16B16 | R16G16B16A16 => (*x as u16).to_le_bytes().to_vec(),
                R32G32B32FLOAT | R32G32B32A32FLOAT => (*x).to_le_bytes().to_vec(),
                _ => panic!(
                    "Metallic roughness texture should have green and blue components in glTF 2.0!"
                ),
            })
            .flatten()
            .collect()
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
        self.ibo = Some({
            let mut ibo = BufferObject::<InstanceTransformVertex>::new(
                gl,
                gl::ARRAY_BUFFER,
                gl::STREAM_DRAW,
                (CONFIG.performance.max_batch_size * 3) as usize,
            );
            ibo
        });
        self.textures = Some(
            self.textures_raw
                .iter()
                .map(|(bytes, width, height)| {
                    Box::new(Texture::new_with_bytes(
                        gl,
                        TextureParameters::default(),
                        bytes,
                        *width as usize,
                        *height as usize,
                        1,
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
        ibo: &BufferObject<InstanceTransformVertex>,
    ) -> Self {
        let vao = objects::VertexArrayObject::new(gl);
        let vbo = Box::new(objects::BufferObject::new_with_vec(
            gl,
            gl::ARRAY_BUFFER,
            &mesh.vertices,
        ));
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

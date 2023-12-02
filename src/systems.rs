use crate::entity::mesh_component::{MeshNode, Model, ModelComponent};
use crate::entity::{camera_component, transform_component, Entity};
use crate::render_gl::data::InstanceTransformVertex;
use crate::render_gl::objects::{Buffer, VertexBufferObject};
use crate::render_gl::shaders::Program;
use crate::*;
use entity::camera_component::CameraComponent;
use entity::transform_component::TransformComponent;
use entity::EntitySystem;
use gl::VertexBindingDivisor;
use gltf::{Glb, Gltf};
use rand::Rng;
use rayon::prelude::{IntoParallelRefIterator, ParallelBridge, ParallelExtend, ParallelIterator};
use render_gl::shaders;
use ritelinked::LinkedHashSet;
use std::collections::HashSet;
use std::ffi::CString;
use std::io::Read;
use std::sync::mpsc::channel;

pub fn add_camera(scene: &mut Scene) {
    let e = scene.entities.new_entity();
    scene.entities.add_component(
        e,
        TransformComponent::new_from_rot_trans(
            glam::Vec3::Y,
            glam::vec3(0.0, 0.0, -3.0),
            gl::STREAM_DRAW,
        ),
    );
    scene
        .entities
        .add_component(e, CameraComponent { fov: 90.0 });
    scene.camera = Some(e);
}

pub fn load_shaders(scene: &mut Scene) {
    let vert_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.vert")).unwrap(),
        gl::VERTEX_SHADER,
    )
    .unwrap();

    let frag_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.frag")).unwrap(),
        gl::FRAGMENT_SHADER,
    )
    .unwrap();
    scene
        .shader_programs
        .push(Program::from_shaders(&[frag_shader, vert_shader]).unwrap());
}

pub fn load_entities(scene: &mut Scene) -> Vec<Entity> {
    let mut trng = rand::thread_rng();
    let assets = [
        "./assets/entities/emily.glb",
        "./assets/entities/cube.glb",
        "./assets/entities/emily.gltf",
    ];
    let mut entities = vec![];
    for i in 0..100 {
        let thing = scene.entities.new_entity();
        scene.entities.add_component(
            thing,
            ModelComponent {
                path: assets[trng.gen_range(0..assets.len())].to_string(),
                shader_program: 0,
            },
        );
        scene.entities.add_component(
            thing,
            TransformComponent::new_from_rot_trans(
                glam::Vec3::ZERO,
                glam::vec3((i - 50) as f32 * 1.0, 0.0, 0.0),
                gl::STATIC_DRAW,
            ),
        );
        entities.push(thing);
    }
    entities
}

pub fn unload_entity_models(
    entities: &EntitySystem,
    new_entities: Vec<Entity>,
    models: &mut HashMap<String, Model>,
) {
    for entity in new_entities {
        let model_component = entities.get_component::<ModelComponent>(entity).unwrap();
        let model = models.get_mut(&model_component.path).unwrap();
        model.entities.remove(&entity);
        model.entities_dirty_flag = true;
        if model.entities.is_empty() {
            models.remove(&model_component.path);
        }
    }
}

pub fn load_entity_models(
    entities: &EntitySystem,
    new_entities: Vec<Entity>,
    models: &mut HashMap<String, Model>,
) {
    let models_requested = new_entities.iter().fold(
        HashMap::new(),
        |mut registry: HashMap<String, LinkedHashSet<Entity>>, entity: &Entity| {
            let mc = entities.get_component::<ModelComponent>(*entity);
            let path = &mc
                .as_ref()
                .expect("Sent entity without a model component to model load system, invalid.")
                .path;

            if let Some(entry) = models.get_mut(path).map(|x| &mut x.entities) {
                entry.insert(*entity);
            } else {
                if let Some(entry) = registry.get_mut(path) {
                    entry.insert(*entity);
                } else {
                    let mut hs = LinkedHashSet::new();
                    hs.insert(*entity);
                    registry.insert(path.clone(), hs);
                }
            }
            registry
        },
    );
    let time = std::time::Instant::now();
    let load_start_time = time.elapsed().as_millis();
    let (sender, receiver) = channel();
    models_requested.par_iter().for_each(|(path, entities)| {
        let start_gltf_time = time.elapsed().as_millis();
        let gltf = gltf::import(path).expect(&format!(
            "Unable to interpret model file {} as glTF 2.0 file.",
            path
        ));
        let end_gltf_time = time.elapsed().as_millis();
        println!(
            "GLTF loaded for {} in time {}ms",
            path,
            end_gltf_time - start_gltf_time
        );

        let start_process_time = time.elapsed().as_millis();
        let mut model = Model::from_gltf(gltf).expect("Unable to load model");
        model.entities.extend(entities);
        let end_process_time = time.elapsed().as_millis();
        println!(
            "GLTF processed to native formats for {} in time {}ms",
            path,
            end_process_time - start_process_time
        );

        let _ = sender.send((path, model));
    });
    let load_end_time = time.elapsed().as_millis();
    let new_models =
        receiver
            .try_iter()
            .fold(HashMap::new(), |mut new_models, (path, mut model)| {
                // We have to do the OpenGL setup all on the main thread
                model.setup_model_gl();
                new_models.insert(path.clone(), model);
                println!("Loaded model: {}", path);
                new_models
            });
    println!(
        "Model loading complete in {}ms",
        load_end_time - load_start_time
    );
    models.extend(new_models);
}

pub fn render(
    camera: Option<Entity>,
    entities: &mut EntitySystem,
    shader_programs: &Vec<Program>,
    models: &mut HashMap<String, Model>,
    width: u32,
    height: u32,
) {
    let mut last_shader_program_index = 0;
    let mut program = &shader_programs[0];
    program.set_used();
    let camera_entity = camera.expect("No camera found");
    utils::camera_prepare_shader(camera_entity, entities, program, width, height);

    for (path, model) in models.iter_mut() {
        if last_shader_program_index != model.shader_program {
            program = &shader_programs[model.shader_program];
            last_shader_program_index = model.shader_program;
            program.set_used();
            utils::camera_prepare_shader(camera_entity, entities, program, width, height);
        }
        // if the total number of entities changed, we need to totally reinitialize the buffer
        if model.entities_dirty_flag {
            let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    entities
                        .get_component::<TransformComponent>(*entity)
                        .expect("Tried to render model for an entity that either doesn't have a transform component, or has been recycled.")
                })
                .map(|tc| InstanceTransformVertex::new(tc.transform.to_matrix().to_cols_array()))
                .collect::<Vec<InstanceTransformVertex>>();
            model
                .ibo
                .as_mut()
                .expect("Model must have an instance buffer object by the time rendering starts.")
                .upload_data(&new_transforms, gl::DYNAMIC_DRAW);
            model.entities_dirty_flag = false;
        } else {
            let mut num = 0;
            // otherwise, we can just update what changed
            for (i, entity) in model.entities.iter().enumerate() {
                let mut tc = entities
                    .get_component_mut::<TransformComponent>(*entity)
                    .expect("Entities must have a transform component to have a model.");

                if tc.dirty_flag {
                    model.ibo
                         .as_mut()
                         .expect("Model must have an instance buffer object by the time rendering starts.")
                         .update_data(
                        &[InstanceTransformVertex::new(
                            tc.transform.to_matrix().to_cols_array(),
                        )],
                        i,
                    );
                    num += 1;
                    tc.dirty_flag = false;
                }
            }
        }

        for node in &model.meshes {
            render_node_tree(&node, &model, program);
        }
    }
}

fn render_node_tree(node: &MeshNode, model: &Model, program: &Program) {
    for mesh in &node.primitives {
        let mesh_gl = mesh
            .gl
            .as_ref()
            .expect("Model must have OpenGL elements setup before rendering it, baka!");
        mesh_gl.vao.bind();

        let material = &model.materials[mesh.material_index];
        material.activate(&model, &program);

        mesh_gl.vao.draw_elements_instanced(
            gl::TRIANGLES,
            mesh_gl.ebo.count() as gl::types::GLint,
            gl::UNSIGNED_INT,
            0,
            model.entities.len() as gl::types::GLint,
        );
        mesh_gl.vao.unbind();
    }
    for child in &node.children {
        render_node_tree(&child, model, program);
    }
}

pub fn physics(scene: &Scene) -> Vec<SceneCommand> {
    let transform_components = &scene.entities.get_component_vec::<TransformComponent>();
    transform_components
        .iter()
        .enumerate()
        .filter_map(|(eid, tc)| {
            if eid % 2 != 0 {
                Some(SceneCommand::DisplaceEntity(eid, glam::vec3(0.0, 0.3, 0.0)))
            } else {
                None
            }
        })
        .collect()
}

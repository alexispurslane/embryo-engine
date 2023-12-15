use crate::entity::mesh_component::{MeshNode, Model, ModelComponent};
use crate::entity::Entity;
use crate::render_gl::data::InstanceTransformVertex;
use crate::render_gl::objects::Buffer;
use crate::render_gl::shaders::Program;
use crate::*;
use entity::camera_component::CameraComponent;
use entity::transform_component::TransformComponent;
use gl::Gl;
use rand::Rng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_gl::shaders;
use ritelinked::LinkedHashSet;
use std::any::Any;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::mpsc::{channel, Sender};

pub fn load_shaders(gl: &Gl, scene: &mut Scene) {
    let vert_shader =
        shaders::Shader::from_file(gl, "./assets/shaders/camera.vert", gl::VERTEX_SHADER).unwrap();

    let frag_shader =
        shaders::Shader::from_file(gl, "./assets/shaders/material.frag", gl::FRAGMENT_SHADER)
            .unwrap();
    scene
        .shader_programs
        .push(Program::from_shaders(gl, &[frag_shader, vert_shader]).unwrap());
}

pub fn load_entities(scene: &mut Scene) -> Vec<Entity> {
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

pub fn unload_entity_models(scene: &mut Scene, new_entities: &Vec<Entity>) {
    for entity in new_entities {
        let model_component = scene
            .entities
            .get_component::<ModelComponent>(*entity)
            .unwrap();
        if let Some(model) = scene.resource_manager.models.get_mut(&model_component.path) {
            model.entities.remove(&entity);
            model.entities_dirty_flag = true;
            if model.entities.is_empty() {
                scene.resource_manager.models.remove(&model_component.path);
            }
        }
    }
}

pub fn load_entity_models(scene: &mut Scene, new_entities: &Vec<Entity>) {
    scene
        .resource_manager
        .request_model_batch(&scene.entities, new_entities)
}

pub fn integrate_loaded_models(gl: &Gl, scene: &mut Scene) {
    scene.resource_manager.try_integrate_loaded_models(gl);
}

pub fn render(gl: &Gl, scene: &mut Scene, width: u32, height: u32) {
    let mut last_shader_program_index = 0;
    let mut program = &scene.shader_programs[0];
    program.set_used();
    let camera_entity = scene.camera.expect("No camera found");
    utils::camera_prepare_shader(camera_entity, &scene.entities, program, width, height);

    for (path, model) in scene.resource_manager.models.iter_mut() {
        if last_shader_program_index != model.shader_program {
            program = &scene.shader_programs[model.shader_program];
            last_shader_program_index = model.shader_program;
            program.set_used();
            utils::camera_prepare_shader(camera_entity, &scene.entities, program, width, height);
        }
        // if the total number of entities changed, we need to totally reinitialize the buffer
        if model.entities_dirty_flag {
            let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    scene.entities
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
                let mut tc = scene
                    .entities
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
            render_node_tree(&gl, &node, &model, program);
        }
    }
}

fn render_node_tree(gl: &Gl, node: &MeshNode, model: &Model, program: &Program) {
    for mesh in &node.primitives {
        let mesh_gl = mesh
            .gl_mesh
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
        render_node_tree(gl, &child, model, program);
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

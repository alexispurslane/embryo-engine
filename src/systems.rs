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

pub fn load_shaders(gl: &Gl, render: &mut RenderState) {
    let vert_shader =
        shaders::Shader::from_file(gl, "./data/shaders/camera.vert", gl::VERTEX_SHADER).unwrap();

    let frag_shader =
        shaders::Shader::from_file(gl, "./data/shaders/material.frag", gl::FRAGMENT_SHADER)
            .unwrap();
    render
        .shader_programs
        .push(Program::from_shaders(gl, &[frag_shader, vert_shader]).unwrap());
}

pub fn load_entities(scene: &mut GameState) -> Vec<Entity> {
    let e = scene.entities.new_entity();
    scene.entities.add_component(
        e,
        TransformComponent::new_from_rot_trans(glam::Vec3::Y, glam::vec3(0.0, 0.0, -3.0), true),
    );
    scene
        .entities
        .add_component(e, CameraComponent { fov: 90.0 });
    scene.camera = Some(e);

    let mut trng = rand::thread_rng();
    let data = ["./data/models/heroine.glb"];
    let mut entities = vec![];
    for i in 0..10000 {
        let thing = scene.entities.new_entity();
        scene.entities.add_component(
            thing,
            ModelComponent {
                path: data[trng.gen_range(0..data.len())].to_string(),
                shader_program: 0,
            },
        );
        scene.entities.add_component(
            thing,
            TransformComponent::new_from_rot_trans(
                glam::Vec3::ZERO,
                glam::vec3(
                    (i % 25) as f32 * 1.0,
                    ((i / 25) % 25) as f32 * 2.0,
                    (i / 625) as f32 * 1.0,
                ),
                false,
            ),
        );
        entities.push(thing);
    }
    entities
}

pub fn unload_entity_models(
    scene: &mut GameState,
    render: &mut RenderState,
    resource_manager: &ResourceManager,
    new_entities: &Vec<Entity>,
) {
    let mut removes = vec![];
    for entity in new_entities {
        let model_component = scene
            .entities
            .get_component::<ModelComponent>(*entity)
            .unwrap();
        removes.push((model_component.path.clone(), *entity));
        if let Some(model) = render.models.get_mut(&model_component.path) {
            model.entities.remove(&entity);
            model.entities_dirty_flag = true;
            if model.entities.is_empty() {
                render.models.remove(&model_component.path);
            }
        }
    }
    resource_manager.request_unload_models(removes);
}

pub fn load_entity_models(
    scene: &mut GameState,
    resource_manager: &ResourceManager,
    new_entities: &Vec<Entity>,
) {
    resource_manager.request_models(
        new_entities
            .iter()
            .map(|e| {
                let model_component = scene.entities.get_component::<ModelComponent>(*e).unwrap();
                (model_component.path.clone(), *e)
            })
            .collect(),
    );
}

pub fn integrate_loaded_models(
    gl: &Gl,
    resource_manager: &ResourceManager,
    render: &mut RenderState,
) {
    resource_manager.try_integrate_loaded_models(&mut render.models, gl);
}

pub fn render(gl: &Gl, render: &mut RenderState, width: u32, height: u32) {
    let camera = render.camera.as_ref().unwrap();
    let mut last_shader_program_index = 0;
    let mut program = &render.shader_programs[0];
    program.set_used();
    utils::camera_prepare_shader(program, camera);

    let models = &mut render.models;
    let egens = &render.entity_generations;
    let etrans = &render.entity_transforms;
    for (path, model) in models.iter_mut() {
        if last_shader_program_index != model.shader_program {
            program = &render.shader_programs[model.shader_program];
            last_shader_program_index = model.shader_program;
            program.set_used();
            utils::camera_prepare_shader(program, camera);
        }
        // if the total number of entities changed, we need to totally reinitialize the buffer
        let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    RenderState::get_entity_transform(egens, etrans, *entity)
                        .expect("Tried to render model for an entity that either doesn't have a transform component, or has been recycled.")
                })
                .map(|mat| InstanceTransformVertex::new(mat.to_cols_array()))
                .collect::<Vec<InstanceTransformVertex>>();
        let batches = (new_transforms.len() as u64).div_ceil(CONFIG.performance.max_batch_size);
        let mbs = CONFIG.performance.max_batch_size as usize;
        for batch in 0..batches {
            let batch_start = batch as usize * mbs;
            let batch_size = mbs.min(new_transforms.len() - batch_start) as usize;
            model
                .ibo
                .as_mut()
                .expect("Model must have an instance buffer object by the time rendering starts.")
                .send_data(&new_transforms[batch_start..batch_start + batch_size], 0);

            for mesh in &model.meshes {
                for mesh in &mesh.primitives {
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
                        batch_size as gl::types::GLint,
                    );
                    mesh_gl.vao.unbind();
                }
            }
        }
    }
}

pub fn physics(scene: &mut GameState, dt: u128) {}

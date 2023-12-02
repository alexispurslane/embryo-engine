use crate::entity::mesh_component::{Model, ModelComponent};
use crate::entity::{transform_component, EntityID};
use crate::render_gl::data::InstanceTransformVertex;
use crate::render_gl::objects::{Buffer, VertexBufferObject};
use crate::render_gl::shaders::Program;
use crate::*;
use entity::camera_component::CameraComponent;
use entity::transform_component::TransformComponent;
use entity::EntitySystem;
use gl::VertexBindingDivisor;
use rand::Rng;
use rayon::prelude::{IntoParallelRefIterator, ParallelBridge, ParallelExtend, ParallelIterator};
use render_gl::shaders;
use russimp::scene::PostProcess;
use std::collections::HashSet;
use std::ffi::CString;

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
    scene.camera = Some(e.id);
}

pub fn add_level(scene: &mut Scene) {
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

    let thing = scene.entities.new_entity();
    scene.entities.add_component(
        thing,
        ModelComponent {
            path: "./assets/entities/emily.glb".to_string(),
        },
    );
    scene.entities.add_component(
        thing,
        TransformComponent::new_from_rot_trans(glam::Vec3::ZERO, glam::Vec3::ZERO, gl::STATIC_DRAW),
    );
}

pub fn load_entity_models(entities: &EntitySystem, models: &mut HashMap<String, Model>) {
    let model_vec = &entities.get_component_vec::<ModelComponent>();
    let has_model = entities.get_with_component(model_vec);

    for (eid, model_component) in has_model {
        if let Some(model) = models.get_mut(&model_component.path) {
            model.entities.push(eid);
            model.entities_dirty_flag = true;
        } else {
            let ai_scene = russimp::scene::Scene::from_file(
                &model_component.path,
                vec![
                    PostProcess::Triangulate,
                    PostProcess::ValidateDataStructure,
                    PostProcess::ImproveCacheLocality,
                    PostProcess::GenerateUVCoords,
                    PostProcess::OptimizeMeshes,
                    PostProcess::FlipUVs,
                ],
            )
            .expect(&format!(
                "Cannot load model from file: {}",
                model_component.path
            ));
            let mut model = Model::from_ai_scene(ai_scene).expect("Unable to load model");
            model.entities.push(eid);
            models.insert(model_component.path.clone(), model);
        }
    }
}

pub fn render(
    camera: Option<EntityID>,
    entities: &mut EntitySystem,
    shader_programs: &Vec<Program>,
    models: &mut HashMap<String, Model>,
    width: u32,
    height: u32,
) {
    let program = &shader_programs[0];
    program.set_used();
    {
        let camera_eid = camera.expect("No camera found");
        let ct = &entities.get_component_vec::<TransformComponent>()[camera_eid];
        let camera_transform = ct
            .as_ref()
            .expect("Camera needs to have TransformComponent");
        let cc = &entities.get_component_vec::<CameraComponent>()[camera_eid];
        let camera_component = cc.as_ref().expect("Camera needs to have CameraComponent");

        program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &camera_transform.point_of_view().to_cols_array(),
        );
        program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &camera_component.project(width, height).to_cols_array(),
        );
    }

    let transform_components = &mut entities.get_component_vec_mut::<TransformComponent>();

    for (path, model) in models.iter_mut() {
        // if the total number of entities changed, we need to totally reinitialize the buffer
        if model.entities_dirty_flag {
            let new_transforms = model
                .entities
                .iter()
                .map(|eid| {
                    transform_components[*eid]
                        .as_ref()
                        .expect("Entities must have a transform component to have a model.")
                })
                .map(|tc| InstanceTransformVertex::new(tc.transform.to_matrix().to_cols_array()))
                .collect::<Vec<InstanceTransformVertex>>();
            model.ibo.upload_data(&new_transforms, gl::DYNAMIC_DRAW);
            model.entities_dirty_flag = false;
        } else {
            // otherwise, we can just update what changed
            for (i, eid) in model.entities.iter().enumerate() {
                let tc = transform_components[*eid]
                    .as_mut()
                    .expect("Entities must have a transform component to have a model.");

                if tc.dirty_flag {
                    model.ibo.update_data(
                        &[InstanceTransformVertex::new(
                            tc.transform.to_matrix().to_cols_array(),
                        )],
                        i,
                    );
                    tc.dirty_flag = false;
                }
            }
        }

        for mesh in &model.meshes {
            mesh.vao.bind();

            let material = &model.materials[mesh.material_index];
            material.activate(&model, &program);

            mesh.vao.draw_elements_instanced(
                gl::TRIANGLES,
                mesh.ebo.count() as gl::types::GLint,
                gl::UNSIGNED_INT,
                0,
                model.entities.len() as gl::types::GLint,
            );
            mesh.vao.unbind();
        }
    }
}

pub fn physics(scene: &Scene) -> Vec<SceneCommand> {
    let transform_components = &scene.entities.get_component_vec::<TransformComponent>();
    transform_components
        .iter()
        .enumerate()
        .map(|(eid, tc)| SceneCommand::DisplaceEntity(eid, glam::vec3(0.0, 0.3, 0.0)))
        .collect()
}

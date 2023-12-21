use crate::entity::light_component::{Attenuation, LightComponent};
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

pub fn load_entities(scene: &mut GameState) -> Vec<Entity> {
    let e = scene.entities_mut().gen_entity();
    scene.entities_mut().add_component(
        e,
        TransformComponent::new_from_rot_trans(glam::Vec3::Y, glam::vec3(0.0, 0.0, -3.0), true),
    );
    scene
        .entities_mut()
        .add_component(e, CameraComponent { fov: 90.0 });
    scene.register_camera(e);
    /*scene.entities_mut().add_component(
        e,
        LightComponent::Spot {
            color: glam::vec3(0.4, 0.6, 0.8),
            ambient: glam::vec3(0.0, 0.2, 0.3),
            cutoff: 0.0,
            fade_exponent: 25.0,
            attenuation: Attenuation {
                constant: 0.2,
                linear: 0.0,
                quadratic: 0.0,
            },
        },
    );
    scene.register_light(e);*/

    let mut trng = rand::thread_rng();
    let colors = &[
        glam::vec3(0.1, 30.2, 0.1),
        glam::vec3(20.2, 0.1, 0.1),
        glam::vec3(0.1, 0.1, 10.2),
    ];
    for i in 0..30 {
        let e = scene.entities_mut().gen_entity();
        scene.entities_mut().add_component(
            e,
            TransformComponent::new_from_rot_trans(
                glam::Vec3::ZERO,
                glam::vec3(
                    trng.gen_range(0..25) as f32,
                    trng.gen_range(0..25) as f32 * 2.0,
                    trng.gen_range(0..16) as f32,
                ),
                true,
            ),
        );
        scene.entities_mut().add_component(
            e,
            LightComponent::Point {
                color: colors[trng.gen_range(0..colors.len())],
                ambient: glam::vec3(0.0, 0.0, 0.0),
                attenuation: Attenuation {
                    constant: 1.0,
                    linear: 0.7,
                    quadratic: 0.3,
                },
            },
        );
        scene.register_light(e);
    }

    let data = ["./data/models/heroine.glb"];
    let mut entities = vec![];
    for i in 0..10000 {
        let thing = scene.entities_mut().gen_entity();
        scene.entities_mut().add_component(
            thing,
            ModelComponent {
                path: data[trng.gen_range(0..data.len())].to_string(),
                shader_program: 0,
            },
        );
        scene.entities_mut().add_component(
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
            .entities()
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
                let model_component = scene
                    .entities()
                    .get_component::<ModelComponent>(*e)
                    .unwrap();
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

pub fn physics(game_state: &mut GameState, dt: f32, time: u128) {}

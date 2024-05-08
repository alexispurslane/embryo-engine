/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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

use self::entity::hierarchy_component::HierarchyComponent;

pub fn load_entities(scene: &mut GameState) {
    let e = scene.gen_entity();
    scene.add_component(
        e,
        TransformComponent::new_from_rot_trans(glam::Vec3::Y, glam::vec3(0.0, 0.0, -3.0), true),
    );
    scene.add_component(e, CameraComponent { fov: 90.0 });
    scene.add_component(
        e,
        LightComponent::Spot {
            color: glam::vec3(14.0, 16.0, 18.0),
            ambient: glam::vec3(0.0, 0.0, 0.0),
            cutoff: 0.0,
            fade_exponent: 15.0,
            attenuation: Attenuation {
                constant: 0.2,
                linear: 9.0,
                quadratic: 1.9,
            },
        },
    );
    scene.register_camera(e);

    let mut trng = rand::thread_rng();
    let colors = &[
        glam::vec3(0.1, 30.2, 0.1),
        glam::vec3(20.2, 0.1, 0.1),
        glam::vec3(0.1, 0.1, 10.2),
    ];
    for i in 0..30 {
        let e = scene.gen_entity();
        scene.add_component(
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
        scene.add_component(
            e,
            LightComponent::Point {
                color: colors[trng.gen_range(0..colors.len())],
                ambient: glam::vec3(0.0, 0.0, 0.0),
                attenuation: Attenuation {
                    constant: 1.5,
                    linear: 9.0,
                    quadratic: 1.9,
                },
            },
        );
    }

    let parent = scene.gen_entity();
    scene.add_component(
        parent,
        TransformComponent::new_from_rot_trans(glam::Vec3::ZERO, glam::Vec3::ZERO, false),
    );
    let data = ["./data/models/heroine.glb"];
    for i in 0..10000 {
        let thing = scene.gen_entity();
        scene.add_component(
            thing,
            ModelComponent {
                path: data[trng.gen_range(0..data.len())].to_string(),
                shader_program: 0,
            },
        );
        if i < 100 {
            scene.add_component(thing, HierarchyComponent::new(parent))
        }
        scene.add_component(
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
    }
}

pub fn load_entity_models(scene: &mut GameState, new_entities: &Vec<Entity>) {}

pub fn physics(game_state: &mut GameState, dt: f32, time: u128) {
    let transforms = &mut game_state
        .entities
        .get_component_vec_mut::<TransformComponent>()
        .unwrap();
    let e = &mut transforms[31];
    e.as_mut().unwrap().displace_by(glam::vec3(0.0, 0.0, 0.005));
}

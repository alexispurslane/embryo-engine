/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{render_gl::data::Cvec3, render_thread::ShaderLight};

use super::*;

#[derive(Clone)]
pub struct Attenuation {
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

#[derive(Clone)]
pub enum LightComponent {
    Ambient {
        ambient: glam::Vec3,
    },
    Directional {
        color: glam::Vec3,
        ambient: glam::Vec3,
    },
    Point {
        color: glam::Vec3,
        ambient: glam::Vec3,
        attenuation: Attenuation,
    },
    Spot {
        color: glam::Vec3,
        ambient: glam::Vec3,
        cutoff: f32,
        fade_exponent: f32,
        attenuation: Attenuation,
    },
}

impl Component for LightComponent {
    fn get_id() -> ComponentID {
        "LightComponent"
    }
    fn add_hook(&mut self, current_entity: Entity, game_state: &mut GameState) {
        game_state.register_light(current_entity)
    }
}

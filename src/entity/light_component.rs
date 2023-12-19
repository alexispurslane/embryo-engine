use crate::{render_gl::data::Cvec3, render_thread::ShaderLight};

use super::*;

#[derive(Clone)]
pub struct Attenuation {
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

#[derive(ComponentId, Clone)]
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

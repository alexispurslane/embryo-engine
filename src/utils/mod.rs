use std::{cell::RefMut, ffi::CString};

use gl::Gl;
use sdl2::image::LoadSurface;
use std::cell::Ref;

use crate::{
    entity::{
        camera_component::CameraComponent,
        transform_component::{self, TransformComponent},
        Entity, EntitySystem,
    },
    render_gl::shaders::Program,
};

pub type Degrees = f32;
pub type Radians = f32;

pub fn create_whitespace_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn clear_screen(gl: &Gl) {
    unsafe {
        gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
}

pub fn setup_viewport(gl: &Gl, (w, h): (u32, u32)) {
    unsafe {
        gl.Viewport(0, 0, w as gl::types::GLint, h as gl::types::GLint);
        gl.ClearColor(0.0, 0.0, 0.0, 1.0);
        gl.Enable(gl::DEPTH_TEST);
    }
}

pub mod shapes {
    use crate::render_gl::data::VertexTex;

    pub fn unit_cube() -> [VertexTex; 36] {
        [
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
        ]
    }
}

pub fn camera_prepare_shader(
    camera_entity: Entity,
    entities: &EntitySystem,
    program: &Program,
    width: u32,
    height: u32,
) {
    if let Some(camera_component) = &entities.get_component::<CameraComponent>(camera_entity) {
        let transform_component = &entities
            .get_component::<TransformComponent>(camera_entity)
            .expect("Camera needs to have TransformComponent");
        program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &transform_component.point_of_view().to_cols_array(),
        );
        program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &camera_component.project(width, height).to_cols_array(),
        );
    } else {
        println!("WARNING: Scene camera points to entity that either has been recycled, or does not have a camera component.");
    }
}

#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(zip!($($y), +))
    )
}
pub use zip;

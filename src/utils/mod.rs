use std::{cell::RefMut, ffi::CString};

use gl::Gl;
use glam::Vec4Swizzles;

use crate::{
    entity::{
        camera_component::CameraComponent,
        light_component::*,
        transform_component::{self, TransformComponent},
        Entity, EntitySystem,
    },
    render_gl::{
        objects::{Buffer, BufferObject},
        shaders::Program,
    },
    render_thread::{light_component_to_shader_light, RenderCameraState, RenderState, ShaderLight},
    CONFIG,
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
        gl.Enable(gl::CULL_FACE);
        #[cfg(debug_assertions)]
        gl.Enable(gl::DEBUG_OUTPUT);
    }
}

pub fn camera_prepare_shader(program: &Program, camera: &RenderCameraState) {
    program.set_uniform_matrix_4fv(
        &CString::new("view_matrix").unwrap(),
        &camera.view.to_cols_array(),
    );
    program.set_uniform_matrix_4fv(
        &CString::new("projection_matrix").unwrap(),
        &camera.proj.to_cols_array(),
    );
}

pub fn lights_prepare_shader(
    gl: &Gl,
    program: &Program,
    lights_ubo: &mut BufferObject<ShaderLight>,
    camera: &RenderCameraState,
    lights: &[ShaderLight],
    round_robin_buffer: usize,
) {
    lights_ubo.write_to_persistant_map(round_robin_buffer * CONFIG.performance.max_lights, lights);
    lights_ubo.bind();
    unsafe {
        gl.BindBufferBase(gl::UNIFORM_BUFFER, 0, lights_ubo.id);
    }
    program.set_uniform_3f(
        &CString::new("cameraDirection").unwrap(),
        (camera.view * glam::Vec4::Z).xyz().to_array().into(),
    );
    program.set_uniform_1ui(
        &CString::new("lightoffset").unwrap(),
        (round_robin_buffer * CONFIG.performance.max_lights) as u32,
    );
}

pub fn shader_set_lightmask(program: &Program, lightmask: u32) {
    // If it is 32, then we physically *couldn't* pass in a value that was too large!
    if CONFIG.performance.max_lights < 32
        && lightmask > 2_u32.pow(CONFIG.performance.max_lights as u32) - 1
    {
        panic!("Cannot enable that many lights!");
    }
    program.set_uniform_1ui(&CString::new("lightmask").unwrap(), lightmask);
}

#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(zip!($($y), +))
    )
}
pub use zip;

pub mod config {
    use serde::Deserialize;
    use std::io::prelude::*;
    #[derive(Deserialize)]
    pub struct PerfConfig {
        pub update_interval: usize,
        pub cap_update_fps: bool,
        pub cap_render_fps: bool,
        pub max_batch_size: usize,
        pub max_lights: usize,
    }

    #[derive(Deserialize)]
    pub struct ControlConfig {
        pub mouse_sensitivity: f32,
        pub motion_speed: f32,
    }

    #[derive(Deserialize)]
    pub struct GameConfig {
        pub performance: PerfConfig,
        pub controls: ControlConfig,
    }

    pub fn read_config() -> GameConfig {
        let mut contents = String::new();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open("./data/config.toml")
        {
            file.read_to_string(&mut contents).unwrap();
            println!("{contents}");
            if contents.len() == 0 {
                contents = r#"
[performance]
update_interval = 16
cap_render_fps = true
cap_update_fps = true
max_batch_size = 1000
max_lights = 32

[controls]
mouse_sensitivity = 1.0
motion_speed = 10.0
"#
                .into();
                file.write(contents.as_bytes()).unwrap();
            }
        }
        let config: GameConfig = toml::from_str(&contents).unwrap();
        if config.performance.update_interval > 33
            || config.performance.max_batch_size < 1
            || (config.performance.max_lights > 32 || config.performance.max_lights < 1)
            || config.controls.mouse_sensitivity < 1.0
            || config.controls.motion_speed <= 0.0
        {
            panic!("Invalid values in config file.");
        }
        config
    }
}

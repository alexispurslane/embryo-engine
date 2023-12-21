#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

extern crate bytes;
extern crate gl;
extern crate glam;
extern crate gltf;
extern crate rayon;
extern crate rmp;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2_support;

use entity::EntitySystem;
use gl::Gl;
use lazy_static::lazy_static;
use render_gl::objects::BufferObject;
use render_thread::{RenderState, RenderStateEvent};
use resource_manager::ResourceManager;
use std::{
    collections::HashMap,
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};
use update_thread::{GameState, GameStateEvent};

mod entity;
mod events;
mod interfaces;
mod render_gl;
mod render_thread;
mod resource_manager;
mod systems;
mod update_thread;
mod utils;

lazy_static! {
    static ref CONFIG: utils::config::GameConfig = utils::config::read_config();
}

pub fn main() {
    ///////// Initialize SDL2 window

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_double_buffer(true);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 6);

    let window = video_subsystem
        .window("Project Gilgamesh v0.1.0", 1920, 1080)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    sdl_context.mouse().set_relative_mouse_mode(true);

    ///////// Initialize OpenGL

    let _image_context = sdl2::image::init(sdl2::image::InitFlag::all());
    let _gl_context = window.gl_create_context().unwrap();
    let gl = gl::Gl::load_with(|s| {
        video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void
    });
    unsafe {
        gl.ClampColor(gl::CLAMP_READ_COLOR, gl::FIXED_ONLY);
    }
    if CONFIG.performance.cap_render_fps {
        let _ = video_subsystem.gl_set_swap_interval(1);
    } else {
        let _ = video_subsystem.gl_set_swap_interval(0);
    }

    ///////// Initialize imGUI

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);
    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    let mut platform = imgui_sdl2_support::SdlPlatform::init(&mut imgui);
    let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, &gl);

    ///////// Initalize game

    let (width, height) = window.size();

    let mut game_state = GameState::new();
    let mut render_state = RenderState::new(&gl, width, height);
    let resource_manager = ResourceManager::new();

    render_state.load_shaders();
    let new_entities = systems::load_entities(&mut game_state);
    systems::unload_entity_models(
        &mut game_state,
        &mut render_state,
        &resource_manager,
        &new_entities,
    );
    systems::load_entity_models(&mut game_state, &resource_manager, &new_entities);

    ///////// Game loop

    let running = Arc::new(AtomicBool::new(true));

    ////// Update thread

    let (render_state_sender, render_state_receiver): (
        Sender<RenderStateEvent>,
        Receiver<RenderStateEvent>,
    ) = channel();
    let (event_sender, event_receiver): (Sender<GameStateEvent>, Receiver<GameStateEvent>) =
        channel();

    update_thread::spawn_update_loop(
        game_state,
        &resource_manager,
        render_state_sender,
        event_receiver,
        &window,
        running.clone(),
    );

    ////// Render thread
    render_state.render_loop(
        &resource_manager,
        render_state_receiver,
        event_sender,
        gl,
        &sdl_context,
        &mut imgui,
        &mut platform,
        &renderer,
        &window,
        running,
    )
}

/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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
extern crate log;
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
use utils::config::WindowMode;

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
    simplelog::TermLogger::init(
        log::LevelFilter::Trace,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();
    info!(
        r#"

                                 @@@@@@@@@@@
                              @@    @@@@@    @@
                           @@@  @@@       @@@  @@@
                          @@ @@ @       @@@@  @@ @@
                         @  @   @     @@    @   @  @
                        @  @  @@     @@      @@  @  @
                       @@ @  @      @@    @@  @@  @ @@
                       @  @  @@@@@@ @@         @  @  @
                       @  @       @@  @@       @  @  @
                       @  @    @@ @            @  @  @
                       @@ @   @@   @@@        @@  @ @@
                        @  @   @             @@  @  @
                         @  @@  @@         @@   @  @
                          @@ @@    @@@@@@@    @@ @@
                           @@@  @@@       @@@  @@@
                              @@    @@@@@    @@
                                 @@@@@@@@@@@


 _____           _                      _____             _
| ____|_ __ ___ | |__  _ __ _   _  ___ | ____|_ __   __ _(_)_ __   ___
|  _| | '_ ` _ \| '_ \| '__| | | |/ _ \|  _| | '_ \ / _` | | '_ \ / _ \
| |___| | | | | | |_) | |  | |_| | (_) | |___| | | | (_| | | | | |  __/
|_____|_| |_| |_|_.__/|_|   \__, |\___/|_____|_| |_|\__, |_|_| |_|\___|
                            |___/                   |___/ v 0.1.0

"#
    );
    info!("Beginning initialization process...");

    ///////// Initialize SDL2 window

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(sdl2::image::InitFlag::all());

    debug!("SDL context created");

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_double_buffer(true);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 6);

    let mut window_builder = video_subsystem.window("Project Gilgamesh v0.1.0", 1920, 1080);
    window_builder.opengl();

    match CONFIG.graphics.fullscreen_mode {
        WindowMode::Windowed => {
            window_builder.position_centered();
        }
        WindowMode::WindowedFullscreen => {
            window_builder.fullscreen_desktop();
        }
        WindowMode::Fullscreen => {
            window_builder.fullscreen();
        }
    }

    let window = window_builder.build().expect("Could not create OS window!");

    sdl_context.mouse().set_relative_mouse_mode(true);

    debug!("SDL window created");

    ///////// Initialize OpenGL

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

    debug!("OpenGL context created and configured");

    ///////// Initialize imGUI

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);
    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    let mut platform = imgui_sdl2_support::SdlPlatform::init(&mut imgui);
    let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, &gl);

    debug!("imGUI context, SDL support, and OpenGL renderer initialized ");

    info!("Game window created!");

    ///////// Initalize game

    let (width, height) = window.size();

    let mut game_state = GameState::new();
    let mut render_state = RenderState::new(&gl, width, height);
    let resource_manager = ResourceManager::new();

    debug!("Initial game state created");

    render_state.load_shaders();

    debug!("Shaders loaded");

    let new_entities = systems::load_entities(&mut game_state);

    debug!("Game world entities constructed");

    systems::unload_entity_models(
        &mut game_state,
        &mut render_state,
        &resource_manager,
        &new_entities,
    );
    systems::load_entity_models(&mut game_state, &resource_manager, &new_entities);

    debug!("3d model loading initiated");

    info!("Initial game state loaded");

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

    info!("Update thread started");

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
    );

    info!("Render thread started");
}

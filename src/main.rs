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
extern crate crossbeam_channel;

use crossbeam_channel::{unbounded, Receiver, Sender};
use entity::EntitySystem;
use gl::Gl;
use lazy_static::lazy_static;
use render_gl::objects::BufferObject;
use render_thread::{RenderWorldState, RendererState};
use resource_manager::ResourceManager;
use sdl2::video::GLContext;
use std::{
    collections::HashMap,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc, Condvar, Mutex},
};
use update_thread::{GameState, GameStateEvent};
use utils::config::WindowMode;

use crate::dead_drop::DeadDrop;

mod dead_drop;
mod entity;
mod events;
mod render_gl;
mod render_thread;
mod resource_manager;
mod systems;
mod update_thread;
mod utils;

lazy_static! {
    static ref CONFIG: utils::config::GameConfig = utils::config::read_config();
}

struct ShareablePtr<T>(*mut T);
unsafe impl<T> Sync for ShareablePtr<T> {}
unsafe impl<T> Send for ShareablePtr<T> {}

struct SendableGl(Gl);
unsafe impl Send for SendableGl {}

impl Deref for SendableGl {
    type Target = Gl;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
    let gl = SendableGl(gl::Gl::load_with(|s| {
        video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void
    }));
    unsafe {
        gl.ClampColor(gl::CLAMP_READ_COLOR, gl::FIXED_ONLY);
    }
    if CONFIG.performance.cap_render_fps {
        let _ = video_subsystem.gl_set_swap_interval(1);
    } else {
        let _ = video_subsystem.gl_set_swap_interval(0);
    }

    debug!("OpenGL context created and configured");

    info!("Game window created!");

    ///////// Initalize game

    let (width, height) = window.size();

    let resource_manager = ResourceManager::new();

    ///////// Game loop

    let running = Arc::new(AtomicBool::new(true));

    ////// Update thread

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let render_state_dead_drop = DeadDrop::default();
    let (event_sender, event_receiver): (Sender<GameStateEvent>, Receiver<GameStateEvent>) =
        unbounded();

    {
        let render_state_dead_drop = render_state_dead_drop.clone();
        let resource_manager = resource_manager.clone();
        let running = running.clone();
        let event_receiver = event_receiver.clone();
        std::thread::Builder::new()
            .name("update".to_string())
            .spawn(move || {
                let res =
                    core_affinity::get_core_ids().map(|ids| core_affinity::set_for_current(ids[0]));
                if res.is_some_and(|r| r) {
                    let mut game_state = GameState::new(resource_manager);
                    game_state.load_initial_entities();
                    info!("Update thread started");
                    game_state.update_loop(
                        render_state_dead_drop,
                        event_receiver,
                        (width, height),
                        running.clone(),
                    );
                }
            });
    }

    ////// Render thread

    // Now we need to transfer the window's GL context to the render thread, to
    // free us up to focus on just the window itself and render on a different
    // thread, which involves... unsafe shenanigans.
    //
    // See https://github.com/vheuken/SDL-Render-Thread-Example/blob/master/main.cpp for a worked example of what I'm trying to do.
    unsafe {
        sdl2::sys::SDL_GL_MakeCurrent(
            window.raw(),
            std::ptr::null::<sdl2::sys::SDL_GLContext>() as *mut std::ffi::c_void,
        );
    }

    let safe_to_continue = Arc::new(Mutex::new(()));
    let shareable_window = ShareablePtr(window.raw());
    let shareable_gl_context;
    unsafe {
        shareable_gl_context = ShareablePtr(_gl_context.raw());
    }

    {
        let renderer_set_up = safe_to_continue.clone();
        let running = running.clone();
        let event_sender = event_sender.clone();
        std::thread::Builder::new()
            .name("render".to_string())
            .spawn(move || {
                let res =
                    core_affinity::get_core_ids().map(|ids| core_affinity::set_for_current(ids[1]));
                if res.is_some_and(|r| r) {
                    let window = shareable_window;
                    let window = window.0;
                    {
                        renderer_set_up.lock();
                        unsafe {
                            let gl_context = shareable_gl_context;
                            let gl_context = gl_context.0;
                            sdl2::sys::SDL_GL_MakeCurrent(
                                window as *mut sdl2::sys::SDL_Window,
                                gl_context as *mut std::ffi::c_void,
                            );
                        }
                    }
                    let mut renderer_state =
                        RendererState::new(gl, resource_manager.clone(), width, height);
                    renderer_state.load_shaders();
                    debug!("Render thread started");
                    renderer_state.render_loop(
                        render_state_dead_drop,
                        event_sender,
                        // NOTE: We want to do this with a callback so that the rest
                        // of the render thread has no access to the window
                        // pointer. This has to be done on the thread where the
                        // GL context is current, because despite taking a
                        // window pointer, this only really talks to the GL
                        // driver, to tell it to swap buffers in FB0
                        move || unsafe {
                            sdl2::sys::SDL_GL_SwapWindow(window);
                        },
                        running,
                    );
                }
            });
    }

    // Only continue when the other thread is done making these consistent.
    //
    safe_to_continue.lock();
    debug!("Event loop thread started");

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mouse_util = sdl_context.mouse();

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        if let Some(event) = event_pump.wait_event_timeout(7) {
            match event {
                sdl2::event::Event::KeyDown {
                    scancode: Some(sdl2::keyboard::Scancode::Escape),
                    ..
                } => {
                    mouse_util.set_relative_mouse_mode(!mouse_util.relative_mouse_mode());
                }
                sdl2::event::Event::Quit { timestamp } => {
                    running.store(false, std::sync::atomic::Ordering::SeqCst);
                }
                _ => {
                    let etype = if event.is_keyboard() {
                        "Keyboard"
                    } else if event.is_mouse() {
                        "Mouse"
                    } else if event.is_window() {
                        "Window"
                    } else {
                        "Other"
                    };
                    trace!("Sending {etype} event to update thread");
                    let _ = event_sender.send(GameStateEvent::SDLEvent(event)).unwrap();
                }
            }
        }

        if mouse_util.relative_mouse_mode() {
            event_sender
                .send(GameStateEvent::FrameEvent(
                    event_pump.keyboard_state().scancodes().collect(),
                    event_pump.relative_mouse_state(),
                ))
                .unwrap();
        }
    }
}

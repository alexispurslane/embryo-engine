extern crate gl;
extern crate glam;
extern crate image;
extern crate rand;
extern crate rayon;
extern crate russimp;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;

use entity::EntitySystem;
use scene::*;
use std::io::{stdout, Write};

mod entity;
mod events;
mod render_gl;
mod scene;
mod systems;
mod utils;

const NUM_INSTANCES: i32 = 100;
const UPDATE_INTERVAL: u128 = 16;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 3);

    let window = video_subsystem
        .window("Project Gilgamesh v0.1.0", 1920, 1080)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    sdl_context.mouse().set_relative_mouse_mode(true);

    let _gl_context = window.gl_create_context().unwrap();
    let _gl =
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    utils::setup_viewport(window.size());

    let mut scene = Scene {
        camera: None,
        command_queue: vec![],
        running: true,
        entities: EntitySystem::new(),
        shader_programs: vec![],
    };

    systems::add_camera(&mut scene);
    systems::add_level(&mut scene);
    systems::setup_mesh_components(&mut scene.entities);

    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;
    let mut lag = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut stdout = stdout();

    while scene.running {
        // Track time
        let time = start_time.elapsed().as_millis();
        dt = time - last_time;
        last_time = time;
        lag += dt;
        print!("\rFPS: {}", 1000.0 / dt as f32);
        stdout.flush().unwrap();

        // Handle player input
        scene.queue_commands(events::handle_window_events(&scene, event_pump.poll_iter()));
        scene.queue_commands(events::handle_keyboard(
            &scene,
            &event_pump.keyboard_state(),
            dt,
        ));
        scene.queue_commands(events::handle_mouse(
            &scene,
            &event_pump.relative_mouse_state(),
        ));
        scene.update(dt as f32);

        while lag >= UPDATE_INTERVAL {
            scene.queue_commands(systems::physics(&scene));
            scene.update(UPDATE_INTERVAL as f32);
            lag -= UPDATE_INTERVAL;
        }

        // Render
        utils::clear_screen();
        let (width, height) = window.size();
        systems::render(&scene, width, height);

        window.gl_swap_window();
    }
}

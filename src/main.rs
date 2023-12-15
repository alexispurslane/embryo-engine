extern crate gl;
extern crate glam;
extern crate gltf;
extern crate rayon;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2_support;

use entity::{
    camera_component::CameraComponent, mesh_component::Model,
    transform_component::TransformComponent, EntitySystem,
};
use render_gl::resources::ResourceManager;
use scene::*;
use std::{collections::HashMap, sync::mpsc::channel};

mod entity;
mod events;
mod render_gl;
mod scene;
mod systems;
mod utils;

const NUM_INSTANCES: i32 = 100;
const UPDATE_INTERVAL: u128 = 16;

pub fn main() {
    ///////// Initialize SDL2 window

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_double_buffer(true);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 6);

    let mut window = video_subsystem
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

    utils::setup_viewport(&gl, window.size());

    ///////// Initialize imGUI

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);
    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    let mut platform = imgui_sdl2_support::SdlPlatform::init(&mut imgui);

    let mut renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, &gl);

    ///////// Initalize game

    let mut scene = Scene {
        camera: None,
        command_queue: vec![],
        running: true,
        entities: EntitySystem::new(),
        shader_programs: vec![],
        resource_manager: ResourceManager::new(),
    };

    systems::load_shaders(&gl, &mut scene);
    let new_entities = systems::load_entities(&mut scene);
    systems::unload_entity_models(&mut scene, &new_entities);
    systems::load_entity_models(&mut scene, &new_entities);

    ///////// Game loop

    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;
    let mut lag = 0;
    let mut last_dts: [f32; 2] = [0.0, 0.0];

    let mut event_pump = sdl_context.event_pump().unwrap();

    while scene.running {
        // Track time
        let time = start_time.elapsed().as_millis();

        dt = time - last_time;
        last_time = time;
        lag += dt;
        last_dts[0] = last_dts[1];
        last_dts[1] = dt as f32;

        // Update world
        scene.queue_commands(events::handle_window_events(
            &mut window,
            &mut imgui,
            &mut platform,
            &mut sdl_context.mouse(),
            event_pump.poll_iter(),
        ));
        scene.queue_commands(events::handle_keyboard(
            &scene,
            &event_pump.keyboard_state(),
            dt,
        ));
        scene.queue_commands(events::handle_mouse(
            &scene,
            &event_pump.relative_mouse_state(),
        ));
        scene.update();

        // Update ui
        platform.prepare_frame(&mut imgui, &window, &event_pump);
        let ui = imgui.new_frame();
        ui.window("Performance Stats")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position([1600.0, 20.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let avg_fps =
                    ((1000.0 / last_dts[0]) + (1000.0 / last_dts[1]) + (1000.0 / dt as f32)) / 3.0;
                ui.text(format!("FPS (3 frame running average): {}", avg_fps));
                ui.text(format!("Simulation lag: {}", lag));
                ui.separator();
                let ct = scene
                    .entities
                    .get_component::<TransformComponent>(scene.camera.unwrap())
                    .unwrap();
                ui.text(format!(
                    "Camera direction: [{}de  {}deg {}deg]",
                    ct.transform.rot.x.round(),
                    ct.transform.rot.y.round(),
                    ct.transform.rot.z.round()
                ));
                ui.text(format!(
                    "Entities allocated: {:?}",
                    scene.entities.entity_count
                ));
                ui.text(format!(
                    "Free entities: {:?}",
                    scene.entities.free_entities.len()
                ));
                ui.text(format!(
                    "Models loaded: {:?}",
                    scene.resource_manager.models.len()
                ));
            });

        while lag >= UPDATE_INTERVAL {
            scene.queue_commands(systems::physics(&scene));
            scene.update();
            lag -= UPDATE_INTERVAL;
        }

        // Render world
        utils::clear_screen(&gl);

        systems::integrate_loaded_models(&gl, &mut scene);

        let (width, height) = window.size();
        systems::render(&gl, &mut scene, width, height);

        // Render ui
        renderer.render(&mut imgui);

        // Display
        window.gl_swap_window();
    }
}

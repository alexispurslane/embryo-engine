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
extern crate serde;

use core_affinity::CoreId;
use double_buffer::DoubleBuffer;
use entity::{
    camera_component::CameraComponent, mesh_component::Model,
    transform_component::TransformComponent, EntitySystem,
};
use events::handle_event;
use lazy_static::lazy_static;
use render_gl::resources::ResourceManager;
use scene::*;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

mod entity;
mod events;
mod render_gl;
mod scene;
mod systems;
mod utils;

#[derive(Deserialize)]
pub struct PerfConfig {
    pub update_interval: u64,
    pub cap_update_fps: bool,
    pub cap_render_fps: bool,
}

#[derive(Deserialize)]
pub struct ControlConfig {
    pub mouse_sensitivity: f32,
    pub motion_speed: f32,
}

#[derive(Deserialize)]
pub struct GameConfig {
    performance: PerfConfig,
    controls: ControlConfig,
}

lazy_static! {
    static ref CONFIG: GameConfig = {
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

[controls]
mouse_sensitivity = 1.0
motion_speed = 10.0
"#
                .into();
                file.write(contents.as_bytes()).unwrap();
            }
        }
        toml::from_str(&contents).unwrap()
    };
}

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
    if CONFIG.performance.cap_render_fps {
        video_subsystem.gl_set_swap_interval(1);
    } else {
        video_subsystem.gl_set_swap_interval(0);
    }

    utils::setup_viewport(&gl, window.size());

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

    let mut resource_manager = ResourceManager::new();
    let mut game_state = GameState {
        camera: None,
        command_queue: vec![],
        entities: EntitySystem::new(),
        running: true,
    };
    let mut render_state = RenderState {
        camera: None,
        shader_programs: vec![],
        models: HashMap::new(),
        entity_transforms: Box::new(vec![]),
        entity_generations: HashMap::new(),
    };

    systems::load_shaders(&gl, &mut render_state);
    let new_entities = systems::load_entities(&mut game_state);
    systems::unload_entity_models(&mut game_state, &mut render_state, &new_entities);
    systems::load_entity_models(&mut game_state, &mut resource_manager, &new_entities);

    ///////// Game loop

    let running = Arc::new(AtomicBool::new(true));

    ////// Update thread

    let UPDATE_INTERVAL = CONFIG.performance.update_interval as u128;
    let (width, height) = window.size();
    let (render_state_sender, render_state_receiver): (
        Sender<RenderStateEvent>,
        Receiver<RenderStateEvent>,
    ) = channel();
    let (event_sender, event_receiver): (Sender<Event>, Receiver<Event>) = channel();
    {
        let core_ids = core_affinity::get_core_ids().unwrap();
        let running = running.clone();
        std::thread::spawn(move || {
            let res = core_affinity::set_for_current(core_ids[0]);
            if res {
                let time = std::time::Instant::now();
                let mut last_time = time.elapsed().as_millis();
                let mut dt: u128;
                let mut lag = 0;
                while game_state.running {
                    let current_time = time.elapsed().as_millis();
                    dt = current_time - last_time;
                    lag += dt;
                    last_time = current_time;

                    let total_lag = lag;
                    // Catch up with things that require a maximum step size to be stable
                    while lag > UPDATE_INTERVAL {
                        let delta_time = lag.min(UPDATE_INTERVAL);
                        systems::physics(&mut game_state, delta_time);
                        lag -= UPDATE_INTERVAL;
                    }

                    if total_lag > UPDATE_INTERVAL {
                        // Catch up with events
                        while let Some(event) = event_receiver.try_iter().next() {
                            if let Event::SDLEvent(sdl2::event::Event::Quit { timestamp }) = event {
                                running.store(false, std::sync::atomic::Ordering::SeqCst);
                            } else {
                                events::handle_event(&mut game_state, event, lag);
                            }
                        }
                        let cam = {
                            let camera = game_state.camera.expect("Must have camera");
                            let cc = game_state
                                .entities
                                .get_component::<CameraComponent>(camera)
                                .expect("Camera must still exist and have camera component!");
                            let ct = game_state
                                .entities
                                .get_component::<TransformComponent>(camera)
                                .expect("Camera must still exist and have transform component!");

                            RenderCameraState {
                                view: ct.point_of_view(),
                                proj: cc.project(width, height),
                            }
                        };
                        let _ = render_state_sender.send(RenderStateEvent {
                            camera: Some(cam),
                            entity_generations: game_state
                                .entities
                                .current_entity_generations
                                .clone(),
                            entity_transforms: Box::new(
                                game_state
                                    .entities
                                    .get_component_vec::<TransformComponent>()
                                    .iter()
                                    .map(|opt_tc| opt_tc.as_ref().map(|tc| tc.get_matrix()))
                                    .collect(),
                            ),
                        });
                        if CONFIG.performance.cap_update_fps {
                            let sleep_time = UPDATE_INTERVAL.checked_sub(dt).unwrap_or(0);
                            if sleep_time > 0 {
                                std::thread::sleep(Duration::from_millis(sleep_time as u64));
                            }
                        }
                    }
                }
                running.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        });
    }

    ////// Render thread

    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;
    let mut last_dts: [f32; 2] = [0.0, 0.0];

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mouse_util = sdl_context.mouse();

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // Track time
        let time = start_time.elapsed().as_millis();

        dt = time - last_time;
        last_time = time;
        last_dts[0] = last_dts[1];
        last_dts[1] = dt as f32;

        if let Ok(new_render_state) = render_state_receiver.try_recv() {
            render_state.camera = new_render_state.camera;
            render_state.entity_generations = new_render_state.entity_generations;
            render_state.entity_transforms = new_render_state.entity_transforms;
        }

        for event in event_pump.poll_iter() {
            platform.handle_event(&mut imgui, &event);
            match event {
                sdl2::event::Event::KeyDown {
                    scancode: Some(sdl2::keyboard::Scancode::Escape),
                    ..
                } => {
                    mouse_util.set_relative_mouse_mode(!mouse_util.relative_mouse_mode());
                }
                _ => {
                    let _ = event_sender.send(scene::Event::SDLEvent(event)).unwrap();
                }
            }
        }

        if mouse_util.relative_mouse_mode() {
            event_sender
                .send(Event::FrameEvent(
                    event_pump.keyboard_state().scancodes().collect(),
                    event_pump.relative_mouse_state(),
                ))
                .unwrap();
        }

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
                ui.separator();

                if let Some(cam) = render_state.camera.as_ref() {
                    let rot = cam
                        .view
                        .to_scale_rotation_translation()
                        .1
                        .to_euler(glam::EulerRot::XYZ);
                    ui.text(format!(
                        "Camera direction: [{} {} {}]",
                        (rot.0 * 180.0 / std::f32::consts::PI).round(),
                        (rot.1 * 180.0 / std::f32::consts::PI).round(),
                        (rot.2 * 180.0 / std::f32::consts::PI).round()
                    ));
                }
                ui.text(format!(
                    "Allocated entities: {:?}",
                    render_state.entity_transforms.len()
                ));
                ui.text(format!("Models loaded: {:?}", render_state.models.len()));
            });

        // Render world
        utils::clear_screen(&gl);

        systems::integrate_loaded_models(&gl, &mut resource_manager, &mut render_state);

        let (width, height) = window.size();
        if render_state.camera.is_some() {
            systems::render(&gl, &mut render_state, width, height);
        }

        // Render ui
        renderer.render(&mut imgui);

        // Display
        window.gl_swap_window();
    }
}

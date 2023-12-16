use crate::{interfaces, render_gl::resources::ResourceManager, systems, utils, CONFIG};
use std::{
    collections::HashMap,
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use gl::Gl;

use crate::scene::{self, RenderState, RenderStateEvent};

pub fn renderer(
    mut render_state: RenderState,
    resource_manager: &ResourceManager,

    render_state_receiver: Receiver<RenderStateEvent>,
    event_sender: Sender<scene::Event>,

    gl: Gl,
    sdl_context: &sdl2::Sdl,
    imgui: &mut imgui::Context,
    platform: &mut imgui_sdl2_support::SdlPlatform,
    renderer: &imgui_opengl_renderer::Renderer,
    window: &sdl2::video::Window,

    running: Arc<AtomicBool>,
) {
    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;
    let mut last_dts: [f32; 2] = [0.0, 0.0];
    let mut lag = 0;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // Track time
        let time = start_time.elapsed().as_millis();

        dt = time - last_time;
        last_time = time;
        last_dts[0] = last_dts[1];
        last_dts[1] = dt as f32;
        lag += dt;

        let avg_fps =
            ((1000.0 / last_dts[0]) + (1000.0 / last_dts[1]) + (1000.0 / dt as f32)) / 3.0;
        let mut event_pump = sdl_context.event_pump().unwrap();
        let mouse_util = sdl_context.mouse();
        if let Ok(new_render_state) = render_state_receiver.try_recv() {
            render_state.camera = new_render_state.camera;
            render_state.entity_generations = new_render_state.entity_generations;
            render_state.entity_transforms = new_render_state.entity_transforms;
        }

        if lag > CONFIG.performance.update_interval as u128 {
            for event in event_pump.poll_iter() {
                platform.handle_event(imgui, &event);
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
                    .send(scene::Event::FrameEvent(
                        event_pump.keyboard_state().scancodes().collect(),
                        event_pump.relative_mouse_state(),
                    ))
                    .unwrap();
            }
            lag = 0;
        }

        // Update ui
        platform.prepare_frame(imgui, &window, &event_pump);
        let ui = imgui.new_frame();
        interfaces::performance_stats_window(ui, &render_state, avg_fps);

        // Render world
        utils::clear_screen(&gl);

        systems::integrate_loaded_models(&gl, resource_manager, &mut render_state);

        let (width, height) = window.size();
        if render_state.camera.is_some() {
            systems::render(&gl, &mut render_state, width, height);
        }

        // Render ui
        renderer.render(imgui);

        // Display
        window.gl_swap_window();
    }
}

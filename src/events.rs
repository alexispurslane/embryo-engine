use crate::scene;
use crate::CONFIG;
use scene::{GameState, SceneCommand};
use sdl2::event::Event;
use sdl2::event::EventPollIterator;
use sdl2::keyboard::{KeyboardState, Scancode};
use sdl2::mouse::MouseUtil;
use sdl2::mouse::RelativeMouseState;
use std::sync::mpsc::Sender;

pub fn handle_keyboard(
    game_state: &mut GameState,
    scancodes: Vec<(sdl2::keyboard::Scancode, bool)>,
    dt: u128,
) {
    let camera_movement = scancodes
        .iter()
        .fold(glam::Vec3::ZERO, |cm, (scancode, pressed)| {
            if *pressed {
                match scancode {
                    Scancode::W => cm + glam::Vec3::Z,
                    Scancode::S => cm - glam::Vec3::Z,
                    Scancode::A => cm + glam::Vec3::X,
                    Scancode::D => cm - glam::Vec3::X,
                    Scancode::E => cm + glam::Vec3::Y,
                    Scancode::F => cm - glam::Vec3::Y,
                    _ => cm,
                }
            } else {
                cm
            }
        });

    game_state.move_camera_by_vector(camera_movement, dt);
}

pub fn handle_mouse(game_state: &mut GameState, mouse_state: &RelativeMouseState, dt: u128) {
    let yo = mouse_state.y() as f32;
    let xo = mouse_state.x() as f32;
    game_state.rotate_camera(glam::vec3(yo, -xo, 0.0), dt);
}

pub fn handle_event(game_state: &mut GameState, event: scene::Event, dt: u128) {
    match event {
        scene::Event::FrameEvent(scancodes, mouse_state) => {
            handle_keyboard(game_state, scancodes, dt);
            handle_mouse(game_state, &mouse_state, dt);
        }
        _ => {}
    }
}

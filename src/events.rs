use crate::scene::*;
use sdl2::event::Event;
use sdl2::event::EventPollIterator;
use sdl2::keyboard::{KeyboardState, Scancode};
use sdl2::mouse::MouseState;

use crate::camera::PitchYawRoll;

pub fn handle_keyboard(
    _scene: &Scene,
    keyboard_state: &KeyboardState,
    _dt: u128,
) -> Vec<SceneCommand> {
    let camera_movement =
        keyboard_state
            .pressed_scancodes()
            .fold(glam::Vec3::ZERO, |cm, scancode: Scancode| match scancode {
                Scancode::W => cm + glam::Vec3::X,
                Scancode::S => cm - glam::Vec3::X,
                Scancode::A => cm - glam::Vec3::Y,
                Scancode::D => cm + glam::Vec3::Y,
                Scancode::E => cm - glam::Vec3::Z,
                Scancode::F => cm + glam::Vec3::Z,
                _ => cm,
            });
    vec![SceneCommand::MoveCameraInDirection(camera_movement)]
}

const MOUSE_SENSITIVITY: f32 = 0.1;

pub struct Mouse {
    pub last_x: i32,
    pub last_y: i32,
    pub is_initial_move: bool,
}

pub fn handle_mouse(_scene: &Scene, mo: &mut Mouse, mouse_state: &MouseState) -> Vec<SceneCommand> {
    let (x, y) = (mouse_state.x(), mouse_state.y());
    if mo.is_initial_move {
        mo.last_x = x;
        mo.last_y = y;
        mo.is_initial_move = false;
    }
    let (xoffset, yoffset) = (x - mo.last_x, mo.last_y - y);
    mo.last_x = x;
    mo.last_y = y;

    let xo = xoffset as f32 * MOUSE_SENSITIVITY;
    let yo = yoffset as f32 * MOUSE_SENSITIVITY;
    vec![SceneCommand::RotateCamera(PitchYawRoll::new(yo, xo, 0.0))]
}

pub fn handle_window_events(_scene: &Scene, events: EventPollIterator) -> Vec<SceneCommand> {
    let mut commands = Vec::<SceneCommand>::new();
    for event in events {
        match event {
            Event::Quit { .. } => commands.push(SceneCommand::Exit()),
            _ => {}
        }
    }
    commands
}

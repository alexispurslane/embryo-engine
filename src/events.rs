use crate::scene::*;
use sdl2::event::Event;
use sdl2::event::EventPollIterator;
use sdl2::keyboard::{KeyboardState, Scancode};
use sdl2::mouse::RelativeMouseState;

pub fn handle_keyboard(
    _scene: &Scene,
    keyboard_state: &KeyboardState,
    _dt: u128,
) -> Vec<SceneCommand> {
    let camera_movement =
        keyboard_state
            .pressed_scancodes()
            .fold(glam::Vec3::ZERO, |cm, scancode: Scancode| match scancode {
                Scancode::W => cm + glam::Vec3::Z,
                Scancode::S => cm - glam::Vec3::Z,
                Scancode::A => cm + glam::Vec3::X,
                Scancode::D => cm - glam::Vec3::X,
                Scancode::E => cm - glam::Vec3::Y,
                Scancode::F => cm + glam::Vec3::Y,
                _ => cm,
            });
    vec![SceneCommand::MoveCameraInDirection(camera_movement)]
}

pub fn handle_mouse(_scene: &Scene, mouse_state: &RelativeMouseState) -> Vec<SceneCommand> {
    let yo = mouse_state.y() as f32;
    let xo = mouse_state.x() as f32;
    vec![SceneCommand::RotateCamera(glam::vec3(yo, -xo, 0.0))]
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

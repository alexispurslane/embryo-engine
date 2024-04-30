/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::update_thread::{GameState, GameStateEvent};
use sdl2::keyboard::Scancode;
use sdl2::mouse::RelativeMouseState;

pub fn handle_keyboard(
    game_state: &mut GameState,
    scancodes: Vec<(sdl2::keyboard::Scancode, bool)>,
    dt: f32,
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

pub fn handle_mouse(game_state: &mut GameState, mouse_state: &RelativeMouseState, dt: f32) {
    let yo = mouse_state.y() as f32;
    let xo = mouse_state.x() as f32;
    game_state.rotate_camera(glam::vec3(yo, -xo, 0.0), dt);
}

pub fn handle_event(game_state: &mut GameState, event: GameStateEvent, dt: f32) {
    match event {
        GameStateEvent::FrameEvent(scancodes, mouse_state) => {
            handle_keyboard(game_state, scancodes, dt);
            handle_mouse(game_state, &mouse_state, dt);
        }
        _ => {}
    }
}

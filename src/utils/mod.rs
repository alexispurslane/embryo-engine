/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{cell::RefMut, ffi::CString};

use gl::Gl;
use glam::Vec4Swizzles;

use crate::{
    entity::{
        camera_component::CameraComponent,
        light_component::*,
        transform_component::{self, TransformComponent},
        Entity, EntitySystem,
    },
    render_gl::{
        objects::{Buffer, BufferObject},
        shaders::{self, Program},
    },
    render_thread::{light_component_to_shader_light, RenderCameraState, RenderState, ShaderLight},
    CONFIG,
};

pub type Degrees = f32;
pub type Radians = f32;

pub fn create_whitespace_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(zip!($($y), +))
    )
}
pub use zip;

pub mod config {
    use serde::Deserialize;
    use std::io::prelude::*;
    #[derive(Deserialize)]
    pub struct PerfConfig {
        pub update_interval: usize,
        pub cap_update_fps: bool,
        pub cap_render_fps: bool,
        pub max_batch_size: usize,
        pub max_lights: usize,
        pub max_quadtree_depth: usize,
        pub max_quadtree_entities: usize,
    }

    #[derive(Deserialize)]
    pub struct ControlConfig {
        pub mouse_sensitivity: f32,
        pub motion_speed: f32,
    }

    #[derive(Deserialize)]
    pub enum WindowMode {
        Windowed,
        WindowedFullscreen,
        Fullscreen,
    }

    #[derive(Deserialize)]
    pub struct GraphicsConfig {
        pub min_log_luminence: f32,
        pub max_log_luminence: f32,
        pub auto_exposure_speed_factor: f32,
        pub bloom: bool,
        pub min_bloom_threshold: f32,
        pub max_bloom_threshold: f32,
        pub bloom_factor: f32,
        pub scene_factor: f32,
        pub fullscreen_mode: WindowMode,
        pub fxaa: bool,
        pub window_width: usize,
        pub window_height: usize,
        pub attenuation_cutoff: f32,
    }

    #[derive(Deserialize)]
    pub struct GameConfig {
        pub performance: PerfConfig,
        pub controls: ControlConfig,
        pub graphics: GraphicsConfig,
    }

    pub fn read_config() -> GameConfig {
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
max_batch_size = 1000
max_lights = 32
max_quadtree_depth = 6
max_quadtree_entities = 30

[graphics]
min_log_luminence = -8.0
max_log_luminence = 3.5
auto_exposure_speed_factor = 1.1
bloom = true
min_bloom_threshold = 0.8
max_bloom_threshold = 1.2
bloom_factor = 1.0
scene_factor = 1.0
fxaa = true
fullscreen_mode = "WindowedFullscreen"
window_width = 1920
window_height = 1080
attenuation_cutoff = 51.2

[controls]
mouse_sensitivity = 1.0
motion_speed = 10.0
"#
                .into();
                file.write(contents.as_bytes()).unwrap();
            }
        }
        let config: GameConfig = toml::from_str(&contents).unwrap();
        if config.performance.update_interval > 33
            || config.performance.max_batch_size < 1
            || (config.performance.max_lights > 32 || config.performance.max_lights < 1)
            || config.controls.mouse_sensitivity < 1.0
            || config.controls.motion_speed <= 0.0
            || config.performance.max_quadtree_depth < 4
            || config.performance.max_quadtree_entities < 10
            || config.performance.max_quadtree_entities > 1000
        {
            panic!("Invalid values in config file.");
        }
        config
    }
}

pub mod quadtree {
    use std::collections::VecDeque;

    use glam::Vec2Swizzles;

    use crate::{entity::Entity, CONFIG};

    #[derive(Clone)]
    pub struct QuadtreeEntity {
        entity: Entity,
        upper_left: (usize, usize),
        bb_size: (usize, usize),
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum QuadtreeNodeType {
        Interior,
        Leaf,
    }

    #[derive(Clone)]
    pub struct QuadtreeNode {
        node_type: QuadtreeNodeType,
        /// center relative to parent
        half_size: (usize, usize),
        center: (usize, usize),
        entities: Vec<QuadtreeEntity>,
    }

    impl QuadtreeNode {
        pub fn new(half_size: (usize, usize), center: (usize, usize)) -> Self {
            Self {
                node_type: QuadtreeNodeType::Leaf,
                half_size,
                center,
                entities: Vec::with_capacity(CONFIG.performance.max_quadtree_entities),
            }
        }
    }

    /// clockwise constant size array quadtree
    pub struct Quadtree {
        pub map_width: usize,
        pub map_height: usize,
        pub nodes: Vec<QuadtreeNode>,
    }

    impl Quadtree {
        pub fn new(map_width: usize, map_height: usize) -> Self {
            let size = 4_u32.pow(CONFIG.performance.max_quadtree_depth as u32) as usize;
            let mut tree = Self {
                map_width,
                map_height,
                nodes: Vec::with_capacity(size),
            };
            tree.nodes[0] = QuadtreeNode::new(
                (map_width / 2, map_height / 2),
                (map_width / 2, map_height / 2),
            );

            let mut frontier = VecDeque::from([0]);
            while let Some(node_index) = frontier.pop_front() {
                if let Some(QuadtreeNode {
                    half_size, center, ..
                }) = tree.nodes.get(node_index).cloned()
                // FIXME: this is an inefficient way to get around the borrow checker
                {
                    // Top left
                    tree.nodes.push(QuadtreeNode::new(
                        (half_size.0 / 2, half_size.1 / 2),
                        (center.0 / 2, center.1 + center.1 / 2),
                    ));
                    frontier.push_back(4 * node_index);

                    // Top right
                    tree.nodes.push(QuadtreeNode::new(
                        (half_size.0 / 2, half_size.1 / 2),
                        (center.0 + center.0 / 2, center.1 + center.1 / 2),
                    ));
                    frontier.push_back(4 * node_index + 1);

                    // Bottom right
                    tree.nodes.push(QuadtreeNode::new(
                        (half_size.0 / 2, half_size.1 / 2),
                        (center.0 + center.0 / 2, center.1 / 2),
                    ));
                    frontier.push_back(4 * node_index + 1);

                    // Bottom left
                    tree.nodes.push(QuadtreeNode::new(
                        (half_size.0 / 2, half_size.1 / 2),
                        (center.0 / 2, center.1 / 2),
                    ));
                    frontier.push_back(4 * node_index + 1);
                }
            }
            tree
        }

        pub fn insert(
            &mut self,
            entity @ QuadtreeEntity {
                upper_left: (px, py),
                bb_size: (bbw, bbh),
                ..
            }: QuadtreeEntity,
            start_node: usize,
        ) {
            let mut current_node = start_node;
            while current_node < self.nodes.len() {
                if current_node == self.nodes.len() - 1 {
                    // We've reached capacity, just add the node here. What can ya do.
                    self.nodes[current_node].entities.push(entity);
                    break;
                }

                let QuadtreeNode {
                    node_type,
                    half_size,
                    center,
                    entities,
                } = &self.nodes[current_node];

                if *node_type == QuadtreeNodeType::Interior {
                    // Still need to find a home
                    let (cx, cy) = self.nodes[current_node].center;

                    if px < cx && px + bbw > cx || py < cy && py + bbh > cy {
                        // If we cross either the x or y plan splitting this
                        // quad, then we have to stay at this level
                        self.nodes[current_node].entities.push(entity);
                        break;
                    }

                    if px <= cx && py >= cy {
                        current_node = 4 * current_node;
                    } else if px >= cx && py >= cy {
                        current_node = 4 * current_node + 1;
                    } else if px >= cx && py <= cy {
                        current_node = 4 * current_node + 2;
                    } else if px <= cx && py <= cy {
                        current_node = 4 * current_node + 3;
                    }
                } else {
                    if self.nodes[current_node].entities.len()
                        < CONFIG.performance.max_quadtree_entities
                    {
                        // There aren't many nodes here yet, so no need to split.
                        self.nodes[current_node].entities.push(entity.clone());
                    } else if self.nodes[current_node].entities.len()
                        == CONFIG.performance.max_quadtree_entities
                    {
                        // Break all the entities out to lower nodes
                        self.nodes[current_node].node_type = QuadtreeNodeType::Interior;
                        let entities: Vec<_> = self.nodes[current_node]
                            .entities
                            .drain(0..CONFIG.performance.max_quadtree_entities)
                            .collect();
                        for e in entities {
                            self.insert(e, current_node);
                        }

                        // Continue the search next loop, starting at the same node!
                    }
                }
            }
        }

        pub fn raycast_find_entities(
            &self,
            (p, v): (glam::Vec2, glam::Vec2),
        ) -> Vec<&QuadtreeEntity> {
            let mut entities_acc = vec![];
            let mut frontier = VecDeque::from([0]);
            while let Some(node_index) = frontier.pop_front() {
                if self.nodes[node_index].node_type == QuadtreeNodeType::Interior {
                    let QuadtreeNode {
                        half_size,
                        center: (dx, dy),
                        ref entities,
                        ..
                    } = self.nodes[node_index];

                    let children = [
                        4 * node_index,
                        4 * node_index + 1,
                        4 * node_index + 2,
                        4 * node_index + 3,
                    ];
                    // Next nodes to search

                    // Find intersection with x plane
                    let tx = -(glam::Vec2::Y.dot(p) + dy as f32) / glam::Vec2::Y.dot(v);
                    let intersect_x = p + tx * v;
                    let intersects_x_plane = dx - half_size.1 <= intersect_x.x as usize
                        && intersect_x.x as usize <= dx + half_size.1;

                    // Find intersection with y plane
                    let ty = -(glam::Vec2::X.dot(p) + dx as f32) / glam::Vec2::Y.dot(v);
                    let intersect_y = p + ty * v;
                    let intersects_y_plane =
                        dy <= intersect_y.y as usize && intersect_y.y as usize <= dy + half_size.1;

                    // if the ray goes through me, it's worth exploring my children
                    if intersects_x_plane || intersects_y_plane {
                        entities_acc.extend(entities);
                        frontier.extend(children);
                    } else {
                        // If it doesn't it won't go through any of my chilren either.
                        continue;
                    }
                }
            }
            entities_acc
        }

        pub fn find_likely_collisions(
            &self,
            QuadtreeEntity {
                upper_left: (px, py),
                bb_size: (bbw, bbh),
                ..
            }: QuadtreeEntity,
        ) -> Vec<&QuadtreeEntity> {
            let mut entities = vec![];
            let mut frontier = VecDeque::from([0]);
            while let Some(node_index) = frontier.pop_front() {
                // Still need to find a home
                entities.extend(&self.nodes[node_index].entities);

                if self.nodes[node_index].node_type == QuadtreeNodeType::Interior {
                    let (cx, cy) = self.nodes[node_index].center;
                    let children = [
                        4 * node_index,
                        4 * node_index + 1,
                        4 * node_index + 2,
                        4 * node_index + 3,
                    ];
                    // Next nodes to search
                    if px < cx && py > cy {
                        frontier.push_back(children[0]);

                        if px + bbw > cx {
                            frontier.push_back(children[1]);
                        }
                    } else if px > cx && py > cy {
                        frontier.push_back(children[1]);
                    } else if px > cx && py < cy {
                        frontier.push_back(children[2]);

                        if py + bbh > cy {
                            frontier.push_back(children[1]);
                        }
                    } else if px < cx && py < cy {
                        frontier.push_back(children[3]);

                        if px + bbw > cx {
                            frontier.push_back(children[2]);
                        }

                        if py + bbh > cy {
                            frontier.push_back(children[0]);
                        }
                    }
                }
            }

            entities
        }
    }
}

pub mod primitives {
    use crate::{
        lazy_static,
        render_gl::data::{VertexPos, VertexTex},
    };
    #[rustfmt::skip]
    lazy_static! {
        pub static ref CUBE: Vec<VertexPos> = vec![
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0,-1.0, 1.0].into() },
            VertexPos { pos: [-1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0, 1.0,-1.0].into() },
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0, 1.0,-1.0].into() },
            VertexPos { pos: [1.0,-1.0, 1.0].into() },
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [1.0,-1.0,-1.0].into() },
            VertexPos { pos: [1.0, 1.0,-1.0].into() },
            VertexPos { pos: [1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0, 1.0, 1.0].into() },
            VertexPos { pos: [-1.0, 1.0,-1.0].into() },
            VertexPos { pos: [1.0,-1.0, 1.0].into() },
            VertexPos { pos: [-1.0,-1.0, 1.0].into() },
            VertexPos { pos: [-1.0,-1.0,-1.0].into() },
            VertexPos { pos: [-1.0, 1.0, 1.0].into() },
            VertexPos { pos: [-1.0,-1.0, 1.0].into() },
            VertexPos { pos: [1.0,-1.0, 1.0].into() },
            VertexPos { pos: [1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0,-1.0,-1.0].into() },
            VertexPos { pos: [1.0, 1.0,-1.0].into() },
            VertexPos { pos: [1.0,-1.0,-1.0].into() },
            VertexPos { pos: [1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0,-1.0, 1.0].into() },
            VertexPos { pos: [1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0, 1.0,-1.0].into() },
            VertexPos { pos: [-1.0, 1.0,-1.0].into() },
            VertexPos { pos: [1.0, 1.0, 1.0].into() },
            VertexPos { pos: [-1.0, 1.0,-1.0].into() },
            VertexPos { pos: [-1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0, 1.0, 1.0].into() },
            VertexPos { pos: [-1.0, 1.0, 1.0].into() },
            VertexPos { pos: [1.0,-1.0, 1.0].into() }
        ];
        pub static ref SPHERE: Vec<VertexPos> = vec![
            VertexPos { pos: [0.0, 0.0, -1.0].into() },
            VertexPos { pos: [0.7236073017120361, -0.5257253050804138, -0.44721952080726624].into() },
            VertexPos { pos: [-0.276388019323349, -0.8506492376327515, -0.4472198486328125].into() },
            VertexPos { pos: [-0.8944262266159058, 0.0, -0.44721561670303345].into() },
            VertexPos { pos: [-0.276388019323349, 0.8506492376327515, -0.4472198486328125].into() },
            VertexPos { pos: [0.7236073017120361, 0.5257253050804138, -0.44721952080726624].into() },
            VertexPos { pos: [0.276388019323349, -0.8506492376327515, 0.4472198486328125].into() },
            VertexPos { pos: [-0.7236073017120361, -0.5257253050804138, 0.44721952080726624].into() },
            VertexPos { pos: [-0.7236073017120361, 0.5257253050804138, 0.44721952080726624].into() },
            VertexPos { pos: [0.276388019323349, 0.8506492376327515, 0.4472198486328125].into() },
            VertexPos { pos: [0.8944262266159058, 0.0, 0.44721561670303345].into() },
            VertexPos { pos: [0.0, 0.0, 1.0].into() },
            VertexPos { pos: [-0.16245555877685547, -0.49999526143074036, -0.8506544232368469].into() },
            VertexPos { pos: [0.42532268166542053, -0.30901139974594116, -0.8506541848182678].into() },
            VertexPos { pos: [0.26286882162094116, -0.8090116381645203, -0.5257376432418823].into() },
            VertexPos { pos: [0.8506478667259216, 0.0, -0.5257359147071838].into() },
            VertexPos { pos: [0.42532268166542053, 0.30901139974594116, -0.8506541848182678].into() },
            VertexPos { pos: [-0.525729775428772, 0.0, -0.8506516814231873].into() },
            VertexPos { pos: [-0.6881893873214722, -0.49999693036079407, -0.5257362127304077].into() },
            VertexPos { pos: [-0.16245555877685547, 0.49999526143074036, -0.8506544232368469].into() },
            VertexPos { pos: [-0.6881893873214722, 0.49999693036079407, -0.5257362127304077].into() },
            VertexPos { pos: [0.26286882162094116, 0.8090116381645203, -0.5257376432418823].into() },
            VertexPos { pos: [0.9510578513145447, -0.30901262164115906, 0.0].into() },
            VertexPos { pos: [0.9510578513145447, 0.30901262164115906, 0.0].into() },
            VertexPos { pos: [0.0, -0.9999999403953552, 0.0].into() },
            VertexPos { pos: [0.5877856016159058, -0.8090167045593262, 0.0].into() },
            VertexPos { pos: [-0.9510578513145447, -0.30901262164115906, 0.0].into() },
            VertexPos { pos: [-0.5877856016159058, -0.8090167045593262, 0.0].into() },
            VertexPos { pos: [-0.5877856016159058, 0.8090167045593262, 0.0].into() },
            VertexPos { pos: [-0.9510578513145447, 0.30901262164115906, 0.0].into() },
            VertexPos { pos: [0.5877856016159058, 0.8090167045593262, 0.0].into() },
            VertexPos { pos: [0.0, 0.9999999403953552, 0.0].into() },
            VertexPos { pos: [0.6881893873214722, -0.49999693036079407, 0.5257362127304077].into() },
            VertexPos { pos: [-0.26286882162094116, -0.8090116381645203, 0.5257376432418823].into() },
            VertexPos { pos: [-0.8506478667259216, 0.0, 0.5257359147071838].into() },
            VertexPos { pos: [-0.26286882162094116, 0.8090116381645203, 0.5257376432418823].into() },
            VertexPos { pos: [0.6881893873214722, 0.49999693036079407, 0.5257362127304077].into() },
            VertexPos { pos: [0.16245555877685547, -0.49999526143074036, 0.8506543636322021].into() },
            VertexPos { pos: [0.525729775428772, 0.0, 0.8506516814231873].into() },
            VertexPos { pos: [-0.42532268166542053, -0.30901139974594116, 0.8506541848182678].into() },
            VertexPos { pos: [-0.42532268166542053, 0.30901139974594116, 0.8506541848182678].into() },
            VertexPos { pos: [0.16245555877685547, 0.49999526143074036, 0.8506543636322021].into() },
        ];
        pub static ref QUAD: Vec<VertexPos> = vec![
            VertexPos {pos: [-1.0, 1.0, 0.0].into(),},
            VertexPos {pos: [-1.0, -1.0, 0.0].into(),},
            VertexPos {pos: [1.0, 1.0, 0.0].into(),},
            VertexPos {pos: [1.0, -1.0, 0.0].into(),},
        ];
    }
}

pub mod necronomicon {
    use std::cell::{Ref, RefCell, RefMut};

    pub struct YogSothoth<'a, T> {
        inner: Ref<'a, RefCell<T>>,
    }

    impl<'a, T> YogSothoth<'a, T> {
        pub fn summon_from_the_deeps(inner: Ref<'a, RefCell<T>>) -> Self {
            Self { inner }
        }
        pub fn borrow(&self) -> Ref<'_, T> {
            self.inner.borrow()
        }
        pub fn borrow_mut(&self) -> RefMut<'_, T> {
            self.inner.borrow_mut()
        }
    }

    pub fn fhtengen<T>(x: &RefCell<RefCell<T>>) -> YogSothoth<T> {
        YogSothoth { inner: x.borrow() }
    }
}

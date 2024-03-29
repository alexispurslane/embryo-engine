/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{
    entity::{
        light_component::LightComponent, mesh_component::Model,
        transform_component::TransformComponent, Entity, EntityID,
    },
    interfaces,
    render_gl::{
        data::{Cvec3, InstanceTransformVertex, VertexPos, VertexTex},
        objects::{
            Buffer, BufferObject, FramebufferObject, Renderbuffer, VertexArray, VertexArrayObject,
        },
        shaders::{self, Program},
        textures::{AbstractTexture, DepthComponent24, Texture, TextureParameters, R16F, RGBA16F},
    },
    resource_manager::ResourceManager,
    systems,
    update_thread::GameStateEvent,
    utils::{self, necronomicon},
    CONFIG,
};
use std::{
    any::Any,
    borrow::BorrowMut,
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use bytes::{BufMut, Bytes, BytesMut};
use gl::{Gl, COLOR_ATTACHMENT0};
use glam::Vec4Swizzles;

pub struct RenderStateEvent {
    pub camera: Option<RenderCameraState>,
    pub entity_generations: Option<HashMap<EntityID, usize>>,
    pub entity_transforms: Option<Box<Vec<Option<glam::Mat4>>>>,
    pub lights: Option<Box<Vec<ShaderLight>>>,
}

pub struct RenderCameraState {
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

const DEFAULT_SHADER: usize = 0;
const METAL_REFLECTIVE_SHADER: usize = 1;
const LUMINANCE_SHADER: usize = 2;
const LUMINANCE_SHADER2: usize = 3;
const TONEMAP_SHADER: usize = 4;
const GAMMA_SHADER: usize = 5;
const GAUSSIAN_SHADER: usize = 6;
const BLOOM_SHADER: usize = 7;
const DOF_SHADER: usize = 8;

pub type PipelineFunction =
    dyn FnMut(&mut RenderState, RefMut<FramebufferObject>, RefMut<FramebufferObject>) -> Vec<u32>;
pub type RenderPipeline = Vec<Box<PipelineFunction>>;

const HDR_ATTACHMENT: u32 = 0;
const BRIGHT_PASS_ATTACHMENT: u32 = 1;
const GAUSSIAN_ATTACHMENT: u32 = 2;

pub struct RenderState {
    gl: Gl,

    // Statistical housekeeping
    pub avg_dt: f32,
    pub lag: f32,
    pub viewport_size: (u32, u32),
    pub round_robin_buffer: usize,

    // High level render state received from game state
    pub camera: Option<RenderCameraState>,
    pub models: HashMap<String, Model>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,
    pub lights: Box<Vec<ShaderLight>>,

    // Low level render state that needs to stick around *somewhere*
    pub shader_programs: HashMap<usize, Program>,
    pub lights_ubo: BufferObject<ShaderLight>,
    pub lights_dirty: bool,
    pub quad_vao: VertexArrayObject,
    pub luminance_avg: Texture<R16F>,
    pub luminance_histogram: BufferObject<u32>,
}

impl RenderState {
    fn create_fbo(gl: &Gl, width: u32, height: u32) -> FramebufferObject {
        let mut fbo = FramebufferObject::new(&gl);

        // HDR scene attachment
        fbo.attach(Texture::<RGBA16F>::new_allocated(
            &gl,
            TextureParameters {
                mips: 1,
                color_attachment_point: Some(gl::COLOR_ATTACHMENT0),
                ..Default::default()
            },
            width as usize,
            height as usize,
            1,
        ));

        // Bright pass
        fbo.attach(Texture::<RGBA16F>::new_allocated(
            &gl,
            TextureParameters {
                mips: 1,
                color_attachment_point: Some(gl::COLOR_ATTACHMENT1),
                ..Default::default()
            },
            width as usize,
            height as usize,
            1,
        ));

        // Gaussian blurred
        fbo.attach(Texture::<RGBA16F>::new_allocated(
            &gl,
            TextureParameters {
                mips: 1,
                color_attachment_point: Some(gl::COLOR_ATTACHMENT2),
                ..Default::default()
            },
            width as usize,
            height as usize,
            1,
        ));

        // Depth attachment
        fbo.attach(
            Renderbuffer::<DepthComponent24>::new_with_size_and_attachment(
                &gl,
                width as usize,
                height as usize,
                gl::DEPTH_ATTACHMENT,
            ),
        );

        fbo
    }

    pub fn new(gl: &Gl, width: u32, height: u32) -> Self {
        RenderState {
            gl: gl.clone(),
            avg_dt: 0.0,
            lag: 0.0,
            viewport_size: (width, height),
            round_robin_buffer: 0,
            camera: None,
            shader_programs: HashMap::new(),
            models: HashMap::new(),
            entity_transforms: Box::new(vec![]),
            entity_generations: HashMap::new(),
            lights_ubo: {
                let mut ubo = BufferObject::new_immutable(
                    &gl,
                    gl::UNIFORM_BUFFER,
                    gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
                    CONFIG.performance.max_lights * 3,
                );
                ubo.persistent_map(gl::WRITE_ONLY);
                ubo
            },
            lights_dirty: true,
            lights: Box::new(vec![]),
            quad_vao: {
                let vao = VertexArrayObject::new(&gl);
                vao.bind();
                let vbo = BufferObject::<VertexPos>::new_with_vec(
                    &gl,
                    gl::ARRAY_BUFFER,
                    &utils::primitives::QUAD,
                );
                vbo.bind();
                vbo.setup_vertex_attrib_pointers();
                vao.unbind();
                std::mem::forget(vbo);
                vao
            },
            luminance_avg: Texture::new_allocated(
                &gl,
                TextureParameters {
                    texture_type: gl::TEXTURE_1D,
                    mips: 1,
                    ..Default::default()
                },
                1,
                1,
                1,
            ),
            luminance_histogram: BufferObject::<u32>::new_immutable(
                &gl,
                gl::SHADER_STORAGE_BUFFER,
                0,
                256,
            ),
        }
    }

    fn run_pipeline(
        &mut self,
        gl: &Gl,
        ping: necronomicon::YogSothoth<FramebufferObject>,
        pong: necronomicon::YogSothoth<FramebufferObject>,
        pipeline: &mut RenderPipeline,
    ) {
        let len = pipeline.len();
        let mut attachments: Vec<u32> = vec![];
        for (stage, function) in pipeline.iter_mut().enumerate() {
            let (mut in_fbo, mut out_fbo) = if stage % 2 == 0 {
                (ping.borrow_mut(), pong.borrow_mut())
            } else {
                (pong.borrow_mut(), ping.borrow_mut())
            };

            // If it's the last stage, draw to window framebuffer
            if stage == len - 1 {
                unsafe {
                    gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                }
            } else {
                for attachment in attachments {
                    unsafe {
                        let attachment1 =
                            in_fbo.get_attachment::<Texture<RGBA16F>>(attachment as usize);
                        let attachment2 =
                            out_fbo.get_attachment_mut::<Texture<RGBA16F>>(attachment as usize);
                        self.gl.CopyImageSubData(
                            attachment1.id,
                            gl::TEXTURE_2D,
                            0,
                            0,
                            0,
                            0,
                            attachment2.id,
                            gl::TEXTURE_2D,
                            0,
                            0,
                            0,
                            0,
                            self.viewport_size.0 as gl::types::GLsizei,
                            self.viewport_size.1 as gl::types::GLsizei,
                            1,
                        );
                    }
                }
                /**/
                unsafe {
                    self.gl
                        .NamedFramebufferReadBuffer(in_fbo.id, gl::DEPTH_ATTACHMENT);
                    self.gl
                        .NamedFramebufferDrawBuffer(out_fbo.id, gl::DEPTH_ATTACHMENT);
                    self.gl.BlitNamedFramebuffer(
                        in_fbo.id,
                        out_fbo.id,
                        0,
                        0,
                        self.viewport_size.0 as gl::types::GLsizei,
                        self.viewport_size.1 as gl::types::GLsizei,
                        0,
                        0,
                        self.viewport_size.0 as gl::types::GLsizei,
                        self.viewport_size.1 as gl::types::GLsizei,
                        gl::DEPTH_BUFFER_BIT,
                        gl::NEAREST,
                    );
                }

                out_fbo.bind_to(gl::DRAW_FRAMEBUFFER);
            }

            in_fbo.bind_to(gl::READ_FRAMEBUFFER);
            attachments = function(self, in_fbo, out_fbo);
        }
    }

    pub fn load_shaders(&mut self) {
        // Standard modified Blinn-Phong BRDF light/material shader
        self.shader_programs.insert(
            DEFAULT_SHADER,
            Program::new_with_shader_files(
                &self.gl,
                &[
                    (gl::VERTEX_SHADER, "./data/shaders/camera.vert"),
                    (gl::FRAGMENT_SHADER, "./data/shaders/material.frag"),
                ],
            ),
        );

        // Tonemapping, gamma correction, and auto exposure shader, the final
        // step in the postprocessing pipeline
        self.shader_programs.insert(
            TONEMAP_SHADER,
            Program::new_with_shader_files(
                &self.gl,
                &[
                    (gl::VERTEX_SHADER, "./data/shaders/passthrough.vert"),
                    (gl::FRAGMENT_SHADER, "./data/shaders/hdr.frag"),
                ],
            ),
        );

        // Compute shaders to support the above by calculating luminence
        // histograms and averaging them
        self.shader_programs.insert(
            LUMINANCE_SHADER,
            Program::new_with_shader_files(
                &self.gl,
                &[(gl::COMPUTE_SHADER, "./data/shaders/luminance.comp")],
            ),
        );

        self.shader_programs.insert(
            LUMINANCE_SHADER2,
            Program::new_with_shader_files(
                &self.gl,
                &[(gl::COMPUTE_SHADER, "./data/shaders/average.comp")],
            ),
        );

        // Gaussian filter, supporting bloom shader below
        self.shader_programs.insert(
            GAUSSIAN_SHADER,
            Program::new_with_shader_files(
                &self.gl,
                &[
                    (gl::VERTEX_SHADER, "./data/shaders/passthrough.vert"),
                    (gl::FRAGMENT_SHADER, "./data/shaders/gaussian.frag"),
                ],
            ),
        );

        // Bloom shader
        self.shader_programs.insert(
            BLOOM_SHADER,
            Program::new_with_shader_files(
                &self.gl,
                &[
                    (gl::VERTEX_SHADER, "./data/shaders/passthrough.vert"),
                    (gl::FRAGMENT_SHADER, "./data/shaders/bloom.frag"),
                ],
            ),
        );
    }

    pub fn merge_changes(&mut self, new_render_state: RenderStateEvent) {
        if let Some(new_cam) = new_render_state.camera {
            self.camera = Some(new_cam);
        }
        if let Some(new_gens) = new_render_state.entity_generations {
            self.entity_generations = new_gens;
        }
        if let Some(new_trans) = new_render_state.entity_transforms {
            self.entity_transforms = new_trans;
        }
        if let Some(new_lights) = new_render_state.lights {
            self.lights = new_lights;
            self.lights_dirty = true;
        }
    }

    pub fn get_entity_transform<'a>(
        entity_generations: &'a HashMap<EntityID, usize>,
        entity_transforms: &'a Vec<Option<glam::Mat4>>,
        e: Entity,
    ) -> Option<&'a glam::Mat4> {
        if entity_generations
            .get(&e.id)
            .filter(|gen| **gen == e.generation)
            .is_some()
        {
            entity_transforms.get(e.id).and_then(|x| x.as_ref())
        } else {
            entity_transforms.get(e.id).and_then(|x| x.as_ref())
        }
    }

    pub fn render_loop(
        &mut self,
        resource_manager: &ResourceManager,

        render_state_receiver: Receiver<RenderStateEvent>,
        event_sender: Sender<GameStateEvent>,

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

        let ping = RefCell::new(RefCell::new(Self::create_fbo(
            &gl,
            self.viewport_size.0,
            self.viewport_size.1,
        )));
        let pong = RefCell::new(RefCell::new(Self::create_fbo(
            &gl,
            self.viewport_size.0,
            self.viewport_size.1,
        )));
        let mut pipeline = vec![
            Box::new(Self::render) as Box<PipelineFunction>,
            Box::new(Self::render_gaussian_1) as Box<PipelineFunction>,
            Box::new(Self::render_gaussian_2) as Box<PipelineFunction>,
            Box::new(Self::render_bloom) as Box<PipelineFunction>,
            Box::new(Self::render_hdr) as Box<PipelineFunction>,
        ];

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Track time
            let time = start_time.elapsed().as_millis();

            dt = (time - last_time) as f32;
            last_time = time;
            last_dts[0] = last_dts[1];
            last_dts[1] = dt;
            self.lag += dt;
            self.avg_dt = ((last_dts[0] + last_dts[1] + dt as f32) / 3.0).round();

            let mut event_pump = sdl_context.event_pump().unwrap();
            let mouse_util = sdl_context.mouse();
            if let Ok(new_render_state) = render_state_receiver.try_recv() {
                self.merge_changes(new_render_state);
            }

            if self.lag > CONFIG.performance.update_interval as f32 {
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
                self.lag = 0.0;
            }

            systems::integrate_loaded_models(&gl, resource_manager, self);

            let (ping, pong) = (
                necronomicon::YogSothoth::summon_from_the_deeps(ping.borrow()),
                necronomicon::YogSothoth::summon_from_the_deeps(pong.borrow()),
            );
            self.run_pipeline(&gl, ping, pong, &mut pipeline);

            // Update ui
            platform.prepare_frame(imgui, &window, &event_pump);
            let ui = imgui.new_frame();
            interfaces::performance_stats_window(ui, &self, self.avg_dt);

            // Render ui
            renderer.render(imgui);

            // Swap buffers!
            window.gl_swap_window();
            self.round_robin_buffer = (self.round_robin_buffer + 1) % 3;
        }
    }

    /// Do ONE STEP of the gaussian blur on the brightness pass texture
    pub fn render_gaussian_1(
        &mut self,
        source_fbo: RefMut<FramebufferObject>,
        dest_fbo: RefMut<FramebufferObject>,
    ) -> Vec<u32> {
        let bright_pass =
            source_fbo.get_attachment::<Texture<RGBA16F>>(BRIGHT_PASS_ATTACHMENT as usize);
        dest_fbo.draw_to_buffers(&[gl::COLOR_ATTACHMENT0 + GAUSSIAN_ATTACHMENT]);
        unsafe {
            self.shader_programs[&GAUSSIAN_SHADER].set_used();
            self.gl.BindImageTexture(
                0,
                bright_pass.id,
                0,
                gl::FALSE,
                0,
                gl::READ_ONLY,
                gl::RGBA16F,
            );
            self.quad_vao.bind();
            self.quad_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.quad_vao.unbind();
        }
        vec![GAUSSIAN_ATTACHMENT]
    }

    /// Do ONE STEP of the gaussian blur on the brightness pass texture
    pub fn render_gaussian_2(
        &mut self,
        source_fbo: RefMut<FramebufferObject>,
        dest_fbo: RefMut<FramebufferObject>,
    ) -> Vec<u32> {
        let bright_pass =
            source_fbo.get_attachment::<Texture<RGBA16F>>(GAUSSIAN_ATTACHMENT as usize);
        dest_fbo.draw_to_buffers(&[gl::COLOR_ATTACHMENT0 + GAUSSIAN_ATTACHMENT]);
        unsafe {
            self.shader_programs[&GAUSSIAN_SHADER].set_used();
            self.gl.BindImageTexture(
                0,
                bright_pass.id,
                0,
                gl::FALSE,
                0,
                gl::READ_ONLY,
                gl::RGBA16F,
            );
            self.quad_vao.bind();
            self.quad_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.quad_vao.unbind();
        }
        vec![GAUSSIAN_ATTACHMENT]
    }

    pub fn render_bloom(
        &mut self,
        source_fbo: RefMut<FramebufferObject>,
        dest_fbo: RefMut<FramebufferObject>,
    ) -> Vec<u32> {
        let hdr_image = source_fbo.get_attachment::<Texture<RGBA16F>>(HDR_ATTACHMENT as usize);
        let gaussian_image =
            source_fbo.get_attachment::<Texture<RGBA16F>>(GAUSSIAN_ATTACHMENT as usize);
        dest_fbo.draw_to_buffers(&[gl::COLOR_ATTACHMENT0 + HDR_ATTACHMENT]);
        unsafe {
            self.shader_programs[&BLOOM_SHADER].set_used();

            self.gl
                .BindImageTexture(0, hdr_image.id, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA16F);

            gaussian_image.bind(0);
            self.shader_programs[&BLOOM_SHADER]
                .set_uniform_1ui(&CString::new("blurImage").unwrap(), 0);

            self.shader_programs[&BLOOM_SHADER]
                .set_uniform_1f(&CString::new("sceneFactor").unwrap(), 0.0);
            self.shader_programs[&BLOOM_SHADER]
                .set_uniform_1f(&CString::new("bloomFactor").unwrap(), 0.0);

            self.quad_vao.bind();
            self.quad_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.quad_vao.unbind();
        }
        vec![HDR_ATTACHMENT]
    }

    pub fn render_hdr(
        &mut self,
        source_fbo: RefMut<FramebufferObject>,
        dest_fbo: RefMut<FramebufferObject>,
    ) -> Vec<u32> {
        utils::setup_viewport(&self.gl, self.viewport_size);
        utils::clear_screen(&self.gl);

        let hdr_image = source_fbo.get_attachment::<Texture<RGBA16F>>(HDR_ATTACHMENT as usize);
        let time_coefficient = (1.0
            - (-(1000.0 / self.avg_dt) * CONFIG.graphics.auto_exposure_speed_factor).exp())
        .clamp(0.0, 1.0);
        unsafe {
            self.shader_programs[&LUMINANCE_SHADER].set_used();

            self.shader_programs[&LUMINANCE_SHADER].set_uniform_4f(
                &CString::new("params").unwrap(),
                [
                    CONFIG.graphics.min_log_luminence,
                    1.0 / (CONFIG.graphics.max_log_luminence - CONFIG.graphics.min_log_luminence),
                    self.viewport_size.0 as f32,
                    self.viewport_size.1 as f32,
                ]
                .into(),
            );

            self.gl
                .BindImageTexture(0, hdr_image.id, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA16F);

            self.gl
                .BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, self.luminance_histogram.id);

            self.gl.DispatchCompute(
                self.viewport_size.0.div_ceil(16) as u32,
                self.viewport_size.1.div_ceil(16) as u32,
                1,
            );

            self.shader_programs[&LUMINANCE_SHADER2].set_used();

            self.shader_programs[&LUMINANCE_SHADER2].set_uniform_4f(
                &CString::new("params").unwrap(),
                [
                    CONFIG.graphics.min_log_luminence,
                    CONFIG.graphics.max_log_luminence - CONFIG.graphics.min_log_luminence,
                    time_coefficient,
                    (self.viewport_size.0 * self.viewport_size.1) as f32,
                ]
                .into(),
            );

            self.gl.BindImageTexture(
                0,
                self.luminance_avg.id,
                0,
                gl::FALSE,
                0,
                gl::READ_WRITE,
                gl::R16F,
            );

            self.gl
                .BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, self.luminance_histogram.id);

            self.gl.DispatchCompute(1, 1, 1);

            self.shader_programs[&TONEMAP_SHADER].set_used();
            self.shader_programs[&TONEMAP_SHADER].set_uniform_4f(
                &CString::new("params").unwrap(),
                [4.9, 0.0, 0.0, 0.0].into(),
            );
            self.gl.BindImageTexture(
                0,
                self.luminance_avg.id,
                0,
                gl::FALSE,
                0,
                gl::READ_WRITE,
                gl::R16F,
            );
            self.gl
                .BindImageTexture(1, hdr_image.id, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA16F);

            self.quad_vao.bind();
            self.quad_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.quad_vao.unbind();
        }
        vec![0]
    }

    pub fn render(
        &mut self,
        _source_fbo: RefMut<FramebufferObject>,
        dest_fbo: RefMut<FramebufferObject>,
    ) -> Vec<u32> {
        dest_fbo.draw_to_buffers(&[
            gl::COLOR_ATTACHMENT0 + HDR_ATTACHMENT,
            gl::COLOR_ATTACHMENT0 + BRIGHT_PASS_ATTACHMENT,
        ]);
        if let Some(camera) = self.camera.as_ref() {
            utils::setup_viewport(&self.gl, self.viewport_size);
            utils::clear_screen(&self.gl);

            // In the future, the list of lights will be a constant-size list stored on
            // RenderState and passed to it from game state, but for now...
            let mut last_shader_program_index = 0;
            let mut program = &self.shader_programs[&DEFAULT_SHADER];
            program.set_used();

            program.set_uniform_2f(
                &CString::new("bloomThreshold").unwrap(),
                [
                    CONFIG.graphics.min_bloom_threshold,
                    CONFIG.graphics.max_bloom_threshold,
                ]
                .into(),
            );

            // Prepare the shader's constant uniforms based on the camera and the lights.
            utils::camera_prepare_shader(program, camera);
            utils::lights_prepare_shader(
                &self.gl,
                program,
                &mut self.lights_ubo,
                camera,
                &self.lights,
                self.round_robin_buffer,
            );
            utils::shader_set_lightmask(&program, 0b11111111111111111111111111111111);

            // Loop through each model and render all instances of it, in batches.
            let models = &mut self.models;
            let egen = &self.entity_generations;
            let etrans = &self.entity_transforms;
            for (path, model) in models.iter_mut() {
                if last_shader_program_index != model.shader_program {
                    program = &self.shader_programs[&model.shader_program];
                    last_shader_program_index = model.shader_program;
                    program.set_used();
                    utils::camera_prepare_shader(program, camera);
                    utils::lights_prepare_shader(
                        &self.gl,
                        program,
                        &mut self.lights_ubo,
                        camera,
                        &self.lights,
                        self.round_robin_buffer,
                    );
                }

                if self.lights_dirty {
                    utils::lights_prepare_shader(
                        &self.gl,
                        program,
                        &mut self.lights_ubo,
                        camera,
                        &self.lights,
                        self.round_robin_buffer,
                    );
                    self.lights_dirty = false;
                }

                // Create the list of transforms of all the instances of this model. We
                // will pull from this for all batches
                let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    Self::get_entity_transform(egen, etrans, *entity)
                        .expect("Tried to render model for an entity that either doesn't have a transform component, or has been recycled.")
                })
                .map(|mat| InstanceTransformVertex::new(mat.to_cols_array()))
                .collect::<Vec<InstanceTransformVertex>>();

                // See how many batches we're gonna have to do
                let batches = new_transforms
                    .len()
                    .div_ceil(CONFIG.performance.max_batch_size);
                let mbs = CONFIG.performance.max_batch_size as usize;

                for batch in 0..batches {
                    // Batch starts after the last batch (or at zero for the first)
                    let batch_start = batch as usize * mbs;
                    // And goes until max batch size, or until the end of the list of transforms.
                    let batch_size = mbs.min(new_transforms.len() - batch_start) as usize;
                    // We call recreate with data here instead of just modifying the
                    // existing buffer, so that a new buffer will be created and
                    // attached to contain this data and be referenced by the new draw
                    // calls, and the old buffer can stick around to be referenced by
                    // any old draw calls still in the pipeline. If we didn't do this,
                    // we'd get race conditions. Hopefully the cost of allocating a new
                    // buffer won't be that large, because the OpenGL driver will just
                    // pull an already-allocated but orphaned buffer (from the previous
                    // frame) out of memory and give it to us instead of creating an all
                    // new one. Essentially, this is an n-buffering system, which we
                    // have to do because we are using the same buffer for every batch
                    // and we don't know up front how many batches there'll be, which is
                    // why we can't use a round robin triple buffering system. We could
                    // set up an n-buffering system ourselves but that doesn't seem
                    // worth the trouble.
                    model
                    .ibo
                    .as_mut()
                    .expect(
                        "Model must have an instance buffer object by the time rendering starts.",
                    )
                    .recreate_with_data(
                        &new_transforms[batch_start..batch_start + batch_size],
                        gl::STREAM_DRAW,
                    );

                    // Provisionally, we can just say each object is effected by the first light in the light list.
                    //program.set_uniform_1ui(&CString::new("lightmask").unwrap(), 0b00000001);

                    for mesh in &model.meshes {
                        for mesh in &mesh.primitives {
                            let mesh_gl = mesh.gl_mesh.as_ref().expect(
                                "Model must have OpenGL elements setup before rendering it, baka!",
                            );
                            mesh_gl.vao.bind();

                            let material = &model.materials[mesh.material_index];
                            material.activate(&model, &program);

                            mesh_gl.vao.draw_elements_instanced(
                                gl::TRIANGLES,
                                mesh_gl.ebo.count() as gl::types::GLint,
                                gl::UNSIGNED_INT,
                                0,
                                batch_size as gl::types::GLint,
                                0,
                            );
                            mesh_gl.vao.unbind();
                        }
                    }
                }
            }
        }
        vec![HDR_ATTACHMENT, BRIGHT_PASS_ATTACHMENT]
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShaderLight {
    pub position: Cvec3,
    pub light_type: u32,
    pub direction: Cvec3,
    pub constant_attenuation: f32,
    pub ambient: Cvec3,
    pub linear_attenuation: f32,
    pub color: Cvec3,
    pub quadratic_attenuation: f32,
    pub cutoff: f32,
    pub exponent: f32,
    padding1: f32,
    padding2: f32,
}

pub fn light_component_to_shader_light(
    source: &LightComponent,
    transform: &TransformComponent,
) -> ShaderLight {
    use LightComponent::*;
    match source {
        Ambient { ambient } => ShaderLight {
            light_type: 0,
            ambient: Cvec3::from_glam(*ambient),
            color: Cvec3::zero(),

            position: Cvec3::zero(),
            direction: Cvec3::zero(),

            constant_attenuation: 0.0,
            linear_attenuation: 0.0,
            quadratic_attenuation: 0.0,

            cutoff: 0.0,
            exponent: 0.0,

            padding1: 0.0,
            padding2: 0.0,
        },
        Directional { color, ambient } => ShaderLight {
            light_type: 1,
            ambient: Cvec3::from_glam(*ambient),
            color: Cvec3::from_glam(*color / std::f32::consts::PI),

            position: Cvec3::zero(),
            direction: Cvec3::from_glam(transform.transform.rot * glam::Vec3::Z),

            constant_attenuation: 0.0,
            linear_attenuation: 0.0,
            quadratic_attenuation: 0.0,

            cutoff: 0.0,
            exponent: 0.0,

            padding1: 0.0,
            padding2: 0.0,
        },
        Point {
            color,
            ambient,
            attenuation,
        } => ShaderLight {
            light_type: 2,
            ambient: Cvec3::from_glam(*ambient),
            color: Cvec3::from_glam(*color / std::f32::consts::PI),

            position: Cvec3::from_glam(transform.transform.trans),
            direction: Cvec3::zero(),

            constant_attenuation: attenuation.constant,
            linear_attenuation: attenuation.linear,
            quadratic_attenuation: attenuation.quadratic,

            cutoff: 0.0,
            exponent: 0.0,

            padding1: 0.0,
            padding2: 0.0,
        },
        Spot {
            color,
            ambient,
            cutoff,
            fade_exponent: exponent,
            attenuation,
        } => ShaderLight {
            light_type: 3,
            ambient: Cvec3::from_glam(*ambient),
            color: Cvec3::from_glam(*color / std::f32::consts::PI),

            position: Cvec3::from_glam(transform.transform.trans),
            direction: Cvec3::from_glam(transform.transform.rot * glam::Vec3::Z),

            constant_attenuation: attenuation.constant,
            linear_attenuation: attenuation.linear,
            quadratic_attenuation: attenuation.quadratic,

            cutoff: *cutoff,
            exponent: *exponent,

            padding1: 0.0,
            padding2: 0.0,
        },
    }
}

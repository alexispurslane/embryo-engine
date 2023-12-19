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
    utils, CONFIG,
};
use std::{
    any::Any,
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use bytes::{Bytes, BytesMut};
use gl::Gl;
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
const BLOOM_SHADER: usize = 6;
const DOF_SHADER: usize = 7;

pub struct RenderState {
    gl: Gl,
    pub camera: Option<RenderCameraState>,
    pub models: HashMap<String, Model>,
    pub shader_programs: HashMap<usize, Program>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,
    pub lights_ubo: BufferObject<ShaderLight>,
    pub lights_dirty: bool,
    pub lights: Box<Vec<ShaderLight>>,
    pub hdr_framebuffer: FramebufferObject,
    pub hdr_vbo: BufferObject<VertexPos>,
    pub hdr_vao: VertexArrayObject,
    pub luminance_avg: Texture<R16F>,
    pub luminance_histogram: BufferObject<u32>,
}

impl RenderState {
    pub fn new(gl: &Gl, width: usize, height: usize) -> Self {
        let vbo = BufferObject::<VertexPos>::new_with_vec(
            &gl,
            gl::ARRAY_BUFFER,
            &utils::primitives::QUAD,
        );
        RenderState {
            gl: gl.clone(),
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
            hdr_framebuffer: {
                let mut fbo = FramebufferObject::new(&gl);
                fbo.attach(Texture::<RGBA16F>::new_allocated(
                    &gl,
                    TextureParameters {
                        mips: 1,
                        color_attachment_point: Some(gl::COLOR_ATTACHMENT0),
                        ..Default::default()
                    },
                    width,
                    height,
                    1,
                ));
                fbo.attach(
                    Renderbuffer::<DepthComponent24>::new_with_size_and_attachment(
                        &gl,
                        width,
                        height,
                        gl::DEPTH_ATTACHMENT,
                    ),
                );
                fbo
            },
            hdr_vao: {
                let vao = VertexArrayObject::new(&gl);
                vao.bind();
                vbo.bind();
                vbo.setup_vertex_attrib_pointers();
                vao.unbind();
                vao
            },
            hdr_vbo: vbo,
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

    pub fn load_shaders(&mut self) {
        {
            let vert_shader = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/camera.vert",
                gl::VERTEX_SHADER,
            )
            .map_err(|e| {
                println!("Could not compile vertex shader. Errors:\n{}", e);
                std::process::exit(1);
            })
            .unwrap();

            let frag_shader = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/material.frag",
                gl::FRAGMENT_SHADER,
            )
            .map_err(|e| {
                println!("Could not compile fragment shader. Errors:\n{}", e);
                std::process::exit(1);
            })
            .unwrap();
            self.shader_programs.insert(
                DEFAULT_SHADER,
                Program::from_shaders(&self.gl, &[frag_shader, vert_shader]).unwrap(),
            );
        }

        {
            let vert_shader = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/passthrough.vert",
                gl::VERTEX_SHADER,
            )
            .map_err(|e| {
                println!("Could not compile vertex shader. Errors:\n{}", e);
                std::process::exit(1);
            })
            .unwrap();
            let frag_shader = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/hdr.frag",
                gl::FRAGMENT_SHADER,
            )
            .map_err(|e| {
                println!("Could not compile fragment shader. Errors:\n{}", e);
                std::process::exit(1);
            })
            .unwrap();
            self.shader_programs.insert(
                TONEMAP_SHADER,
                Program::from_shaders(&self.gl, &[vert_shader, frag_shader]).unwrap(),
            );
        }

        {
            let luminance = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/luminance.comp",
                gl::COMPUTE_SHADER,
            )
            .map_err(|e| {
                println!(
                    "Could not compile post-processing compute shader. Errors:\n{}",
                    e
                );
                std::process::exit(1);
            })
            .unwrap();

            let average_luminance = shaders::Shader::from_file(
                &self.gl,
                "./data/shaders/average.comp",
                gl::COMPUTE_SHADER,
            )
            .map_err(|e| {
                println!(
                    "Could not compile post-processing compute shader. Errors:\n{}",
                    e
                );
                std::process::exit(1);
            })
            .unwrap();

            self.shader_programs.insert(
                LUMINANCE_SHADER,
                Program::from_shaders(&self.gl, &[luminance]).unwrap(),
            );

            self.shader_programs.insert(
                LUMINANCE_SHADER2,
                Program::from_shaders(&self.gl, &[average_luminance]).unwrap(),
            );
        }
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
        let mut lag = 0;
        let mut round_robin_buffer = 0;

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Track time
            let time = start_time.elapsed().as_millis();

            dt = time - last_time;
            last_time = time;
            last_dts[0] = last_dts[1];
            last_dts[1] = dt as f32;
            lag += dt;

            let mut event_pump = sdl_context.event_pump().unwrap();
            let mouse_util = sdl_context.mouse();
            if let Ok(new_render_state) = render_state_receiver.try_recv() {
                self.merge_changes(new_render_state);
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
                lag = 0;
            }

            systems::integrate_loaded_models(&gl, resource_manager, self);

            // Render world to HDR framebuffer
            self.hdr_framebuffer.bind_to(gl::FRAMEBUFFER);

            utils::setup_viewport(&gl, window.size());
            utils::clear_screen(&gl);

            self.render(round_robin_buffer);

            round_robin_buffer = (round_robin_buffer + 1) % 3;

            self.hdr_framebuffer.unbind();

            utils::setup_viewport(&gl, window.size());
            utils::clear_screen(&gl);

            let avg_dt = ((last_dts[0] + last_dts[1] + dt as f32) / 3.0).round();
            self.render_hdr(window.size(), avg_dt);

            // Update ui
            platform.prepare_frame(imgui, &window, &event_pump);
            let ui = imgui.new_frame();
            interfaces::performance_stats_window(ui, &self, avg_dt);

            // Render ui
            renderer.render(imgui);

            // Display
            window.gl_swap_window();
        }
    }

    pub fn render_hdr(&mut self, (width, height): (u32, u32), avg_dt: f32) {
        let minLogLum = -8.0f32;
        let maxLogLum = 3.5f32;
        let tau = 1.1f32;
        let timeCoeff = (1.0 - (-(1000.0 / avg_dt) * tau).exp()).clamp(0.0, 1.0);
        unsafe {
            self.shader_programs[&LUMINANCE_SHADER].set_used();

            self.shader_programs[&LUMINANCE_SHADER].set_uniform_4f(
                &CString::new("params").unwrap(),
                [
                    minLogLum,
                    1.0 / (maxLogLum - minLogLum),
                    width as f32,
                    height as f32,
                ]
                .into(),
            );

            let texbuf = self.hdr_framebuffer.get_attachment::<Texture<RGBA16F>>(0);
            self.gl
                .BindImageTexture(0, texbuf.id, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA16F);

            self.gl
                .BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, self.luminance_histogram.id);

            self.gl
                .DispatchCompute(width.div_ceil(16) as u32, height.div_ceil(16) as u32, 1);

            self.shader_programs[&LUMINANCE_SHADER2].set_used();

            self.shader_programs[&LUMINANCE_SHADER2].set_uniform_4f(
                &CString::new("params").unwrap(),
                [
                    minLogLum,
                    maxLogLum - minLogLum,
                    timeCoeff,
                    (width * height) as f32,
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
            let texbuf = self.hdr_framebuffer.get_attachment::<Texture<RGBA16F>>(0);
            self.gl
                .BindImageTexture(1, texbuf.id, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA16F);

            self.hdr_vao.bind();
            self.hdr_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.hdr_vao.unbind();
        }
    }

    pub fn render(&mut self, round_robin_buffer: usize) {
        if let Some(camera) = self.camera.as_ref() {
            // In the future, the list of lights will be a constant-size list stored on
            // RenderState and passed to it from game state, but for now...
            let mut last_shader_program_index = 0;
            let mut program = &self.shader_programs[&DEFAULT_SHADER];
            program.set_used();

            // Prepare the shader's constant uniforms based on the camera and the lights.
            utils::camera_prepare_shader(program, camera);
            utils::lights_prepare_shader(
                &self.gl,
                program,
                &mut self.lights_ubo,
                camera,
                &self.lights,
                round_robin_buffer,
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
                        round_robin_buffer,
                    );
                }

                if self.lights_dirty {
                    utils::lights_prepare_shader(
                        &self.gl,
                        program,
                        &mut self.lights_ubo,
                        camera,
                        &self.lights,
                        round_robin_buffer,
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
            color: Cvec3::from_glam(*color),

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
            color: Cvec3::from_glam(*color),

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
            color: Cvec3::from_glam(*color),

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

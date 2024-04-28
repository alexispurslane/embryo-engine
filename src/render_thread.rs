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
        textures::{
            AbstractTexture, Depth24Stencil8, DepthComponent24, Texture, TextureParameters, R16F,
            RGBA16F,
        },
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

use bytes::{BufMut, Bytes, BytesMut};
use gl::Gl;
use glam::Vec4Swizzles;
use sdl2::event::Event;

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
const LIGHT_SHADER: usize = 9;
const SIMPLE_PROJECT_SHADER: usize = 10;

pub struct RenderState {
    gl: Gl,

    pub viewport_size: (u32, u32),

    pub camera: Option<RenderCameraState>,

    pub models: HashMap<String, Model>,

    pub shader_programs: HashMap<usize, Program>,

    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,

    pub light_ubo: BufferObject<ShaderLight>,
    pub lights: Box<Vec<ShaderLight>>,
    pub light_sphere_vao: VertexArrayObject,

    pub luminance_avg: Texture<R16F>,
    pub luminance_histogram: BufferObject<u32>,

    pub g_buffer: FramebufferObject,

    pub hdr_framebuffer: FramebufferObject,

    pub sdr_vao: VertexArrayObject,
}

impl RenderState {
    pub fn new(gl: &Gl, width: u32, height: u32) -> Self {
        let depthstencil = Texture::<Depth24Stencil8>::new_allocated(
            &gl,
            TextureParameters {
                mips: 1,
                color_attachment_point: Some(gl::DEPTH_STENCIL_ATTACHMENT),
                ..Default::default()
            },
            width as usize,
            height as usize,
            1,
        );
        RenderState {
            gl: gl.clone(),
            viewport_size: (width, height),
            camera: None,
            shader_programs: HashMap::new(),
            models: HashMap::new(),
            entity_transforms: Box::new(vec![]),
            entity_generations: HashMap::new(),
            lights: Box::new(vec![]),
            light_ubo: BufferObject::new(&gl, gl::UNIFORM_BUFFER, gl::STREAM_DRAW, 1),
            g_buffer: {
                let mut fbo = FramebufferObject::new(&gl);
                // (pos_x, pos_y, pos_z, _)
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

                // (norm_x, norm_y, norm_z, _)
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

                // (diff_r, diff_g, diff_b, diff_a)
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

                // (spec_r, spec_g, spec_b, shininess)
                fbo.attach(Texture::<RGBA16F>::new_allocated(
                    &gl,
                    TextureParameters {
                        mips: 1,
                        color_attachment_point: Some(gl::COLOR_ATTACHMENT3),
                        ..Default::default()
                    },
                    width as usize,
                    height as usize,
                    1,
                ));
                // Depth buffer
                fbo.attach(depthstencil.clone());

                fbo
            },
            hdr_framebuffer: {
                let mut fbo = FramebufferObject::new(&gl);

                // Color buffer
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

                // Bright colors (for bloom)
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

                fbo.attach(depthstencil);

                fbo.draw_to_buffers(&[gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1]);

                fbo
            },
            light_sphere_vao: {
                let vao = VertexArrayObject::new(&gl);

                let vbo = BufferObject::<VertexPos>::new_with_vec(
                    &gl,
                    gl::ARRAY_BUFFER,
                    &utils::primitives::CUBE,
                );

                vao.bind();

                vbo.bind();
                vbo.setup_vertex_attrib_pointers();

                vao.unbind();
                std::mem::forget(vbo);

                vao
            },
            sdr_vao: {
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

    pub fn shader(&mut self, shader_name: usize, shaders: &[&'static str]) {
        let shaders = shaders
            .into_iter()
            .map(|file| {
                trace!("Loading shader './data/shaders/{file}'");
                let shader_type = match file.rsplit_once('.').unwrap().1 {
                    "comp" => gl::COMPUTE_SHADER,
                    "frag" => gl::FRAGMENT_SHADER,
                    "vert" => gl::VERTEX_SHADER,
                    e => panic!("Unknown shader extension {e}, I don't know what to do with this."),
                };
                shaders::Shader::from_file(&self.gl, &format!("./data/shaders/{file}"), shader_type)
                    .unwrap_or_else(|e| {
                        error!(
                            "Shader compilation error: could not compile shader '{file}', got errors:\n{e}"
                        );
                        std::process::exit(1);
                    })
            })
            .collect::<Vec<_>>();
        trace!(
            "Calling shader program link function with {} shaders",
            shaders.len()
        );
        self.shader_programs.insert(
            shader_name,
            Program::from_shaders(&self.gl, &shaders).unwrap(),
        );
    }

    pub fn load_shaders(&mut self) {
        self.shader(DEFAULT_SHADER, &["camera.vert", "material.frag"]);
        self.shader(LIGHT_SHADER, &["light_camera.vert", "light.frag"]);
        self.shader(SIMPLE_PROJECT_SHADER, &["light_camera.vert"]);
        self.shader(TONEMAP_SHADER, &["passthrough.vert", "hdr.frag"]);
        self.shader(LUMINANCE_SHADER, &["luminance.comp"]);
        self.shader(LUMINANCE_SHADER2, &["average.comp"]);
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

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Track time
            let time = start_time.elapsed().as_millis();

            dt = time - last_time;
            last_time = time;
            last_dts[0] = last_dts[1];
            last_dts[1] = dt as f32;

            let mut event_pump = sdl_context.event_pump().unwrap();
            let mouse_util = sdl_context.mouse();
            if let Ok(new_render_state) = render_state_receiver.try_recv() {
                trace!("Receieved new render state, merging in new changes");
                self.merge_changes(new_render_state);
            }

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
                        let etype = if event.is_keyboard() {
                            "Keyboard"
                        } else if event.is_mouse() {
                            "Mouse"
                        } else if event.is_window() {
                            "Window"
                        } else {
                            "Other"
                        };
                        trace!("Sending {etype} event to update thread");
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

            systems::integrate_loaded_models(&gl, resource_manager, self);

            // Render world to gbuffer
            self.render_to_g();

            // render to hdr buffer using light sources
            self.render_g_to_hdr();

            // Render HDR buffer to screen with tone mapping, gamma correction, and auto exposure
            let avg_dt = ((last_dts[0] + last_dts[1] + dt as f32) / 3.0).round();
            self.render_hdr_to_sdr(avg_dt, dt as f32);

            // Update ui
            platform.prepare_frame(imgui, &window, &event_pump);
            let ui = imgui.new_frame();
            interfaces::performance_stats_window(ui, &self, avg_dt);

            // Render ui
            renderer.render(imgui);

            // Swap buffers!
            window.gl_swap_window();
        }
    }

    pub fn render_to_g(&mut self) {
        if let Some(camera) = self.camera.as_ref() {
            self.g_buffer.bind_to(gl::DRAW_FRAMEBUFFER);
            self.g_buffer.draw_to_buffers(&[
                gl::COLOR_ATTACHMENT0,
                gl::COLOR_ATTACHMENT1,
                gl::COLOR_ATTACHMENT2,
                gl::COLOR_ATTACHMENT3,
            ]);

            setup_viewport(&self.gl, self.viewport_size);
            unsafe {
                self.gl.DepthMask(gl::TRUE);
                self.gl.Enable(gl::DEPTH_TEST);
                self.gl.Enable(gl::CULL_FACE);
            }
            clear_screen(&self.gl);

            let program = &self.shader_programs[&DEFAULT_SHADER];
            program.set_used();

            // Prepare the shader's constant uniforms based on the camera and the lights.
            camera_prepare_shader(program, camera);

            // Loop through each model and render all instances of it, in batches.
            let models = &mut self.models;
            let egen = &self.entity_generations;
            let etrans = &self.entity_transforms;
            for (path, model) in models.iter_mut() {
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
        unsafe {
            self.gl.DepthMask(gl::FALSE);
            self.gl.Disable(gl::DEPTH_TEST);
        }
        self.g_buffer.unbind();
    }

    pub fn render_g_to_hdr(&mut self) {
        if let Some(camera) = self.camera.as_ref() {
            self.hdr_framebuffer.bind_to(gl::DRAW_FRAMEBUFFER);
            unsafe {
                self.gl.Clear(gl::COLOR_BUFFER_BIT);
            }

            setup_viewport(&self.gl, self.viewport_size);

            for light in self.lights.iter() {
                let light_model_matrix = {
                    let brightest_color = [light.color.d0, light.color.d1, light.color.d2]
                        .into_iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);

                    let radius = (-light.linear_attenuation
                        + (light.linear_attenuation.powi(2)
                            - 4.0
                                * light.quadratic_attenuation
                                * (light.constant_attenuation
                                    - brightest_color * CONFIG.graphics.attenuation_cutoff))
                            .sqrt())
                        / (2.0 * light.quadratic_attenuation);
                    glam::Mat4::from_scale_rotation_translation(
                        glam::vec3(radius, radius, radius),
                        glam::Quat::IDENTITY,
                        glam::vec3(light.position.d0, light.position.d1, light.position.d2),
                    )
                };

                let program = &self.shader_programs[&SIMPLE_PROJECT_SHADER];
                program.set_used();
                camera_prepare_shader(&program, camera);

                program.set_uniform_matrix_4fv(
                    &CString::new("model_matrix").unwrap(),
                    &light_model_matrix.to_cols_array(),
                );

                // Set up stencil buffer for this light so we don't overdraw
                self.g_buffer.bind_to(gl::DRAW_FRAMEBUFFER);
                unsafe {
                    self.gl.Enable(gl::STENCIL_TEST);
                    self.gl.DrawBuffer(gl::NONE);
                    self.gl.Enable(gl::DEPTH_TEST);
                    self.gl.Disable(gl::CULL_FACE);
                    self.gl.Clear(gl::STENCIL_BUFFER_BIT);

                    self.gl.StencilFunc(gl::ALWAYS, 0, 0);

                    self.gl
                        .StencilOpSeparate(gl::BACK, gl::KEEP, gl::INCR_WRAP, gl::KEEP);
                    self.gl
                        .StencilOpSeparate(gl::FRONT, gl::KEEP, gl::DECR_WRAP, gl::KEEP);
                }

                self.light_sphere_vao.bind();
                self.light_sphere_vao.draw_arrays_instanced(
                    gl::TRIANGLES,
                    0,
                    utils::primitives::SPHERE.len() as gl::types::GLint,
                    1,
                );
                self.light_sphere_vao.unbind();

                // Draw light bounding volumes
                //
                self.hdr_framebuffer.bind_to(gl::DRAW_FRAMEBUFFER);

                let program = &self.shader_programs[&LIGHT_SHADER];
                program.set_used();
                camera_prepare_shader(&program, camera);

                program.set_uniform_3f(
                    &CString::new("cameraDirection").unwrap(),
                    (camera.view * glam::Vec4::Z).xyz().to_array().into(),
                );

                program.set_uniform_matrix_4fv(
                    &CString::new("model_matrix").unwrap(),
                    &light_model_matrix.to_cols_array(),
                );

                unsafe {
                    self.gl.Enable(gl::STENCIL_TEST);
                    self.gl.StencilFunc(gl::NOTEQUAL, 0, 0xFF);
                    self.gl.Disable(gl::DEPTH_TEST);
                    self.gl.Enable(gl::BLEND);
                    self.gl.BlendEquation(gl::FUNC_ADD);
                    self.gl.BlendFunc(gl::ONE, gl::ONE);
                    self.gl.Enable(gl::CULL_FACE);
                    self.gl.CullFace(gl::FRONT);

                    for i in 0..=3 {
                        self.gl.BindImageTexture(
                            i,
                            self.g_buffer
                                .get_attachment::<Texture<RGBA16F>>(i as usize)
                                .id,
                            0,
                            gl::FALSE,
                            0,
                            gl::READ_ONLY,
                            gl::RGBA16F,
                        );
                    }

                    self.light_ubo
                        .recreate_with_data(std::slice::from_ref(light), gl::STREAM_DRAW);
                    self.gl
                        .BindBufferBase(gl::UNIFORM_BUFFER, 4, self.light_ubo.id)
                }

                unsafe {
                    self.gl.UniformSubroutinesuiv(
                        gl::FRAGMENT_SHADER,
                        1,
                        &[light.light_type] as *const gl::types::GLuint,
                    );
                }

                self.light_sphere_vao.bind();
                self.light_sphere_vao.draw_arrays_instanced(
                    gl::TRIANGLES,
                    0,
                    utils::primitives::SPHERE.len() as gl::types::GLint,
                    1,
                );
                self.light_sphere_vao.unbind();

                unsafe {
                    self.gl.CullFace(gl::BACK);
                    self.gl.Disable(gl::BLEND);
                }
            }

            self.hdr_framebuffer.unbind();
            unsafe {
                self.gl.Disable(gl::STENCIL_TEST);
            }
        }
    }

    pub fn render_hdr_to_sdr(&mut self, avg_dt: f32, lag: f32) {
        setup_viewport(&self.gl, self.viewport_size);
        clear_screen(&self.gl);

        let hdr_image = self
            .hdr_framebuffer
            .get_attachment_mut::<Texture<RGBA16F>>(0);
        let min_log_luminance = -8.0f32;
        let max_log_luminance = 3.5f32;
        let tau = 1.1f32;
        let time_coefficient = (1.0 - (-(1000.0 / avg_dt) * tau).exp()).clamp(0.0, 1.0);
        unsafe {
            self.shader_programs[&LUMINANCE_SHADER].set_used();

            self.shader_programs[&LUMINANCE_SHADER].set_uniform_4f(
                &CString::new("params").unwrap(),
                [
                    min_log_luminance,
                    1.0 / (max_log_luminance - min_log_luminance),
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
                    min_log_luminance,
                    max_log_luminance - min_log_luminance,
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

            self.sdr_vao.bind();
            self.sdr_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            self.sdr_vao.unbind();
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
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

pub fn clear_screen(gl: &Gl) {
    unsafe {
        gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
}

pub fn setup_viewport(gl: &Gl, (w, h): (u32, u32)) {
    unsafe {
        gl.Viewport(0, 0, w as gl::types::GLint, h as gl::types::GLint);
        gl.ClearColor(0.0, 0.0, 0.0, 1.0);
        #[cfg(debug_assertions)]
        gl.Enable(gl::DEBUG_OUTPUT);
    }
}

pub fn camera_prepare_shader(program: &Program, camera: &RenderCameraState) {
    program.set_uniform_matrix_4fv(
        &CString::new("view_matrix").unwrap(),
        &camera.view.to_cols_array(),
    );
    program.set_uniform_matrix_4fv(
        &CString::new("projection_matrix").unwrap(),
        &camera.proj.to_cols_array(),
    );
}

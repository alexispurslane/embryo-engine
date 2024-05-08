use crate::{
    dead_drop::DeadDrop,
    entity::{
        light_component::LightComponent,
        mesh_component::Model,
        transform_component::{Transform, TransformComponent},
        Entity, EntityID,
    },
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
    text::FontRenderer,
    update_thread::GameStateEvent,
    utils, CONFIG,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gl::Gl;
use rayon::{iter::IntoParallelRefIterator, slice::ParallelSliceMut};
use std::{
    any::Any,
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock},
    time::Duration,
};

use crate::SendableGl;
use bytes::{BufMut, Bytes, BytesMut};
use glam::Vec4Swizzles;
use sdl2::{event::Event, video::GLContext};

pub struct RenderWorldState {
    pub active_camera: Option<RenderCameraState>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub lights: Vec<ShaderLight>,
    pub entity_transforms: HashMap<EntityID, glam::Mat4>,
}

#[derive(Clone)]
pub struct RenderCameraState {
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Shaders {
    Default,
    MetalReflective,
    LuminanceFreq,
    LuminanceAvg,
    Tonemap,
    Gamma,
    Gaussian,
    Bloom,
    DepthOfField,
    Light,
    SimpleProject,
    Font,
}

pub struct RendererState {
    gl: Gl,

    pub render_world_state: RenderWorldState,
    pub resource_manager: ResourceManager,

    pub viewport_size: (u32, u32),

    pub models: HashMap<String, Model>,

    pub shader_programs: HashMap<Shaders, Program>,

    pub light_ubo: BufferObject<ShaderLight>,
    pub light_sphere_vao: VertexArrayObject,

    pub luminance_avg: Texture<R16F>,
    pub luminance_histogram: BufferObject<u32>,

    pub g_buffer: FramebufferObject,

    pub hdr_framebuffer: FramebufferObject,

    pub sdr_vao: VertexArrayObject,

    pub ui_font: FontRenderer,
}

impl RendererState {
    pub fn new(gl: SendableGl, resource_manager: ResourceManager, width: u32, height: u32) -> Self {
        let gl = gl.0;
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
        let lib = freetype::Library::init().unwrap();
        RendererState {
            gl: gl.clone(),
            resource_manager,
            render_world_state: RenderWorldState {
                active_camera: None,
                entity_generations: HashMap::new(),
                lights: Vec::new(),
                entity_transforms: HashMap::new(),
            },
            viewport_size: (width, height),
            shader_programs: HashMap::new(),
            models: HashMap::new(),
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
            ui_font: FontRenderer::new("Teko", &gl, lib, 128 as char, (width, height)),

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

    pub fn shader(&mut self, shader_name: Shaders, shaders: &[&'static str]) {
        self.shader_programs.insert(
            shader_name,
            Program::new_with_shader_files(&self.gl, shaders),
        );
    }

    pub fn load_shaders(&mut self) {
        self.shader(Shaders::Default, &["camera.vert", "material.frag"]);
        self.shader(Shaders::SimpleProject, &["light_camera.vert"]);
        self.shader(Shaders::Tonemap, &["passthrough.vert", "hdr.frag"]);
        self.shader(Shaders::LuminanceFreq, &["luminance.comp"]);
        self.shader(Shaders::LuminanceAvg, &["average.comp"]);
        self.shader(Shaders::Light, &["light_camera.vert", "light.frag"]);
    }

    pub fn render_loop(
        &mut self,

        rws_receiver: DeadDrop<RenderWorldState>,
        event_sender: Sender<GameStateEvent>,
        swap_buffers: impl Fn(),

        running: Arc<AtomicBool>,
    ) {
        let start_time = std::time::Instant::now();
        let mut last_time = start_time.elapsed().as_millis();
        let mut dt;
        let mut avg_dt = 0.0;
        let mut avg_fps;

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Track time
            let time = start_time.elapsed().as_millis();

            dt = time - last_time;
            last_time = time;
            avg_dt = (avg_dt + dt as f32) / 2.0;

            avg_fps = 1000.0 / avg_dt;

            if let Some(new_render_state) = rws_receiver.recv() {
                self.render_world_state = new_render_state;
            }

            self.resource_manager
                .try_integrate_loaded_models(&mut self.models, &self.gl);

            // Render world to gbuffer
            self.render_to_g();

            // render to hdr buffer using light sources
            self.render_g_to_hdr();

            // Render HDR buffer to screen with tone mapping, gamma correction, and auto exposure
            self.render_hdr_to_sdr(avg_dt, dt as f32);

            self.ui_font.render_lines(
                format!(
                    "FPS: {:03}\nEntities in worldspace: {}",
                    avg_fps.round(),
                    self.render_world_state.entity_transforms.len()
                ),
                (20.0, 20.0),
                12.0,
                (1.0, 1.0, 1.0),
                18.0,
            );

            swap_buffers();
        }
    }

    /// Render all the models in the world to the G-buffer. This just composits
    /// together all the info the next step needs to actually render a frame.
    pub fn render_to_g(&mut self) {
        if let Some(camera) = self.render_world_state.active_camera.as_ref() {
            // Set up G-buffer for world mesh drawing
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

            // Use the default shader
            let program = &self.shader_programs[&Shaders::Default];
            program.set_used();

            // Prepare the shader's constant uniforms based on the camera and the lights.
            camera_prepare_shader(program, camera);

            // Loop through each model and render all instances of it, in batches.
            let models = &mut self.models;
            let egen = &self.render_world_state.entity_generations;
            let etrans = &self.render_world_state.entity_transforms;
            for (path, model) in models.iter_mut() {
                // Create the list of transforms of all the instances of this model. We
                // will pull from this for all batches
                let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    utils::get_entity_transform(egen, etrans, *entity)
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
                    // Send batch of transforms to the model's instance buffer
                    //
                    // NOTE: We call recreate with data here instead of just modifying the
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

                    // Render each mesh (primitive) in the model using that
                    // instance buffer, so they all get rendered together
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
        // Unset some of the things we won't need later
        unsafe {
            self.gl.DepthMask(gl::FALSE);
            self.gl.Disable(gl::DEPTH_TEST);
        }
        self.g_buffer.unbind();
    }

    /// Render the light volumes onto the HDR buffer using the G-buffer to
    /// determine what the light should actually illuminate. This produces the
    /// actual frame.
    pub fn render_g_to_hdr(&mut self) {
        if let Some(camera) = self.render_world_state.active_camera.as_ref() {
            self.hdr_framebuffer.bind_to(gl::DRAW_FRAMEBUFFER);
            // We don't want to clear the depth buffer because this framebuffer
            // and the g buffer share a depth buffer so that we can use the
            // depth information from the previous step automatically, and we'll
            // be using that information throughout this whole step.
            unsafe {
                self.gl.Clear(gl::COLOR_BUFFER_BIT);
            }

            setup_viewport(&self.gl, self.viewport_size);

            // We have to render each light individually because we need to be
            // able to set a different shader subroutine for each light type to
            // render the light, and we don't have the lights grouped by type,
            // so we can't use instancing. TODO: Actually group lights by type
            // so we can use instancing on lights too
            for light in self.render_world_state.lights.iter() {
                // This information is shared between the stencil and drawing phases
                let light_model_matrix = light.light_volume_model_matrix();

                let program = &self.shader_programs[&Shaders::SimpleProject];
                program.set_used();
                camera_prepare_shader(&program, camera);

                program.set_uniform_matrix_4fv(
                    &CString::new("model_matrix").unwrap(),
                    &light_model_matrix.to_cols_array(),
                );

                // 1. Prepare light stencil buffer
                //
                // Set up stencil buffer for this light so we don't have the
                // light draw things that are in front of or behind its bounding
                // volume as if they are effected by it.
                self.g_buffer.bind_to(gl::DRAW_FRAMEBUFFER);
                unsafe {
                    self.gl.Enable(gl::STENCIL_TEST);
                    self.gl.DrawBuffer(gl::NONE);
                    // We're testing fragment position in space relative to the
                    // camera, so we need depth
                    self.gl.Enable(gl::DEPTH_TEST);
                    // We need to test both the front and back faces of the
                    // light's bounding volume against the depth, so render both
                    // for testing
                    self.gl.Disable(gl::CULL_FACE);
                    self.gl.Clear(gl::STENCIL_BUFFER_BIT);

                    // Don't apply the stencil buffer to our own drawing in the stencil buffer
                    self.gl.StencilFunc(gl::ALWAYS, 0, 0);

                    // If you look at (*), you'll see we'll only be drawing the
                    // light where the stencil buffer is not zero. So:

                    // If the back face bounding volume fragment to be drawn is
                    // behind the object (fails the depth test), increment the
                    // stencil buffer in that area by one, meaning only draw the
                    // light in areas where the object in that area is in front
                    // of the back of the light's bounding volume. However, this
                    // leaves things too close to the camera being effected.
                    // Hence, the next step...
                    self.gl
                        .StencilOpSeparate(gl::BACK, gl::KEEP, gl::INCR_WRAP, gl::KEEP);
                    // Once all the back faces are drawn, for all the front
                    // faces, if that fragment in the front face is behind the
                    // object, decrement the buffer again. For things that were
                    // already past the back side of the volume, this will
                    // return them to zero, excluding things that were past the
                    // back of the volume but are *also* past the front (and
                    // thus not within the light's bounding volume). For things
                    // inside, e.g. against which the back side test fails, but
                    // the front side test succeeds, they are left at one, and
                    // thus, can be drawn.
                    self.gl
                        .StencilOpSeparate(gl::FRONT, gl::KEEP, gl::DECR_WRAP, gl::KEEP);
                }

                // Draw the front and back sides of the bounding volume into the
                // stencil buffer according to the rules above.
                self.light_sphere_vao.bind();
                self.light_sphere_vao.draw_arrays_instanced(
                    gl::TRIANGLES,
                    0,
                    utils::primitives::SPHERE.len() as gl::types::GLint,
                    1,
                );
                self.light_sphere_vao.unbind();

                // 2. Draw light bounding volume
                //
                // Draw the light using the information it covers in the
                // G-buffer to draw the places the light illuminates as effected
                // by the light, and nothing else.
                self.hdr_framebuffer.bind_to(gl::DRAW_FRAMEBUFFER);

                let program = &self.shader_programs[&Shaders::Light];
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
                    // Actually apply stenciling
                    self.gl.Enable(gl::STENCIL_TEST);

                    // Only draw a fragment for this light if the stencil buffer at that fragment is zero
                    self.gl.StencilFunc(gl::NOTEQUAL, 0, 0xFF); // (*)

                    // Only draw the back faces of light bounding volumes, so
                    // the light isn't drawn twice, and is visible while you're
                    // inside it.
                    self.gl.CullFace(gl::FRONT);

                    // Light is additive.
                    self.gl.Enable(gl::BLEND);
                    self.gl.BlendEquation(gl::FUNC_ADD);
                    self.gl.BlendFunc(gl::ONE, gl::ONE);

                    // Fix other settings
                    self.gl.Disable(gl::DEPTH_TEST);
                    self.gl.Enable(gl::CULL_FACE);

                    // Bind each of the G-buffer layers to its respective binding point in the shader
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

                    // Send the light struct using a UBO to the shader
                    self.light_ubo
                        .recreate_with_data(std::slice::from_ref(light), gl::STREAM_DRAW);
                    self.gl
                        .BindBufferBase(gl::UNIFORM_BUFFER, 4, self.light_ubo.id)
                }

                // Select the appropriate shader subroutine for this light
                unsafe {
                    self.gl.UniformSubroutinesuiv(
                        gl::FRAGMENT_SHADER,
                        1,
                        &[light.light_type] as *const gl::types::GLuint,
                    );
                }

                // Render the light!
                self.light_sphere_vao.bind();
                self.light_sphere_vao.draw_arrays_instanced(
                    gl::TRIANGLES,
                    0,
                    utils::primitives::SPHERE.len() as gl::types::GLint,
                    1,
                );
                self.light_sphere_vao.unbind();

                // Prepare for the next iteration
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

    /// Renders the HDR buffer to the standard definition window framebuffer
    /// using tonemapping supplied by the tone mapping shader
    pub fn render_hdr_to_sdr(&mut self, avg_dt: f32, lag: f32) {
        setup_viewport(&self.gl, self.viewport_size);
        clear_screen(&self.gl);

        // We'll be using the HDR image as an input in several places here, so
        // grab it preemptively
        let hdr_image = self
            .hdr_framebuffer
            .get_attachment_mut::<Texture<RGBA16F>>(0);

        let min_log_luminance = -8.0f32;
        let max_log_luminance = 3.5f32;
        let tau = 1.1f32;
        let time_coefficient = (1.0 - (-(1000.0 / avg_dt) * tau).exp()).clamp(0.0, 1.0);

        // First, we need to get the average luminance of the HDR buffer.
        // We'll use two compute shaders for that
        unsafe {
            self.shader_programs[&Shaders::LuminanceFreq].set_used();

            self.shader_programs[&Shaders::LuminanceFreq].set_uniform_4f(
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

            self.shader_programs[&Shaders::LuminanceAvg].set_used();

            self.shader_programs[&Shaders::LuminanceAvg].set_uniform_4f(
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

            self.shader_programs[&Shaders::Tonemap].set_used();
            self.shader_programs[&Shaders::Tonemap].set_uniform_4f(
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
        }
        self.sdr_vao.bind();
        self.sdr_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
        self.sdr_vao.unbind();
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

impl ShaderLight {
    fn light_volume_model_matrix(&self) -> glam::Mat4 {
        let brightest_color = [self.color.d0, self.color.d1, self.color.d2]
            .into_iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let radius = (-self.linear_attenuation
            + (self.linear_attenuation.powi(2)
                - 4.0
                    * self.quadratic_attenuation
                    * (self.constant_attenuation
                        - brightest_color * CONFIG.graphics.attenuation_cutoff))
                .sqrt())
            / (2.0 * self.quadratic_attenuation);
        glam::Mat4::from_scale_rotation_translation(
            glam::vec3(radius, radius, radius),
            glam::Quat::IDENTITY,
            glam::vec3(self.position.d0, self.position.d1, self.position.d2),
        )
    }
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

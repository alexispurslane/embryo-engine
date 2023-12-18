use crate::{
    entity::{
        light_component::LightComponent, mesh_component::Model,
        transform_component::TransformComponent, Entity, EntityID,
    },
    interfaces,
    render_gl::{
        data::{Cvec3, InstanceTransformVertex},
        objects::{Buffer, BufferObject},
        shaders::Program,
    },
    resource_manager::ResourceManager,
    systems,
    update_thread::GameStateEvent,
    utils, CONFIG,
};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

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

pub struct RenderState {
    pub camera: Option<RenderCameraState>,
    pub models: HashMap<String, Model>,
    pub shader_programs: Vec<Program>,
    pub entity_generations: HashMap<EntityID, usize>,
    pub entity_transforms: Box<Vec<Option<glam::Mat4>>>,
    pub lights_ubo: BufferObject<ShaderLight>,
    pub lights_dirty: bool,
    pub lights: Box<Vec<ShaderLight>>,
}

impl RenderState {
    pub fn new(gl: &Gl) -> Self {
        RenderState {
            camera: None,
            shader_programs: vec![],
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
}

pub fn renderer(
    mut render_state: RenderState,
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
            render_state.merge_changes(new_render_state);
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

        // Update ui
        platform.prepare_frame(imgui, &window, &event_pump);
        let ui = imgui.new_frame();
        let avg_dt = ((last_dts[0] + last_dts[1] + dt as f32) / 3.0).round();
        interfaces::performance_stats_window(ui, &render_state, avg_dt);

        // Render world
        utils::clear_screen(&gl);

        systems::integrate_loaded_models(&gl, resource_manager, &mut render_state);

        if render_state.camera.is_some() {
            render(&gl, &mut render_state, round_robin_buffer);
        }
        round_robin_buffer = (round_robin_buffer + 1) % 3;

        // Render ui
        renderer.render(imgui);

        // Display
        window.gl_swap_window();
    }
}

pub fn render(gl: &Gl, render: &mut RenderState, round_robin_buffer: usize) {
    // In the future, the list of lights will be a constant-size list stored on
    // RenderState and passed to it from game state, but for now...
    let mut last_shader_program_index = 0;
    let mut program = &render.shader_programs[0];
    program.set_used();

    let camera = render.camera.as_ref().unwrap();
    // Prepare the shader's constant uniforms based on the camera and the lights.
    utils::camera_prepare_shader(program, camera);
    utils::lights_prepare_shader(
        gl,
        program,
        &mut render.lights_ubo,
        camera,
        &render.lights,
        round_robin_buffer,
    );
    utils::shader_set_lightmask(&program, 0b11111111111111111111111111111111);

    // Loop through each model and render all instances of it, in batches.
    let models = &mut render.models;
    let egens = &render.entity_generations;
    let etrans = &render.entity_transforms;
    for (path, model) in models.iter_mut() {
        if last_shader_program_index != model.shader_program {
            program = &render.shader_programs[model.shader_program];
            last_shader_program_index = model.shader_program;
            program.set_used();
            utils::camera_prepare_shader(program, camera);
            utils::lights_prepare_shader(
                gl,
                program,
                &mut render.lights_ubo,
                camera,
                &render.lights,
                round_robin_buffer,
            );
        }

        if render.lights_dirty {
            utils::lights_prepare_shader(
                gl,
                program,
                &mut render.lights_ubo,
                camera,
                &render.lights,
                round_robin_buffer,
            );
            render.lights_dirty = false;
        }

        // Create the list of transforms of all the instances of this model. We
        // will pull from this for all batches
        let new_transforms = model
                .entities
                .iter()
                .map(|entity| {
                    RenderState::get_entity_transform(egens, etrans, *entity)
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
                .expect("Model must have an instance buffer object by the time rendering starts.")
                .recreate_with_data(
                    &new_transforms[batch_start..batch_start + batch_size],
                    gl::STREAM_DRAW,
                );

            // Provisionally, we can just say each object is effected by the first light in the light list.
            //program.set_uniform_1ui(&CString::new("lightmask").unwrap(), 0b00000001);

            for mesh in &model.meshes {
                for mesh in &mesh.primitives {
                    let mesh_gl = mesh
                        .gl_mesh
                        .as_ref()
                        .expect("Model must have OpenGL elements setup before rendering it, baka!");
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
            exponent,
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

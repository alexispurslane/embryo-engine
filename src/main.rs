extern crate gl;
extern crate glam;
extern crate image;
extern crate rand;
extern crate rayon;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;

use entity::render_component::{self, RenderComponent};
use entity::EntitySystem;
use rand::Rng;
use render_gl::textures;
use std::ffi::CString;
use std::io::{stdout, Write};

mod camera;
mod entity;
mod events;
mod render_gl;
mod scene;
mod utils;
use crate::entity::transform_component::PitchYawRoll;
use camera::*;
use render_gl::data::InstanceTransformVertex;
use render_gl::{objects, shaders};
use scene::*;

const NUM_INSTANCES: i32 = 10;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 3);

    let window = video_subsystem
        .window("Project Gilgamesh v0.1.0", 1024, 768)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    sdl_context.mouse().set_relative_mouse_mode(true);
    utils::setup_viewport(window.size());

    let _gl_context = window.gl_create_context().unwrap();
    let _gl =
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    let mut scene = Scene {
        camera: Box::new(FlyingCamera::new(
            2.5,
            glam::Vec3::Y,
            glam::vec3(0.0, 0.0, 3.0),
            glam::vec3(0.0, 0.0, -1.0),
            PitchYawRoll::new(0.0, -90.0, 0.0),
            50.0,
        )),
        command_queue: vec![],
        running: true,
        entities: EntitySystem::new(),
    };

    add_textured_cube_instances(&mut scene);

    render_component::setup_render_components_system(&mut scene.entities);

    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut stdout = stdout();
    while scene.running {
        // Track time
        let time = start_time.elapsed().as_millis();
        dt = time - last_time;
        last_time = time;
        print!("\rFPS: {}", 1000.0 / dt as f32);
        stdout.flush().unwrap();

        // Handle keyboard and window events
        scene.queue_commands(events::handle_window_events(&scene, event_pump.poll_iter()));

        scene.queue_commands(events::handle_keyboard(
            &scene,
            &event_pump.keyboard_state(),
            dt,
        ));
        scene.queue_commands(events::handle_mouse(
            &scene,
            &event_pump.relative_mouse_state(),
        ));

        scene.update(dt);

        // Render
        utils::clear_screen();

        render_component::render_system(&scene);

        window.gl_swap_window();
    }
}

pub fn add_textured_cube_instances(scene: &mut Scene) {
    // Create box object instances with shaders
    let vert_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.vert")).unwrap(),
        gl::VERTEX_SHADER,
    )
    .unwrap();

    let frag_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.frag")).unwrap(),
        gl::FRAGMENT_SHADER,
    )
    .unwrap();

    let vbo =
        objects::VertexBufferObject::new_with_vec(gl::ARRAY_BUFFER, utils::shapes::unit_cube());

    let mut ibo = objects::VertexBufferObject::<InstanceTransformVertex>::new(gl::ARRAY_BUFFER);

    let mut rng = rand::thread_rng();
    let instance_model_matrices: Vec<InstanceTransformVertex> = (0..NUM_INSTANCES)
        .map(|_| {
            let model = glam::Mat4::from_translation(glam::vec3(
                rng.gen_range::<f32, _>(-5.0..5.0),
                rng.gen_range::<f32, _>(-5.0..5.0),
                rng.gen_range::<f32, _>(-5.0..5.0),
            ));
            InstanceTransformVertex::new(model.to_cols_array())
        })
        .collect();

    let texture1 = textures::get_texture_simple("container.jpg");
    let texture2 = textures::get_texture_simple("awesomeface.png");

    let boxes = scene.entities.new_entity();
    scene.entities.add_component(
        boxes.id,
        RenderComponent::new(
            &[frag_shader, vert_shader],
            Box::new(vbo),
            objects::ElementBufferObject::new(),
            &[
                ("texture1", Box::new(texture1)),
                ("texture2", Box::new(texture2)),
            ],
        ),
    );
}

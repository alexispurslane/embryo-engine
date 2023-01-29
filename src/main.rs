extern crate gl;
extern crate glam;
extern crate image;
extern crate rand;
extern crate rayon;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;

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
    gl_attr.set_context_version(4, 5);

    let window = video_subsystem
        .window("Project Gilgamesh v0.1.0", 1024, 768)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let _gl_context = window.gl_create_context().unwrap();
    let _gl =
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

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

    let shader_program = shaders::Program::from_shaders(&[vert_shader, frag_shader]).unwrap();

    let vbo =
        objects::VertexBufferObject::new_with_vec(gl::ARRAY_BUFFER, utils::shapes::unit_cube());

    let mut ibo = objects::VertexBufferObject::<InstanceTransformVertex>::new(gl::ARRAY_BUFFER);

    let mut rng = rand::thread_rng();
    let instance_model_matrices: Vec<InstanceTransformVertex> = (0..NUM_INSTANCES)
        .map(|_| {
            let model = glam::Mat4::from_translation(glam::vec3(
                rng.gen_range::<f32, _>(-10.0..10.0),
                rng.gen_range::<f32, _>(-10.0..10.0),
                rng.gen_range::<f32, _>(-10.0..10.0),
            ));
            InstanceTransformVertex::new(model.to_cols_array())
        })
        .collect();

    // Craate vertex array object to represent boxes
    let vao = objects::VertexArrayObject::new();
    vao.bind();

    vbo.bind();
    vbo.setup_vertex_attrib_pointers();

    ibo.bind();
    ibo.upload_data(instance_model_matrices, gl::STREAM_DRAW);
    ibo.setup_vertex_attrib_pointers();

    vao.unbind();

    let (width, height, pixels) = utils::load_image_u8("container.jpg");

    let texture1 = textures::Texture::new_with_bytes(
        gl::TEXTURE_2D,
        textures::TextureParameters::default(),
        &pixels,
        width,
        height,
    );

    let (width, height, pixels) = utils::load_image_u8("awesomeface.png");

    let texture2 = textures::Texture::new_with_bytes(
        gl::TEXTURE_2D,
        textures::TextureParameters::default(),
        &pixels,
        width,
        height,
    );

    unsafe {
        gl::Viewport(0, 0, 1024, 768);
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
        gl::Enable(gl::DEPTH_TEST);
    }

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
    };

    let start_time = std::time::Instant::now();
    let mut last_time = start_time.elapsed().as_millis();
    let mut dt;

    let mut mouse = events::Mouse {
        is_initial_move: true,
        last_x: 1024 / 2,
        last_y: 768 / 2,
    };

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut stdout = stdout();
    while scene.running {
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
            &mut mouse,
            &event_pump.mouse_state(),
        ));

        scene.update(dt);

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        // Update box uniforms
        shader_program.set_used();
        shader_program.set_uniform_1i(&CString::new("texture1").unwrap(), 0);
        shader_program.set_uniform_1i(&CString::new("texture1").unwrap(), 1);
        shader_program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &scene.camera.view().to_cols_array(),
        );
        shader_program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &scene.camera.project(1024, 768).to_cols_array(),
        );

        // Render boxes
        vao.bind();
        texture1.bind_to_texture_unit(gl::TEXTURE0);
        texture2.bind_to_texture_unit(gl::TEXTURE1);

        vao.draw_arrays_instanced(gl::TRIANGLES, 0, 36, NUM_INSTANCES);
        vao.unbind();

        window.gl_swap_window();
    }
}

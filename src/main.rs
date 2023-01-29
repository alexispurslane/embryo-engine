extern crate gl;
extern crate glam;
extern crate image;
extern crate sdl2;
#[macro_use]
extern crate project_gilgamesh_render_gl_derive as render_gl_derive;

use render_gl::textures;
use sdl2::event::Event;
use std::ffi::CString;

mod render_gl;
mod utils;
use render_gl::data::{Cvec4, InstanceLocationVertex, InstanceTransformVertex, VertexRGBTex};
use render_gl::{objects, shaders};

const NUM_INSTANCES: i32 = 100;

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

    let vbo = objects::VertexBufferObject::new_with_vec(
        gl::ARRAY_BUFFER,
        vec![
            VertexRGBTex {
                pos: (0.5, 0.5, 0.0).into(),
                clr: (1.0, 0.0, 0.0).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexRGBTex {
                pos: (0.5, -0.5, 0.0).into(),
                clr: (0.0, 1.0, 0.0).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexRGBTex {
                pos: (-0.5, -0.5, 0.0).into(),
                clr: (0.0, 0.0, 1.0).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexRGBTex {
                pos: (-0.5, 0.5, 0.0).into(),
                clr: (1.0, 1.0, 0.0).into(),
                tex: (0.0, 1.0).into(),
            },
        ],
    );
    let positions: Vec<Cvec4> = (0..NUM_INSTANCES)
        .map(|i| {
            (
                (2.0 / 10.0) * (i as f32 % 10.0) - 1.0,
                (2.0 / 10.0) * (i as f32 / 10.0).floor() - 1.0,
                0.0,
                0.0,
            )
                .into()
        })
        .collect();
    let mut ibo = objects::VertexBufferObject::<InstanceTransformVertex>::new(gl::ARRAY_BUFFER);

    let ebo = objects::ElementBufferObject::new_with_vec(vec![0, 1, 3, 1, 2, 3]);

    let vao = objects::VertexArrayObject::new();
    vao.bind();
    ebo.bind();

    vbo.bind();
    vbo.setup_vertex_attrib_pointers();

    ibo.bind();
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
    }

    let start_time = std::time::Instant::now();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                _ => {}
            }
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        shader_program.set_used();
        shader_program.set_uniform_1i(&CString::new("texture1").unwrap(), 0);
        shader_program.set_uniform_1i(&CString::new("texture1").unwrap(), 1);

        vao.bind();
        texture1.bind_to_texture_unit(gl::TEXTURE0);
        texture2.bind_to_texture_unit(gl::TEXTURE1);

        let time = start_time.elapsed().as_millis();

        let instance_matrices: Vec<InstanceTransformVertex> = positions
            .iter()
            .enumerate()
            .map(|(i, position)| {
                let mut trans = glam::Mat4::IDENTITY;
                trans *= glam::Mat4::from_rotation_z((time as f32 / 100.0).to_radians());
                let scalef = ((time as f32 / (i as f32 * 10.0)).sin() + 1.1) * 0.3;
                trans *= glam::Mat4::from_scale(glam::vec3(scalef, scalef, scalef));
                let instance_trans =
                    glam::Mat4::from_translation(glam::vec3(position.d0, position.d1, position.d2))
                        * trans;
                InstanceTransformVertex::new(instance_trans.to_cols_array())
            })
            .collect();

        ibo.upload_data(instance_matrices, gl::STREAM_DRAW);

        vao.draw_elements_instanced(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0, NUM_INSTANCES);
        vao.unbind();

        window.gl_swap_window();
    }
}

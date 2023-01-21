extern crate gl;
extern crate sdl2;

use sdl2::event::Event;
use std::ffi::CString;

mod utils;
use utils::*;

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

    let vert_shader =
        shaders::Shader::from_vert_source(&CString::new(include_str!("triangle.vert")).unwrap())
            .unwrap();

    let frag_shader =
        shaders::Shader::from_frag_source(&CString::new(include_str!("triangle.frag")).unwrap())
            .unwrap();

    let shader_program = render_gl::Program::from_shaders(&[vert_shader, frag_shader]).unwrap();

    let vertices: Vec<f32> = vec![
        0.5, 0.5, 0.0, // top right
        0.5, -0.5, 0.0, // bottom right
        -0.5, -0.5, 0.0, // bottom left
        -0.5, 0.5, 0.0, // top left
    ];

    let indices: Vec<i32> = vec![
        0, 1, 3, // triangle 1
        1, 2, 3, // triangle 2
    ];

    let mut vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
    }

    let mut ebo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut ebo);
    }

    let mut vao: gl::types::GLuint = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
    }

    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertices.as_ptr() as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            indices.as_ptr() as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            (3 * std::mem::size_of::<i32>()) as gl::types::GLint,
            std::ptr::null(),
        );
        gl::BindVertexArray(0);
    }

    unsafe {
        gl::Viewport(0, 0, 1024, 768);
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
    }

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

        unsafe {
            gl::BindVertexArray(vao);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            gl::BindVertexArray(0);
        }

        window.gl_swap_window();
    }
}

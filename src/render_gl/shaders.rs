use crate::utils::*;
use std::ffi::{CStr, CString};

use super::data::{Cvec3, Cvec4};

pub struct Shader {
    pub id: gl::types::GLuint,
}

impl Shader {
    pub fn from_source(source: &CStr, shader_type: gl::types::GLuint) -> Result<Shader, String> {
        let id = unsafe { gl::CreateShader(shader_type) };
        unsafe {
            gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring(len as usize);
            unsafe {
                gl::GetShaderInfoLog(
                    id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar,
                );
            }
            return Err(error.to_string_lossy().into_owned());
        }

        Ok(Shader { id })
    }

    pub fn from_file(path: &'static str, shader_type: gl::types::GLuint) -> Result<Shader, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|_| format!("Couldn't locate shader source at {:?}", path))?;
        let source =
            CString::new(contents).map_err(|_| "Couldn't convert shader source to C string")?;
        Self::from_source(&source, shader_type)
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

pub struct Program {
    pub id: gl::types::GLuint,
}

impl Program {
    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(program_id, shader.id);
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        let mut len: gl::types::GLint = 0;
        if success == 0 {
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring(len as usize);
            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar,
                );
            }
            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe {
                gl::DetachShader(program_id, shader.id);
            }
        }

        Ok(Program { id: program_id })
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_uniform_1b(&self, name: &CStr, b: bool) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform1i(loc, b as gl::types::GLint);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1i(&self, name: &CStr, x: i32) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform1i(loc, x as gl::types::GLint);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1f(&self, name: &CStr, x: f32) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform1f(loc, x as gl::types::GLfloat);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_3f(&self, name: &CStr, x: f32, y: f32, z: f32) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform3f(loc, x, y, z);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_4f(&self, name: &CStr, x: f32, y: f32, z: f32, w: f32) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform4f(loc, x, y, z, w);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1fv(&self, name: &CStr, fv: &[f32]) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform1fv(loc, fv.len() as gl::types::GLsizei, fv.as_ptr());
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_3fv(&self, name: &CStr, fv: &[Cvec3]) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform3fv(
                    loc,
                    fv.len() as gl::types::GLsizei,
                    fv.as_ptr() as *const gl::types::GLfloat,
                );
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_4fv(&self, name: &CStr, fv: &[Cvec4]) {
        unsafe {
            let loc = gl::GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                gl::Uniform4fv(
                    loc,
                    fv.len() as gl::types::GLsizei,
                    fv.as_ptr() as *const gl::types::GLfloat,
                );
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub struct VertexBufferObject<T: super::data::Vertex> {
    pub id: gl::types::GLuint,
    pub buffer_type: gl::types::GLenum,
    marker: std::marker::PhantomData<T>,
}

impl<T: super::data::Vertex> VertexBufferObject<T> {
    pub fn new(bt: gl::types::GLenum) -> Self {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }

        VertexBufferObject {
            id: vbo,
            buffer_type: bt,
            marker: std::marker::PhantomData,
        }
    }

    pub fn new_with_vec(bt: gl::types::GLenum, vs: Vec<T>) -> Self {
        let mut vbo = Self::new(bt);
        vbo.upload_static_draw_data(vs);
        vbo
    }

    pub fn upload_static_draw_data(&mut self, vs: Vec<T>) {
        unsafe {
            gl::BindBuffer(self.buffer_type, self.id);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vs.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                vs.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
            gl::BindBuffer(self.buffer_type, 0);
        }
    }

    pub fn setup_vertex_attrib_pointers(&self) {
        T::setup_vertex_attrib_pointers();
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.buffer_type, self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(self.buffer_type, 0);
        }
    }
}

impl<T: super::data::Vertex> Drop for VertexBufferObject<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
        }
    }
}

pub struct VertexArrayObject {
    pub id: gl::types::GLuint,
}

impl VertexArrayObject {
    pub fn new() -> Self {
        let mut vao: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }
        VertexArrayObject { id: vao }
    }

    pub fn draw_arrays(
        &self,
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLsizei,
    ) {
        unsafe {
            gl::DrawArrays(mode, first, count);
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for VertexArrayObject {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.id);
        }
    }
}

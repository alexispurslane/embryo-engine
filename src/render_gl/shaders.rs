use crate::utils::*;
use std::ffi::CStr;

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

    pub fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
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

        if success == 0 {
            let mut len: gl::types::GLint = 0;
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
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub trait BindableObject {
    fn bind(&self);
    fn unbind(&self);
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
        self.bind();
        T::setup_vertex_attrib_pointers();
        self.unbind();
    }
}

impl<T: super::data::Vertex> BindableObject for VertexBufferObject<T> {
    fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.buffer_type, self.id);
        }
    }

    fn unbind(&self) {
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

pub struct VertexArrayObject<T: super::data::Vertex> {
    pub id: gl::types::GLuint,
    pub array_buffer: VertexBufferObject<T>,
}

impl<T: super::data::Vertex> VertexArrayObject<T> {
    pub fn new(vbo: VertexBufferObject<T>) -> Self {
        let mut vao: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }
        VertexArrayObject {
            id: vao,
            array_buffer: vbo,
        }
    }

    pub fn draw_arrays(
        &self,
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLsizei,
    ) {
        self.bind();
        unsafe {
            gl::DrawArrays(mode, first, count);
        }
        self.unbind();
    }

    pub fn setup_vertex_attrib_pointers(&self) {
        self.bind();
        self.array_buffer.setup_vertex_attrib_pointers();
        self.unbind();
    }
}

impl<T: super::data::Vertex> BindableObject for VertexArrayObject<T> {
    fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

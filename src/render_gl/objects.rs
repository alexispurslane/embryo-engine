#![allow(unused)]

use std::marker::PhantomData;

use super::data;

pub trait Buffer {
    /// Number of vertices or indices in the buffer
    fn count(&self) -> usize;
    /// Bind the buffer to its buffer handle
    fn bind(&self);
    /// Bind the buffer handle back to zero
    fn unbind(&self);
    /// Set up per-vertex vertex attribute pointers (interleaved)
    fn setup_vertex_attrib_pointers(&self);
}

pub struct VertexBufferObject<T: super::data::Vertex> {
    /// The internal buffer object ID OpenGL uses to bind/unbind the object.
    pub id: gl::types::GLuint,
    /// The type of buffer this is (gl::ARRAY_BUFFER)
    pub buffer_type: gl::types::GLenum,
    /// The per-vertex data is a homogenous format that shoudln't change for one
    /// buffer, so we store this statically. Per-instance data is dynamic.
    marker: std::marker::PhantomData<T>,
    count: usize,
}

impl<T: super::data::Vertex> VertexBufferObject<T> {
    /// Request a new buffer from OpenGL and creates a struct to wrap the
    /// returned ID. Doesn't initialize the buffer.
    pub fn new(bt: gl::types::GLenum) -> Self {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }

        VertexBufferObject {
            id: vbo,
            buffer_type: bt,
            marker: std::marker::PhantomData,
            count: 0,
        }
    }

    /// Request a new buffer and initialize it with the given vector.
    pub fn new_with_vec(bt: gl::types::GLenum, vs: &[T]) -> Self {
        let mut vbo = Self::new(bt);
        vbo.count = vs.len();
        vbo.bind();
        vbo.upload_data(vs, gl::STATIC_DRAW);
        vbo.unbind();
        vbo
    }

    /// Writes in new per-vertex data
    pub fn upload_data(&mut self, buffer: &[T], flag: gl::types::GLenum) {
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
            gl::NamedBufferData(
                self.id,
                buf_size,
                buffer.as_ptr() as *const gl::types::GLvoid,
                flag,
            );
        }
        self.count = buffer.len();
    }

    /// Overwrites a section of the vertex buffer at the given offset without
    /// clearing the rest or resizing or changing the flag
    pub fn update_data(&mut self, buffer: &[T], offset_in_ibo: usize) {
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
            gl::NamedBufferSubData(
                self.id,
                (offset_in_ibo * std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                buf_size,
                buffer.as_ptr() as *const gl::types::GLvoid,
            )
        }
    }
}

impl<T: data::Vertex> Buffer for VertexBufferObject<T> {
    fn count(&self) -> usize {
        self.count
    }

    fn setup_vertex_attrib_pointers(&self) {
        T::setup_vertex_attrib_pointers();
    }

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

pub struct ElementBufferObject {
    pub id: gl::types::GLuint,
    count: usize,
}

impl ElementBufferObject {
    pub fn new() -> Self {
        let mut ebo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut ebo);
        }
        ElementBufferObject { id: ebo, count: 0 }
    }

    pub fn new_with_vec(is: &[u32]) -> Self {
        let mut ebo = Self::new();
        ebo.count = is.len();
        ebo.bind();
        ebo.upload_data(is, gl::STATIC_DRAW);
        ebo.unbind();
        ebo
    }

    /// Writes in new vertex occurence data
    pub fn upload_data(&mut self, buffer: &[u32], flag: gl::types::GLenum) {
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<u32>()) as gl::types::GLsizeiptr;
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                buf_size,
                buffer.as_ptr() as *const gl::types::GLvoid,
                flag,
            );
        }
        self.count = buffer.len();
    }
}

impl Buffer for ElementBufferObject {
    fn count(&self) -> usize {
        self.count
    }

    fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        }
    }

    fn setup_vertex_attrib_pointers(&self) {
        unimplemented!();
    }
}

impl Drop for ElementBufferObject {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
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

    pub fn draw_elements(
        &self,
        mode: gl::types::GLenum,
        count: gl::types::GLint,
        ty: gl::types::GLenum,
        offset: gl::types::GLint,
    ) {
        unsafe {
            gl::DrawElements(mode, count, ty, offset as *const gl::types::GLvoid);
        }
    }

    pub fn draw_elements_instanced(
        &self,
        mode: gl::types::GLenum,
        count: gl::types::GLint,
        ty: gl::types::GLenum,
        offset: gl::types::GLint,
        num_instances: gl::types::GLint,
    ) {
        unsafe {
            gl::DrawElementsInstanced(
                mode,
                count,
                ty,
                offset as *const gl::types::GLvoid,
                num_instances,
            );
        }
    }

    pub fn draw_arrays_instanced(
        &self,
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLint,
        num_instances: gl::types::GLint,
    ) {
        unsafe {
            gl::DrawArraysInstanced(mode, first, count, num_instances);
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

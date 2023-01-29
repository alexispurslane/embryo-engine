#![allow(unused)]

pub struct VertexBufferObject<T: super::data::Vertex> {
    /// The internal buffer object ID OpenGL uses to bind/unbind the object.
    pub id: gl::types::GLuint,
    /// The type of buffer this is (gl::ARRAY_BUFFER)
    pub buffer_type: gl::types::GLenum,
    /// The per-vertex data is a homogenous format that shoudln't change for one
    /// buffer, so we store this statically. Per-instance data is dynamic.
    marker: std::marker::PhantomData<T>,
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
        }
    }

    /// Request a new buffer and initialize it with the given vector.
    pub fn new_with_vec(bt: gl::types::GLenum, vs: Vec<T>) -> Self {
        let mut vbo = Self::new(bt);
        vbo.bind();
        vbo.upload_static_draw_data(vs);
        vbo.unbind();
        vbo
    }

    /// Writes in new per-vertex data, resets per-instance data.
    pub fn upload_static_draw_data(&mut self, vs: Vec<T>) {
        let buf_size = (vs.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
        unsafe {
            gl::BufferData(
                self.buffer_type,
                buf_size,
                vs.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }

    /// Writes in new per-vertex data, resets per-instance data.
    pub fn upload_data(&mut self, vs: Vec<T>, flag: gl::types::GLenum) {
        let buf_size = (vs.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
        unsafe {
            gl::BufferData(
                self.buffer_type,
                buf_size,
                vs.as_ptr() as *const gl::types::GLvoid,
                flag,
            );
        }
    }

    /// Set up per-vertex vertex attribute pointers (interleaved)
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

pub struct ElementBufferObject {
    pub id: gl::types::GLuint,
}

impl ElementBufferObject {
    pub fn new() -> Self {
        let mut ebo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut ebo);
        }
        ElementBufferObject { id: ebo }
    }

    pub fn new_with_vec(is: Vec<u32>) -> Self {
        let ebo = Self::new();
        ebo.bind();
        ebo.upload_static_draw_data(is);
        ebo.bind();
        ebo
    }

    pub fn upload_static_draw_data(&self, is: Vec<u32>) {
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (is.len() * std::mem::size_of::<u32>()) as gl::types::GLsizeiptr,
                is.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        }
    }
}

impl Drop for ElementBufferObject {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
        }
    }
}

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
        let vbo = Self::new(bt);
        vbo.bind();
        vbo.upload_static_draw_data(vs);
        vbo.unbind();
        vbo
    }

    pub fn upload_static_draw_data(&self, vs: Vec<T>) {
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vs.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                vs.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
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

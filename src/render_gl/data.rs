#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Cvec3 {
    pub d0: f32,
    pub d1: f32,
    pub d2: f32,
}

impl Cvec3 {
    pub fn new(d0: f32, d1: f32, d2: f32) -> Cvec3 {
        Cvec3 { d0, d1, d2 }
    }

    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(stride: usize, location: usize, offset: usize) {
        gl::EnableVertexAttribArray(location as gl::types::GLuint);
        gl::VertexAttribPointer(
            location as gl::types::GLuint,
            3,
            gl::FLOAT,
            gl::FALSE,
            stride as gl::types::GLint,
            offset as *const gl::types::GLvoid,
        );
    }
}

impl From<(f32, f32, f32)> for Cvec3 {
    fn from(other: (f32, f32, f32)) -> Self {
        Cvec3::new(other.0, other.1, other.2)
    }
}

pub trait Vertex {
    fn setup_vertex_attrib_pointers();
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct OpaqueColorVertex {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub clr: Cvec3,
}

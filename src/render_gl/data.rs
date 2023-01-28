pub trait VertexAttribute {
    /// Initialize a vertex attribute containing this type at this location,
    /// with this stride and offset.
    unsafe fn vertex_attrib_pointer(stride: usize, location: usize, offset: usize);
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Cvec2 {
    pub d0: f32,
    pub d1: f32,
}

impl Cvec2 {
    pub fn new(d0: f32, d1: f32) -> Cvec2 {
        Cvec2 { d0, d1 }
    }
}

impl VertexAttribute for Cvec2 {
    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(stride: usize, location: usize, offset: usize) {
        gl::EnableVertexAttribArray(location as gl::types::GLuint);
        gl::VertexAttribPointer(
            location as gl::types::GLuint,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride as gl::types::GLint,
            offset as *const gl::types::GLvoid,
        );
    }
}

impl From<(f32, f32)> for Cvec2 {
    fn from(other: (f32, f32)) -> Self {
        Cvec2::new(other.0, other.1)
    }
}

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
}

impl VertexAttribute for Cvec3 {
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

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Cvec4 {
    pub d0: f32,
    pub d1: f32,
    pub d2: f32,
    pub d3: f32,
}

impl Cvec4 {
    pub fn new(d0: f32, d1: f32, d2: f32, d3: f32) -> Self {
        Cvec4 { d0, d1, d2, d3 }
    }
}

impl VertexAttribute for Cvec4 {
    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(stride: usize, location: usize, offset: usize) {
        gl::EnableVertexAttribArray(location as gl::types::GLuint);
        gl::VertexAttribPointer(
            location as gl::types::GLuint,
            4,
            gl::FLOAT,
            gl::FALSE,
            stride as gl::types::GLint,
            offset as *const gl::types::GLvoid,
        );
    }
}

impl From<(f32, f32, f32, f32)> for Cvec4 {
    fn from(other: (f32, f32, f32, f32)) -> Self {
        Cvec4::new(other.0, other.1, other.2, other.3)
    }
}

pub trait Vertex {
    fn setup_vertex_attrib_pointers();
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct InstanceLocationVertex {
    #[location = 3]
    #[divisor = 1]
    pub pos: Cvec4,
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexRGB {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub clr: Cvec3,
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexRGBTex {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub clr: Cvec3,
    #[location = 2]
    pub tex: Cvec2,
}

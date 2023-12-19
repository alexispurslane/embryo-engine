use gl::Gl;

pub trait VertexAttribute {
    /// Initialize a vertex attribute containing this type at this location,
    /// with this stride and offset.
    unsafe fn vertex_attrib_pointer(gl: &Gl, stride: usize, location: usize, offset: usize);
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
    pub fn zero() -> Self {
        Self { d0: 0.0, d1: 0.0 }
    }
}

impl VertexAttribute for Cvec2 {
    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(gl: &Gl, stride: usize, location: usize, offset: usize) {
        gl.EnableVertexAttribArray(location as gl::types::GLuint);
        gl.VertexAttribPointer(
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

impl From<[f32; 2]> for Cvec2 {
    fn from(other: [f32; 2]) -> Self {
        Cvec2::new(other[0], other[1])
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

    pub fn zero() -> Self {
        Self {
            d0: 0.0,
            d1: 0.0,
            d2: 0.0,
        }
    }

    pub fn from_glam(v: glam::Vec3) -> Self {
        Self {
            d0: v.x,
            d1: v.y,
            d2: v.z,
        }
    }
}

impl From<[f32; 3]> for Cvec3 {
    fn from(value: [f32; 3]) -> Self {
        Self {
            d0: value[0],
            d1: value[1],
            d2: value[2],
        }
    }
}

impl VertexAttribute for Cvec3 {
    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(gl: &Gl, stride: usize, location: usize, offset: usize) {
        gl.EnableVertexAttribArray(location as gl::types::GLuint);
        gl.VertexAttribPointer(
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

    pub fn zero() -> Self {
        Self {
            d0: 0.0,
            d1: 0.0,
            d2: 0.0,
            d3: 0.0,
        }
    }
}

impl From<[f32; 4]> for Cvec4 {
    fn from(value: [f32; 4]) -> Self {
        Self {
            d0: value[0],
            d1: value[1],
            d2: value[2],
            d3: value[3],
        }
    }
}

impl VertexAttribute for Cvec4 {
    /// Enable and set the values for the one vertex attribute this vector represents
    unsafe fn vertex_attrib_pointer(gl: &Gl, stride: usize, location: usize, offset: usize) {
        gl.EnableVertexAttribArray(location as gl::types::GLuint);
        gl.VertexAttribPointer(
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
impl From<&[f32]> for Cvec4 {
    fn from(other: &[f32]) -> Self {
        Cvec4::new(other[0], other[1], other[2], other[3])
    }
}

pub trait Vertex {
    fn setup_vertex_attrib_pointers(gl: &Gl);
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
pub struct InstanceTransformVertex {
    #[location = 4]
    #[divisor = 1]
    pub d0: Cvec4,
    #[location = 5]
    #[divisor = 1]
    pub d1: Cvec4,
    #[location = 6]
    #[divisor = 1]
    pub d2: Cvec4,
    #[location = 7]
    #[divisor = 1]
    pub d3: Cvec4,
}

impl InstanceTransformVertex {
    pub fn new(values: [f32; 16]) -> Self {
        Self {
            d0: values[0..=3].into(),
            d1: values[4..=7].into(),
            d2: values[8..=11].into(),
            d3: values[12..=15].into(),
        }
    }
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexPos {
    #[location = 0]
    pub pos: Cvec3,
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexTex {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub tex: Cvec2,
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

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexNormTex {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub norm: Cvec3,
    #[location = 2]
    pub tex: Cvec2,
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct VertexNormTexTan {
    #[location = 0]
    pub pos: Cvec3,
    #[location = 1]
    pub norm: Cvec3,
    #[location = 2]
    pub tex: Cvec2,
    #[location = 3]
    pub tan: Cvec4,
}

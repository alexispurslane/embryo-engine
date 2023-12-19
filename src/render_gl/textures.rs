use gl::Gl;
use half::f16;

use crate::utils;
use std::{any::Any, marker::PhantomData};

use super::objects::FramebufferAttachment;

pub struct TextureParameters {
    pub texture_type: gl::types::GLenum,
    pub color_attachment_point: Option<gl::types::GLenum>,
    pub wrap_s: gl::types::GLint,
    pub wrap_t: gl::types::GLint,
    pub min_filter: gl::types::GLint,
    pub mag_filter: gl::types::GLint,
    pub mips: gl::types::GLint,
}

impl Default for TextureParameters {
    fn default() -> Self {
        TextureParameters {
            texture_type: gl::TEXTURE_2D,
            color_attachment_point: None,
            mips: 4,
            wrap_s: gl::REPEAT as gl::types::GLint,
            wrap_t: gl::REPEAT as gl::types::GLint,
            min_filter: gl::LINEAR_MIPMAP_LINEAR as gl::types::GLint,
            mag_filter: gl::LINEAR as gl::types::GLint,
        }
    }
}

pub trait ColorDepth {
    fn get_gl_type() -> gl::types::GLenum;
    fn get_pixel_format() -> gl::types::GLenum;
    fn get_sized_internal_format() -> gl::types::GLenum;
}

pub type RGB8 = u8;
impl ColorDepth for RGB8 {
    fn get_gl_type() -> gl::types::GLenum {
        gl::UNSIGNED_BYTE
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGB
    }
    fn get_sized_internal_format() -> gl::types::GLenum {
        gl::RGB8
    }
}

#[repr(transparent)]
pub struct R16F(f16);
impl ColorDepth for R16F {
    fn get_gl_type() -> gl::types::GLenum {
        gl::UNSIGNED_SHORT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::R16
    }
    fn get_sized_internal_format() -> gl::types::GLenum {
        gl::R16F
    }
}

pub type RGBA32F = f32;
impl ColorDepth for RGBA32F {
    fn get_gl_type() -> gl::types::GLenum {
        gl::FLOAT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGBA
    }
    fn get_sized_internal_format() -> gl::types::GLenum {
        gl::RGB32F
    }
}

pub type RGBA16F = f16;
impl ColorDepth for RGBA16F {
    fn get_gl_type() -> gl::types::GLenum {
        gl::HALF_FLOAT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGBA
    }
    fn get_sized_internal_format() -> gl::types::GLenum {
        gl::RGBA16F
    }
}
pub type DepthComponent24 = u32;
impl ColorDepth for DepthComponent24 {
    fn get_gl_type() -> gl::types::GLenum {
        gl::UNSIGNED_INT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::DEPTH_COMPONENT
    }
    fn get_sized_internal_format() -> gl::types::GLenum {
        gl::DEPTH_COMPONENT24
    }
}

pub trait AbstractTexture {
    fn bind(&self, tex_unit: usize);
    fn unbind(&self, tex_unit: usize);
}

pub struct Texture<T: ColorDepth> {
    gl: Gl,
    pub id: gl::types::GLuint,
    pub parameters: TextureParameters,
    phantom: PhantomData<T>,
}

impl<T: ColorDepth> Texture<T> {
    pub fn new(gl: &Gl, parameters: TextureParameters) -> Self {
        let mut texture: gl::types::GLuint = 0;
        unsafe {
            gl.CreateTextures(parameters.texture_type, 1, &mut texture);
        }
        Self {
            gl: gl.clone(),
            id: texture,
            parameters,
            phantom: PhantomData,
        }
    }

    pub fn new_allocated(
        gl: &Gl,
        parameters: TextureParameters,
        width: usize,
        height: usize,
        depth: usize,
    ) -> Self {
        let tex = Self::new(gl, parameters);
        tex.allocate_storage(width, height, depth);
        tex
    }

    pub fn new_with_bytes(
        gl: &Gl,
        parameters: TextureParameters,
        bytes: &Vec<T>,
        width: usize,
        height: usize,
        depth: usize,
    ) -> Self {
        let tex = Self::new_allocated(gl, parameters, width, height, depth);
        tex.update_texture(bytes, 0, 0, 0, width, height, depth);
        tex
    }

    fn allocate_storage(&self, width: usize, height: usize, depth: usize) {
        unsafe {
            match self.parameters.texture_type {
                gl::TEXTURE_1D => {
                    self.gl.TextureStorage1D(
                        self.id,
                        self.parameters.mips,
                        T::get_sized_internal_format(),
                        width as gl::types::GLsizei,
                    );
                }
                gl::TEXTURE_2D | gl::TEXTURE_1D_ARRAY => {
                    self.gl.TextureStorage2D(
                        self.id,
                        self.parameters.mips,
                        T::get_sized_internal_format(),
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                    );
                }
                gl::TEXTURE_3D | gl::TEXTURE_2D_ARRAY => {
                    self.gl.TextureStorage3D(
                        self.id,
                        self.parameters.mips,
                        T::get_sized_internal_format(),
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                        depth as gl::types::GLsizei,
                    );
                }
                _ => {
                    unimplemented!()
                }
            }
        }
    }

    pub fn update_texture(
        &self,
        bytes: &Vec<T>,
        xoffset: usize,
        yoffset: usize,
        zoffset: usize,
        width: usize,
        height: usize,
        depth: usize,
    ) {
        unsafe {
            self.gl
                .TextureParameteri(self.id, gl::TEXTURE_WRAP_S, self.parameters.wrap_s);
            self.gl
                .TextureParameteri(self.id, gl::TEXTURE_WRAP_T, self.parameters.wrap_t);
            self.gl
                .TextureParameteri(self.id, gl::TEXTURE_MIN_FILTER, self.parameters.min_filter);
            self.gl
                .TextureParameteri(self.id, gl::TEXTURE_MAG_FILTER, self.parameters.mag_filter);
            match self.parameters.texture_type {
                gl::TEXTURE_1D => {
                    self.gl.TextureSubImage1D(
                        self.id,
                        0,
                        xoffset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        T::get_pixel_format(),
                        if T::get_gl_type() == gl::HALF_FLOAT {
                            gl::FLOAT
                        } else {
                            T::get_gl_type()
                        }, // special weird-ass rule only for writing
                        bytes.as_ptr() as *const gl::types::GLvoid,
                    );
                }
                gl::TEXTURE_2D | gl::TEXTURE_1D_ARRAY => {
                    self.gl.TextureSubImage2D(
                        self.id,
                        0,
                        xoffset as gl::types::GLint,
                        yoffset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                        T::get_pixel_format(),
                        if T::get_gl_type() == gl::HALF_FLOAT {
                            gl::FLOAT
                        } else {
                            T::get_gl_type()
                        }, // special weird-ass rule only for writing
                        bytes.as_ptr() as *const gl::types::GLvoid,
                    );
                }
                gl::TEXTURE_3D | gl::TEXTURE_2D_ARRAY => {
                    self.gl.TextureSubImage3D(
                        self.id,
                        0,
                        xoffset as gl::types::GLint,
                        yoffset as gl::types::GLint,
                        zoffset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                        depth as gl::types::GLsizei,
                        T::get_pixel_format(),
                        if T::get_gl_type() == gl::HALF_FLOAT {
                            gl::FLOAT
                        } else {
                            T::get_gl_type()
                        }, // special weird-ass rule only for writing
                        bytes.as_ptr() as *const gl::types::GLvoid,
                    );
                }
                _ => unimplemented!(),
            }

            self.gl.GenerateTextureMipmap(self.id);
        }
    }
}

impl<T: ColorDepth> AbstractTexture for Texture<T> {
    fn bind(&self, tex_unit: usize) {
        unsafe {
            self.gl
                .BindTextureUnit(tex_unit as gl::types::GLuint, self.id);
        }
    }
    fn unbind(&self, tex_unit: usize) {
        unsafe {
            self.gl.BindTextureUnit(tex_unit as gl::types::GLuint, 0);
        }
    }
}

impl<T: ColorDepth> Drop for Texture<T> {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteTextures(1, &mut self.id);
        }
    }
}

impl<T: ColorDepth + 'static> FramebufferAttachment for Texture<T> {
    fn attachment_point(&self) -> gl::types::GLenum {
        self.parameters.color_attachment_point.expect(
            "Cannot bind texture with no defined attachment point to a frame buffer object!",
        )
    }

    fn id(&self) -> gl::types::GLuint {
        self.id
    }

    fn internal_format(&self) -> gl::types::GLenum {
        T::get_sized_internal_format()
    }

    fn attachment_type(&self) -> super::objects::FramebufferAttachmentType {
        super::objects::FramebufferAttachmentType::Texture
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

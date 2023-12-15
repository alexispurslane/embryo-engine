use gl::Gl;
use half::f16;

use crate::utils;
use std::marker::PhantomData;

pub struct TextureParameters {
    wrap_s: gl::types::GLint,
    wrap_t: gl::types::GLint,
    min_filter: gl::types::GLint,
    mag_filter: gl::types::GLint,
}

impl Default for TextureParameters {
    fn default() -> Self {
        TextureParameters {
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
}

pub type RGB8 = u8;
impl ColorDepth for RGB8 {
    fn get_gl_type() -> gl::types::GLenum {
        gl::UNSIGNED_BYTE
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGB
    }
}
pub type RGB16 = u16;
impl ColorDepth for RGB16 {
    fn get_gl_type() -> gl::types::GLenum {
        gl::UNSIGNED_SHORT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGB
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
}
pub type RGBA16F = f16;
impl ColorDepth for RGBA16F {
    fn get_gl_type() -> gl::types::GLenum {
        gl::FLOAT
    }
    fn get_pixel_format() -> gl::types::GLenum {
        gl::RGBA
    }
}

pub trait AbstractTexture {
    fn activate(&self, tex_unit: gl::types::GLenum);
    fn bind(&self);
    fn unbind(&self);
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
            gl.GenTextures(1, &mut texture);
        }
        Self {
            gl: gl.clone(),
            id: texture,
            parameters,
            phantom: PhantomData,
        }
    }

    pub fn new_with_bytes(
        gl: &Gl,
        parameters: TextureParameters,
        bytes: &Vec<T>,
        width: u32,
        height: u32,
    ) -> Self {
        let tex = Self::new(gl, parameters);
        tex.bind();
        tex.load_texture_from_bytes(bytes, width, height);
        tex.unbind();
        tex
    }

    pub fn load_texture_from_bytes(&self, bytes: &Vec<T>, width: u32, height: u32) {
        unsafe {
            self.gl
                .TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, self.parameters.wrap_s);
            self.gl
                .TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, self.parameters.wrap_t);
            self.gl.TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                self.parameters.min_filter,
            );
            self.gl.TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                self.parameters.mag_filter,
            );
            self.gl.TexImage2D(
                gl::TEXTURE_2D,
                0,
                T::get_pixel_format() as gl::types::GLint,
                width as gl::types::GLsizei,
                height as gl::types::GLsizei,
                0,
                T::get_pixel_format(),
                T::get_gl_type(),
                bytes.as_ptr() as *const gl::types::GLvoid,
            );
            self.gl.GenerateMipmap(gl::TEXTURE_2D);
        }
    }
}

impl<T: ColorDepth> AbstractTexture for Texture<T> {
    fn activate(&self, tex_unit: gl::types::GLenum) {
        unsafe {
            self.gl.ActiveTexture(tex_unit);
        }
    }

    fn bind(&self) {
        unsafe {
            self.gl.BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            self.gl.BindTexture(gl::TEXTURE_2D, 0);
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

pub trait IntoTextureUnit {
    fn to_texture_unit(&self) -> gl::types::GLenum;
}

impl IntoTextureUnit for usize {
    fn to_texture_unit(&self) -> gl::types::GLenum {
        gl::TEXTURE0 + *self as u32
    }
}

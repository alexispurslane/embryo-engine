#![allow(unused)]
use gl::Gl;

use crate::utils::*;
use std::ffi::{CStr, CString};

use super::data::{Cvec3, Cvec4};

pub struct Shader {
    gl: Gl,
    pub id: gl::types::GLuint,
}

impl Shader {
    pub fn from_source(
        gl: &Gl,
        source: &CStr,
        shader_type: gl::types::GLuint,
    ) -> Result<Shader, String> {
        let id = unsafe { gl.CreateShader(shader_type) };
        unsafe {
            gl.ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
            gl.CompileShader(id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl.GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl.GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring(len as usize);
            unsafe {
                gl.GetShaderInfoLog(
                    id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar,
                );
            }
            return Err(error.to_string_lossy().into_owned());
        }

        Ok(Shader { gl: gl.clone(), id })
    }

    pub fn from_file(
        gl: &Gl,
        path: &'static str,
        shader_type: gl::types::GLuint,
    ) -> Result<Shader, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|_| format!("Couldn't locate shader source at {:?}", path))?;
        let source =
            CString::new(contents).map_err(|_| "Couldn't convert shader source to C string")?;
        Self::from_source(gl, &source, shader_type)
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteShader(self.id);
        }
    }
}

pub struct Program {
    gl: Gl,
    pub id: gl::types::GLuint,
}

impl Program {
    pub fn from_shaders(gl: &Gl, shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl.CreateProgram() };

        for shader in shaders {
            unsafe {
                gl.AttachShader(program_id, shader.id);
            }
        }

        unsafe {
            gl.LinkProgram(program_id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl.GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        let mut len: gl::types::GLint = 0;
        if success == 0 {
            unsafe {
                gl.GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring(len as usize);
            unsafe {
                gl.GetProgramInfoLog(
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
                gl.DetachShader(program_id, shader.id);
            }
        }

        Ok(Program {
            gl: gl.clone(),
            id: program_id,
        })
    }

    pub fn set_used(&self) {
        unsafe {
            self.gl.UseProgram(self.id);
        }
    }

    pub fn set_uniform_1b(&self, name: &CStr, b: bool) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform1i(loc, b as gl::types::GLint);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1ui(&self, name: &CStr, b: u32) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform1ui(loc, b as gl::types::GLuint);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1i(&self, name: &CStr, x: i32) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform1i(loc, x as gl::types::GLint);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1f(&self, name: &CStr, x: f32) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform1f(loc, x as gl::types::GLfloat);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_3f(&self, name: &CStr, vec: Cvec3) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform3f(loc, vec.d0, vec.d1, vec.d2);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_4f(&self, name: &CStr, vec: Cvec4) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform4f(loc, vec.d0, vec.d1, vec.d2, vec.d3);
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_1fv(&self, name: &CStr, fv: &[f32]) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl
                    .Uniform1fv(loc, fv.len() as gl::types::GLsizei, fv.as_ptr());
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_3fv(&self, name: &CStr, fv: &[Cvec3]) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform3fv(
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
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.Uniform4fv(
                    loc,
                    fv.len() as gl::types::GLsizei,
                    fv.as_ptr() as *const gl::types::GLfloat,
                );
            } else {
                panic!("Cannot get uniform {:?} in program {:?}", name, self.id);
            }
        }
    }

    pub fn set_uniform_matrix_4fv(&self, name: &CStr, fv: &[f32; 16]) {
        unsafe {
            let loc = self.gl.GetUniformLocation(self.id, name.as_ptr());
            if loc != -1 {
                self.gl.UniformMatrix4fv(
                    loc,
                    1,
                    gl::FALSE,
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
            self.gl.DeleteProgram(self.id);
        }
    }
}

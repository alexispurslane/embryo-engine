/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![allow(unused)]

use std::{any::Any, marker::PhantomData};

use gl::Gl;

use super::{data, textures::ColorDepth};

pub trait Buffer {
    /// Number of vertices or indices in the buffer
    fn count(&self) -> usize;
    /// Bind the buffer to its buffer handle
    fn bind(&self);
    /// Bind the buffer handle back to zero
    fn unbind(&self);
}

pub trait VertexArray {
    /// Set up per-vertex vertex attribute pointers (interleaved)
    fn setup_vertex_attrib_pointers(&self);
}

pub struct BufferObject<T: Sized> {
    gl: Gl,
    /// The internal buffer object ID OpenGL uses to bind/unbind the object.
    pub id: gl::types::GLuint,
    /// The type of buffer this is (gl::ARRAY_BUFFER)
    pub buffer_type: gl::types::GLenum,
    /// The per-vertex data is a homogenous format that shoudln't change for one
    /// buffer, so we store this statically. Per-instance data is dynamic.
    marker: std::marker::PhantomData<T>,
    count: usize,
    immutable: bool,
    persistant_map_addr: Option<*mut std::ffi::c_void>,
}

impl<T: Sized> BufferObject<T> {
    /// Request a new buffer from OpenGL and creates a struct to wrap the
    /// returned ID. Creates/registers the buffer (unlike GenBuffers), but
    /// doesn't initialize it with any valid contents, so this is not a public
    /// constructor, because it could give the user (indirect) access to
    /// uninitialized memory. The only way to actually create a new VBO using
    /// this library is with one of the methods that explicitly initalizes it
    /// and sizes it, so RAII!
    fn new_inner(gl: &Gl, bt: gl::types::GLenum) -> Self {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl.CreateBuffers(1, &mut vbo);
        }

        BufferObject {
            gl: gl.clone(),
            id: vbo,
            buffer_type: bt,
            marker: std::marker::PhantomData,
            count: 0,
            immutable: false,
            persistant_map_addr: None,
        }
    }

    /// Request a new buffer and initialize it to the given size, with null
    pub fn new(gl: &Gl, bt: gl::types::GLenum, flags: gl::types::GLenum, count: usize) -> Self {
        let mut vbo = Self::new_inner(gl, bt);
        vbo.count = count;

        let buf_size = (count * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
        unsafe {
            gl.NamedBufferData(vbo.id, buf_size, std::ptr::null(), flags);
        }
        vbo
    }

    /// Request a new buffer and initialize it with the given vector.
    pub fn new_with_vec(gl: &Gl, bt: gl::types::GLenum, vs: &[T]) -> Self {
        let mut vbo = Self::new_inner(gl, bt);
        vbo.count = vs.len();
        vbo.recreate_with_data(vs, gl::STATIC_DRAW);
        vbo
    }

    /// Creates the buffer with glNamedBufferStorage instead of
    /// glNamedBufferData, so that it cannot be resized or deallocated. Buffers
    /// created with this function will be flagged as such, and recreation will
    /// be disabled on it. The buffer is then initialized with null.
    pub fn new_immutable(
        gl: &Gl,
        bt: gl::types::GLenum,
        flags: gl::types::GLenum,
        count: usize,
    ) -> Self {
        let mut vbo = Self::new_inner(gl, bt);
        let buf_size = (count * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
        vbo.count = count;
        vbo.immutable = true;

        unsafe {
            gl.NamedBufferStorage(vbo.id, buf_size, std::ptr::null(), flags);
        }
        vbo
    }

    /// Requests a new immutable (non de-allocable and non-resizable) buffer with the given vector as its data
    pub fn new_immutable_with_vec(
        gl: &Gl,
        bt: gl::types::GLenum,
        flags: gl::types::GLenum,
        vs: &[T],
    ) -> Self {
        let mut vbo = Self::new_inner(gl, bt);
        vbo.count = vs.len();
        vbo.immutable = true;

        let buf_size = (vs.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
        unsafe {
            gl.NamedBufferStorage(
                vbo.id,
                buf_size,
                vs.as_ptr() as *const gl::types::GLvoid,
                flags,
            );
        }
        vbo
    }

    /// Recreates/regenerates the buffer with a given size and content
    pub fn recreate_with_data(&mut self, buffer: &[T], flag: gl::types::GLenum) {
        if self.immutable {
            panic!("Cannot re-allocate immutable buffer created with gl*BufferStorage!");
        }
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
            self.gl.NamedBufferData(
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
    pub fn send_data(&mut self, buffer: &[T], offset: usize) {
        if buffer.len() > self.count {
            panic!("Tried to write more data to the buffer object than it can hold. If this buffer is mutable, try using recreate_with_data() instead. If not, you're shit out of luck, bub.");
        }
        if offset > self.count {
            panic!("Tried to write at an offset past this buffer object's size!");
        }
        if buffer.len() > self.count - offset {
            panic!("Tried to write past the end of this buffer object. Try less data or a smaller offset.");
        }
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr;
            self.gl.NamedBufferSubData(
                self.id,
                (offset * std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                buf_size,
                buffer.as_ptr() as *const gl::types::GLvoid,
            )
        }
    }

    pub fn persistent_map(&mut self, access_policy: gl::types::GLenum) {
        if !self.immutable {
            panic!("Do not map a non-immutable buffer, it's a very bad idea.");
        }
        if self.persistant_map_addr.is_some() {
            panic!(
                "Tried to persistently map a buffer object that was already persistently mapped!"
            );
        }
        unsafe {
            let ptr = self.gl.MapNamedBuffer(self.id, access_policy);
            if !ptr.is_null() {
                self.persistant_map_addr = Some(ptr);
            } else {
                println!(
                    "WARNING: Cannot map buffer {:?}. Error code: 0x{:X}",
                    self.id,
                    self.gl.GetError()
                );
            }
        }
    }

    pub fn write_to_persistant_map(&mut self, offset: usize, buffer: &[T]) {
        if let Some(ptr) = self.persistant_map_addr {
            if buffer.len() > self.count {
                panic!("Tried to write more data to the buffer object than it can hold. This object is immutable, so just don't do this.");
            }
            if offset > self.count {
                panic!("Tried to write at an offset past this buffer object's size!");
            }
            if buffer.len() > self.count - offset {
                panic!("Tried to write past the end of this buffer object. Try less data or a smaller offset.");
            }
            unsafe {
                let mut offset_ptr = ptr.wrapping_add(offset * std::mem::size_of::<T>());
                std::ptr::copy_nonoverlapping::<T>(
                    buffer.as_ptr() as *const T,
                    offset_ptr as *mut T,
                    buffer.len(),
                );
            }
        } else {
            panic!("Tried to write to a nonexistent persistant map!");
        }
    }
}

impl<T: Sized> Buffer for BufferObject<T> {
    fn count(&self) -> usize {
        self.count
    }

    fn bind(&self) {
        unsafe {
            self.gl.BindBuffer(self.buffer_type, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            self.gl.BindBuffer(self.buffer_type, 0);
        }
    }
}

impl<T: data::Vertex> VertexArray for BufferObject<T> {
    fn setup_vertex_attrib_pointers(&self) {
        T::setup_vertex_attrib_pointers(&self.gl);
    }
}

pub struct ElementBufferObject {
    gl: Gl,
    pub id: gl::types::GLuint,
    count: usize,
}

impl ElementBufferObject {
    pub fn new(gl: &Gl) -> Self {
        let mut ebo: gl::types::GLuint = 0;
        unsafe {
            gl.CreateBuffers(1, &mut ebo);
        }
        ElementBufferObject {
            gl: gl.clone(),
            id: ebo,
            count: 0,
        }
    }

    pub fn new_with_vec(gl: &Gl, is: &[u32]) -> Self {
        let mut ebo = Self::new(gl);
        ebo.count = is.len();
        ebo.recreate_with_data(is, gl::STATIC_DRAW);
        ebo
    }

    /// Writes in new vertex occurence data
    pub fn recreate_with_data(&mut self, buffer: &[u32], flag: gl::types::GLenum) {
        unsafe {
            let buf_size = (buffer.len() * std::mem::size_of::<u32>()) as gl::types::GLsizeiptr;
            self.gl.NamedBufferData(
                self.id,
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
            self.gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            self.gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        }
    }
}

impl Drop for ElementBufferObject {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteBuffers(1, &mut self.id);
        }
    }
}

impl<T: Sized> Drop for BufferObject<T> {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteBuffers(1, &mut self.id);
        }
    }
}

pub struct VertexArrayObject {
    gl: Gl,
    pub id: gl::types::GLuint,
}

impl VertexArrayObject {
    pub fn new(gl: &Gl) -> Self {
        let mut vao: gl::types::GLuint = 0;
        unsafe {
            gl.GenVertexArrays(1, &mut vao);
        }
        VertexArrayObject {
            gl: gl.clone(),
            id: vao,
        }
    }

    pub fn draw_arrays(
        &self,
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLsizei,
    ) {
        unsafe {
            self.gl.DrawArrays(mode, first, count);
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
            self.gl
                .DrawElements(mode, count, ty, offset as *const gl::types::GLvoid);
        }
    }

    pub fn draw_elements_instanced(
        &self,
        mode: gl::types::GLenum,
        count: gl::types::GLint,
        ty: gl::types::GLenum,
        index_offset: gl::types::GLint,
        num_instances: gl::types::GLint,
        instance_offset: gl::types::GLuint,
    ) {
        unsafe {
            self.gl.DrawElementsInstancedBaseInstance(
                mode,
                count,
                ty,
                index_offset as *const gl::types::GLvoid,
                num_instances,
                instance_offset,
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
            self.gl
                .DrawArraysInstanced(mode, first, count, num_instances);
        }
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.BindVertexArray(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            self.gl.BindVertexArray(0);
        }
    }
}

impl Drop for VertexArrayObject {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteVertexArrays(1, &mut self.id);
        }
    }
}

#[derive(Debug)]
pub enum FramebufferAttachmentType {
    Texture,
    Renderbuffer,
}

pub trait FramebufferAttachment {
    fn internal_format(&self) -> gl::types::GLenum;
    fn attachment_point(&self) -> gl::types::GLenum;
    fn id(&self) -> gl::types::GLuint;
    fn attachment_type(&self) -> FramebufferAttachmentType;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct FramebufferObject {
    gl: Gl,
    pub id: gl::types::GLuint,

    pub bind_point: Option<gl::types::GLenum>,
    pub attachments: Vec<Box<dyn FramebufferAttachment>>,
}

impl FramebufferObject {
    pub fn new(gl: &Gl) -> Self {
        let mut fbo: gl::types::GLuint = 0;
        unsafe {
            gl.CreateFramebuffers(1, &mut fbo);
        }

        FramebufferObject {
            gl: gl.clone(),
            id: fbo,
            bind_point: None,
            attachments: vec![],
        }
    }

    pub fn bind_to(&mut self, bind_point: gl::types::GLenum) {
        self.bind_point = Some(bind_point);
        unsafe {
            self.gl.BindFramebuffer(bind_point, self.id);
        }
    }

    pub fn unbind(&mut self) {
        if let Some(bp) = self.bind_point {
            unsafe {
                self.gl.BindFramebuffer(bp, 0);
            }
            self.bind_point = None;
        }
    }

    pub fn attach<A: FramebufferAttachment + 'static>(&mut self, attachment: A) {
        unsafe {
            match attachment.attachment_type() {
                FramebufferAttachmentType::Renderbuffer => {
                    self.gl.NamedFramebufferRenderbuffer(
                        self.id,
                        attachment.attachment_point(),
                        gl::RENDERBUFFER,
                        attachment.id(),
                    );
                }
                FramebufferAttachmentType::Texture => self.gl.NamedFramebufferTexture(
                    self.id,
                    attachment.attachment_point(),
                    attachment.id(),
                    0,
                ),
            }
        }
        println!(
            "Attaching {:?} object {}",
            attachment.attachment_type(),
            attachment.id()
        );
        self.attachments.push(Box::new(attachment));
    }

    pub fn get_attachment<A: FramebufferAttachment + 'static>(&self, index: usize) -> &A {
        (self.attachments.get(index).unwrap().as_any())
            .downcast_ref::<A>()
            .unwrap()
    }
    pub fn get_attachment_mut<A: FramebufferAttachment + 'static>(
        &mut self,
        index: usize,
    ) -> &mut A {
        (self.attachments.get_mut(index).unwrap().as_any_mut())
            .downcast_mut::<A>()
            .unwrap()
    }

    pub fn draw_to_buffers(&self, buffers: &[gl::types::GLenum]) {
        unsafe {
            self.gl.NamedFramebufferDrawBuffers(
                self.id,
                buffers.len() as gl::types::GLsizei,
                buffers.as_ptr() as *const gl::types::GLenum,
            );
        }
    }
}

impl Drop for FramebufferObject {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteFramebuffers(1, &mut self.id);
        }
    }
}

pub struct Renderbuffer<T: ColorDepth> {
    gl: Gl,
    marker: std::marker::PhantomData<T>,

    pub id: gl::types::GLuint,
    pub renderbuffer_type: gl::types::GLenum,
}

impl<T: ColorDepth> Renderbuffer<T> {
    pub fn new_with_size_and_attachment(
        gl: &Gl,
        width: usize,
        height: usize,
        renderbuffer_type: gl::types::GLenum,
    ) -> Self {
        let mut rb: gl::types::GLuint = 0;
        unsafe {
            gl.CreateRenderbuffers(1, &mut rb);
            gl.NamedRenderbufferStorage(
                rb,
                T::get_sized_internal_format(),
                width as gl::types::GLsizei,
                height as gl::types::GLsizei,
            );
        }

        Self {
            gl: gl.clone(),
            id: rb,
            marker: std::marker::PhantomData,
            renderbuffer_type,
        }
    }
}

impl<T: ColorDepth + 'static> FramebufferAttachment for Renderbuffer<T> {
    fn internal_format(&self) -> gl::types::GLenum {
        T::get_sized_internal_format()
    }
    fn attachment_point(&self) -> gl::types::GLenum {
        self.renderbuffer_type
    }
    fn id(&self) -> gl::types::GLuint {
        self.id
    }
    fn attachment_type(&self) -> FramebufferAttachmentType {
        FramebufferAttachmentType::Renderbuffer
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

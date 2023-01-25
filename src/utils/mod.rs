use std::{ffi::CString, path::Path};

use crate::render_gl::textures::ColorDepth;

pub fn create_whitespace_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn load_image_u8(path: &str) -> (u32, u32, Vec<u8>) {
    let tex = image::open(path).expect("Cannnot open texture 'container.jpg' for read");
    (
        tex.width(),
        tex.height(),
        tex.flipv().into_rgb8().into_vec(),
    )
}

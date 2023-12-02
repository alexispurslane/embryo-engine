use std::ffi::CString;

use sdl2::image::LoadSurface;

pub type Degrees = f32;
pub type Radians = f32;

pub fn create_whitespace_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn load_image_u8(path: &str) -> (u32, u32, Vec<u8>) {
    let image_surface = sdl2::surface::Surface::from_file(path)
        .expect(&format!(
            "Cannnot open texture '{}' for read from working directory {}",
            path,
            std::env::current_dir().unwrap().to_string_lossy()
        ))
        .convert_format(sdl2::pixels::PixelFormatEnum::RGB24)
        .unwrap();
    image_surface.with_lock(|pixels| {
        (
            image_surface.width(),
            image_surface.height(),
            pixels.to_vec(),
        )
    })
}

pub fn clear_screen() {
    unsafe {
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
}

pub fn setup_viewport((w, h): (u32, u32)) {
    unsafe {
        gl::Viewport(0, 0, w as gl::types::GLint, h as gl::types::GLint);
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Enable(gl::DEPTH_TEST);
    }
}

pub mod shapes {
    use crate::render_gl::data::VertexTex;

    pub fn unit_cube() -> [VertexTex; 36] {
        [
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, -0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, -0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, -0.5).into(),
                tex: (1.0, 1.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (0.5, 0.5, 0.5).into(),
                tex: (1.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, 0.5).into(),
                tex: (0.0, 0.0).into(),
            },
            VertexTex {
                pos: (-0.5, 0.5, -0.5).into(),
                tex: (0.0, 1.0).into(),
            },
        ]
    }
}

pub fn material_get_property(
    material: &russimp::material::Material,
    name: &'static str,
) -> Option<russimp::material::PropertyTypeInfo> {
    material.properties.iter().find_map(|matprop| {
        if matprop.key == name {
            Some(matprop.data.clone())
        } else {
            None
        }
    })
}

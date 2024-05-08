use std::{collections::HashMap, ffi::CString};

use gl::Gl;

use crate::{
    render_gl::{
        data::VertexTex,
        objects::{Buffer, BufferObject, VertexArray, VertexArrayObject},
        shaders::Program,
        textures::{AbstractTexture, Red, Texture, TextureParameters},
    },
    utils,
};

pub struct FreeTypeCharacter {
    pub texture: Texture<Red>,
    pub size: glam::IVec2,
    pub bearing: glam::IVec2,
    pub advance: usize,
}

pub struct FontRenderer {
    gl: Gl,
    font_shader: Program,
    characters: HashMap<char, FreeTypeCharacter>,
    viewport_size: (u32, u32),
    char_quad_vao: VertexArrayObject,
    text_proj: glam::Mat4,
    pub kerning: usize,
}

impl FontRenderer {
    pub fn new(
        font_name: &'static str,
        gl: &Gl,
        lib: freetype::Library,
        max_char: char,
        viewport_size: (u32, u32),
    ) -> Self {
        let face = lib
            .new_face(&format!("./data/fonts/{font_name}.ttf"), 0)
            .unwrap();

        face.set_pixel_sizes(0, 48).unwrap();
        unsafe {
            gl.PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        }
        Self {
            gl: gl.clone(),
            viewport_size,
            kerning: 20,
            font_shader: Program::new_with_shader_files(&gl, &["font.vert", "font.frag"]),
            text_proj: glam::Mat4::orthographic_rh_gl(
                0.0,
                viewport_size.0 as f32,
                0.0,
                viewport_size.1 as f32,
                -1.0,
                1.0,
            ),
            char_quad_vao: {
                let vao = VertexArrayObject::new(&gl);
                vao.bind();
                let vbo = BufferObject::<VertexTex>::new_with_vec(
                    &gl,
                    gl::ARRAY_BUFFER,
                    &utils::primitives::TEXTURED_2D_QUAD,
                );
                vbo.bind();
                vbo.setup_vertex_attrib_pointers();
                vao.unbind();
                std::mem::forget(vbo);
                vao
            },
            characters: (0..max_char as u8)
                .filter_map(|c| {
                    face.load_char(c as usize, freetype::face::LoadFlag::RENDER)
                        .unwrap();

                    let bitmap = face.glyph().bitmap();
                    let bytes = bitmap.buffer();

                    let tex = Texture::new_with_bytes(
                        gl,
                        TextureParameters {
                            wrap_s: gl::CLAMP_TO_EDGE,
                            wrap_t: gl::CLAMP_TO_EDGE,
                            min_filter: gl::LINEAR,
                            mag_filter: gl::LINEAR,
                            mips: 4,
                            ..Default::default()
                        },
                        &bytes.iter().map(|x| Red(*x)).collect(),
                        bitmap.width() as usize,
                        bitmap.rows() as usize,
                        1,
                    );
                    let character = FreeTypeCharacter {
                        texture: tex,
                        size: glam::ivec2(bitmap.width(), bitmap.rows()),
                        bearing: glam::ivec2(face.glyph().bitmap_left(), face.glyph().bitmap_top()),
                        advance: face.glyph().advance().x as usize,
                    };
                    Some((c as char, character))
                })
                .collect(),
        }
    }

    pub fn get_char(&self, c: char) -> Option<&FreeTypeCharacter> {
        self.characters.get(&c)
    }

    pub fn render_lines(
        &mut self,
        string: String,
        (x, y): (f32, f32),
        pixel_size: f32,
        color: (f32, f32, f32),
        line_height: f32,
    ) {
        for (i, line) in string.split('\n').enumerate() {
            self.render_string(
                line.to_string(),
                (x, y + i as f32 * line_height),
                pixel_size,
                color,
            )
        }
    }

    pub fn render_string(
        &mut self,
        string: String,
        (x, y): (f32, f32),
        pixel_size: f32,
        color: (f32, f32, f32),
    ) {
        let scale = pixel_size / 48.0;

        unsafe {
            self.gl.Enable(gl::BLEND);
            self.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.char_quad_vao.bind();

        self.font_shader.set_used();
        self.font_shader
            .set_uniform_3f(&CString::new("textColor").unwrap(), color.into());

        self.font_shader.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &self.text_proj.to_cols_array(),
        );
        let mut advance = 0;
        for (i, c) in string.chars().enumerate() {
            let ch = &self.get_char(c).unwrap();

            let x_pos = x + scale * (advance as f32 + ch.bearing.x as f32);
            let y_pos = y - scale * (ch.bearing.y) as f32;
            ch.texture.bind(0);
            self.font_shader.set_uniform_matrix_4fv(
                &CString::new("model_matrix").unwrap(),
                &glam::Mat4::from_scale_rotation_translation(
                    glam::vec3(scale * (ch.size.x as f32), scale * (ch.size.y as f32), 1.0),
                    glam::Quat::IDENTITY,
                    glam::vec3(x_pos, self.viewport_size.1 as f32 - y_pos, 0.0),
                )
                .to_cols_array(),
            );
            self.char_quad_vao.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
            advance += (ch.advance >> 6) + self.kerning;
        }
        self.char_quad_vao.unbind();
        unsafe {
            self.gl.Disable(gl::BLEND);
            self.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
    }
}

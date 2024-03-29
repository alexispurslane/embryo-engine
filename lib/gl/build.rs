extern crate gl_generator;

use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator, DebugStructGenerator};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut file_gl = File::create(&Path::new(&out_dir).join("bindings.rs")).unwrap();
    if cfg!(debug_assertions) {
        Registry::new(
            Api::Gl,
            (4, 6),
            Profile::Core,
            Fallbacks::All,
            ["GL_EXT_texture_filter_anisotropic"],
        )
            .write_bindings(StructGenerator, &mut file_gl)
            .unwrap()
    } else {
        Registry::new(
            Api::Gl,
            (4, 6),
            Profile::Core,
            Fallbacks::All,
            ["GL_EXT_texture_filter_anisotropic"],
        )
            .write_bindings(StructGenerator, &mut file_gl)
            .unwrap()
    }
}

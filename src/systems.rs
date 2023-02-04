use crate::*;
use entity::camera_component::CameraComponent;
use entity::render_component::RenderComponent;
use entity::transform_component::TransformComponent;
use entity::EntitySystem;
use objects::Buffer;
use rand::Rng;
use render_gl::textures;
use render_gl::{objects, shaders};
use std::ffi::CString;
use textures::IntoTextureUnit;

pub fn add_camera(scene: &mut Scene) {
    let e = scene.entities.new_entity();
    scene.entities.add_component(
        e.id,
        TransformComponent::new_from_rot_trans(
            glam::Vec3::Y,
            glam::vec3(0.0, 0.0, -3.0),
            gl::STREAM_DRAW,
        ),
    );
    scene
        .entities
        .add_component(e.id, CameraComponent { fov: 90.0 });
    scene.camera = Some(e.id);
}

pub fn add_textured_cube_instances(scene: &mut Scene) {
    // Create box object instances with shaders
    let vert_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.vert")).unwrap(),
        gl::VERTEX_SHADER,
    )
    .unwrap();

    let frag_shader = shaders::Shader::from_source(
        &CString::new(include_str!("triangle.frag")).unwrap(),
        gl::FRAGMENT_SHADER,
    )
    .unwrap();

    let cube = utils::shapes::unit_cube();
    let vbo = objects::VertexBufferObject::new_with_vec(gl::ARRAY_BUFFER, &cube);

    let texture1 = textures::get_texture_simple("container.jpg");
    let texture2 = textures::get_texture_simple("awesomeface.png");

    let boxes = scene.entities.new_entity();
    scene.entities.add_component(
        boxes.id,
        RenderComponent::new(
            &[frag_shader, vert_shader],
            Box::new(vbo),
            None,
            vec![
                ("texture1", Box::new(texture1)),
                ("texture2", Box::new(texture2)),
            ],
        ),
    );

    let mut rng = rand::thread_rng();
    scene.entities.add_component(
        boxes.id,
        TransformComponent::new_from_rot_trans_instances(
            (0..NUM_INSTANCES)
                .map(|_| {
                    (
                        glam::Vec3::X,
                        glam::vec3(
                            rng.gen_range::<f32, _>(-5.0..5.0),
                            rng.gen_range::<f32, _>(-5.0..5.0),
                            rng.gen_range::<f32, _>(-5.0..5.0),
                        ),
                    )
                })
                .collect(),
            gl::STATIC_DRAW,
        ),
    );
}

pub fn setup_render_components(entities: &mut EntitySystem) {
    let mut has_renderable = entities.get_component_vec_mut::<RenderComponent>();
    let mut has_transform = entities.get_component_vec_mut::<TransformComponent>();
    for (_eid, rc, tc) in entities.get_with_components_mut(&mut has_renderable, &mut has_transform)
    {
        // Set up the vertex array object we'll be using to render
        rc.vao.bind();

        // Add in the vertex info
        rc.vbo.bind();
        rc.vbo.setup_vertex_attrib_pointers();

        if let Some(ebo) = &rc.ebo {
            // Add in the index info
            ebo.bind();
        }

        // Add in the instance info
        tc.ibo.bind();
        tc.ibo.setup_vertex_attrib_pointers();
        rc.vao.unbind();
    }
}

pub fn render(scene: &Scene, width: u32, height: u32) {
    let has_renderable = scene.entities.get_component_vec::<RenderComponent>();
    let has_transform = scene.entities.get_component_vec::<TransformComponent>();

    let camera_eid = scene.camera.expect("No camera found");
    let ct = &scene.entities.get_component_vec::<TransformComponent>()[camera_eid];
    let camera_transform = ct
        .as_ref()
        .expect("Camera needs to have TransformComponent");
    let cc = &scene.entities.get_component_vec::<CameraComponent>()[camera_eid];
    let camera_component = cc.as_ref().expect("Camera needs to have CameraComponent");
    for (_eid, rc, tc) in scene
        .entities
        .get_with_components(&has_renderable, &has_transform)
    {
        rc.render(
            tc.instances,
            camera_transform.point_of_view(0),
            camera_component.project(width, height),
        );
    }
}

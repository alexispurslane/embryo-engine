use crate::entity::mesh_component::ModelComponent;
use crate::render_gl::shaders::Program;
use crate::*;
use entity::camera_component::CameraComponent;
use entity::transform_component::TransformComponent;
use entity::EntitySystem;
use render_gl::shaders;
use std::ffi::CString;

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

pub fn add_level(scene: &mut Scene) {
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
    scene
        .shader_programs
        .push(Program::from_shaders(&[frag_shader, vert_shader]).unwrap());

    let boxes = scene.entities.new_entity();
    scene.entities.add_component(
        boxes.id,
        ModelComponent::from_file("./assets/entities/cube.glb".to_string()).unwrap(),
    );

    scene.entities.add_component(
        boxes.id,
        TransformComponent::new_from_rot_trans(glam::Vec3::ZERO, glam::Vec3::ZERO, gl::STATIC_DRAW),
    );
}

pub fn setup_mesh_components(entities: &mut EntitySystem) {
    let mut has_model = entities.get_component_vec_mut::<ModelComponent>();
    let mut has_transform = entities.get_component_vec_mut::<TransformComponent>();
    for (_eid, model, transform) in
        entities.get_with_components_mut(&mut has_model, &mut has_transform)
    {
        // Set up the vertex array object we'll be using to render
        model.setup_mesh_components(&transform.ibo);
    }
}

pub fn render(scene: &Scene, width: u32, height: u32) {
    let has_renderable = scene.entities.get_component_vec::<ModelComponent>();
    let has_transform = scene.entities.get_component_vec::<TransformComponent>();

    let camera_eid = scene.camera.expect("No camera found");
    let ct = &scene.entities.get_component_vec::<TransformComponent>()[camera_eid];
    let camera_transform = ct
        .as_ref()
        .expect("Camera needs to have TransformComponent");
    let cc = &scene.entities.get_component_vec::<CameraComponent>()[camera_eid];
    let camera_component = cc.as_ref().expect("Camera needs to have CameraComponent");

    let program = &scene.shader_programs[0];
    program.set_used();

    for (_eid, rc, tc) in scene
        .entities
        .get_with_components(&has_renderable, &has_transform)
    {
        program.set_uniform_matrix_4fv(
            &CString::new("view_matrix").unwrap(),
            &camera_transform.point_of_view(0).to_cols_array(),
        );
        program.set_uniform_matrix_4fv(
            &CString::new("projection_matrix").unwrap(),
            &camera_component.project(width, height).to_cols_array(),
        );

        rc.render(tc.instances, &program);
    }
}

pub fn physics(scene: &Scene) -> Vec<SceneCommand> {
    vec![]
}

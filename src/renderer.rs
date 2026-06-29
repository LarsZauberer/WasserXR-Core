use glam::{EulerRot, Mat3, Mat4, Quat, Vec3, camera::rh::proj::opengl::perspective};
use glium::{Program, Surface, dynamic_uniform, winit::window::Window};
use wasserxr::{Uuid, scene::Scene, system, warn};

use crate::{model_asset::Mesh, window::component::Display};

#[system(entities=[["Window"], ["Camera"], ["Model"]])]
fn renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a window and a camera
    if entities[0].is_empty() {
        return;
    }
    if entities[1].is_empty() {
        return;
    }

    // Get the window and camera entity
    let window_entity = entities[0][0];
    let camera_entity = entities[1][0];

    // Getting the OpenGL Context
    let Ok((display, window)) =
        scene.query::<(&Display, &Window)>(window_entity, "Window", &["display", "window"])
    else {
        warn!(
            scene,
            "Failed to get the OpenGL context from the Window entity: {}", window_entity
        );
        return;
    };

    // Getting Camera Specs
    let Ok((fov, near, far)) =
        scene.query::<(&f32, &f32, &f32)>(camera_entity, "Camera", &["fov", "near", "far"])
    else {
        warn!(
            scene,
            "Failed to get camera properties from entity: {}", camera_entity
        );
        return;
    };

    let aspect_ratio: f32 =
        (window.inner_size().width as f32) / (window.inner_size().height as f32);

    // Camera position
    let mut cam_position: [f32; 3] = [0.0, 0.0, 0.0];
    let mut cam_rotation: [f32; 3] = [0.0, 0.0, 0.0];
    let fov = fov.to_radians();
    let near = *near;
    let far = *far;

    if let Ok((position, rotation)) =
        scene.query::<(&[f32; 3], &[f32; 3])>(camera_entity, "Transform", &["position", "rotation"])
    {
        cam_position = *position;
        cam_rotation = *rotation;
    }

    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 0.0, 1.0);

    for entity in &entities[2] {
        // Get all assets and information
        let Ok((model_path, shader_path)) =
            scene.query::<(&String, &String)>(*entity, "Model", &["model", "shader"])
        else {
            continue;
        };

        if model_path.is_empty() || shader_path.is_empty() {
            continue;
        }

        let model_path = model_path.clone();
        let shader_path = shader_path.clone();

        if scene
            .ensure_asset_loaded("ShaderAsset", &shader_path)
            .is_err()
        {
            continue;
        }

        if scene
            .ensure_asset_loaded("ModelAsset", &model_path)
            .is_err()
        {
            continue;
        }

        let Ok((shader_program,)) =
            scene.asset_query_loaded::<(&Program,)>("ShaderAsset", &shader_path, &["shader"])
        else {
            continue;
        };

        let Ok((meshes,)) =
            scene.asset_query_loaded::<(&Vec<Mesh>,)>("ModelAsset", &model_path, &["meshes"])
        else {
            continue;
        };

        let (transform, normal_transform) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                *entity,
                "Transform",
                &["position", "rotation", "scale"],
            )
            .ok()
            .map(|(position, rotation, scale)| {
                // Compute Model Transform
                let translation = Vec3::from_array(*position);
                let scale = Vec3::from_array(*scale);
                let rotation = make_quat(*rotation);
                let model_transform =
                    Mat4::from_scale_rotation_translation(scale, rotation, translation);

                // Compute View Transform
                let translation = Vec3::from_array(cam_position);
                let rotation = make_quat(cam_rotation);
                let view_transform = Mat4::from_rotation_translation(rotation, translation);

                // Compute Projection Transform
                let projection_transform = perspective(fov, aspect_ratio, near, far);

                // Show compute full transformation
                let mvp = projection_transform * view_transform * model_transform;
                let normal_transform = Mat3::from_mat4(model_transform).inverse().transpose();

                (mvp.to_cols_array_2d(), normal_transform.to_cols_array_2d())
            })
            .unwrap_or_else(|| {
                (
                    Mat4::IDENTITY.to_cols_array_2d(),
                    Mat3::IDENTITY.to_cols_array_2d(),
                )
            });

        // Build Uniforms
        let mut uniforms = dynamic_uniform! {};
        uniforms.add("transform", &transform);
        uniforms.add("normal_transform", &normal_transform);

        // Final draw calls
        for mesh in meshes {
            match frame.draw(
                &mesh.vertices,
                &mesh.indices,
                shader_program,
                &uniforms,
                &Default::default(),
            ) {
                Ok(_) => {}
                Err(err) => {
                    warn!(scene, "Failed to draw entity {}: {:?}", entity, err);
                    continue;
                }
            };
        }
    }

    frame.finish().unwrap();
}

fn to_rotation_vec3_from_array(vec: [f32; 3]) -> Vec3 {
    Vec3::new(
        vec[0].to_radians(),
        vec[1].to_radians(),
        vec[2].to_radians(),
    )
}

fn make_quat(vec: [f32; 3]) -> Quat {
    let rotation = to_rotation_vec3_from_array(vec);
    Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z)
}

use glam::{EulerRot, Mat3, Mat4, Quat, Vec3, camera::rh::proj::opengl::perspective};
use glium::{DrawParameters, Program, Surface, dynamic_uniform, winit::window::Window};
use wasserxr::{Uuid, attacher, scene::Scene, system, warn};

use crate::{model_asset::Mesh, window::get_event_loop};

pub type Display = glium::backend::glutin::Display<glium::glutin::surface::WindowSurface>;

pub(crate) fn get_window_display(scene: &mut Scene) -> &mut (Window, Display) {
    if scene
        .get_resource::<(Window, Display)>("render_window")
        .is_err()
    {
        let event_loop = get_event_loop(scene);
        let rendering_window = glium::backend::glutin::SimpleWindowBuilder::new().build(event_loop);
        let _ =
            scene.add_resource::<(Window, Display)>("render_window".to_owned(), rendering_window);
    }

    scene
        .get_mut_resource::<(Window, Display)>("render_window")
        .expect("Failed to get the Rendering Window")
}

#[attacher(renderer)]
fn renderer_attach(scene: &mut Scene) {
    let _ = get_window_display(scene);
}

#[system(entities=[["Camera"], ["Model"]])]
fn renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a camera
    if entities[0].is_empty() {
        return;
    }

    let Ok((window, display)) = scene.get_resource::<(Window, Display)>("render_window") else {
        return;
    };

    // Get the window and camera entity
    let camera_entity = entities[0][0];

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
    frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let draw_params = DrawParameters {
        depth: glium::Depth {
            test: glium::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    for entity in &entities[1] {
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
                &draw_params,
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

use glium::{Program, Surface, uniform};
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
    let Ok((display,)) = scene.query::<(&Display,)>(window_entity, "Window", &["display"]) else {
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

    let mut frame = display.draw();

    frame.clear_color(0.0, 0.0, 0.0, 1.0);

    for entity in &entities[2] {
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

        let uniforms = uniform! {};

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

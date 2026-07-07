use std::{cell::RefCell, collections::HashMap};

use glam::{EulerRot, Mat3, Mat4, Quat, Vec3, camera::rh::proj::opengl::perspective};
use glium::{DrawParameters, Program, Surface, dynamic_uniform, winit::window::Window};
use wasserxr::{Uuid, attacher, scene::Scene, system, warn};

use crate::{
    material_asset::MaterialData,
    model_asset::Mesh,
    opengl::{Display, WINDOW_DISPLAY_RESOURCE, ensure_opengl_window},
};

/// The camera settings read from the camera entity, ready for building
/// the view and projection matrices.
pub(crate) struct Camera {
    /// Vertical field of view in radians.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
}

/// Reads fov/near/far and the optional Transform from the camera entity.
pub(crate) fn get_camera(scene: &Scene, camera_entity: Uuid) -> Option<Camera> {
    let Ok((fov, near, far)) =
        scene.query::<(&f32, &f32, &f32)>(camera_entity, "Camera", &["fov", "near", "far"])
    else {
        warn!(
            scene,
            "Failed to get camera properties from entity: {}", camera_entity
        );
        return None;
    };

    let mut camera = Camera {
        fov: fov.to_radians(),
        near: *near,
        far: *far,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };

    if let Ok((position, rotation)) =
        scene.query::<(&[f32; 3], &[f32; 3])>(camera_entity, "Transform", &["position", "rotation"])
    {
        camera.position = *position;
        camera.rotation = *rotation;
    }

    Some(camera)
}

/// Builds the view matrix (world space -> camera space) from the camera transform.
pub(crate) fn view_matrix(camera: &Camera) -> Mat4 {
    let translation = Vec3::from_array(camera.position);
    let rotation = make_quat(camera.rotation);
    Mat4::from_rotation_translation(rotation, translation).inverse()
}

/// Everything needed to draw one Model entity, resolved once per frame.
pub(crate) struct RenderItem {
    entity: Uuid,
    model_path: String,
    material_path: String,
    shader_path: String,
    /// None when the entity has no Transform component; it is then drawn
    /// with the identity transform (no camera applied).
    model_transform: Option<Mat4>,
    normal_transform: [[f32; 3]; 3],
}

/// Resolves the assets and the model transform of every Model entity that
/// can be drawn. Entities with missing or unloadable assets are skipped.
pub(crate) fn collect_render_items(scene: &mut Scene, model_entities: &[Uuid]) -> Vec<RenderItem> {
    let mut render_items = Vec::new();
    for entity in model_entities {
        // Get all assets and information
        let Ok((model_path, material_path)) =
            scene.query::<(&String, &String)>(*entity, "Model", &["model", "material"])
        else {
            continue;
        };

        if model_path.is_empty() || material_path.is_empty() {
            continue;
        }

        let model_path = model_path.clone();
        let material_path = material_path.clone();

        if scene
            .ensure_asset_loaded("MaterialAsset", &material_path)
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

        let Ok((shader_path,)) =
            scene.asset_query_loaded::<(&String,)>("MaterialAsset", &material_path, &["shader"])
        else {
            continue;
        };

        let shader_path = shader_path.clone();

        if scene
            .ensure_asset_loaded("ShaderAsset", &shader_path)
            .is_err()
        {
            continue;
        }

        let (model_transform, normal_transform) = scene
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
                let normal_transform = Mat3::from_mat4(model_transform).inverse().transpose();

                (Some(model_transform), normal_transform.to_cols_array_2d())
            })
            .unwrap_or((None, Mat3::IDENTITY.to_cols_array_2d()));

        render_items.push(RenderItem {
            entity: *entity,
            model_path,
            material_path,
            shader_path,
            model_transform,
            normal_transform,
        });
    }

    render_items
}

/// Clears `surface` and draws every render item to it.
///
/// `view_projection` is `projection * view`; the model transform of each
/// item is applied on top to build the final vertex transform.
pub(crate) fn draw_render_items(
    scene: &Scene,
    surface: &mut impl Surface,
    view_projection: Mat4,
    render_items: &[RenderItem],
) {
    surface.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let draw_params = DrawParameters {
        depth: glium::Depth {
            test: glium::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    for item in render_items {
        let Ok((shader_program,)) =
            scene.asset_query_loaded::<(&Program,)>("ShaderAsset", &item.shader_path, &["shader"])
        else {
            continue;
        };

        let Ok((material_data,)) = scene.asset_query_loaded::<(&HashMap<String, MaterialData>,)>(
            "MaterialAsset",
            &item.material_path,
            &["data"],
        ) else {
            continue;
        };

        let Ok((meshes,)) =
            scene.asset_query_loaded::<(&Vec<Mesh>,)>("ModelAsset", &item.model_path, &["meshes"])
        else {
            continue;
        };

        let transform = item
            .model_transform
            .map(|model_transform| view_projection * model_transform)
            .unwrap_or(Mat4::IDENTITY)
            .to_cols_array_2d();

        // Build Uniforms
        let mut uniforms = dynamic_uniform! {};
        uniforms.add("transform", &transform);
        uniforms.add("normal_transform", &item.normal_transform);

        // Build Uniform from material data
        for (key, value) in material_data.iter() {
            uniforms.add(key, value);
        }

        // Final draw calls
        for mesh in meshes {
            if let Err(err) = surface.draw(
                &mesh.vertices,
                &mesh.indices,
                shader_program,
                &uniforms,
                &draw_params,
            ) {
                warn!(scene, "Failed to draw entity {}: {:?}", item.entity, err);
            }
        }
    }
}

#[attacher(renderer)]
fn renderer_attach(scene: &mut Scene) {
    ensure_opengl_window(scene);
}

#[system(entities=[["Camera"], ["Model"]])]
fn renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a camera
    if entities[0].is_empty() {
        return;
    }

    ensure_opengl_window(scene);

    let Some(camera) = get_camera(scene, entities[0][0]) else {
        return;
    };

    let render_items = collect_render_items(scene, &entities[1]);

    let Ok(window_display) =
        scene.get_resource::<RefCell<(Window, Display)>>(WINDOW_DISPLAY_RESOURCE)
    else {
        return;
    };
    let window_display = window_display.borrow();
    let (window, display) = &*window_display;

    let aspect_ratio: f32 =
        (window.inner_size().width as f32) / (window.inner_size().height as f32);
    let projection = perspective(camera.fov, aspect_ratio, camera.near, camera.far);
    let view_projection = projection * view_matrix(&camera);

    let mut frame = display.draw();
    draw_render_items(scene, &mut frame, view_projection, &render_items);
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

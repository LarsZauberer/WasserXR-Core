//! Keeps the Camera entity in sync with the real headset.
//!
//! Every frame the headset pose (relative to the play-space origin) is read
//! from OpenXR and written into the Camera entity's Transform, placed
//! relative to the entity with the XROrigin component. Game code moves the
//! XROrigin entity; the headset moves the camera around it.

use std::cell::RefCell;

use glam::{EulerRot, Mat4};
use wasserxr::{Uuid, scene::Scene, system};

use crate::{
    renderer::transform_matrix,
    xr::math::pose_matrix,
    xr::renderer::{XR_RENDERER_RESOURCE, XRRenderer},
};

#[system(entities=[["Camera"], ["XROrigin"]])]
fn xr_camera_sync(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Without a camera or an XROrigin there is nothing to sync.
    if entities[0].is_empty() || entities[1].is_empty() {
        return;
    }
    let camera_entity = entities[0][0];
    let origin_entity = entities[1][0];

    // Ask OpenXR where the headset currently is, relative to the play-space
    // origin. Not available before the first frame or when tracking is lost.
    let head_pose = {
        let Ok(renderer) = scene.get_resource::<RefCell<XRRenderer>>(XR_RENDERER_RESOURCE) else {
            return;
        };
        let renderer = renderer.borrow();
        let Some(head_pose) = renderer.locate_head() else {
            return;
        };
        head_pose
    };

    // Where the XROrigin entity sits in the game world.
    let origin_world = scene
        .query::<(&[f32; 3], &[f32; 3])>(origin_entity, "Transform", &["position", "rotation"])
        .map(|(position, rotation)| transform_matrix(*position, *rotation))
        .unwrap_or(Mat4::IDENTITY);

    // Place the headset pose relative to the origin: world <- origin <- head.
    let camera_world = origin_world * pose_matrix(head_pose);

    // Write the result into the camera's Transform, using the same
    // convention the renderer reads: position + XYZ euler angles in degrees.
    let (_, rotation, position) = camera_world.to_scale_rotation_translation();
    let (x, y, z) = rotation.to_euler(EulerRot::XYZ);

    let Ok((camera_position, camera_rotation)) = scene.query_mut::<(&mut [f32; 3], &mut [f32; 3])>(
        camera_entity,
        "Transform",
        &["position", "rotation"],
    ) else {
        return;
    };
    *camera_position = position.to_array();
    *camera_rotation = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
}

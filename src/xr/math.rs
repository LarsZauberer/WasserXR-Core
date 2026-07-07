//! Small conversions between OpenXR pose types and glam matrices.

use glam::{Mat4, Quat, Vec3};

/// Converts an OpenXR pose (position + orientation quaternion) into a
/// transformation matrix.
pub(crate) fn pose_matrix(pose: openxr::Posef) -> Mat4 {
    let rotation = Quat::from_xyzw(
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    );
    let position = Vec3::new(pose.position.x, pose.position.y, pose.position.z);
    Mat4::from_rotation_translation(rotation, position)
}

/// Converts a transformation matrix (rotation + translation only) back into
/// an OpenXR pose.
pub(crate) fn matrix_to_pose(matrix: Mat4) -> openxr::Posef {
    let (_, rotation, position) = matrix.to_scale_rotation_translation();
    openxr::Posef {
        orientation: openxr::Quaternionf {
            x: rotation.x,
            y: rotation.y,
            z: rotation.z,
            w: rotation.w,
        },
        position: openxr::Vector3f {
            x: position.x,
            y: position.y,
            z: position.z,
        },
    }
}

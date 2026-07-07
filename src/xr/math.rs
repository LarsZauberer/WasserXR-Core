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

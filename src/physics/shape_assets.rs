use rapier3d::prelude::{SharedShape, Vector};
use wasserxr::{asset_type, asset_type_creator, scene::Scene};

use crate::model_asset::RawMesh;

#[asset_type]
struct ColliderShapeAsset {
    shape: SharedShape,
}

#[asset_type_creator(ColliderShapeAsset)]
fn create_collider_shape_asset(scene: &mut Scene, data: &str) -> Option<ColliderShapeAsset> {
    let shape = match primitive_shape(data) {
        Some(primitive) => primitive,
        None => {
            let (vertices, indices) = model_mesh_data(scene, data)?;
            SharedShape::trimesh(vertices, indices).ok()?
        }
    };

    Some(ColliderShapeAsset { shape })
}

#[asset_type]
struct RigidBodyShapeAsset {
    shape: SharedShape,
}

#[asset_type_creator(RigidBodyShapeAsset)]
fn create_rigid_body_shape_asset(scene: &mut Scene, data: &str) -> Option<RigidBodyShapeAsset> {
    let shape = match primitive_shape(data) {
        Some(primitive) => primitive,
        None => {
            let (vertices, indices) = model_mesh_data(scene, data)?;
            SharedShape::convex_decomposition(&vertices, &indices)
        }
    };

    Some(RigidBodyShapeAsset { shape })
}

// Data strings matching a rapier primitive name resolve to a unit-sized primitive
// instead of a model file.
pub(crate) fn primitive_shape(data: &str) -> Option<SharedShape> {
    match data {
        "ball" => Some(SharedShape::ball(1.0)),
        "cuboid" => Some(SharedShape::cuboid(1.0, 1.0, 1.0)),
        "capsule" => Some(SharedShape::capsule_y(0.5, 0.5)),
        "cylinder" => Some(SharedShape::cylinder(1.0, 1.0)),
        "cone" => Some(SharedShape::cone(1.0, 1.0)),
        _ => None,
    }
}

pub(crate) fn is_primitive(data: &str) -> bool {
    primitive_shape(data).is_some()
}

fn model_mesh_data(scene: &mut Scene, model: &str) -> Option<(Vec<Vector>, Vec<[u32; 3]>)> {
    scene.ensure_asset_loaded("ModelAsset", model).ok()?;
    let (raw_meshes,) = scene
        .asset_query_loaded::<(&Vec<RawMesh>,)>("ModelAsset", model, &["raw_meshes"])
        .ok()?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for mesh in raw_meshes {
        let base = vertices.len() as u32;
        vertices.extend(
            mesh.vertices
                .iter()
                .map(|vertex| Vector::new(vertex[0], vertex[1], vertex[2])),
        );
        indices.extend(
            mesh.indices
                .chunks_exact(3)
                .map(|triangle| [triangle[0] + base, triangle[1] + base, triangle[2] + base]),
        );
    }

    if vertices.is_empty() || indices.is_empty() {
        return None;
    }

    Some((vertices, indices))
}

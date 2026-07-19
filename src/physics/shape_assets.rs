use rapier3d::prelude::{SharedShape, Vector};
use wasserxr::{asset_type, asset_type_creator, scene::Scene};

use crate::model_asset::RawMesh;

#[asset_type]
struct PhysicsShapeAsset {
    shape: SharedShape,
}

#[asset_type_creator(PhysicsShapeAsset)]
fn create_physics_shape_asset(scene: &mut Scene, data: &str) -> Option<PhysicsShapeAsset> {
    let (vertices, indices) = model_mesh_data(scene, data)?;
    let shape = SharedShape::trimesh(vertices, indices).ok()?;

    Some(PhysicsShapeAsset { shape })
}

#[asset_type]
struct ConvexPhysicsShapeAsset {
    shape: SharedShape,
}

#[asset_type_creator(ConvexPhysicsShapeAsset)]
fn create_convex_physics_shape_asset(
    scene: &mut Scene,
    data: &str,
) -> Option<ConvexPhysicsShapeAsset> {
    let (vertices, indices) = model_mesh_data(scene, data)?;
    let shape = SharedShape::convex_decomposition(&vertices, &indices);

    Some(ConvexPhysicsShapeAsset { shape })
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

use asset_importer::{Importer, postprocess::PostProcessSteps};
use wasserxr::{asset_type, asset_type_creator, scene::Scene, utils::paths::get_asset_path, warn};

pub struct RawMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[asset_type]
struct ModelAsset {
    raw_meshes: Vec<RawMesh>,
}

#[asset_type_creator(ModelAsset)]
fn create_model_asset(scene: &mut Scene, data: &str) -> Option<ModelAsset> {
    let Some(path) = get_asset_path(data) else {
        warn!(scene, "Failed to find the path to the model: {}", data);
        return None;
    };
    let Ok(model_scene) = Importer::new()
        .read_file(path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
        .import()
    else {
        warn!(scene, "Failed to read the model file: {}", data);
        return None;
    };

    let raw_meshes = model_scene
        .meshes()
        .map(|mesh| {
            let vertices = mesh
                .vertices_iter()
                .map(|vertex| [vertex.x, vertex.y, vertex.z])
                .collect::<Vec<[f32; 3]>>();
            let normals = mesh
                .normals_iter()
                .map(|normals| [normals.x, normals.y, normals.z])
                .collect::<Vec<[f32; 3]>>();
            let tex_coords = mesh
                .texture_coords2(0)
                .unwrap_or_default()
                .iter()
                .map(|tex_coord| [tex_coord.x, tex_coord.y])
                .collect::<Vec<[f32; 2]>>();
            let indices = mesh.triangles().into_iter().flatten().collect::<Vec<u32>>();

            RawMesh {
                vertices,
                normals,
                tex_coords,
                indices,
            }
        })
        .collect();

    Some(ModelAsset { raw_meshes })
}

use asset_importer::{Importer, postprocess::PostProcessSteps};
use glium::{
    IndexBuffer, VertexBuffer, index::PrimitiveType::TrianglesList, winit::window::Window,
};
use wasserxr::{asset_type, asset_type_creator, scene::Scene, warn};

use crate::renderer::Display;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coord: [f32; 2],
}

glium::implement_vertex!(Vertex, position, normal, tex_coord);

pub struct Mesh {
    pub vertices: VertexBuffer<Vertex>,
    pub indices: IndexBuffer<u32>,
}

#[asset_type]
struct ModelAsset {
    meshes: Vec<Mesh>,
}

#[asset_type_creator(ModelAsset)]
fn create_model_asset(scene: &mut Scene, path: &str) -> Option<ModelAsset> {
    let Ok(model_scene) = Importer::new()
        .read_file(path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
        .import()
    else {
        warn!(scene, "Failed to read the model file: {}", path);
        return None;
    };

    // Get the OpenGL Context
    let Ok((_, display)) = scene.get_resource::<(Window, Display)>("render_window") else {
        return None;
    };

    let meshes: Vec<Mesh> = model_scene
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
            let tex_coords = mesh.texture_coords2(0).unwrap_or_default();
            let vertices = vertices
                .into_iter()
                .enumerate()
                .map(|(index, position)| Vertex {
                    position,
                    normal: normals.get(index).copied().unwrap_or([0.0, 0.0, 0.0]),
                    tex_coord: tex_coords
                        .get(index)
                        .map(|tex_coord| [tex_coord.x, tex_coord.y])
                        .unwrap_or([0.0, 0.0]),
                })
                .collect::<Vec<Vertex>>();
            let indices = mesh.triangles().into_iter().flatten().collect::<Vec<u32>>();

            let vertices_buffer =
                VertexBuffer::new(display, &vertices).expect("Failed to create vertices buffer");
            let indices_buffer = IndexBuffer::new(display, TrianglesList, &indices)
                .expect("Failed to create index buffer");

            Mesh {
                vertices: vertices_buffer,
                indices: indices_buffer,
            }
        })
        .collect();

    let model = ModelAsset { meshes };

    Some(model)
}

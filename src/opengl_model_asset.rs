use glium::{IndexBuffer, VertexBuffer, index::PrimitiveType::TrianglesList};
use wasserxr::{asset_type, asset_type_creator, scene::Scene, warn};

use crate::{
    model_asset::RawMesh,
    opengl::{OPENGL_WINDOW_RESOURCE, OpenGLWindow},
};

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
struct OpenGLModelAsset {
    meshes: Vec<Mesh>,
}

#[asset_type_creator(OpenGLModelAsset)]
fn create_opengl_model_asset(scene: &mut Scene, data: &str) -> Option<OpenGLModelAsset> {
    if scene.ensure_asset_loaded("ModelAsset", data).is_err() {
        warn!(scene, "Failed to load the model data: {}", data);
        return None;
    }

    // Get the OpenGL Context
    let Ok(opengl_window) = scene.get_resource::<OpenGLWindow>(OPENGL_WINDOW_RESOURCE) else {
        return None;
    };
    let display = &opengl_window.display;

    let Ok((raw_meshes,)) =
        scene.asset_query_loaded::<(&Vec<RawMesh>,)>("ModelAsset", data, &["raw_meshes"])
    else {
        return None;
    };

    let meshes = raw_meshes
        .iter()
        .map(|raw_mesh| {
            let vertices = raw_mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(index, position)| Vertex {
                    position: *position,
                    normal: raw_mesh
                        .normals
                        .get(index)
                        .copied()
                        .unwrap_or([0.0, 0.0, 0.0]),
                    tex_coord: raw_mesh
                        .tex_coords
                        .get(index)
                        .copied()
                        .unwrap_or([0.0, 0.0]),
                })
                .collect::<Vec<Vertex>>();

            let vertices_buffer =
                VertexBuffer::new(display, &vertices).expect("Failed to create vertices buffer");
            let indices_buffer = IndexBuffer::new(display, TrianglesList, &raw_mesh.indices)
                .expect("Failed to create index buffer");

            Mesh {
                vertices: vertices_buffer,
                indices: indices_buffer,
            }
        })
        .collect();

    Some(OpenGLModelAsset { meshes })
}

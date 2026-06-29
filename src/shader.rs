use std::fs;

use glium::Program;
use wasserxr::{asset_type, asset_type_creator, scene::Scene, warn};

use crate::window::component::Display;

#[asset_type]
struct Shader {
    shader: Program,
}

#[asset_type_creator(Shader)]
fn shader_creator(scene: &mut Scene, path: &str) -> Option<Shader> {
    let vertex_path = path.to_owned() + ".vert";
    let fragment_path = path.to_owned() + ".frag";

    let Ok(vertex) = fs::read_to_string(&vertex_path) else {
        warn!(
            scene,
            "Failed to find the vertex shader code: {}", vertex_path
        );
        return None;
    };
    let Ok(fragment) = fs::read_to_string(&fragment_path) else {
        warn!(
            scene,
            "Failed to find the fragment shader code: {}", fragment_path
        );
        return None;
    };

    // Get the opengl context
    let window_entity = scene.get_entity_with_component("Window")?;
    let (display,) = scene
        .query::<(&Display,)>(window_entity, "Window", &["display"])
        .ok()?;

    let program = match Program::from_source(display, &vertex, &fragment, None) {
        Ok(program) => program,
        Err(glium::ProgramCreationError::CompilationError(msg, t)) => {
            let shader_type = match t {
                glium::program::ShaderType::Vertex => "Vertex Shader",
                glium::program::ShaderType::Fragment => "Fragment Shader",
                _ => "Shader",
            };
            warn!(
                scene,
                "Failed to compile {} `{}`: {}", shader_type, path, msg
            );
            return None;
        }
        Err(glium::ProgramCreationError::LinkingError(msg)) => {
            warn!(scene, "Failed to link the shader `{}`: {}", path, msg);
            return None;
        }
        Err(err) => {
            warn!(
                scene,
                "Unknown error while shader program `{}` building: {:?}", path, err
            );
            return None;
        }
    };
    Some(Shader { shader: program })
}

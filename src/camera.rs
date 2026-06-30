use wasserxr::{component, component_creator, scene::Scene};

#[component]
struct Camera {
    #[mutable]
    fov: f32,

    #[mutable]
    near: f32,

    #[mutable]
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 90.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

#[component_creator(Camera)]
fn create_transform(_scene: &mut Scene) -> Option<Camera> {
    Some(Camera::default())
}

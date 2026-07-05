use wasserxr::{component, component_creator, scene::Scene};

#[component]
pub struct Transform {
    #[mutable]
    pub position: [f32; 3],

    #[mutable]
    pub rotation: [f32; 3],

    #[mutable]
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

#[component_creator(Transform)]
fn create_transform(_scene: &mut Scene) -> Option<Transform> {
    Some(Transform::default())
}

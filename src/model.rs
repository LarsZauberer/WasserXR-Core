use wasserxr::{component, component_creator, scene::Scene};

#[component]
#[derive(Default)]
struct Model {
    #[mutable]
    model: String,

    #[mutable]
    shader: String,
}

#[component_creator(Model)]
fn create_model(_scene: &mut Scene) -> Option<Model> {
    Some(Model::default())
}

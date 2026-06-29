use wasserxr::component;

#[component]
#[derive(Default)]
struct Model {
    #[mutable]
    model: String,

    #[mutable]
    shader: String,
}

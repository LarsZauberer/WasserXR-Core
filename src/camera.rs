use wasserxr::component;

#[component]
#[derive(Default)]
struct Camera {
    #[mutable]
    fov: f32,

    #[mutable]
    near: f32,

    #[mutable]
    far: f32,
}

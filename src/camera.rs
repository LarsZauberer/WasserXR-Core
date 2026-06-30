use wasserxr::component;

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

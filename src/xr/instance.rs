use std::cell::RefCell;

use wasserxr::scene::Scene;

pub struct XRInstance(openxr::Instance);

impl XRInstance {
    fn new() -> Self {
        let core_version = core_version();
        let engine_version = engine_version();
        let entry = unsafe { openxr::Entry::load().expect("Failed to load OpenXR loader") };
        let mut extensions = openxr::ExtensionSet::default();
        extensions.khr_opengl_enable = true;
        let instance = entry
            .create_instance(
                &openxr::ApplicationInfo {
                    // TODO: Provide a way to specify the application_name.
                    application_name: "WasserXR",
                    application_version: version_u32(core_version),
                    engine_name: "WasserXR",
                    engine_version: version_u32(engine_version),
                    api_version: version_openxr(engine_version),
                },
                &extensions,
                &[],
            )
            .expect("Failed to create OpenXR instance");

        Self(instance)
    }

    pub fn instance(&self) -> &openxr::Instance {
        &self.0
    }
}

fn core_version() -> (u16, u16, u32) {
    (
        env!("CARGO_PKG_VERSION_MAJOR")
            .parse()
            .expect("Invalid WasserXR-Core major version"),
        env!("CARGO_PKG_VERSION_MINOR")
            .parse()
            .expect("Invalid WasserXR-Core minor version"),
        env!("CARGO_PKG_VERSION_PATCH")
            .parse()
            .expect("Invalid WasserXR-Core patch version"),
    )
}

fn engine_version() -> (u16, u16, u32) {
    let version = include_str!("../../Cargo.toml")
        .split("wasserxr = { version = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("Failed to read WasserXR dependency version");

    parse_version(version)
}

fn parse_version(version: &str) -> (u16, u16, u32) {
    let mut parts = version.split('.');

    (
        parts
            .next()
            .expect("Missing major version")
            .parse()
            .expect("Invalid major version"),
        parts
            .next()
            .expect("Missing minor version")
            .parse()
            .expect("Invalid minor version"),
        parts
            .next()
            .expect("Missing patch version")
            .parse()
            .expect("Invalid patch version"),
    )
}

fn version_u32(version: (u16, u16, u32)) -> u32 {
    u32::from(version.0) * 1_000_000 + u32::from(version.1) * 1_000 + version.2
}

fn version_openxr(version: (u16, u16, u32)) -> openxr::Version {
    openxr::Version::new(version.0, version.1, version.2)
}

pub fn ensure_xrinstance(scene: &mut Scene) {
    if scene
        .get_resource::<RefCell<XRInstance>>("xrinstance")
        .is_err()
    {
        let _ = scene.add_resource("xrinstance".to_owned(), RefCell::new(XRInstance::new()));
    }
}

use std::cell::RefCell;

use wasserxr::{info, scene::Scene};

pub struct XRInstance(openxr::Instance);

impl XRInstance {
    fn new() -> Result<Self, String> {
        let core_version = core_version()?;
        let engine_version = engine_version()?;
        let entry = unsafe { openxr::Entry::load() }
            .map_err(|err| format!("Failed to load OpenXR loader: {err}"))?;
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
                    api_version: version_openxr(),
                },
                &extensions,
                &[],
            )
            .map_err(|err| format!("Failed to create OpenXR instance: {err}"))?;

        Ok(Self(instance))
    }

    pub fn instance(&self) -> &openxr::Instance {
        &self.0
    }
}

fn core_version() -> Result<(u16, u16, u32), String> {
    Ok((
        env!("CARGO_PKG_VERSION_MAJOR")
            .parse()
            .map_err(|err| format!("Invalid WasserXR-Core major version: {err}"))?,
        env!("CARGO_PKG_VERSION_MINOR")
            .parse()
            .map_err(|err| format!("Invalid WasserXR-Core minor version: {err}"))?,
        env!("CARGO_PKG_VERSION_PATCH")
            .parse()
            .map_err(|err| format!("Invalid WasserXR-Core patch version: {err}"))?,
    ))
}

fn engine_version() -> Result<(u16, u16, u32), String> {
    let manifest = include_str!("../../Cargo.toml");
    let version = manifest
        .split("wasserxr = { version = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .or_else(|| {
            manifest
                .split("[dependencies.wasserxr]")
                .nth(1)?
                .lines()
                .find_map(|line| line.trim().strip_prefix("version = \"")?.split('"').next())
        })
        .ok_or_else(|| "Failed to read WasserXR dependency version".to_owned())?;

    parse_version(version)
}

fn parse_version(version: &str) -> Result<(u16, u16, u32), String> {
    let mut parts = version.split('.');

    Ok((
        parts
            .next()
            .ok_or_else(|| "Missing major version".to_owned())?
            .parse()
            .map_err(|err| format!("Invalid major version: {err}"))?,
        parts
            .next()
            .ok_or_else(|| "Missing minor version".to_owned())?
            .parse()
            .map_err(|err| format!("Invalid minor version: {err}"))?,
        parts
            .next()
            .ok_or_else(|| "Missing patch version".to_owned())?
            .parse()
            .map_err(|err| format!("Invalid patch version: {err}"))?,
    ))
}

fn version_u32(version: (u16, u16, u32)) -> u32 {
    u32::from(version.0) * 1_000_000 + u32::from(version.1) * 1_000 + version.2
}

fn version_openxr() -> openxr::Version {
    openxr::Version::new(1, 0, 0)
}

pub fn ensure_xrinstance(scene: &mut Scene) -> Result<(), String> {
    if scene
        .get_resource::<RefCell<XRInstance>>("xrinstance")
        .is_err()
    {
        let instance = XRInstance::new()?;
        let runtime = instance
            .instance()
            .properties()
            .map_err(|err| format!("Failed to get OpenXR instance properties: {err}"))?;
        let core_version = core_version()?;
        let engine_version = engine_version()?;
        info!(
            scene,
            "OpenXR instance created\n\tapplication_name: WasserXR\n\tapplication_version: {}\n\tengine_name: WasserXR\n\tengine_version: {}\n\tapi_version: {}\n\truntime_name: {}\n\truntime_version: {}",
            version_u32(core_version),
            version_u32(engine_version),
            version_openxr(),
            runtime.runtime_name,
            runtime.runtime_version
        );
        scene
            .add_resource("xrinstance".to_owned(), RefCell::new(instance))
            .map_err(|err| format!("Failed to add OpenXR instance resource: {err:?}"))?;
    }

    Ok(())
}

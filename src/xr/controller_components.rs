use serde::{Deserialize, Serialize};
use wasserxr::{component, component_creator, scene::Scene};

/// Which hand a controller entity belongs to.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum XRControllerType {
    LeftHandController,
    RightHandController,
}

/// Tags an entity as the visible representation of a VR controller.
/// The xr_controller_sync system keeps the Transform of tagged entities at
/// the real controller pose, creating the entities if they are missing.
///
/// The field is named `controller_type` because `type` is a Rust keyword
/// and the component macro cannot generate bindings for it.
#[component]
pub struct XRController {
    #[mutable]
    pub controller_type: XRControllerType,
}

#[component_creator(XRController)]
fn create_xr_controller(_scene: &mut Scene) -> Option<XRController> {
    Some(XRController {
        controller_type: XRControllerType::LeftHandController,
    })
}

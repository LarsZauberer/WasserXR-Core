use wasserxr::{component, component_creator, scene::Scene};

/// Marks the entity whose Transform anchors the XR play space in the game
/// world: the origin of the headset's tracking space is placed exactly at
/// this entity. Moving this entity moves the whole play space (and with it
/// the player) through the world.
#[component]
struct XROrigin {}

#[component_creator(XROrigin)]
fn create_xr_origin(_scene: &mut Scene) -> Option<XROrigin> {
    Some(XROrigin {})
}

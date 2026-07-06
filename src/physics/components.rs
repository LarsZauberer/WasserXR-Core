use wasserxr::{component, component_creator, scene::Scene};

#[component]
#[derive(Default)]
struct BoxCollider {}

#[component_creator(BoxCollider)]
fn create_box_collider(_scene: &mut Scene) -> Option<BoxCollider> {
    Some(BoxCollider::default())
}

#[component]
#[derive(Default)]
struct RigidBox {}

#[component_creator(RigidBox)]
fn create_rigid_box(_scene: &mut Scene) -> Option<RigidBox> {
    Some(RigidBox::default())
}

#[component]
struct PhysicsEngine {
    #[mutable]
    gravity: [f32; 3],
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
        }
    }
}

#[component_creator(PhysicsEngine)]
fn create_physics_engine(_scene: &mut Scene) -> Option<PhysicsEngine> {
    Some(PhysicsEngine::default())
}

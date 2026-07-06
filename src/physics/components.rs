use wasserxr::{component, component_creator, scene::Scene};

#[component]
struct BoxCollider {
    #[mutable]
    scale: [f32; 3],
}

impl Default for BoxCollider {
    fn default() -> Self {
        Self {
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[component_creator(BoxCollider)]
fn create_box_collider(_scene: &mut Scene) -> Option<BoxCollider> {
    Some(BoxCollider::default())
}

#[component]
struct RigidBox {
    #[mutable]
    scale: [f32; 3],
}

impl Default for RigidBox {
    fn default() -> Self {
        Self {
            scale: [1.0, 1.0, 1.0],
        }
    }
}

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

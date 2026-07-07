use wasserxr::{component, component_creator, scene::Scene};

#[component]
struct Collider {
    #[mutable]
    scale: [f32; 3],

    #[mutable]
    model: String,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            scale: [1.0, 1.0, 1.0],
            model: "./models/cube.obj".to_owned(),
        }
    }
}

#[component_creator(Collider)]
fn create_collider(_scene: &mut Scene) -> Option<Collider> {
    Some(Collider::default())
}

#[component]
struct RigidBody {
    #[mutable]
    scale: [f32; 3],

    #[mutable]
    model: String,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            scale: [1.0, 1.0, 1.0],
            model: "./models/cube.obj".to_owned(),
        }
    }
}

#[component_creator(RigidBody)]
fn create_rigid_body(_scene: &mut Scene) -> Option<RigidBody> {
    Some(RigidBody::default())
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

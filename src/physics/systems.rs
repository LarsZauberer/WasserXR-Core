use std::collections::HashMap;

use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, PhysicsWorld as RapierPhysicsWorld, Pose, RigidBodyBuilder,
    RigidBodyHandle, Rotation, SharedShape, Vector, glamx::EulerRot,
};
use wasserxr::{Uuid, scene::Scene, system};

const PHYSICS_WORLD_RESOURCE: &str = "physics_world";

#[derive(Default)]
pub(crate) struct PhysicsWorld {
    world: RapierPhysicsWorld,
    colliders: HashMap<Uuid, ColliderHandle>,
    rigid_boxes: HashMap<Uuid, (RigidBodyHandle, ColliderHandle)>,
}

#[derive(Clone, Copy)]
struct TransformData {
    entity: Uuid,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
}

pub(crate) fn ensure_physics_world(scene: &mut Scene) {
    if scene
        .get_resource::<PhysicsWorld>(PHYSICS_WORLD_RESOURCE)
        .is_err()
    {
        let _ = scene.add_resource(PHYSICS_WORLD_RESOURCE.to_owned(), PhysicsWorld::default());
    }
}

#[system(entities=[["PhysicsEngine"], ["BoxCollider", "Transform"], ["RigidBox", "Transform"]])]
fn physics(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    if entities[0].is_empty() {
        return;
    }

    let gravity = scene
        .query::<(&[f32; 3],)>(entities[0][0], "PhysicsEngine", &["gravity"])
        .map(|(gravity,)| *gravity)
        .unwrap_or([0.0, -9.81, 0.0]);

    let colliders = collect_transforms(scene, &entities[1], "BoxCollider");
    let rigid_boxes = collect_transforms(scene, &entities[2], "RigidBox");

    ensure_physics_world(scene);

    let updates = {
        let Ok(world) = scene.get_mut_resource::<PhysicsWorld>(PHYSICS_WORLD_RESOURCE) else {
            return;
        };

        world.world.gravity = Vector::new(gravity[0], gravity[1], gravity[2]);
        world.sync_colliders(&colliders);
        world.sync_rigid_boxes(&rigid_boxes);
        world.world.step();
        world.rigid_box_updates()
    };

    for (entity, position, rotation) in updates {
        let Ok((transform_position, transform_rotation)) = scene
            .query_mut::<(&mut [f32; 3], &mut [f32; 3])>(
                entity,
                "Transform",
                &["position", "rotation"],
            )
        else {
            continue;
        };

        *transform_position = position;
        *transform_rotation = rotation;
    }
}

impl PhysicsWorld {
    fn sync_colliders(&mut self, transforms: &[TransformData]) {
        let stale_entities: Vec<_> = self
            .colliders
            .keys()
            .filter(|entity| {
                !transforms
                    .iter()
                    .any(|transform| transform.entity == **entity)
            })
            .copied()
            .collect();

        for entity in stale_entities {
            if let Some(handle) = self.colliders.remove(&entity) {
                let _ = self.world.remove_collider(handle);
            }
        }

        for transform in transforms {
            let pose = pose_from_transform(*transform);
            if let Some(handle) = self.colliders.get(&transform.entity) {
                if let Some(collider) = self.world.colliders.get_mut(*handle) {
                    collider.set_position(pose);
                    collider.set_shape(cuboid_shape(transform.scale));
                }
            } else {
                let handle = self.world.insert_collider(
                    ColliderBuilder::new(cuboid_shape(transform.scale)).position(pose),
                    None,
                );
                self.colliders.insert(transform.entity, handle);
            }
        }
    }

    fn sync_rigid_boxes(&mut self, transforms: &[TransformData]) {
        let stale_entities: Vec<_> = self
            .rigid_boxes
            .keys()
            .filter(|entity| {
                !transforms
                    .iter()
                    .any(|transform| transform.entity == **entity)
            })
            .copied()
            .collect();

        for entity in stale_entities {
            if let Some((body, _)) = self.rigid_boxes.remove(&entity) {
                let _ = self.world.remove_body(body);
            }
        }

        for transform in transforms {
            let pose = pose_from_transform(*transform);
            if let Some((body_handle, collider_handle)) = self.rigid_boxes.get(&transform.entity) {
                if let Some(body) = self.world.bodies.get_mut(*body_handle) {
                    // TODO: Optimization later to not always wake up every physics entity at every
                    // tick. Check if the position has really changed
                    body.set_position(pose, true);
                }
                if let Some(collider) = self.world.colliders.get_mut(*collider_handle) {
                    collider.set_shape(cuboid_shape(transform.scale));
                }
            } else {
                let (body, collider) = self.world.insert(
                    RigidBodyBuilder::dynamic().pose(pose),
                    ColliderBuilder::new(cuboid_shape(transform.scale)),
                );
                self.rigid_boxes.insert(transform.entity, (body, collider));
            }
        }
    }

    fn rigid_box_updates(&self) -> Vec<(Uuid, [f32; 3], [f32; 3])> {
        self.rigid_boxes
            .iter()
            .filter_map(|(entity, (body_handle, _))| {
                let body = self.world.bodies.get(*body_handle)?;
                let position = body.translation();
                let (x, y, z) = body.rotation().to_euler(EulerRot::XYZ);

                Some((
                    *entity,
                    [position.x, position.y, position.z],
                    [x.to_degrees(), y.to_degrees(), z.to_degrees()],
                ))
            })
            .collect()
    }
}

fn collect_transforms(scene: &Scene, entities: &[Uuid], component: &str) -> Vec<TransformData> {
    entities
        .iter()
        .filter_map(|entity| {
            let Ok((position, rotation)) = scene.query::<(&[f32; 3], &[f32; 3])>(
                *entity,
                "Transform",
                &["position", "rotation"],
            ) else {
                return None;
            };

            // The box size lives on the collider/rigidbox component, not the Transform.
            let Ok((scale,)) = scene.query::<(&[f32; 3],)>(*entity, component, &["scale"]) else {
                return None;
            };

            Some(TransformData {
                entity: *entity,
                position: *position,
                rotation: *rotation,
                scale: *scale,
            })
        })
        .collect()
}

fn pose_from_transform(transform: TransformData) -> Pose {
    Pose::from_parts(
        Vector::new(
            transform.position[0],
            transform.position[1],
            transform.position[2],
        ),
        Rotation::from_euler(
            EulerRot::XYZ,
            transform.rotation[0].to_radians(),
            transform.rotation[1].to_radians(),
            transform.rotation[2].to_radians(),
        ),
    )
}

fn cuboid_shape(scale: [f32; 3]) -> SharedShape {
    SharedShape::cuboid(
        half_extent(scale[0]),
        half_extent(scale[1]),
        half_extent(scale[2]),
    )
}

// cube.obj is 2 units wide, so a component scale maps 1:1 to the rapier cuboid
// half-extent: full collider size = 2 * scale = the rendered size at that Transform scale.
fn half_extent(scale: f32) -> f32 {
    scale.abs().max(0.001)
}

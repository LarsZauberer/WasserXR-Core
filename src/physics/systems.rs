use std::collections::HashMap;

use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, PhysicsWorld as RapierPhysicsWorld, Pose, RigidBodyBuilder,
    RigidBodyHandle, Rotation, SharedShape, Vector, glamx::EulerRot,
};
use wasserxr::{Uuid, scene::Scene, system};

use crate::utils::object_sync::sync_objects;

const PHYSICS_WORLD_RESOURCE: &str = "physics_world";

#[derive(Default)]
pub(crate) struct PhysicsWorld {
    world: RapierPhysicsWorld,
    colliders: HashMap<Uuid, TrackedCollider>,
    rigid_bodies: HashMap<Uuid, TrackedRigidBody>,
}

struct TrackedCollider {
    handle: ColliderHandle,
    model: String,
    scale: [f32; 3],
}

struct TrackedRigidBody {
    body: RigidBodyHandle,
    collider: ColliderHandle,
    model: String,
    scale: [f32; 3],
}

struct TransformData {
    entity: Uuid,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    model: String,
    shape: SharedShape,
}

pub(crate) fn ensure_physics_world(scene: &mut Scene) {
    if scene
        .get_resource::<PhysicsWorld>(PHYSICS_WORLD_RESOURCE)
        .is_err()
    {
        let _ = scene.add_resource(PHYSICS_WORLD_RESOURCE.to_owned(), PhysicsWorld::default());
    }
}

#[system(entities=[["PhysicsEngine"], ["Collider", "Transform"], ["RigidBody", "Transform"]])]
fn physics(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    if entities[0].is_empty() {
        return;
    }

    let gravity = scene
        .query::<(&[f32; 3],)>(entities[0][0], "PhysicsEngine", &["gravity"])
        .map(|(gravity,)| *gravity)
        .unwrap_or([0.0, -9.81, 0.0]);

    let colliders = collect_transforms(scene, &entities[1], "Collider", "ColliderShapeAsset");
    let rigid_bodies = collect_transforms(scene, &entities[2], "RigidBody", "RigidBodyShapeAsset");

    ensure_physics_world(scene);

    let updates = {
        let Ok(world) = scene.get_mut_resource::<PhysicsWorld>(PHYSICS_WORLD_RESOURCE) else {
            return;
        };

        world.world.gravity = Vector::new(gravity[0], gravity[1], gravity[2]);
        world.sync_colliders(&colliders);
        world.sync_rigid_bodies(&rigid_bodies);
        world.world.step();
        world.rigid_body_updates()
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
        sync_objects(
            &mut self.world,
            &mut self.colliders,
            transforms,
            |transform| transform.entity,
            |world, transform| TrackedCollider {
                handle: world.insert_collider(
                    ColliderBuilder::new(scaled_shape(transform))
                        .position(pose_from_transform(transform)),
                    None,
                ),
                model: transform.model.clone(),
                scale: transform.scale,
            },
            |world, tracked| {
                let _ = world.remove_collider(tracked.handle);
            },
            |world, transform, tracked| {
                if let Some(collider) = world.colliders.get_mut(tracked.handle) {
                    collider.set_position(pose_from_transform(transform));

                    // Rescaling a shape is expensive, only do it when the shape changed
                    if tracked.model != transform.model || tracked.scale != transform.scale {
                        collider.set_shape(scaled_shape(transform));
                        tracked.model = transform.model.clone();
                        tracked.scale = transform.scale;
                    }
                }
            },
        );
    }

    fn sync_rigid_bodies(&mut self, transforms: &[TransformData]) {
        sync_objects(
            &mut self.world,
            &mut self.rigid_bodies,
            transforms,
            |transform| transform.entity,
            |world, transform| {
                let (body, collider) = world.insert(
                    RigidBodyBuilder::dynamic().pose(pose_from_transform(transform)),
                    ColliderBuilder::new(scaled_shape(transform)),
                );
                TrackedRigidBody {
                    body,
                    collider,
                    model: transform.model.clone(),
                    scale: transform.scale,
                }
            },
            |world, tracked| {
                let _ = world.remove_body(tracked.body);
            },
            |world, transform, tracked| {
                if let Some(body) = world.bodies.get_mut(tracked.body) {
                    // TODO: Optimization later to not always wake up every physics entity at every
                    // tick. Check if the position has really changed
                    body.set_position(pose_from_transform(transform), true);
                }

                // Rescaling a shape is expensive, only do it when the shape changed
                if tracked.model != transform.model || tracked.scale != transform.scale {
                    if let Some(collider) = world.colliders.get_mut(tracked.collider) {
                        collider.set_shape(scaled_shape(transform));
                    }
                    tracked.model = transform.model.clone();
                    tracked.scale = transform.scale;
                }
            },
        );
    }

    fn rigid_body_updates(&self) -> Vec<(Uuid, [f32; 3], [f32; 3])> {
        self.rigid_bodies
            .iter()
            .filter_map(|(entity, tracked)| {
                let body = self.world.bodies.get(tracked.body)?;
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

fn collect_transforms(
    scene: &mut Scene,
    entities: &[Uuid],
    component: &str,
    shape_asset: &str,
) -> Vec<TransformData> {
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
            let position = *position;
            let rotation = *rotation;

            // The shape size and model live on the collider/rigidbody component, not the
            // Transform.
            let Ok((scale, model)) =
                scene.query::<(&[f32; 3], &String)>(*entity, component, &["scale", "model"])
            else {
                return None;
            };
            let scale = *scale;
            let model = model.clone();

            if model.is_empty() || scene.ensure_asset_loaded(shape_asset, &model).is_err() {
                return None;
            }

            let Ok((shape,)) =
                scene.asset_query_loaded::<(&SharedShape,)>(shape_asset, &model, &["shape"])
            else {
                return None;
            };
            let shape = shape.clone();

            Some(TransformData {
                entity: *entity,
                position,
                rotation,
                scale,
                model,
                shape,
            })
        })
        .collect()
}

fn pose_from_transform(transform: &TransformData) -> Pose {
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

// Number of subdivisions used when scaling turns a round shape (e.g. a non-uniformly
// scaled ball) into an approximated convex mesh.
const SCALE_SUBDIVISIONS: u32 = 20;

// The cached asset shape is used 1:1 with the component scale, matching a rendered
// model at that Transform scale.
fn scaled_shape(transform: &TransformData) -> SharedShape {
    if transform.scale == [1.0, 1.0, 1.0] {
        return transform.shape.clone();
    }

    let scale = Vector::new(transform.scale[0], transform.scale[1], transform.scale[2]);
    transform
        .shape
        .scale_dyn(scale, SCALE_SUBDIVISIONS)
        .map(|shape| SharedShape(shape.into()))
        .unwrap_or_else(|| transform.shape.clone())
}

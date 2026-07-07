use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use wasserxr::{Uuid, debug, detacher, info, scene::Scene, system};

use crate::utils::object_sync::sync_objects;

static DEBUG_COLLIDER: LazyLock<Mutex<HashMap<Uuid, Uuid>>> =
    LazyLock::new(|| Mutex::new(HashMap::default()));
static DEBUG_RIGID: LazyLock<Mutex<HashMap<Uuid, Uuid>>> =
    LazyLock::new(|| Mutex::new(HashMap::default()));

#[system(entities=[["BoxCollider", "Transform"], ["RigidBox", "Transform"]])]
fn debug_physics(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    let Ok(mut colliders) = DEBUG_COLLIDER.lock() else {
        return;
    };
    let Ok(mut rigids) = DEBUG_RIGID.lock() else {
        return;
    };

    // A hot-reload serializes the scene together with the debug entities we spawned and
    // resurrects them while our tracking maps come back empty. Drop any untracked
    // `_physics_debug` leftovers so we don't duplicate them.
    remove_orphan_debug_entities(scene, &colliders, &rigids);

    sync_debug_entities(
        scene,
        &mut colliders,
        &entities[0],
        "BoxCollider",
        "./models/cube.obj",
        "./materials/debug.json",
    );

    sync_debug_entities(
        scene,
        &mut rigids,
        &entities[1],
        "RigidBox",
        "./models/cube.obj",
        "./materials/debug.json",
    );
}

#[detacher(debug_physics)]
fn detach_debug_physics(scene: &mut Scene) {
    debug!(scene, "Getting Locks!");
    let Ok(mut colliders) = DEBUG_COLLIDER.lock() else {
        return;
    };
    let Ok(mut rigids) = DEBUG_RIGID.lock() else {
        return;
    };

    for entity in colliders.values() {
        let _ = scene.remove_entity(*entity);
    }
    colliders.clear();

    for entity in rigids.values() {
        let _ = scene.remove_entity(*entity);
    }
    rigids.clear();

    info!(scene, "Cleared all the Colliders and Rigids!");
}

fn sync_debug_entities(
    scene: &mut Scene,
    map: &mut HashMap<Uuid, Uuid>,
    entities: &[Uuid],
    component: &str,
    model: &str,
    material: &str,
) {
    sync_objects(
        scene,
        map,
        entities,
        |entity| *entity,
        |scene, entity| {
            let debug_entity = scene.add_entity();
            if let Ok(name) = scene.get_entity_name(*entity) {
                let new_name = name.to_owned() + "_physics_debug";
                let _ = scene.set_entity_name(debug_entity, new_name);
            }
            debug_entity
        },
        |scene, debug_entity| {
            let _ = scene.remove_entity(debug_entity);
        },
        |scene, entity, debug_entity| {
            update_debug_entity(scene, *entity, *debug_entity, component, model, material);
        },
    );
}

// Make sure the debug entity has its model/material set with the correct transform
fn update_debug_entity(
    scene: &mut Scene,
    entity: Uuid,
    debug_entity: Uuid,
    component: &str,
    model: &str,
    material: &str,
) {
    ensure_component_exists(scene, debug_entity, "Model");
    ensure_component_exists(scene, debug_entity, "Transform");

    let Ok((entity_model, entity_material)) = scene.query_mut::<(&mut String, &mut String)>(
        debug_entity,
        "Model",
        &["model", "material"],
    ) else {
        return;
    };

    *entity_model = model.to_owned();
    *entity_material = material.to_owned();

    let Ok((position, rotation)) =
        scene.query::<(&[f32; 3], &[f32; 3])>(entity, "Transform", &["position", "rotation"])
    else {
        return;
    };

    let position = *position;
    let rotation = *rotation;

    // The box size lives on the collider/rigidbox component and is used 1:1 with
    // the Transform scale (cube.obj is 2 units, matching the collider's full size).
    let Ok((scale,)) = scene.query::<(&[f32; 3],)>(entity, component, &["scale"]) else {
        return;
    };
    let scale = *scale;

    let Ok((debug_position, debug_rotation, debug_scale)) =
        scene.query_mut::<(&mut [f32; 3], &mut [f32; 3], &mut [f32; 3])>(
            debug_entity,
            "Transform",
            &["position", "rotation", "scale"],
        )
    else {
        return;
    };

    *debug_position = position;
    *debug_rotation = rotation;
    *debug_scale = scale;
}

fn remove_orphan_debug_entities(
    scene: &mut Scene,
    colliders: &HashMap<Uuid, Uuid>,
    rigids: &HashMap<Uuid, Uuid>,
) {
    for entity in scene.get_entities() {
        let tracked = colliders
            .values()
            .chain(rigids.values())
            .any(|debug| *debug == entity);

        if !tracked
            && scene
                .get_entity_name(entity)
                .is_ok_and(|name| name.ends_with("_physics_debug"))
        {
            let _ = scene.remove_entity(entity);
        }
    }
}

fn ensure_component_exists(scene: &mut Scene, entity: Uuid, component_id: &str) {
    if !scene.has_component(entity, component_id) {
        let _ = scene.add_component(entity, component_id.to_owned());
    }
}

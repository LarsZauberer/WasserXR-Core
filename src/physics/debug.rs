use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use wasserxr::{Uuid, debug, detacher, info, scene::Scene, system};

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
    // Update the entities
    let stale_entities: Vec<Uuid> = map
        .keys()
        .filter(|id| !entities.contains(id))
        .copied()
        .collect();
    let new_entities: Vec<Uuid> = entities
        .iter()
        .filter(|id| !map.contains_key(id))
        .copied()
        .collect();

    for entity in stale_entities {
        let Some(debug_entity) = map.remove(&entity) else {
            continue;
        };
        let _ = scene.remove_entity(debug_entity);
    }

    for entity in new_entities {
        let debug_entity = scene.add_entity();
        if let Ok(name) = scene.get_entity_name(entity) {
            let new_name = name.to_owned() + "_physics_debug";
            let _ = scene.set_entity_name(debug_entity, new_name);
        }
        let _ = map.insert(entity, debug_entity);
    }

    // Make sure all the debug entities have their model/material set with their correct transform
    for (entity, debug_entity) in map.iter() {
        ensure_component_exists(scene, *debug_entity, "Model");
        ensure_component_exists(scene, *debug_entity, "Transform");

        {
            let Ok((entity_model, entity_material)) = scene
                .query_mut::<(&mut String, &mut String)>(
                    *debug_entity,
                    "Model",
                    &["model", "material"],
                )
            else {
                continue;
            };

            *entity_model = model.to_owned();
            *entity_material = material.to_owned();
        }

        {
            let Ok((position, rotation)) = scene.query::<(&[f32; 3], &[f32; 3])>(
                *entity,
                "Transform",
                &["position", "rotation"],
            ) else {
                continue;
            };

            let position = *position;
            let rotation = *rotation;

            // The box size lives on the collider/rigidbox component. cube.obj spans
            // 2 units, so halve the scale to match the collider's world-space size.
            let Ok((scale,)) = scene.query::<(&[f32; 3],)>(*entity, component, &["scale"]) else {
                continue;
            };
            let scale = [scale[0] * 0.5, scale[1] * 0.5, scale[2] * 0.5];

            let Ok((debug_position, debug_rotation, debug_scale)) =
                scene.query_mut::<(&mut [f32; 3], &mut [f32; 3], &mut [f32; 3])>(
                    *debug_entity,
                    "Transform",
                    &["position", "rotation", "scale"],
                )
            else {
                continue;
            };

            *debug_position = position;
            *debug_rotation = rotation;
            *debug_scale = scale;
        }
    }
}

fn ensure_component_exists(scene: &mut Scene, entity: Uuid, component_id: &str) {
    if !scene.has_component(entity, component_id) {
        let _ = scene.add_component(entity, component_id.to_owned());
    }
}

//! Makes the VR controllers visible in the scene.
//!
//! OpenXR never hands out controller poses directly. Input goes through
//! *actions*: the application declares an abstract action ("hand pose"),
//! suggests which physical input it maps to per controller type (an
//! *interaction profile*), and attaches everything to the session. Each hand
//! then gets a *space* that follows the controller and can be located like
//! the headset spaces.
//!
//! This system mirrors those poses into two scene entities (one per hand,
//! tagged with the XRController component) with a Transform and a Model, so
//! the normal renderers draw the controllers like any other object.

use std::cell::RefCell;

use glam::EulerRot;
use wasserxr::{Uuid, scene::Scene, system};

use crate::{
    renderer::transform_matrix,
    xr::controller::XRControllerType,
    xr::math::pose_matrix,
    xr::renderer::{XR_RENDERER_RESOURCE, XRRenderer},
    xr::session::{XRSession, ensure_xrsession},
};

pub(crate) const XR_CONTROLLERS_RESOURCE: &str = "xr_controllers";

/// The model drawn at each controller position.
const CONTROLLER_MODEL: &str = "./models/cube.obj";

/// The OpenXR input state for the two controllers.
pub(crate) struct XRControllers {
    /// The action set has to be synced every frame to get fresh input.
    action_set: openxr::ActionSet,
    /// One pose space per hand: left, then right.
    hand_spaces: [openxr::Space; 2],
}

pub(crate) fn ensure_xr_controllers(scene: &mut Scene) {
    ensure_xrsession(scene);

    if scene
        .get_resource::<RefCell<XRControllers>>(XR_CONTROLLERS_RESOURCE)
        .is_ok()
    {
        return;
    }

    let controllers = {
        let session = scene
            .get_resource::<RefCell<XRSession>>("xrsession")
            .expect("Failed to get OpenXR session");
        let session = session.borrow();
        create_xr_controllers(&session)
    };
    let _ = scene.add_resource(XR_CONTROLLERS_RESOURCE.to_owned(), RefCell::new(controllers));
}

fn create_xr_controllers(session: &XRSession) -> XRControllers {
    let instance = session.session().instance();

    // Paths are OpenXR's way of naming things; these identify the two hands.
    let left_hand = instance
        .string_to_path("/user/hand/left")
        .expect("Failed to create left hand path");
    let right_hand = instance
        .string_to_path("/user/hand/right")
        .expect("Failed to create right hand path");

    // An action set groups related actions; ours holds a single "hand pose"
    // action that exists once per hand (the subaction paths).
    let action_set = instance
        .create_action_set("wasserxr", "WasserXR", 0)
        .expect("Failed to create action set");
    let hand_pose = action_set
        .create_action::<openxr::Posef>("hand_pose", "Hand Pose", &[left_hand, right_hand])
        .expect("Failed to create hand pose action");

    // Suggest which physical input the action maps to. The "simple
    // controller" profile is the generic fallback every runtime understands;
    // the grip pose is where the hand holds the controller. More profiles
    // (e.g. oculus/touch_controller) can be suggested for better mappings.
    instance
        .suggest_interaction_profile_bindings(
            instance
                .string_to_path("/interaction_profiles/khr/simple_controller")
                .expect("Failed to create interaction profile path"),
            &[
                openxr::Binding::new(
                    &hand_pose,
                    instance
                        .string_to_path("/user/hand/left/input/grip/pose")
                        .expect("Failed to create left grip path"),
                ),
                openxr::Binding::new(
                    &hand_pose,
                    instance
                        .string_to_path("/user/hand/right/input/grip/pose")
                        .expect("Failed to create right grip path"),
                ),
            ],
        )
        .expect("Failed to suggest interaction profile bindings");

    // Attaching makes the actions usable; it can only happen once per session.
    session
        .session()
        .attach_action_sets(&[&action_set])
        .expect("Failed to attach action sets");

    // A locatable space per hand that follows the controller around.
    let hand_spaces = [left_hand, right_hand].map(|hand| {
        hand_pose
            .create_space(session.session(), hand, openxr::Posef::IDENTITY)
            .expect("Failed to create hand space")
    });

    XRControllers {
        action_set,
        hand_spaces,
    }
}

/// Finds the left and right controller entity among the ones tagged with
/// the XRController component, creating missing ones. The first entity per
/// hand wins; extra entities with the same hand are ignored. Identifying
/// them by component (which survives scene serialization) keeps a
/// hot-reload from duplicating them: the scene restores the entities while
/// our resources start over empty.
fn ensure_controller_entities(scene: &mut Scene, tagged: &[Uuid]) -> [Uuid; 2] {
    [
        XRControllerType::LeftHandController,
        XRControllerType::RightHandController,
    ]
    .map(|controller_type| {
        let existing = tagged.iter().find(|entity| {
            scene
                .query::<(&XRControllerType,)>(**entity, "XRController", &["controller_type"])
                .is_ok_and(|(tagged_type,)| *tagged_type == controller_type)
        });
        if let Some(entity) = existing {
            return *entity;
        }

        let entity = scene.add_entity();
        let _ = scene.add_component(entity, "XRController".to_owned());
        let _ = scene.add_component(entity, "Transform".to_owned());
        let _ = scene.add_component(entity, "Model".to_owned());
        // The XRController component starts as a left hand; set the real hand.
        if let Ok((tagged_type,)) = scene.query_mut::<(&mut XRControllerType,)>(
            entity,
            "XRController",
            &["controller_type"],
        ) {
            *tagged_type = controller_type;
        }
        // The Model component starts with the default material but no
        // model; give it the cube so the controller is visible.
        if let Ok((model,)) = scene.query_mut::<(&mut String,)>(entity, "Model", &["model"]) {
            *model = CONTROLLER_MODEL.to_owned();
        }
        entity
    })
}

#[system(entities=[["XROrigin"], ["XRController"]])]
fn xr_controller_sync(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Controllers are tracked relative to the play space; without an
    // XROrigin we don't know where that is in the game world.
    if entities[0].is_empty() {
        return;
    }

    ensure_xr_controllers(scene);
    let controller_entities = ensure_controller_entities(scene, &entities[1]);

    // Where the XROrigin entity sits in the game world.
    let origin_world = scene
        .query::<(&[f32; 3], &[f32; 3])>(entities[0][0], "Transform", &["position", "rotation"])
        .map(|(position, rotation)| transform_matrix(*position, *rotation))
        .unwrap_or(glam::Mat4::IDENTITY);

    // Ask OpenXR where both controllers currently are.
    let hand_poses = {
        let Ok(session) = scene.get_resource::<RefCell<XRSession>>("xrsession") else {
            return;
        };
        let Ok(renderer) = scene.get_resource::<RefCell<XRRenderer>>(XR_RENDERER_RESOURCE) else {
            return;
        };
        let Ok(controllers) = scene.get_resource::<RefCell<XRControllers>>(XR_CONTROLLERS_RESOURCE)
        else {
            return;
        };
        let session = session.borrow();
        let renderer = renderer.borrow();
        let controllers = controllers.borrow();

        if !renderer.is_running() {
            return;
        }

        // Syncing pulls fresh input data for our action set for this frame.
        session
            .session()
            .sync_actions(&[openxr::ActiveActionSet::new(&controllers.action_set)])
            .expect("Failed to sync OpenXR actions");

        [
            renderer.locate(&controllers.hand_spaces[0]),
            renderer.locate(&controllers.hand_spaces[1]),
        ]
    };

    // Write the poses into the entities, placed relative to the XROrigin —
    // the same convention the camera sync uses.
    for (entity, pose) in controller_entities.iter().zip(hand_poses) {
        // A controller that is off or out of view has no pose right now.
        let Some(pose) = pose else {
            continue;
        };

        let controller_world = origin_world * pose_matrix(pose);
        let (_, rotation, position) = controller_world.to_scale_rotation_translation();
        let (x, y, z) = rotation.to_euler(EulerRot::XYZ);

        let Ok((entity_position, entity_rotation)) = scene
            .query_mut::<(&mut [f32; 3], &mut [f32; 3])>(
                *entity,
                "Transform",
                &["position", "rotation"],
            )
        else {
            continue;
        };
        *entity_position = position.to_array();
        *entity_rotation = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
    }
}

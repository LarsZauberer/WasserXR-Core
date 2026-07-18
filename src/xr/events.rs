use std::cell::RefCell;

use wasserxr::{Uuid, error, scene::Scene, system};

use crate::xr::{instance::XRInstance, session::ensure_xrsession};

/// Owned copy of the OpenXR events we care about.
///
/// `openxr::Event` borrows from the poll buffer, so it can't be stored in a
/// resource. We only keep the variants that are useful right now.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum XREvent {
    SessionStateChanged(openxr::SessionState),
    InstanceLossPending,
}

impl XREvent {
    /// Whether this event means the runtime wants the application to stop.
    fn is_termination(self) -> bool {
        matches!(
            self,
            XREvent::InstanceLossPending
                | XREvent::SessionStateChanged(
                    openxr::SessionState::EXITING
                        | openxr::SessionState::LOSS_PENDING
                        | openxr::SessionState::STOPPING
                )
        )
    }
}

pub fn ensure_xr_events(scene: &mut Scene) -> Result<(), String> {
    if scene.get_resource::<Vec<XREvent>>("xr_events").is_err() {
        scene
            .add_resource::<Vec<XREvent>>("xr_events".to_owned(), Vec::new())
            .map_err(|err| format!("Failed to add OpenXR events resource: {err:?}"))?;
    }

    Ok(())
}

#[system]
fn xr_events_read(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    if let Err(err) = ensure_xrsession(scene).and_then(|_| ensure_xr_events(scene)) {
        error!(scene, "Failed to initialize OpenXR events: {}", err);
        return;
    }

    let mut events: Vec<XREvent> = Vec::new();
    {
        let Ok(instance) = scene.get_resource::<RefCell<XRInstance>>("xrinstance") else {
            error!(scene, "Failed to read OpenXR events: instance unavailable");
            return;
        };
        let instance = instance.borrow();
        let mut buffer = openxr::EventDataBuffer::default();
        loop {
            let event = match instance.instance().poll_event(&mut buffer) {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(err) => {
                    error!(scene, "Failed to poll OpenXR events: {}", err);
                    return;
                }
            };
            match event {
                openxr::Event::SessionStateChanged(e) => {
                    events.push(XREvent::SessionStateChanged(e.state()));
                }
                openxr::Event::InstanceLossPending(_) => {
                    events.push(XREvent::InstanceLossPending);
                }
                _ => {}
            }
        }
    }

    let Ok(xr_events) = scene.get_mut_resource::<Vec<XREvent>>("xr_events") else {
        error!(scene, "Failed to update OpenXR events resource");
        return;
    };
    *xr_events = events;
}

#[system]
fn xr_events_clear(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    if let Err(err) = ensure_xr_events(scene) {
        error!(scene, "Failed to initialize OpenXR events: {}", err);
        return;
    }
    let Ok(xr_events) = scene.get_mut_resource::<Vec<XREvent>>("xr_events") else {
        error!(scene, "Failed to clear OpenXR events resource");
        return;
    };
    xr_events.clear();
}

#[system]
fn xr_event_close(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    if let Err(err) = ensure_xr_events(scene) {
        error!(scene, "Failed to initialize OpenXR events: {}", err);
        return;
    }
    let Ok(xr_events) = scene.get_resource::<Vec<XREvent>>("xr_events") else {
        error!(scene, "Failed to read OpenXR events resource");
        return;
    };

    if xr_events.iter().any(|event| event.is_termination()) {
        scene.should_exit();
    }
}

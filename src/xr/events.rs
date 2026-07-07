use std::cell::RefCell;

use wasserxr::{Uuid, scene::Scene, system};

use crate::xr::instance::{XRInstance, ensure_xrinstance};

/// Owned copy of the OpenXR events we care about.
///
/// `openxr::Event` borrows from the poll buffer, so it can't be stored in a
/// resource. We only keep the variants that are useful right now.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum XrEvent {
    SessionStateChanged(openxr::SessionState),
    InstanceLossPending,
}

impl XrEvent {
    /// Whether this event means the runtime wants the application to stop.
    fn is_termination(self) -> bool {
        matches!(
            self,
            XrEvent::InstanceLossPending
                | XrEvent::SessionStateChanged(
                    openxr::SessionState::EXITING | openxr::SessionState::LOSS_PENDING
                )
        )
    }
}

pub fn ensure_xr_events(scene: &mut Scene) {
    if scene.get_resource::<Vec<XrEvent>>("xr_events").is_err() {
        let _ = scene.add_resource::<Vec<XrEvent>>("xr_events".to_owned(), Vec::new());
    }
}

#[system]
fn xr_events_read(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    ensure_xrinstance(scene);
    ensure_xr_events(scene);

    let mut events: Vec<XrEvent> = Vec::new();
    {
        let instance = scene
            .get_resource::<RefCell<XRInstance>>("xrinstance")
            .expect("Failed to get OpenXR instance");
        let instance = instance.borrow();
        let mut buffer = openxr::EventDataBuffer::default();
        while let Some(event) = instance
            .instance()
            .poll_event(&mut buffer)
            .expect("Failed to poll OpenXR events")
        {
            match event {
                openxr::Event::SessionStateChanged(e) => {
                    events.push(XrEvent::SessionStateChanged(e.state()));
                }
                openxr::Event::InstanceLossPending(_) => {
                    events.push(XrEvent::InstanceLossPending);
                }
                _ => {}
            }
        }
    }

    if let Ok(xr_events) = scene.get_mut_resource::<Vec<XrEvent>>("xr_events") {
        *xr_events = events;
    }
}

#[system]
fn xr_events_clear(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    ensure_xr_events(scene);
    if let Ok(xr_events) = scene.get_mut_resource::<Vec<XrEvent>>("xr_events") {
        xr_events.clear();
    }
}

#[system]
fn xr_event_close(scene: &mut Scene, _entities: Vec<Vec<Uuid>>) {
    ensure_xr_events(scene);
    let Ok(xr_events) = scene.get_resource::<Vec<XrEvent>>("xr_events") else {
        return;
    };

    if xr_events.iter().any(|event| event.is_termination()) {
        scene.should_exit();
    }
}

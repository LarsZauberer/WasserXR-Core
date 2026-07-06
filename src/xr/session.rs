use std::cell::RefCell;

use wasserxr::scene::Scene;

use crate::xr::instance::{XRInstance, ensure_xrinstance};

pub struct XRSession(openxr::Session<openxr::Headless>);

impl XRSession {
    fn new(instance: &XRInstance) -> Self {
        let system = instance
            .instance()
            .system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .expect("Failed to get OpenXR system");
        // ponytail: headless until rendering owns the graphics session binding.
        let (session, _, _) = unsafe {
            instance
                .instance()
                .create_session::<openxr::Headless>(system, &openxr::headless::SessionCreateInfo {})
                .expect("Failed to create OpenXR session")
        };

        Self(session)
    }

    pub fn session(&self) -> &openxr::Session<openxr::Headless> {
        &self.0
    }
}

pub fn ensure_xrsession(scene: &mut Scene) {
    ensure_xrinstance(scene);

    if scene
        .get_resource::<RefCell<XRSession>>("xrsession")
        .is_err()
    {
        let instance = scene
            .get_resource::<RefCell<XRInstance>>("xrinstance")
            .expect("Failed to get OpenXR instance");
        let session = XRSession::new(&instance.borrow());
        let _ = scene.add_resource("xrsession".to_owned(), RefCell::new(session));
    }
}

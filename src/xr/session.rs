use std::cell::RefCell;

use wasserxr::scene::Scene;

use crate::opengl::{OPENGL_CONTEXT_RESOURCE, OpenGLContext, ensure_opengl_window};
use crate::xr::instance::{XRInstance, ensure_xrinstance};

pub struct XRSession(openxr::Session<openxr::OpenGL>);

impl XRSession {
    fn new(instance: &XRInstance, opengl_context: &OpenGLContext) -> Self {
        let system = instance
            .instance()
            .system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .expect("Failed to get OpenXR system");
        let session_create_info = opengl_context.session_create_info();
        let (session, _, _) = unsafe {
            instance
                .instance()
                .create_session::<openxr::OpenGL>(system, &session_create_info)
                .expect("Failed to create OpenXR session")
        };

        Self(session)
    }

    pub fn session(&self) -> &openxr::Session<openxr::OpenGL> {
        &self.0
    }
}

pub fn ensure_xrsession(scene: &mut Scene) {
    ensure_xrinstance(scene);
    ensure_opengl_window(scene);

    if scene
        .get_resource::<RefCell<XRSession>>("xrsession")
        .is_err()
    {
        let session = {
            let instance = scene
                .get_resource::<RefCell<XRInstance>>("xrinstance")
                .expect("Failed to get OpenXR instance");
            let opengl_context = scene
                .get_resource::<RefCell<OpenGLContext>>(OPENGL_CONTEXT_RESOURCE)
                .expect("Failed to get OpenGL context");
            let instance = instance.borrow();
            let opengl_context = opengl_context.borrow();
            XRSession::new(&instance, &opengl_context)
        };
        let _ = scene.add_resource("xrsession".to_owned(), RefCell::new(session));
    }
}

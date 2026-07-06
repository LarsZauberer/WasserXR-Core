use std::cell::RefCell;

use wasserxr::scene::Scene;

use crate::renderer::{OpenGLContext, get_window_display};
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
    let _ = get_window_display(scene);

    if scene
        .get_resource::<RefCell<XRSession>>("xrsession")
        .is_err()
    {
        let instance = scene
            .get_resource::<RefCell<XRInstance>>("xrinstance")
            .expect("Failed to get OpenXR instance");
        let opengl_context = scene
            .get_resource::<OpenGLContext>("render_opengl_context")
            .expect("Failed to get OpenGL context");
        let session = XRSession::new(&instance.borrow(), opengl_context);
        let _ = scene.add_resource("xrsession".to_owned(), RefCell::new(session));
    }
}

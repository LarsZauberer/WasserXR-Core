//! OpenGL window and context setup.
//!
//! `Window` is the OS window from winit. It owns user-visible window state such
//! as size, title, and platform window handles.
//!
//! `Display` is Glium's drawing facade. Rendering code uses it to create
//! buffers, shaders, frames, and draw calls. It hides the lower-level glutin
//! handles so normal 2D window rendering can stay simple.
//!
//! `glutin::display::Display` is the connection to the platform OpenGL display
//! provider, such as GLX on X11 or EGL/Wayland. It can create configs,
//! contexts, and surfaces.
//!
//! `Surface` is the drawable OpenGL target attached to the window. Glium uses
//! it as the window backbuffer target, while OpenXR only needs its raw drawable
//! handle when creating an OpenGL-backed session.
//!
//! `PossiblyCurrentContext` is the actual OpenGL context after it has been made
//! current on the window surface. OpenXR needs the raw context handle so the XR
//! runtime can bind the session to the same OpenGL context family.
//!
//! `OpenGLContext` is WasserXR-Core's small copied-handle summary for OpenXR.
//! It is not the renderer's draw API; it only carries the native handles needed
//! to build `openxr::opengl::SessionCreateInfo`.

use std::{cell::RefCell, num::NonZeroU32};

use glium::{
    glutin::{
        self,
        config::{AsRawConfig, ConfigTemplateBuilder, RawConfig},
        context::{AsRawContext, ContextApi, ContextAttributesBuilder, RawContext, Version},
        display::{AsRawDisplay, GetGlDisplay, RawDisplay},
        platform::x11::X11GlConfigExt,
        prelude::*,
        surface::{AsRawSurface, RawSurface, SurfaceAttributesBuilder, WindowSurface},
    },
    winit::{dpi::PhysicalSize, window::Window},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use wasserxr::scene::Scene;

use crate::window::get_event_loop;
use crate::xr::instance::{XRInstance, ensure_xrinstance};

pub(crate) type Display = glium::backend::glutin::Display<WindowSurface>;

pub(crate) const OPENGL_WINDOW_RESOURCE: &str = "opengl_window";

pub(crate) struct OpenGLWindow {
    pub(crate) window: Window,
    pub(crate) display: Display,
    pub(crate) context: OpenGLContext,
}

pub(crate) enum OpenGLContext {
    Xlib {
        x_display: *mut openxr::sys::platform::Display,
        visualid: u32,
        glx_fb_config: openxr::sys::platform::GLXFBConfig,
        glx_drawable: openxr::sys::platform::GLXDrawable,
        glx_context: openxr::sys::platform::GLXContext,
    },
    Wayland {
        display: *mut openxr::sys::platform::wl_display,
    },
}

impl OpenGLContext {
    pub(crate) fn session_create_info(&self) -> openxr::opengl::SessionCreateInfo {
        match *self {
            Self::Xlib {
                x_display,
                visualid,
                glx_fb_config,
                glx_drawable,
                glx_context,
            } => openxr::opengl::SessionCreateInfo::Xlib {
                x_display,
                visualid,
                glx_fb_config,
                glx_drawable,
                glx_context,
            },
            Self::Wayland { display } => openxr::opengl::SessionCreateInfo::Wayland { display },
        }
    }

    fn from_glutin(
        window: &Window,
        config: &glutin::config::Config,
        context: &glutin::context::PossiblyCurrentContext,
        surface: &glutin::surface::Surface<WindowSurface>,
    ) -> Self {
        if let (
            RawDisplay::Glx(x_display),
            RawConfig::Glx(glx_fb_config),
            RawContext::Glx(glx_context),
            RawSurface::Glx(glx_drawable),
        ) = (
            config.display().raw_display(),
            config.raw_config(),
            context.raw_context(),
            surface.raw_surface(),
        ) {
            return Self::Xlib {
                x_display: x_display.cast_mut().cast(),
                visualid: config
                    .x11_visual()
                    .expect("Failed to get X11 visual")
                    .visual_id() as u32,
                glx_fb_config: glx_fb_config.cast_mut(),
                glx_drawable,
                glx_context: glx_context.cast_mut(),
            };
        }

        if let RawDisplayHandle::Wayland(handle) = window
            .display_handle()
            .expect("Failed to get raw display handle")
            .as_raw()
        {
            return Self::Wayland {
                display: handle.display.as_ptr().cast(),
            };
        }

        panic!("Unsupported OpenGL context for OpenXR");
    }
}

pub(crate) fn ensure_opengl_window(scene: &mut Scene) {
    if scene
        .get_resource::<RefCell<OpenGLWindow>>(OPENGL_WINDOW_RESOURCE)
        .is_ok()
    {
        return;
    }

    let version = required_opengl_version(scene);
    let opengl_window = create_render_window(scene, version);
    scene
        .add_resource(
            OPENGL_WINDOW_RESOURCE.to_owned(),
            RefCell::new(opengl_window),
        )
        .expect("Failed to add OpenGL window resource");
}

fn required_opengl_version(scene: &mut Scene) -> Version {
    ensure_xrinstance(scene);

    let requirements = {
        let instance = scene
            .get_resource::<RefCell<XRInstance>>("xrinstance")
            .expect("Failed to get OpenXR instance");
        let instance = instance.borrow();
        let system = instance
            .instance()
            .system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .expect("Failed to get OpenXR system");
        instance
            .instance()
            .graphics_requirements::<openxr::OpenGL>(system)
            .expect("Failed to get OpenGL graphics requirements")
    };
    let version = requirements
        .min_api_version_supported
        .max(openxr::Version::new(3, 3, 0));

    assert!(
        version <= requirements.max_api_version_supported,
        "OpenXR runtime does not support the required OpenGL 3.3 API"
    );

    Version::new(
        version
            .major()
            .try_into()
            .expect("Invalid OpenGL major version"),
        version
            .minor()
            .try_into()
            .expect("Invalid OpenGL minor version"),
    )
}

pub(crate) fn create_render_window(scene: &mut Scene, version: Version) -> OpenGLWindow {
    let event_loop = get_event_loop(scene);
    let attributes = Window::default_attributes()
        // TODO: Make the title parameterizable
        .with_title("WasserXR")
        .with_inner_size(PhysicalSize::new(800, 480));
    let config_template = ConfigTemplateBuilder::new().with_multisampling(8);
    let (window, config) = DisplayBuilder::new()
        .with_window_attributes(Some(attributes))
        .build(event_loop, config_template, |mut configs| {
            configs.next().expect("Failed to find OpenGL config")
        })
        .expect("Failed to create OpenGL window");
    let window = window.expect("Failed to create Window");
    let (width, height): (u32, u32) = window.inner_size().into();
    let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window
            .window_handle()
            .expect("Failed to get raw window handle")
            .into(),
        NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()),
        NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap()),
    );
    let surface = unsafe {
        config
            .display()
            .create_window_surface(&config, &surface_attributes)
            .expect("Failed to create OpenGL surface")
    };
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(version)))
        .build(Some(
            window
                .window_handle()
                .expect("Failed to get raw window handle")
                .into(),
        ));
    let context = unsafe {
        config
            .display()
            .create_context(&config, &context_attributes)
            .expect("Failed to create OpenGL context")
    }
    .make_current(&surface)
    .expect("Failed to make OpenGL context current");
    let opengl_context = OpenGLContext::from_glutin(&window, &config, &context, &surface);
    let display =
        Display::from_context_surface(context, surface).expect("Failed to create Display");

    OpenGLWindow {
        window,
        display,
        context: opengl_context,
    }
}

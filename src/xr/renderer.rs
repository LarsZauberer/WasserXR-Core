//! Renders the scene into the VR headset through OpenXR.
//!
//! This is a deliberately simple renderer: there is no 3DOF/6DOF head
//! tracking. The scene is rendered once per eye from the engine camera
//! (with a small horizontal offset for stereo depth) and shown as a static
//! image fixed in front of the eyes.
//!
//! It reuses the flatscreen renderer helpers from `crate::renderer` and
//! only replaces the render target: instead of the window backbuffer it
//! draws into OpenXR swapchain textures.

use std::cell::RefCell;

use glam::{Mat4, Vec3, camera::rh::proj::opengl::perspective};
use glium::{
    framebuffer::{DepthRenderBuffer, SimpleFrameBuffer},
    texture::{DepthFormat, Dimensions, MipmapsOption, SrgbFormat, SrgbTexture2d},
    winit::window::Window,
};
use wasserxr::{Uuid, attacher, scene::Scene, system};

use crate::{
    opengl::{Display, WINDOW_DISPLAY_RESOURCE},
    renderer::{collect_render_items, draw_render_items, get_camera, view_matrix},
    xr::session::{XRSession, ensure_xrsession},
};

pub(crate) const XR_RENDERER_RESOURCE: &str = "xr_renderer";

/// GL_SRGB8_ALPHA8, the standard color format for OpenGL XR swapchains.
const GL_SRGB8_ALPHA8: u32 = 0x8C43;

/// How far each eye is shifted sideways from the camera, in meters.
/// Half of an average human eye distance (~64mm), gives stereo depth.
const EYE_OFFSET: f32 = 0.032;

/// One eye's render target: the OpenXR swapchain (a small pool of GL
/// textures owned by the XR runtime) plus the glium wrappers to draw into it.
struct EyeTarget {
    swapchain: openxr::Swapchain<openxr::OpenGL>,
    /// One glium texture per swapchain image, wrapping the runtime's GL textures.
    textures: Vec<SrgbTexture2d>,
    /// Our own depth buffer; the runtime only provides color textures.
    depth: DepthRenderBuffer,
    width: u32,
    height: u32,
}

pub(crate) struct XRRenderer {
    /// VIEW reference space: it follows the head, so rendering with an
    /// identity pose keeps the image fixed in front of the eyes.
    space: openxr::Space,
    /// Left eye, then right eye.
    eyes: Vec<EyeTarget>,
    /// True while the runtime allows rendering (between READY and STOPPING).
    session_running: bool,
}

pub(crate) fn ensure_xr_renderer(scene: &mut Scene) {
    ensure_xrsession(scene);

    if scene
        .get_resource::<RefCell<XRRenderer>>(XR_RENDERER_RESOURCE)
        .is_ok()
    {
        return;
    }

    let renderer = {
        let session = scene
            .get_resource::<RefCell<XRSession>>("xrsession")
            .expect("Failed to get OpenXR session");
        let window_display = scene
            .get_resource::<RefCell<(Window, Display)>>(WINDOW_DISPLAY_RESOURCE)
            .expect("Failed to get OpenGL display");
        let session = session.borrow();
        let window_display = window_display.borrow();
        create_xr_renderer(&session, &window_display.1)
    };
    let _ = scene.add_resource(XR_RENDERER_RESOURCE.to_owned(), RefCell::new(renderer));
}

fn create_xr_renderer(session: &XRSession, display: &Display) -> XRRenderer {
    let instance = session.session().instance();
    let system = instance
        .system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .expect("Failed to get OpenXR system");

    // The runtime tells us the resolution each eye should be rendered at.
    let view_configs = instance
        .enumerate_view_configuration_views(system, openxr::ViewConfigurationType::PRIMARY_STEREO)
        .expect("Failed to enumerate view configurations");

    let eyes = view_configs
        .iter()
        .map(|config| {
            let width = config.recommended_image_rect_width;
            let height = config.recommended_image_rect_height;

            // A swapchain is a small pool of textures owned by the XR runtime.
            // Every frame we acquire one, draw into it and hand it back.
            let swapchain = session
                .session()
                .create_swapchain(&openxr::SwapchainCreateInfo {
                    create_flags: openxr::SwapchainCreateFlags::EMPTY,
                    usage_flags: openxr::SwapchainUsageFlags::COLOR_ATTACHMENT
                        | openxr::SwapchainUsageFlags::SAMPLED,
                    format: GL_SRGB8_ALPHA8,
                    sample_count: 1,
                    width,
                    height,
                    face_count: 1,
                    array_size: 1,
                    mip_count: 1,
                })
                .expect("Failed to create OpenXR swapchain");

            // Wrap the runtime's raw GL texture ids in glium textures so the
            // renderer helpers can draw into them. `owned: false` because the
            // runtime stays responsible for deleting them.
            let textures = swapchain
                .enumerate_images()
                .expect("Failed to enumerate swapchain images")
                .into_iter()
                .map(|texture_id| unsafe {
                    SrgbTexture2d::from_id(
                        display,
                        SrgbFormat::U8U8U8U8,
                        texture_id,
                        false,
                        MipmapsOption::NoMipmap,
                        Dimensions::Texture2d { width, height },
                    )
                })
                .collect();

            let depth = DepthRenderBuffer::new(display, DepthFormat::I24, width, height)
                .expect("Failed to create depth buffer");

            EyeTarget {
                swapchain,
                textures,
                depth,
                width,
                height,
            }
        })
        .collect();

    let space = session
        .session()
        .create_reference_space(openxr::ReferenceSpaceType::VIEW, openxr::Posef::IDENTITY)
        .expect("Failed to create reference space");

    XRRenderer {
        space,
        eyes,
        session_running: false,
    }
}

/// Handles OpenXR session lifecycle events: the runtime tells us when we may
/// start rendering (READY) and when we have to stop (STOPPING).
fn poll_session_state(session: &XRSession, session_running: &mut bool) {
    let mut buffer = openxr::EventDataBuffer::new();
    while let Some(event) = session
        .session()
        .instance()
        .poll_event(&mut buffer)
        .expect("Failed to poll OpenXR events")
    {
        if let openxr::Event::SessionStateChanged(changed) = event {
            match changed.state() {
                openxr::SessionState::READY => {
                    session
                        .session()
                        .begin(openxr::ViewConfigurationType::PRIMARY_STEREO)
                        .expect("Failed to begin OpenXR session");
                    *session_running = true;
                }
                openxr::SessionState::STOPPING => {
                    session
                        .session()
                        .end()
                        .expect("Failed to end OpenXR session");
                    *session_running = false;
                }
                _ => {}
            }
        }
    }
}

/// A symmetric OpenXR field of view matching `perspective(fov, aspect_ratio, ...)`.
/// The compositor needs to know the fov the image was rendered with.
fn symmetric_fov(fov: f32, aspect_ratio: f32) -> openxr::Fovf {
    let half_fov_vertical = fov / 2.0;
    let half_fov_horizontal = (half_fov_vertical.tan() * aspect_ratio).atan();
    openxr::Fovf {
        angle_left: -half_fov_horizontal,
        angle_right: half_fov_horizontal,
        angle_up: half_fov_vertical,
        angle_down: -half_fov_vertical,
    }
}

#[attacher(xr_renderer)]
fn xr_renderer_attach(scene: &mut Scene) {
    ensure_xr_renderer(scene);
}

#[system(entities=[["Camera"], ["Model"]])]
fn xr_renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a camera
    if entities[0].is_empty() {
        return;
    }

    ensure_xr_renderer(scene);

    let Some(camera) = get_camera(scene, entities[0][0]) else {
        return;
    };

    let render_items = collect_render_items(scene, &entities[1]);

    let Ok(session) = scene.get_resource::<RefCell<XRSession>>("xrsession") else {
        return;
    };
    let Ok(renderer) = scene.get_resource::<RefCell<XRRenderer>>(XR_RENDERER_RESOURCE) else {
        return;
    };
    let Ok(window_display) =
        scene.get_resource::<RefCell<(Window, Display)>>(WINDOW_DISPLAY_RESOURCE)
    else {
        return;
    };
    let mut session = session.borrow_mut();
    let mut renderer = renderer.borrow_mut();
    let window_display = window_display.borrow();
    let display = &window_display.1;
    let XRRenderer {
        space,
        eyes,
        session_running,
    } = &mut *renderer;

    // The runtime drives the session through states; react to them first.
    poll_session_state(&session, session_running);
    if !*session_running {
        return;
    }

    // OpenXR frame loop: wait (paces us to the display) -> begin -> draw -> end.
    let frame_state = session
        .frame_waiter()
        .wait()
        .expect("Failed to wait for OpenXR frame");
    session
        .frame_stream()
        .begin()
        .expect("Failed to begin OpenXR frame");

    if !frame_state.should_render {
        // The runtime doesn't want an image right now (e.g. headset idle),
        // but the frame still has to be ended.
        session
            .frame_stream()
            .end(
                frame_state.predicted_display_time,
                openxr::EnvironmentBlendMode::OPAQUE,
                &[],
            )
            .expect("Failed to end OpenXR frame");
        return;
    }

    let view = view_matrix(&camera);

    // Render the scene once per eye into the eye's swapchain texture.
    for (index, eye) in eyes.iter_mut().enumerate() {
        // Ask the runtime for the next free texture of this swapchain.
        let image_index = eye
            .swapchain
            .acquire_image()
            .expect("Failed to acquire swapchain image") as usize;
        eye.swapchain
            .wait_image(openxr::Duration::INFINITE)
            .expect("Failed to wait for swapchain image");

        // Shift the camera a little to the left/right so each eye gets a
        // slightly different image (stereo depth).
        let side = if index == 0 { -1.0 } else { 1.0 };
        let eye_view = Mat4::from_translation(Vec3::new(-side * EYE_OFFSET, 0.0, 0.0)) * view;

        let aspect_ratio = eye.width as f32 / eye.height as f32;
        let projection = perspective(camera.fov, aspect_ratio, camera.near, camera.far);

        // Draw with the same helpers as the flatscreen renderer, just into
        // the swapchain texture instead of the window backbuffer.
        let mut framebuffer =
            SimpleFrameBuffer::with_depth_buffer(display, &eye.textures[image_index], &eye.depth)
                .expect("Failed to create framebuffer");
        draw_render_items(scene, &mut framebuffer, projection * eye_view, &render_items);

        eye.swapchain
            .release_image()
            .expect("Failed to release swapchain image");
    }

    // Describe to the compositor what we rendered: one projection view per
    // eye with an identity pose (static image, no tracking) and the fov the
    // image was rendered with.
    let layer_views: Vec<_> = eyes
        .iter()
        .map(|eye| {
            openxr::CompositionLayerProjectionView::new()
                .pose(openxr::Posef::IDENTITY)
                .fov(symmetric_fov(camera.fov, eye.width as f32 / eye.height as f32))
                .sub_image(
                    openxr::SwapchainSubImage::new()
                        .swapchain(&eye.swapchain)
                        .image_rect(openxr::Rect2Di {
                            offset: openxr::Offset2Di { x: 0, y: 0 },
                            extent: openxr::Extent2Di {
                                width: eye.width as i32,
                                height: eye.height as i32,
                            },
                        }),
                )
        })
        .collect();

    let layer = openxr::CompositionLayerProjection::new()
        .space(space)
        .views(&layer_views);
    session
        .frame_stream()
        .end(
            frame_state.predicted_display_time,
            openxr::EnvironmentBlendMode::OPAQUE,
            &[&layer],
        )
        .expect("Failed to end OpenXR frame");
}

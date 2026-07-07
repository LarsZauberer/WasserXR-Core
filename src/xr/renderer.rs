//! Renders the scene into the VR headset through OpenXR (6DOF).
//!
//! The scene is rendered once per eye from the Camera entity, which the
//! xr_camera_sync system keeps at the headset pose relative to the XROrigin
//! entity. Each eye adds its own small offset and fov, both reported by the
//! runtime, so moving and turning the head moves the view through the world.
//!
//! It reuses the flatscreen renderer helpers from `crate::renderer` and
//! only replaces the render target: instead of the window backbuffer it
//! draws into OpenXR swapchain textures.

use std::cell::RefCell;

use glam::{Mat4, camera::rh::proj::opengl::frustum};
use glium::{
    framebuffer::{DepthRenderBuffer, SimpleFrameBuffer},
    texture::{DepthFormat, Dimensions, MipmapsOption, SrgbFormat, SrgbTexture2d},
    winit::window::Window,
};
use wasserxr::{Uuid, attacher, scene::Scene, system, warn};

use crate::{
    opengl::{Display, WINDOW_DISPLAY_RESOURCE},
    renderer::{collect_render_items, draw_render_items, get_camera, transform_matrix},
    xr::math::{matrix_to_pose, pose_matrix},
    xr::session::{XRSession, ensure_xrsession},
};

pub(crate) const XR_RENDERER_RESOURCE: &str = "xr_renderer";

/// GL_SRGB8_ALPHA8, the standard color format for OpenGL XR swapchains.
const GL_SRGB8_ALPHA8: u32 = 0x8C43;

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
    /// LOCAL reference space: the world-anchored play-space origin, fixed
    /// where the headset was when the session started. Headset poses are
    /// measured relative to this space.
    local_space: openxr::Space,
    /// VIEW reference space: it follows the head. Eye poses are queried
    /// relative to it, and locating it inside `local_space` yields the
    /// headset pose.
    view_space: openxr::Space,
    /// Left eye, then right eye.
    eyes: Vec<EyeTarget>,
    /// True while the runtime allows rendering (between READY and STOPPING).
    session_running: bool,
    /// Display time of the last rendered frame. Poses can only be queried
    /// for a point in time, and this is the most recent one we know.
    predicted_display_time: openxr::Time,
}

impl XRRenderer {
    /// True while frames are being rendered; poses can only be queried then.
    pub(crate) fn is_running(&self) -> bool {
        self.session_running && self.predicted_display_time.as_nanos() != 0
    }

    /// Where is `space` right now, relative to the play-space origin?
    /// None while the session isn't rendering yet or tracking is lost.
    pub(crate) fn locate(&self, space: &openxr::Space) -> Option<openxr::Posef> {
        if !self.is_running() {
            return None;
        }
        let location = space
            .locate(&self.local_space, self.predicted_display_time)
            .ok()?;
        let valid = openxr::SpaceLocationFlags::ORIENTATION_VALID
            | openxr::SpaceLocationFlags::POSITION_VALID;
        if !location.location_flags.contains(valid) {
            return None;
        }
        Some(location.pose)
    }

    /// Where is the headset right now, relative to the play-space origin?
    pub(crate) fn locate_head(&self) -> Option<openxr::Posef> {
        self.locate(&self.view_space)
    }
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

    let local_space = session
        .session()
        .create_reference_space(openxr::ReferenceSpaceType::LOCAL, openxr::Posef::IDENTITY)
        .expect("Failed to create local reference space");
    let view_space = session
        .session()
        .create_reference_space(openxr::ReferenceSpaceType::VIEW, openxr::Posef::IDENTITY)
        .expect("Failed to create view reference space");

    XRRenderer {
        local_space,
        view_space,
        eyes,
        session_running: false,
        predicted_display_time: openxr::Time::from_nanos(0),
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

/// Builds the projection matrix for one eye from the asymmetric fov OpenXR
/// reports. VR lenses are not centered in front of the eyes, so the view
/// frustum reaches differently far to the left/right/up/down.
fn projection_from_fov(fov: openxr::Fovf, near: f32, far: f32) -> Mat4 {
    frustum(
        fov.angle_left.tan() * near,
        fov.angle_right.tan() * near,
        fov.angle_down.tan() * near,
        fov.angle_up.tan() * near,
        near,
        far,
    )
}

#[attacher(xr_renderer)]
fn xr_renderer_attach(scene: &mut Scene) {
    ensure_xr_renderer(scene);
}

#[system(entities=[["Camera"], ["Model"], ["XROrigin"]])]
fn xr_renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a camera
    if entities[0].is_empty() {
        return;
    }

    // The XROrigin entity anchors the play space in the game world; without
    // it we don't know where the player is supposed to stand.
    if entities[2].is_empty() {
        warn!(scene, "XR rendering needs an entity with an XROrigin component");
        return;
    }

    ensure_xr_renderer(scene);

    let Some(camera) = get_camera(scene, entities[0][0]) else {
        return;
    };

    let render_items = collect_render_items(scene, &entities[1]);

    // Where the XROrigin entity sits in the game world.
    let origin_world = scene
        .query::<(&[f32; 3], &[f32; 3])>(entities[2][0], "Transform", &["position", "rotation"])
        .map(|(position, rotation)| transform_matrix(*position, *rotation))
        .unwrap_or(Mat4::IDENTITY);

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
        local_space,
        view_space,
        eyes,
        session_running,
        predicted_display_time,
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
    // Remember the display time so other systems (like the camera sync) can
    // ask the runtime for poses at this point in time.
    *predicted_display_time = frame_state.predicted_display_time;
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

    // Ask the runtime where each eye is relative to the head (it knows the
    // player's real eye distance) and through which fov each eye looks.
    let (_, eye_views) = session
        .session()
        .locate_views(
            openxr::ViewConfigurationType::PRIMARY_STEREO,
            frame_state.predicted_display_time,
            view_space,
        )
        .expect("Failed to locate views");

    // The Camera entity is the head: the xr_camera_sync system keeps it at
    // the headset pose, so rendering from it follows the player's head.
    let camera_world = transform_matrix(camera.position, camera.rotation);

    // Render the scene once per eye into the eye's swapchain texture.
    for (eye, eye_view) in eyes.iter_mut().zip(&eye_views) {
        // Ask the runtime for the next free texture of this swapchain.
        let image_index = eye
            .swapchain
            .acquire_image()
            .expect("Failed to acquire swapchain image") as usize;
        eye.swapchain
            .wait_image(openxr::Duration::INFINITE)
            .expect("Failed to wait for swapchain image");

        // The pose of this eye in the game world: the camera (= head) plus
        // the small offset of the eye from the head center.
        let eye_world = camera_world * pose_matrix(eye_view.pose);
        let projection = projection_from_fov(eye_view.fov, camera.near, camera.far);

        // Draw with the same helpers as the flatscreen renderer, just into
        // the swapchain texture instead of the window backbuffer.
        let mut framebuffer =
            SimpleFrameBuffer::with_depth_buffer(display, &eye.textures[image_index], &eye.depth)
                .expect("Failed to create framebuffer");
        draw_render_items(
            scene,
            &mut framebuffer,
            projection * eye_world.inverse(),
            &render_items,
        );

        eye.swapchain
            .release_image()
            .expect("Failed to release swapchain image");
    }

    // Describe to the compositor what we rendered: for each eye the pose it
    // was rendered from, expressed in the play-space (world -> play-space by
    // undoing the XROrigin transform), and the fov that was used.
    let origin_inverse = origin_world.inverse();
    let layer_views: Vec<_> = eyes
        .iter()
        .zip(&eye_views)
        .map(|(eye, eye_view)| {
            let eye_world = camera_world * pose_matrix(eye_view.pose);
            openxr::CompositionLayerProjectionView::new()
                .pose(matrix_to_pose(origin_inverse * eye_world))
                .fov(eye_view.fov)
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
        .space(local_space)
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

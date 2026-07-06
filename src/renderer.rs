use std::{collections::HashMap, num::NonZeroU32};

use glam::{EulerRot, Mat3, Mat4, Quat, Vec3, camera::rh::proj::opengl::perspective};
use glium::{
    DrawParameters, Program, Surface, dynamic_uniform,
    glutin::{
        self,
        config::{AsRawConfig, ConfigTemplateBuilder, RawConfig},
        context::{AsRawContext, ContextAttributesBuilder, RawContext},
        display::{AsRawDisplay, GetGlDisplay, RawDisplay},
        platform::x11::X11GlConfigExt,
        prelude::*,
        surface::{AsRawSurface, RawSurface, SurfaceAttributesBuilder, WindowSurface},
    },
    winit::{dpi::PhysicalSize, window::Window},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use wasserxr::{Uuid, attacher, scene::Scene, system, warn};

use crate::{material_asset::MaterialData, model_asset::Mesh, window::get_event_loop};

pub type Display = glium::backend::glutin::Display<glium::glutin::surface::WindowSurface>;

const OPENGL_CONTEXT_RESOURCE: &str = "opengl_context";
const WINDOW_DISPLAY_RESOURCE: &str = "opengl_window_display";

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

pub(crate) fn get_window_display(scene: &mut Scene) -> &mut (Window, Display) {
    if scene
        .get_resource::<(Window, Display)>("render_window")
        .is_err()
    {
        let (rendering_window, opengl_context) = create_render_window(scene);
        let _ = scene.add_resource(WINDOW_DISPLAY_RESOURCE.to_owned(), rendering_window);
        let _ = scene.add_resource(OPENGL_CONTEXT_RESOURCE.to_owned(), opengl_context);
    }

    scene
        .get_mut_resource::<(Window, Display)>(WINDOW_DISPLAY_RESOURCE)
        .expect("Failed to get the Rendering Window")
}

fn create_render_window(scene: &mut Scene) -> ((Window, Display), OpenGLContext) {
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
    let context_attributes = ContextAttributesBuilder::new().build(Some(
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

    ((window, display), opengl_context)
}

#[attacher(renderer)]
fn renderer_attach(scene: &mut Scene) {
    let _ = get_window_display(scene);
}

#[system(entities=[["Camera"], ["Model"]])]
fn renderer(scene: &mut Scene, entities: Vec<Vec<Uuid>>) {
    // Check if there is a camera
    if entities[0].is_empty() {
        return;
    }

    let Ok((window, display)) = scene.get_resource::<(Window, Display)>("render_window") else {
        return;
    };

    // Get the window and camera entity
    let camera_entity = entities[0][0];

    // Getting Camera Specs
    let Ok((fov, near, far)) =
        scene.query::<(&f32, &f32, &f32)>(camera_entity, "Camera", &["fov", "near", "far"])
    else {
        warn!(
            scene,
            "Failed to get camera properties from entity: {}", camera_entity
        );
        return;
    };

    let aspect_ratio: f32 =
        (window.inner_size().width as f32) / (window.inner_size().height as f32);

    // Camera position
    let mut cam_position: [f32; 3] = [0.0, 0.0, 0.0];
    let mut cam_rotation: [f32; 3] = [0.0, 0.0, 0.0];
    let fov = fov.to_radians();
    let near = *near;
    let far = *far;

    if let Ok((position, rotation)) =
        scene.query::<(&[f32; 3], &[f32; 3])>(camera_entity, "Transform", &["position", "rotation"])
    {
        cam_position = *position;
        cam_rotation = *rotation;
    }

    let mut frame = display.draw();
    frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let draw_params = DrawParameters {
        depth: glium::Depth {
            test: glium::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    for entity in &entities[1] {
        // Get all assets and information
        let Ok((model_path, material_path)) =
            scene.query::<(&String, &String)>(*entity, "Model", &["model", "material"])
        else {
            continue;
        };

        if model_path.is_empty() || material_path.is_empty() {
            continue;
        }

        let model_path = model_path.clone();
        let material_path = material_path.clone();

        if scene
            .ensure_asset_loaded("MaterialAsset", &material_path)
            .is_err()
        {
            continue;
        }

        if scene
            .ensure_asset_loaded("ModelAsset", &model_path)
            .is_err()
        {
            continue;
        }

        let Ok((shader_path,)) =
            scene.asset_query_loaded::<(&String,)>("MaterialAsset", &material_path, &["shader"])
        else {
            continue;
        };

        let shader_path = shader_path.clone();

        if scene
            .ensure_asset_loaded("ShaderAsset", &shader_path)
            .is_err()
        {
            continue;
        }

        let Ok((shader_program,)) =
            scene.asset_query_loaded::<(&Program,)>("ShaderAsset", &shader_path, &["shader"])
        else {
            continue;
        };

        let Ok((material_data,)) = scene.asset_query_loaded::<(&HashMap<String, MaterialData>,)>(
            "MaterialAsset",
            &material_path,
            &["data"],
        ) else {
            continue;
        };

        let Ok((meshes,)) =
            scene.asset_query_loaded::<(&Vec<Mesh>,)>("ModelAsset", &model_path, &["meshes"])
        else {
            continue;
        };

        let (transform, normal_transform) = scene
            .query::<(&[f32; 3], &[f32; 3], &[f32; 3])>(
                *entity,
                "Transform",
                &["position", "rotation", "scale"],
            )
            .ok()
            .map(|(position, rotation, scale)| {
                // Compute Model Transform
                let translation = Vec3::from_array(*position);
                let scale = Vec3::from_array(*scale);
                let rotation = make_quat(*rotation);
                let model_transform =
                    Mat4::from_scale_rotation_translation(scale, rotation, translation);

                // Compute View Transform
                let translation = Vec3::from_array(cam_position);
                let rotation = make_quat(cam_rotation);
                let view_transform =
                    Mat4::from_rotation_translation(rotation, translation).inverse();

                // Compute Projection Transform
                let projection_transform = perspective(fov, aspect_ratio, near, far);

                // Show compute full transformation
                let mvp = projection_transform * view_transform * model_transform;
                let normal_transform = Mat3::from_mat4(model_transform).inverse().transpose();

                (mvp.to_cols_array_2d(), normal_transform.to_cols_array_2d())
            })
            .unwrap_or_else(|| {
                (
                    Mat4::IDENTITY.to_cols_array_2d(),
                    Mat3::IDENTITY.to_cols_array_2d(),
                )
            });

        // Build Uniforms
        let mut uniforms = dynamic_uniform! {};
        uniforms.add("transform", &transform);
        uniforms.add("normal_transform", &normal_transform);

        // Build Uniform from material data
        for (key, value) in material_data.iter() {
            uniforms.add(key, value);
        }

        // Final draw calls
        for mesh in meshes {
            match frame.draw(
                &mesh.vertices,
                &mesh.indices,
                shader_program,
                &uniforms,
                &draw_params,
            ) {
                Ok(_) => {}
                Err(err) => {
                    warn!(scene, "Failed to draw entity {}: {:?}", entity, err);
                    continue;
                }
            };
        }
    }

    frame.finish().unwrap();
}

fn to_rotation_vec3_from_array(vec: [f32; 3]) -> Vec3 {
    Vec3::new(
        vec[0].to_radians(),
        vec[1].to_radians(),
        vec[2].to_radians(),
    )
}

fn make_quat(vec: [f32; 3]) -> Quat {
    let rotation = to_rotation_vec3_from_array(vec);
    Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z)
}

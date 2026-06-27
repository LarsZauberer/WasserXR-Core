use wasserxr::component;

pub type Display = glium::backend::glutin::Display<glium::glutin::surface::WindowSurface>;

#[component]
struct Window {
    #[getter]
    #[mutable]
    window: glium::winit::window::Window,

    #[getter]
    #[mutable]
    display: Display,

    #[getter]
    #[mutable]
    event_loop: glium::winit::event_loop::EventLoop<()>,

    #[getter]
    #[mutable]
    events: Vec<glium::winit::event::WindowEvent>,
}

impl Default for Window {
    fn default() -> Self {
        // TODO: Add better failing
        let event_loop =
            glium::winit::event_loop::EventLoop::new().expect("Failed to create event loop");

        // TODO: Add support for titles
        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("WasserXR")
            .build(&event_loop);

        Self {
            window,
            display,
            event_loop,
            events: Vec::default(),
        }
    }
}

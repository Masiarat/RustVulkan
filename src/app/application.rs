use crate::renderer::Renderer;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
}

impl App {
    pub fn default() -> App{
        App { window: None, renderer: None }
    }

}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some ( event_loop.create_window(Window::default_attributes()).expect("Failed to create window") );
        self.renderer = Some ( Renderer::new(WINDOW_WIDTH, WINDOW_HEIGHT, self.window.as_ref().unwrap()) );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            _ => {}
        }
    }

}
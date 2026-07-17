use crate::renderer::Renderer;
use std::cell::RefCell;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    frame_number: RefCell<usize>,
}

impl App {
    pub fn default() -> App{
        App { window: None, renderer: None, frame_number: RefCell::new(0) }
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut frame_number = self.frame_number.borrow_mut();
        *frame_number += 1;
        let log_message = format!("Frame number: {}", *frame_number);
        self.renderer.as_mut().expect("Renderer not initialized").draw(*frame_number).expect(&log_message);
        // println!("frame: {}", *frame_number);
    }

}
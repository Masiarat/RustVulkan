use crate::app::application::App;
use winit::event_loop;

mod utils;
mod renderer;
mod app;

fn main() {
    let event_loop = event_loop::EventLoop::new().expect("Could not create event loop");
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Cant run app");
}


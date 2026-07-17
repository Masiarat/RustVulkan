use crate::app::App;
use winit::event_loop;
use winit::event_loop::ControlFlow;

mod renderer;
mod app;

fn main() {
    let event_loop = event_loop::EventLoop::new().expect("Could not create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("Cant run app");
}


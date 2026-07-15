use crate::renderer::context::Context;
use winit::window::Window;

mod context;
mod debug_messenger;
mod swapchain;

pub struct Renderer{
    context: Context,
}

impl Renderer{
    pub fn new(window: &Window) -> Renderer{
        Renderer { context: Context::new(window) }
    }
}
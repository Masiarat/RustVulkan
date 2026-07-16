use crate::renderer::context::Context;
use crate::renderer::swap_chain::SwapChain;
use winit::window::Window;

mod context;
mod debug_messenger;
mod swap_chain;

pub struct Renderer{
    swap_chain: SwapChain,
    context: Context,
}

impl Renderer{
    pub fn new(window_width: u32, window_height: u32, window: &Window) -> Renderer{
        let context = Context::new(window);
        let swap_chain = SwapChain::new(window_width, window_height, &context);
        Renderer { context , swap_chain }
    }
}

impl Drop for Renderer{
    fn drop(&mut self) {
        self.swap_chain.destroy(&self.context);
        self.context.destroy();
    }
}
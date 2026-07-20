use crate::renderer::context::SharedContext;
use crate::renderer::frame::Frame;
use crate::renderer::image::Image;
use crate::renderer::swap_chain::SwapChain;
use ash::vk;
use std::sync::Arc;
use std::time::Duration;
use winit::window::Window;

const FRAME_COUNT: usize = 2;

mod context;
mod debug_messenger;
mod swap_chain;
mod frame;
mod image;
pub mod util;
mod pipeline;
mod descriptor;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub struct Renderer{
    draw_image: Image,
    frames: Vec<Frame>,
    swap_chain: SwapChain,
    context: Arc<SharedContext>,
}

impl Renderer{

    fn current_frame(&self, frame_number: usize) -> &Frame{
        &self.frames[ frame_number % FRAME_COUNT ]
    }

    pub fn new(window_width: u32, window_height: u32, window: &Window) -> Renderer{
        let context = Arc::new(SharedContext::new(window));
        let swap_chain = SwapChain::new(window_width, window_height, context.clone());
        let frames = (0..FRAME_COUNT).map(
            |_| Frame::new(context.clone()).expect("Failed to create frame!")
        ).collect();
        let draw_image = Image::new_display(context.clone(), vk::Extent2D{width: window_width, height: window_height}, "draw_image").expect("Failed to create draw Image");

        Renderer { draw_image, context, swap_chain, frames }
    }

    pub fn draw(&self, frame_number: usize) -> Result<(), vk::Result>{
        let current_frame = self.current_frame(frame_number);

        unsafe {self.context.device.wait_for_fences(&[current_frame.render_fence], true, ONE_SECOND.as_nanos() as u64)? }
        unsafe {self.context.device.reset_fences(&[current_frame.render_fence])? };

        let (swap_chain_image_index, _) = unsafe {
            self.swap_chain.swapchain_loader.acquire_next_image(
                self.swap_chain.swapchain,
                ONE_SECOND.as_nanos() as u64,
                current_frame.swap_chain_semaphore,
                vk::Fence::null())?
        };
        let current_swap_chain_image = self.swap_chain.swapchain_images[swap_chain_image_index as usize];

        let command_buffer = current_frame.command_buffer;
        unsafe { self.context.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())? };

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.context.device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?; }

        // image::transition_image(&self.context, command_buffer, current_swap_chain_image.image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
        self.draw_image.transition_layout(command_buffer, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);

        let flash = (frame_number as f32 / 120.0).sin().abs();
        let color = vk::ClearColorValue { float32: [0.0, 0.0, flash, 0.0] };

        self.draw_image.clear_color(command_buffer, color);

        self.draw_image.transition_layout(command_buffer, vk::ImageLayout::GENERAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        current_swap_chain_image.transition_image(&self.context, command_buffer, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        self.draw_image.copy_to(command_buffer, current_swap_chain_image.image, self.swap_chain.image_extent);

        current_swap_chain_image.transition_image(&self.context, command_buffer, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);

        unsafe { self.context.device.end_command_buffer(command_buffer)? };

        let command_buffer_submit_info = vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer);
        let wait_info = vk::SemaphoreSubmitInfo::default()
            .semaphore(current_frame.swap_chain_semaphore)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
            .value(1);
        let signal_info = vk::SemaphoreSubmitInfo::default()
            .semaphore(current_swap_chain_image.render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)
            .value(1);

        let w_infos = &[wait_info];
        let s_infos = &[signal_info];
        let cmb_infos = &[command_buffer_submit_info];
        let submit_info = vk::SubmitInfo2::default()
            .wait_semaphore_infos(w_infos)
            .signal_semaphore_infos(s_infos)
            .command_buffer_infos(cmb_infos);

        unsafe { self.context.device.queue_submit2(self.context.present_queue, &[submit_info], current_frame.render_fence)? };

        let swap_chains = &[self.swap_chain.swapchain];
        let wait_semaphores = &[current_swap_chain_image.render_finished];
        let image_indices = &[swap_chain_image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(swap_chains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        unsafe { self.swap_chain.swapchain_loader.queue_present(self.context.present_queue, &present_info)? };

        Ok(())
    }


}

use crate::error::{RendererError, RendererResult};
use crate::renderer::core::util::{standard_rendering_attachment_info, standard_rendering_info};
use crate::renderer::pass::compute_drawer::ComputeDrawer;
use crate::renderer::pass::mesh_drawer::MeshDrawer;
use ash::vk;
use core::context::SharedContext;
use core::immediate_submitter::ImmediateSubmitter;
use core::util::{
    standard_command_buffer_begin_info, standard_command_buffer_submit_info, standard_submit_info,
};
use pass::egui_painter::EguiPainter;
use resource::descriptor;
use resource::descriptor::PoolSizeRatio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sync::frame::Frame;
use sync::swap_chain::SwapChain;
use winit::window::Window;

const FRAME_COUNT: usize = 2;

pub mod core;
pub mod pass;
mod pipeline;
pub mod resource;
pub mod sync;
mod mesh;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub struct Renderer {
    pub mesh_drawer: MeshDrawer,
    pub compute_drawer: ComputeDrawer,
    pub egui_painter: EguiPainter,
    descriptor_allocator: Mutex<descriptor::DescAllocator>,
    immediate_submitter: ImmediateSubmitter,
    frames: Vec<Frame>,
    swap_chain: SwapChain,
    context: Arc<SharedContext>,
}

impl Renderer {
    fn current_frame(&self, frame_number: usize) -> &Frame {
        &self.frames[frame_number % FRAME_COUNT]
    }

    pub fn new(window_width: u32, window_height: u32, window: &Window) -> RendererResult<Self> {
        let context = Arc::new(SharedContext::new(window));
        let swap_chain = SwapChain::new(window_width, window_height, context.clone());

        let frames = (0..FRAME_COUNT)
            .map(|_| {
                Frame::new(context.clone())
                    .map_err(|e| RendererError::FrameSyncError(format!("{:?}", e)))
            })
            .collect::<RendererResult<Vec<_>>>()?;

        let immediate_submitter = ImmediateSubmitter::new(context.clone())?;

        let sizes = vec![PoolSizeRatio {
            ty: vk::DescriptorType::STORAGE_IMAGE,
            ratio: 1.0,
        }];
        let mut descr_alloc = descriptor::DescAllocator::new(
            context.clone(),
            10,
            &sizes,
            vk::DescriptorPoolCreateFlags::empty(),
        )?;

        let egui_painter = EguiPainter::new(context.clone(), swap_chain.image_format)?;

        let compute_drawer = ComputeDrawer::new(
            context.clone(),
            vk::Extent2D {
                width: window_width,
                height: window_height,
            },
            &mut descr_alloc,
        )?;

        let mesh_drawer = MeshDrawer::new(context.clone(), swap_chain.image_format, &immediate_submitter)?;

        Ok(Renderer {
            mesh_drawer,
            compute_drawer,
            context,
            swap_chain,
            frames,
            descriptor_allocator: Mutex::new(descr_alloc),
            immediate_submitter,
            egui_painter,
        })
    }

    pub fn draw(
        &mut self,
        frame_number: usize,
        primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
        screen_size: [u32; 2],
        pixels_per_point: f32,
    ) -> RendererResult<()> {
        let (render_fence, swap_chain_semaphore, cmb) = {
            self.current_frame(frame_number);
            (
                self.current_frame(frame_number).render_fence,
                self.current_frame(frame_number).swap_chain_semaphore,
                self.current_frame(frame_number).command_buffer,
            )
        };
        let frame_index = frame_number % FRAME_COUNT;

        let device = &self.context.device;
        let present_queue = self.context.present_queue;


        unsafe { device.wait_for_fences(&[render_fence], true, ONE_SECOND.as_nanos() as u64)? }
        self.egui_painter.cleanup(frame_index)?;
        self.egui_painter.update_textures(
            &self.immediate_submitter,
            textures_delta,
            frame_index,
        )?;
        self.frames[frame_index].with_destroy_queue(|q| q.flush(&self.context))??;
        unsafe { device.reset_fences(&[render_fence])? };

        let (swap_chain_image_index, _) = unsafe {
            self.swap_chain.swapchain_loader.acquire_next_image(
                self.swap_chain.swapchain,
                ONE_SECOND.as_nanos() as u64,
                swap_chain_semaphore,
                vk::Fence::null(),
            )?
        };
        let current_swap_chain_image =
            self.swap_chain.swapchain_images[swap_chain_image_index as usize];

        unsafe { device.reset_command_buffer(cmb, vk::CommandBufferResetFlags::empty())? };

        let command_buffer_begin_info =
            standard_command_buffer_begin_info(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(cmb, &command_buffer_begin_info)? }

        self.compute_drawer
            .draw(cmb, &current_swap_chain_image, self.swap_chain.image_extent);

        current_swap_chain_image.transition_layout(
            &self.context,
            cmb,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let color_attachment = standard_rendering_attachment_info(current_swap_chain_image.image_view);
        let rendering_info = standard_rendering_info(std::slice::from_ref(&color_attachment), self.swap_chain.image_extent);

        unsafe { device.cmd_begin_rendering(cmb, &rendering_info) };

        self.mesh_drawer.draw_geometry(&self.context, cmb, vk::Extent2D { width: screen_size[0], height: screen_size[1] });

        self.egui_painter.paint(
            cmb,
            frame_index,
            primitives,
            screen_size,
            pixels_per_point,
        )?;

        unsafe { device.cmd_end_rendering(cmb) };

        current_swap_chain_image.transition_layout(
            &self.context,
            cmb,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );
        unsafe { device.end_command_buffer(cmb)? };

        let command_buffer_submit_info = standard_command_buffer_submit_info(cmb);
        let wait_info = vk::SemaphoreSubmitInfo::default()
            .semaphore(swap_chain_semaphore)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
            .value(1);
        let signal_info = vk::SemaphoreSubmitInfo::default()
            .semaphore(current_swap_chain_image.render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)
            .value(1);

        let w_infos = &[wait_info];
        let s_infos = &[signal_info];
        let cmb_infos = &[command_buffer_submit_info];
        let submit_info = standard_submit_info(w_infos, s_infos, cmb_infos);

        unsafe { device.queue_submit2(present_queue, &[submit_info], render_fence)? };

        let swap_chains = &[self.swap_chain.swapchain];
        let wait_semaphores = &[current_swap_chain_image.render_finished];
        let image_indices = &[swap_chain_image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(swap_chains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        unsafe {
            self.swap_chain
                .swapchain_loader
                .queue_present(present_queue, &present_info)?
        };
        self.context
            .collect_garbage(&mut self.frames[frame_index])?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let device = &self.context.device;
            device
                .device_wait_idle()
                .expect("Failed to wait for device idle");
        }
    }
}

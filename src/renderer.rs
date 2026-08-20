use crate::app::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::error::{RendererError, RendererResult};
use crate::renderer::pipeline::load_shader_module;
use ash::vk;
use core::context::SharedContext;
use core::immediate_submitter::ImmediateSubmitter;
use core::util::{standard_command_buffer_begin_info, standard_command_buffer_submit_info, standard_submit_info};
use pass::egui_painter::EguiPainter;
use resource::descriptor;
use resource::descriptor::PoolSizeRatio;
use resource::image::Image;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sync::frame::Frame;
use sync::swap_chain::SwapChain;
use winit::window::Window;

const FRAME_COUNT: usize = 2;

mod pipeline;
pub mod core;
pub mod resource;
pub mod sync;
pub mod pass;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub struct Renderer {
    gradient_pipeline: Cell<vk::Pipeline>,
    gradient_pipeline_layout: Cell<vk::PipelineLayout>,
    draw_image_descriptor: Cell<vk::DescriptorSet>,
    draw_image_descr_layout: Cell<vk::DescriptorSetLayout>,
    pub egui_painter: EguiPainter,
    descriptor_allocator: Mutex<descriptor::Allocator>,
    draw_image: Image,
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
            .map(|_| Frame::new(context.clone()).map_err(|e| RendererError::FrameSyncError(format!("{:?}", e))))
            .collect::<RendererResult<Vec<_>>>()?;
        let immediate_submitter = ImmediateSubmitter::new(context.clone())?;
        let draw_image = Image::new_display(
            context.clone(),
            vk::Extent2D {
                width: window_width,
                height: window_height,
            },
            "draw_image",
        ).map_err(|e| RendererError::ImageAllocationError(e.to_string()))?;

        let sizes = vec![PoolSizeRatio {
            ty: vk::DescriptorType::STORAGE_IMAGE,
            ratio: 1.0,
        }];
        let descr_alloc = descriptor::Allocator::new(
            context.clone(),
            10,
            &sizes,
            vk::DescriptorPoolCreateFlags::empty(),
        )?;

        let egui_painter = EguiPainter::new(context.clone(), swap_chain.image_format)?;

        let mut renderer = Renderer {
            gradient_pipeline: Cell::new(Default::default()),
            gradient_pipeline_layout: Cell::new(Default::default()),
            draw_image_descriptor: Cell::default(),
            draw_image,
            context,
            swap_chain,
            frames,
            descriptor_allocator: Mutex::new(descr_alloc),
            draw_image_descr_layout: Cell::default(),
            immediate_submitter,
            egui_painter,
        };
        renderer.init()?;
        Ok(renderer)
    }

    pub fn init(&mut self) -> RendererResult<()> {
        self.init_descriptors()?;
        self.init_pipelines()?;
        Ok(())
    }

    pub fn init_descriptors(&mut self) -> RendererResult<()> {
        let mut builder = descriptor::LayoutBuilder::new();
        let device = &self.context.device;
        builder.add_binding(0, vk::DescriptorType::STORAGE_IMAGE);
        let layout = unsafe {
            builder.build(
                device,
                vk::ShaderStageFlags::COMPUTE,
                vk::DescriptorSetLayoutCreateFlags::empty(),
            )?
        };
        self.draw_image_descr_layout = Cell::from(layout);

        let mut alloc_guard = self
            .descriptor_allocator
            .lock()
            .map_err(|_| RendererError::DescriptorAllocationError("Mutex poisoned".into()))?;
        let descriptor = alloc_guard.allocate(self.draw_image_descr_layout.get())?;
        self.draw_image_descriptor = Cell::from(descriptor);

        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.draw_image.image_view);

        let image_infos = &[image_info];
        let draw_image_write = vk::WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_set(self.draw_image_descriptor.get())
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(image_infos);

        unsafe { device.update_descriptor_sets(&[draw_image_write], &[]) }
        Ok(())
    }

    pub fn init_pipelines(&mut self) -> RendererResult<()> {
        self.init_background_pipelines()
    }

    pub fn init_background_pipelines(&mut self) -> RendererResult<()> {
        let descriptor_layouts = &[self.draw_image_descr_layout.get()];
        let compute_layout = vk::PipelineLayoutCreateInfo::default().set_layouts(descriptor_layouts);
        let device = &self.context.device;

        let gradient_pipeline_layout =
            unsafe { device.create_pipeline_layout(&compute_layout, None)? };
        self.gradient_pipeline_layout = Cell::from(gradient_pipeline_layout);

        let compute_draw_shader = unsafe {
            load_shader_module(
                device,
                concat!(env!("OUT_DIR"), "/gradient.spv").as_ref(),
            )?
        };

        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_draw_shader)
            .name(c"main");

        let compute_draw_pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(gradient_pipeline_layout);

        let pipelines = unsafe {
            device.create_compute_pipelines(vk::PipelineCache::null(), &[compute_draw_pipeline_info], None)
                .map_err(|(_, e)| e)?
        };
        self.gradient_pipeline = Cell::from(pipelines[0]);
        unsafe { device.destroy_shader_module(compute_draw_shader, None) };
        Ok(())
    }

    pub fn draw(
        &mut self,
        frame_number: usize,
        primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
        screen_size: [u32; 2],
        pixels_per_point: f32,
    ) -> RendererResult<()> {
        self.egui_painter.update_textures(&self.immediate_submitter, textures_delta)?;

        let (render_fence, swap_chain_semaphore, cmb) = {
            self.current_frame(frame_number);
            (
                self.current_frame(frame_number).render_fence,
                self.current_frame(frame_number).swap_chain_semaphore,
                self.current_frame(frame_number).command_buffer,
            )
        };

        let device = &self.context.device;
        let present_queue = self.context.present_queue;

        unsafe { device.wait_for_fences(&[render_fence], true, ONE_SECOND.as_nanos() as u64)? }
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

        self.draw_image
            .transition_layout(cmb, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);

        unsafe {
            device.cmd_bind_pipeline(
                cmb,
                vk::PipelineBindPoint::COMPUTE,
                self.gradient_pipeline.get(),
            )
        };

        unsafe {
            device.cmd_bind_descriptor_sets(
                cmb,
                vk::PipelineBindPoint::COMPUTE,
                self.gradient_pipeline_layout.get(),
                0,
                &[self.draw_image_descriptor.get()],
                &[],
            )
        };

        unsafe {
            device.cmd_dispatch(
                cmb,
                (WINDOW_WIDTH as f32 / 16.0).ceil() as u32,
                (WINDOW_HEIGHT as f32 / 16.0).ceil() as u32,
                1,
            )
        };

        self.draw_image.transition_layout(
            cmb,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );

        current_swap_chain_image.transition_image(
            &self.context,
            cmb,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        self.draw_image.copy_to(
            cmb,
            current_swap_chain_image.image,
            self.swap_chain.image_extent,
        );

        current_swap_chain_image.transition_image(
            &self.context,
            cmb,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(current_swap_chain_image.image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swap_chain.image_extent,
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment));

        unsafe { device.cmd_begin_rendering(cmb, &rendering_info) };

        self.egui_painter.paint(cmb, primitives, screen_size, pixels_per_point)?;

        unsafe { device.cmd_end_rendering(cmb) };

        current_swap_chain_image.transition_image(
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

            device.destroy_pipeline(self.gradient_pipeline.get(), None);
            device.destroy_pipeline_layout(self.gradient_pipeline_layout.get(), None);
        }
    }
}

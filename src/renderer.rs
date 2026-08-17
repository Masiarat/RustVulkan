use crate::app::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::renderer::context::SharedContext;
use crate::renderer::descriptor::PoolSizeRatio;
use crate::renderer::frame::Frame;
use crate::renderer::image::Image;
use crate::renderer::pipeline::load_shader_module;
use crate::renderer::swap_chain::SwapChain;
use ash::vk;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
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
mod immediate_submitter;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub struct Renderer{
    gradient_pipeline: Cell<vk::Pipeline>,
    gradient_pipeline_layout: Cell<vk::PipelineLayout>,
    draw_image_descriptor: Cell<vk::DescriptorSet>,
    draw_image_descr_layout: Cell<vk::DescriptorSetLayout>,
    descriptor_allocator: Mutex<descriptor::Allocator>,
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

        let sizes = vec![PoolSizeRatio{ ty: vk::DescriptorType::STORAGE_IMAGE , ratio: 1.0 }];
        let descr_alloc = descriptor::Allocator::new(context.clone(), 10, &sizes).expect("Failed to create descriptor allocator");

        let mut renderer = Renderer { gradient_pipeline: Cell::new(Default::default()), gradient_pipeline_layout: Cell::new(Default::default()), draw_image_descriptor: Cell::default(), draw_image, context, swap_chain, frames, descriptor_allocator: Mutex::new(descr_alloc), draw_image_descr_layout: Cell::default() };
        renderer.init();
        renderer
    }

    pub fn init(&mut self){
        self.init_descriptors();
        self.init_pipelines()
    }

    pub fn init_descriptors(&mut self){
        let mut builder = descriptor::LayoutBuilder::new();
        let device = &self.context.device;
        builder.add_binding(0, vk::DescriptorType::STORAGE_IMAGE);
        self.draw_image_descr_layout =  Cell::from( unsafe{ builder.build(device, vk::ShaderStageFlags::COMPUTE, vk::DescriptorSetLayoutCreateFlags::empty()).expect("Failed to create descriptor layout") } );

        self.draw_image_descriptor = Cell::from ( self.descriptor_allocator.get_mut().unwrap().allocate(self.draw_image_descr_layout.get()).expect("Failed to allocate descriptor") );

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
    }

    pub fn init_pipelines(&mut self){
        self.init_background_pipelines();
    }

    pub fn init_background_pipelines(&mut self){
        let descriptor_layouts = &[self.draw_image_descr_layout.get()];
        let compute_layout = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(descriptor_layouts);
        let device = &self.context.device;

        self.gradient_pipeline_layout = Cell::from( unsafe{ device.create_pipeline_layout(&compute_layout, None).expect("Failed to create pipeline layout") } );

        let compute_draw_shader = unsafe{ load_shader_module(device, concat!(env!("OUT_DIR"), "/gradient.spv").as_ref()).expect("Failed to load shader") };

        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_draw_shader)
            .name(c"main");

        let compute_draw_pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(self.gradient_pipeline_layout.get());

        self.gradient_pipeline = Cell::from( unsafe{ device.create_compute_pipelines(vk::PipelineCache::null(), &[compute_draw_pipeline_info], None).expect("Failed to create compute pipeline")[0] } );
        unsafe { device.destroy_shader_module(compute_draw_shader, None) };
    }

    pub fn draw(&self, frame_number: usize) -> Result<(), vk::Result>{
        let current_frame = self.current_frame(frame_number);
        let device = &self.context.device;
        let present_queue = self.context.present_queue;

        unsafe {device.wait_for_fences(&[current_frame.render_fence], true, ONE_SECOND.as_nanos() as u64)? }
        unsafe {device.reset_fences(&[current_frame.render_fence])? };

        let (swap_chain_image_index, _) = unsafe {
            self.swap_chain.swapchain_loader.acquire_next_image(
                self.swap_chain.swapchain,
                ONE_SECOND.as_nanos() as u64,
                current_frame.swap_chain_semaphore,
                vk::Fence::null())?
        };
        let current_swap_chain_image = self.swap_chain.swapchain_images[swap_chain_image_index as usize];

        let command_buffer = current_frame.command_buffer;
        unsafe { device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())? };

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?; }

        // image::transition_image(&self.context, command_buffer, current_swap_chain_image.image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
        self.draw_image.transition_layout(command_buffer, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);

        unsafe { device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, self.gradient_pipeline.get()) };

        unsafe { device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::COMPUTE, self.gradient_pipeline_layout.get(), 0, &[self.draw_image_descriptor.get()], &[]) };

        unsafe { device.cmd_dispatch(command_buffer, (WINDOW_WIDTH as f32 / 16.0).ceil() as u32, (WINDOW_HEIGHT as f32 / 16.0).ceil() as u32, 1) };
        // let flash = (frame_number as f32 / 120.0).sin().abs();
        // let color = vk::ClearColorValue { float32: [0.0, 0.0, flash, 0.0] };
        //
        // self.draw_image.clear_color(command_buffer, color);

        self.draw_image.transition_layout(command_buffer, vk::ImageLayout::GENERAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        current_swap_chain_image.transition_image(&self.context, command_buffer, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        self.draw_image.copy_to(command_buffer, current_swap_chain_image.image, self.swap_chain.image_extent);

        current_swap_chain_image.transition_image(&self.context, command_buffer, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);

        unsafe { device.end_command_buffer(command_buffer)? };

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

        unsafe { device.queue_submit2(present_queue, &[submit_info], current_frame.render_fence)? };

        let swap_chains = &[self.swap_chain.swapchain];
        let wait_semaphores = &[current_swap_chain_image.render_finished];
        let image_indices = &[swap_chain_image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(swap_chains)
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices);

        unsafe { self.swap_chain.swapchain_loader.queue_present(present_queue, &present_info)? };

        Ok(())
    }
}

impl Drop for Renderer{
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

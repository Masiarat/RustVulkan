use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::destroy_queue::DestroyQueue;
use crate::renderer::core::{destroy_queue, util};
use ash::vk;
use std::sync::Arc;

pub struct Frame {
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,

    pub render_fence: vk::Fence,
    pub swap_chain_semaphore: vk::Semaphore,

    destroy_queue: DestroyQueue,
    context: Arc<SharedContext>
}

impl Frame {
    pub fn new(context: Arc<SharedContext>) -> RendererResult<Frame>{
        let device = &context.device;

        let command_pool_create_info = util::standard_command_pool_create_info(context.present_queue_family);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None)? };

        let command_buffer_allocate_info = util::standard_command_buffer_allocate_info(command_pool);
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)?[0] };

        let render_fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = unsafe { device.create_fence(&render_fence_create_info, None)? };

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let swap_chain_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None)? };

        let destroy_queue = destroy_queue::DestroyQueue::new();

        Ok( Frame { command_pool, command_buffer, render_fence, swap_chain_semaphore, destroy_queue, context } )
    }

    pub fn with_destroy_queue<R>(&mut self, f: impl FnOnce(&mut DestroyQueue) -> R) -> RendererResult<R> {
        Ok(f(&mut self.destroy_queue))
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            let device = &self.context.device;

            device.device_wait_idle().expect("Idle device failed!");
            device.destroy_command_pool(self.command_pool, None);

            device.destroy_fence(self.render_fence, None);
            device.destroy_semaphore(self.swap_chain_semaphore, None);

            let context = &self.context.clone();
            if let Err(e) = context.collect_garbage(self){
                eprintln!("Failed to collect garbage: {:?}", e);
            }

            if let Err(e) = self.destroy_queue.flush(context){
                eprintln!("Failed to flush destroy queue: {:?}", e);
            }
        };
    }
}
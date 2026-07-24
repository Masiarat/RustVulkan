use crate::renderer::context::SharedContext;
use ash::vk;
use std::sync::Arc;

pub struct Frame {
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,

    pub render_fence: vk::Fence,
    pub swap_chain_semaphore: vk::Semaphore,

    context: Arc<SharedContext>
}

impl Frame {
    pub fn new(context: Arc<SharedContext>) -> Result<Frame, vk::Result>{
        let command_pool_create_info =
            vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(context.present_queue_family);
        let command_pool = unsafe { context.device.create_command_pool(&command_pool_create_info, None)? };

        let command_buffer_allocate_info =
            vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .command_buffer_count(1)
                .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe { context.device.allocate_command_buffers(&command_buffer_allocate_info)?[0] };

        let render_fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = unsafe { context.device.create_fence(&render_fence_create_info, None)? };

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let swap_chain_semaphore = unsafe { context.device.create_semaphore(&semaphore_create_info, None)? };

        Ok( Frame { command_pool, command_buffer, render_fence, swap_chain_semaphore, context } )
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            self.context.device.device_wait_idle().expect("Idle device failed!");
            self.context.device.destroy_command_pool(self.command_pool, None);

            self.context.device.destroy_fence(self.render_fence, None);
            self.context.device.destroy_semaphore(self.swap_chain_semaphore, None);
        };
    }
}
use crate::renderer::context::SharedContext;
use crate::renderer::util::{StandardCommandBufferAllocateInfo, StandardCommandPoolCreateInfo};
use ash::vk;
use ash::vk::{CommandBufferAllocateInfo, CommandPoolCreateInfo};
use std::sync::Arc;

pub struct ImmediateSubmitter{
    fence: vk::Fence,
    command_buffer: vk::CommandBuffer,
    command_pool: vk::CommandPool,
    context: Arc<SharedContext>,
}

impl ImmediateSubmitter{
    pub fn new(context: Arc<SharedContext>) -> Result<ImmediateSubmitter, vk::Result> {
        let device = &context.device;

        let command_pool_create_info = CommandPoolCreateInfo::from(StandardCommandPoolCreateInfo { queue_family_index: context.present_queue_family });
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None)? };

        let command_buffer_allocate_info = CommandBufferAllocateInfo::from(StandardCommandBufferAllocateInfo { command_pool });
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)?[0] };

        let render_fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = unsafe { device.create_fence(&render_fence_create_info, None)? };
        Ok(ImmediateSubmitter { fence, command_buffer, command_pool, context })
    }

    pub fn submit(&self, f: impl FnOnce(vk::CommandBuffer) -> ()) -> Result<(), vk::Result> {
        let device = &self.context.device;
        unsafe { device.reset_fences(&[self.fence])? }
        unsafe { device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())? }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info)?; }

        Ok(())
    }
}

impl Drop for ImmediateSubmitter {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_command_pool(self.command_pool, None); }
        unsafe { self.context.device.destroy_fence(self.fence, None); }
    }
}


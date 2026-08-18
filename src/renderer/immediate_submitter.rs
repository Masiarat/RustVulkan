use crate::renderer::context::SharedContext;
use crate::renderer::util;
use crate::renderer::util::{standard_command_buffer_begin_info, standard_command_buffer_submit_info, standard_submit_info};
use ash::vk;
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

        let command_pool_create_info = util::standard_command_pool_create_info(context.present_queue_family);
        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None)? };

        let command_buffer_allocate_info = util::standard_command_buffer_allocate_info(command_pool);
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)?[0] };

        let render_fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = unsafe { device.create_fence(&render_fence_create_info, None)? };
        Ok(ImmediateSubmitter { fence, command_buffer, command_pool, context })
    }

    pub fn submit(&self, f: impl FnOnce(vk::CommandBuffer) -> ()) -> Result<(), vk::Result> {
        let device = &self.context.device;
        unsafe { device.reset_fences(&[self.fence])? }
        unsafe { device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())? }

        let command_buffer_begin_info = standard_command_buffer_begin_info(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info)?; }

        f(self.command_buffer);

        unsafe { device.end_command_buffer(self.command_buffer)?; }

        let cmb_submit_info = standard_command_buffer_submit_info(self.command_buffer);
        let cmb_infos = &[cmb_submit_info];
        let submit = standard_submit_info(&[], &[], cmb_infos);

        unsafe { device.queue_submit2(self.context.present_queue, &[submit], self.fence)?; }
        unsafe { device.wait_for_fences(&[self.fence], true, u64::MAX)?; }

        Ok(())
    }
}

impl Drop for ImmediateSubmitter {
    fn drop(&mut self) {
        println!("Destroying ImmediateSubmitter");
        unsafe { self.context.device.destroy_command_pool(self.command_pool, None); }
        unsafe { self.context.device.destroy_fence(self.fence, None); }
    }
}


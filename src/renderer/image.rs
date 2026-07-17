use crate::renderer::context::Context;
use ash::vk;

pub fn transition_image(context: &Context, command_buffer: vk::CommandBuffer, image: vk::Image, current_layout: vk::ImageLayout, new_layout: vk::ImageLayout){
    let image_memory_barrier = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
        .old_layout(current_layout)
        .new_layout(new_layout)
        .subresource_range(vk::ImageSubresourceRange::default()
            .base_mip_level(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_array_layer(0)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .aspect_mask(
                (new_layout == vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                    .then_some(vk::ImageAspectFlags::DEPTH)
                    .unwrap_or(vk::ImageAspectFlags::COLOR)
            ))
        .image(image);

    let barriers = [image_memory_barrier];
    let dependency_info = vk::DependencyInfo::default()
        .image_memory_barriers(&barriers);

    unsafe { context.device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }
}
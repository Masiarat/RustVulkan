// STANDARD CREATE INFOS

use ash::vk;
use ash::vk::{CommandBufferBeginInfo, CommandBufferSubmitInfo, Offset3D, SemaphoreSubmitInfo};

pub fn to_offset_zero(src_extent: vk::Extent2D) -> [Offset3D; 2] {
    [
        Offset3D::default().x(0).y(0).z(0),
        Offset3D::default().x(src_extent.width as i32).y(src_extent.height as i32).z(1),
    ]
}

pub fn standard_sampler_create_info() -> vk::SamplerCreateInfo<'static>{
    vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .min_lod(0.0)
        .max_lod(1.0)
}

pub fn standard_command_pool_create_info(queue_family_index: u32) -> vk::CommandPoolCreateInfo<'static> {
    vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family_index)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
}

pub fn standard_command_buffer_allocate_info(command_pool: vk::CommandPool) -> vk::CommandBufferAllocateInfo<'static> {
    vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1)
}

pub fn standard_image_memory_barrier2(image: vk::Image, src_layout: vk::ImageLayout, dst_layout: vk::ImageLayout) -> vk::ImageMemoryBarrier2<'static> {
    vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
        .old_layout(src_layout)
        .new_layout(dst_layout)
        .subresource_range(vk::ImageSubresourceRange::default()
            .base_mip_level(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_array_layer(0)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .aspect_mask(
                (dst_layout == vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                    .then_some(vk::ImageAspectFlags::DEPTH)
                    .unwrap_or(vk::ImageAspectFlags::COLOR)
            ))
        .image(image)
}

pub fn standard_copy_image_blit(src_extent: vk::Extent2D, dst_extent: vk::Extent2D) -> vk::ImageBlit2<'static> {
    vk::ImageBlit2::default()
        .src_offsets(to_offset_zero(src_extent))
        .dst_offsets(to_offset_zero(dst_extent))
        .src_subresource(vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0))
        .dst_subresource(vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .layer_count(1)
            .base_array_layer(0))
}

pub fn standard_image_view_create_info(format: vk::Format, image: vk::Image, aspect_flags: vk::ImageAspectFlags) -> vk::ImageViewCreateInfo<'static> {
    vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange::default()
            .aspect_mask(aspect_flags)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
        )
}

pub fn standard_buffer_create_info(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> vk::BufferCreateInfo<'static> {
    vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
}

pub fn standard_image_create_info(format: vk::Format, usage: vk::ImageUsageFlags, extent: vk::Extent3D) -> vk::ImageCreateInfo<'static> {
    vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .format(format)
        .usage(usage)
        .extent(extent)
}

pub fn standard_command_buffer_begin_info(flags: vk::CommandBufferUsageFlags) -> CommandBufferBeginInfo<'static> {
    vk::CommandBufferBeginInfo::default().flags(flags)
}

pub fn standard_command_buffer_submit_info(command_buffer: vk::CommandBuffer) -> vk::CommandBufferSubmitInfo<'static> {
    vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)
}

pub fn standard_submit_info<'a>(w_infos: &'a[SemaphoreSubmitInfo], s_infos: &'a[SemaphoreSubmitInfo], cmb_infos: &'a[CommandBufferSubmitInfo]) -> vk::SubmitInfo2<'a>{
    vk::SubmitInfo2::default()
        .wait_semaphore_infos(w_infos)
        .signal_semaphore_infos(s_infos)
        .command_buffer_infos(cmb_infos)
}

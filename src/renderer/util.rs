// STANDARD CREATE INFOS

use ash::vk;
use ash::vk::Offset3D;

pub fn to_offset_zero(src_extent: vk::Extent2D) -> [Offset3D; 2] {
    [
        Offset3D::default().x(0).y(0).z(0),
        Offset3D::default().x(src_extent.width as i32).y(src_extent.height as i32).z(1),
    ]
}

pub struct StandardCommandPoolCreateInfo {
    pub queue_family_index: u32,
}

pub struct StandardCommandBufferAllocateInfo {
    pub command_pool: vk::CommandPool,
}

pub struct StandardImageMemoryBarrier2 {
    pub image: vk::Image,
    pub src_layout: vk::ImageLayout,
    pub dst_layout: vk::ImageLayout,
}

pub struct StandardCopyImageBlit {
    pub src_extent: vk::Extent2D,
    pub dst_extent: vk::Extent2D,
}

pub struct StandardImageViewCreateInfo {
    pub format: vk::Format,
    pub image: vk::Image,
    pub aspect_flags: vk::ImageAspectFlags,
}

pub struct StandardImageCreateInfo {
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub extent: vk::Extent3D
}


impl From<StandardCommandPoolCreateInfo> for vk::CommandPoolCreateInfo<'_> {
    fn from(value: StandardCommandPoolCreateInfo) -> Self {
        vk::CommandPoolCreateInfo::default()
            .queue_family_index(value.queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
    }
}

impl From<StandardCommandBufferAllocateInfo> for vk::CommandBufferAllocateInfo<'_> {
    fn from(value: StandardCommandBufferAllocateInfo) -> Self {
        vk::CommandBufferAllocateInfo::default()
            .command_pool(value.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1)
    }
}

impl From<StandardImageMemoryBarrier2> for vk::ImageMemoryBarrier2<'_> {
    fn from(value: StandardImageMemoryBarrier2) -> Self {
        vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
            .old_layout(value.src_layout)
            .new_layout(value.dst_layout)
            .subresource_range(vk::ImageSubresourceRange::default()
                .base_mip_level(0)
                .level_count(vk::REMAINING_MIP_LEVELS)
                .base_array_layer(0)
                .layer_count(vk::REMAINING_ARRAY_LAYERS)
                .aspect_mask(
                    (value.dst_layout == vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                        .then_some(vk::ImageAspectFlags::DEPTH)
                        .unwrap_or(vk::ImageAspectFlags::COLOR)
                ))
            .image(value.image)
    }
}

impl From<StandardCopyImageBlit> for vk::ImageBlit2<'_> {
    fn from(value : StandardCopyImageBlit) -> Self {
        vk::ImageBlit2::default()
            .src_offsets(to_offset_zero(value.src_extent))
            .dst_offsets(to_offset_zero(value.dst_extent))
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
}

impl From<StandardImageViewCreateInfo> for vk::ImageViewCreateInfo<'_> {
    fn from(value: StandardImageViewCreateInfo) -> Self {
        vk::ImageViewCreateInfo::default()
            .image(value.image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(value.format)
            .subresource_range(vk::ImageSubresourceRange::default()
                .aspect_mask(value.aspect_flags)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
            )
    }
}

impl From<StandardImageCreateInfo> for vk::ImageCreateInfo<'_> {
    fn from(value: StandardImageCreateInfo) -> Self {
        vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .format(value.format)
            .usage(value.usage)
            .extent(value.extent)
    }
}
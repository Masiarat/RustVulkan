use crate::renderer::context::Context;
use std::error::Error;

use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;

struct Image {
    image: vk::Image,
    allocation: Allocation
}

impl Image {
    fn new_display(window_width: u32, window_height: u32, name: &str, device: &Device, allocator: &mut Allocator) -> Result<Image, Box<dyn Error>>{
        let extent = vk::Extent3D { width: window_width, height: window_height, depth: 1 };
        let format = vk::Format::R16G16B16A16_SFLOAT;
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT;

        let image_create_info = vk::ImageCreateInfo::from(StandardImageCreateInfo{format, usage, extent});
        let image = unsafe { device.create_image(&image_create_info, None)? };

        let allocation = Self::allocate(device, allocator, image, name, MemoryLocation::GpuOnly)?;

        Ok( Image { image, allocation } )
    }

    fn allocate(device: &Device, allocator: &mut Allocator, image: vk::Image, name: &str, location: MemoryLocation) -> Result<Allocation, Box<dyn Error>>{
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation_create_desc = gpu_allocator::vulkan::AllocationCreateDesc{
            name,
            requirements,
            location,
            linear: false,
            allocation_scheme: AllocationScheme::DedicatedImage(image)
        };
        let allocation = allocator.allocate(&allocation_create_desc)?;

        Ok(allocation)
    }

    fn destroy(self, device: &Device, allocator: &mut Allocator){
        allocator.free(self.allocation).expect("Failed to free image allocation");
        unsafe { device.destroy_image(self.image, None) };
    }
}

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

struct StandardImageCreateInfo {
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    extent: vk::Extent3D
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
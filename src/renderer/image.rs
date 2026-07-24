use crate::renderer::context::SharedContext;
use ash::vk::{CommandBuffer, Offset3D};
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::error::Error;
use std::sync::Arc;

pub struct Image {
    image: vk::Image,
    pub image_view: vk::ImageView,
    allocation: Option<Allocation>,
    extent: vk::Extent2D,

    context: Arc<SharedContext>
}

impl Image {

    pub fn copy_to(&self, command_buffer: CommandBuffer, other: vk::Image, other_extent: vk::Extent2D){
        cpy_image_to_image(&self.context.device, command_buffer, self.image, other, self.extent, other_extent)
    }

    pub fn transition_layout(&self, command_buffer: CommandBuffer, current_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
        transition_image(&self.context, command_buffer, self.image, current_layout, new_layout)
    }

    pub fn clear_color(&self, command_buffer: CommandBuffer, color: vk::ClearColorValue){
        let clear_range = vk::ImageSubresourceRange::default()
            .base_mip_level(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_array_layer(0)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .aspect_mask(vk::ImageAspectFlags::COLOR);

        unsafe { self.context.device.cmd_clear_color_image(command_buffer, self.image, vk::ImageLayout::GENERAL, &color, &[clear_range]) }
    }

    pub fn new_display(context: Arc<SharedContext>, extent: vk::Extent2D, name: &str) -> Result<Image, Box<dyn Error>>{
        let extent3d = vk::Extent3D::from(extent);
        let format = vk::Format::R16G16B16A16_SFLOAT;
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT;

        let image_create_info = vk::ImageCreateInfo::from(StandardImageCreateInfo{format, usage, extent: extent3d });
        let image = unsafe { context.device.create_image(&image_create_info, None)? };

        let allocation = context.with_allocator(|allocator| {
            Self::allocate(&context.device, allocator, image, name, MemoryLocation::GpuOnly)
        })?;

        unsafe { context.device.bind_image_memory(image, allocation.memory(), allocation.offset())? };

        let image_view_create_info = vk::ImageViewCreateInfo::from(
            StandardImageViewCreateInfo{format, image, aspect_flags: vk::ImageAspectFlags::COLOR});
        let image_view = unsafe { context.device.create_image_view(&image_view_create_info, None)? };

        Ok( Image { image, image_view, allocation: Some(allocation), extent, context } )
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

}

impl Drop for Image{
    fn drop(&mut self) {
        println!("Destroying image view");
        unsafe { self.context.device.destroy_image_view(self.image_view, None) };
        self.context.allocator.lock().as_mut().expect("Allocator mutex poisoned").as_mut().expect("Allocator was invalid")
            .free(self.allocation.take().expect("Allocation was invalid"))
            .expect("Failed to free image allocation");
        println!("Destroying image");
        unsafe { self.context.device.destroy_image(self.image, None) };
    }
}

pub fn transition_image(context: &SharedContext, command_buffer: vk::CommandBuffer, image: vk::Image, current_layout: vk::ImageLayout, new_layout: vk::ImageLayout){
    let image_memory_barrier = vk::ImageMemoryBarrier2::from(
        StandardImageMemoryBarrier2{image, src_layout: current_layout, dst_layout: new_layout});

    let barriers = [image_memory_barrier];
    let dependency_info = vk::DependencyInfo::default()
        .image_memory_barriers(&barriers);

    unsafe { context.device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }
}

pub fn cpy_image_to_image(device: &Device, command_buffer: CommandBuffer, src_image: vk::Image, dst_image: vk::Image, src_extent: vk::Extent2D, dst_extent: vk::Extent2D){
    let blit_region = vk::ImageBlit2::from(CopyImageBlit2{src_extent, dst_extent});

    let regions = [blit_region];
    let blit_info = vk::BlitImageInfo2::default()
        .dst_image(dst_image)
        .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .src_image(src_image)
        .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .filter(vk::Filter::LINEAR)
        .regions(&regions);

    unsafe { device.cmd_blit_image2(command_buffer, &blit_info) }
}

fn to_offset_zero(src_extent: vk::Extent2D) -> [Offset3D; 2] {
    [
        Offset3D::default().x(0).y(0).z(0),
        Offset3D::default().x(src_extent.width as i32).y(src_extent.height as i32).z(1),
    ]
}

// STANDARD CREATE INFOS

struct StandardImageMemoryBarrier2 {
    image: vk::Image,
    src_layout: vk::ImageLayout,
    dst_layout: vk::ImageLayout,
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

struct CopyImageBlit2{
    src_extent: vk::Extent2D,
    dst_extent: vk::Extent2D,
}

impl From<CopyImageBlit2> for vk::ImageBlit2<'_> {
    fn from(value : CopyImageBlit2) -> Self {
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

pub struct StandardImageViewCreateInfo {
    pub(crate) format: vk::Format,
    pub(crate) image: vk::Image,
    pub(crate) aspect_flags: vk::ImageAspectFlags,
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
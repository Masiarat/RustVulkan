use crate::error::RendererResult;
use crate::renderer::core::context::destroy_queue::Destroyable;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::util;
use ash::vk::{CommandBuffer, Extent2D, Format, ImageUsageFlags};
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use gpu_allocator::MemoryLocation;
use std::error::Error;
use std::sync::Arc;

pub struct Image {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    allocation: Option<Allocation>,
    extent: vk::Extent2D,

    context: Arc<SharedContext>,
}

impl Image {
    pub fn copy_to(
        &self,
        command_buffer: CommandBuffer,
        other: vk::Image,
        other_extent: vk::Extent2D,
    ) {
        cpy_image_to_image(
            &self.context.device,
            command_buffer,
            self.image,
            other,
            self.extent,
            other_extent,
        )
    }

    pub fn transition_layout(
        &self,
        command_buffer: CommandBuffer,
        current_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        transition_image(
            &self.context,
            command_buffer,
            self.image,
            current_layout,
            new_layout,
        )
    }

    pub fn clear_color(&self, command_buffer: CommandBuffer, color: vk::ClearColorValue) {
        let clear_range = vk::ImageSubresourceRange::default()
            .base_mip_level(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_array_layer(0)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .aspect_mask(vk::ImageAspectFlags::COLOR);

        unsafe {
            self.context.device.cmd_clear_color_image(
                command_buffer,
                self.image,
                vk::ImageLayout::GENERAL,
                &color,
                &[clear_range],
            )
        }
    }

    fn new(
        context: Arc<SharedContext>,
        extent: Extent2D,
        name: &str,
        format: Format,
        usage: ImageUsageFlags,
    ) -> Result<Result<Image, Box<dyn Error>>, Box<dyn Error>> {
        let extent3d = vk::Extent3D::from(extent);
        let image_create_info = util::standard_image_create_info(format, usage, extent3d);
        let device = &context.device;
        let image = unsafe { device.create_image(&image_create_info, None)? };

        let allocation = Self::allocate(&context, image, name, MemoryLocation::GpuOnly)?;

        unsafe { device.bind_image_memory(image, allocation.memory(), allocation.offset())? };

        let image_view_create_info =
            util::standard_image_view_create_info(format, image, vk::ImageAspectFlags::COLOR);
        let image_view = unsafe { device.create_image_view(&image_view_create_info, None)? };

        Ok(Ok(Image {
            image,
            image_view,
            allocation: Some(allocation),
            extent,
            context,
        }))
    }

    pub fn new_display(
        context: Arc<SharedContext>,
        extent: vk::Extent2D,
        name: &str,
    ) -> Result<Image, Box<dyn Error>> {
        let format = vk::Format::R16G16B16A16_SFLOAT;
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT;

        Self::new(context, extent, name, format, usage)?
    }

    pub fn new_texture(
        context: Arc<SharedContext>,
        extent: vk::Extent2D,
        name: &str,
    ) -> Result<Image, Box<dyn Error>> {
        let format = vk::Format::R8G8B8A8_UNORM;
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::SAMPLED;

        Self::new(context, extent, name, format, usage)?
    }

    fn allocate(
        context: &SharedContext,
        image: vk::Image,
        name: &str,
        location: MemoryLocation,
    ) -> RendererResult<Allocation> {
        let requirements = unsafe { context.device.get_image_memory_requirements(image) };

        let allocation_create_desc = gpu_allocator::vulkan::AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: false,
            allocation_scheme: AllocationScheme::DedicatedImage(image),
        };
        context.with_allocator(|allocator| allocator.allocate(&allocation_create_desc))?
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            let retired = RetiredImage { image: self.image, image_view: self.image_view, allocation };
            if let Err(e) = self.context.with_destroy_queue(|queue| queue.push(Box::new(retired))){
                println!("Failed to queue image for destruction: {}", e);
            }
        }
        else {
            println!("Error queueing image for destruction, allocation is None");
        }
    }
}

#[derive(Debug)]
struct RetiredImage {
    image: vk::Image,
    image_view: vk::ImageView,
    allocation: Allocation,
}

impl Destroyable for RetiredImage {
    fn destroy(self: Box<RetiredImage>, context: &SharedContext) -> RendererResult<()> {
        let device = &context.device;
        unsafe{
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
        }
        context.with_allocator(|allocator| allocator.free_allocation(self.allocation))?
    }
}

pub fn transition_image(
    context: &SharedContext,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    current_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    let image_memory_barrier =
        util::standard_image_memory_barrier2(image, current_layout, new_layout);

    let barriers = [image_memory_barrier];
    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);

    unsafe {
        context
            .device
            .cmd_pipeline_barrier2(command_buffer, &dependency_info)
    }
}

pub fn cpy_image_to_image(
    device: &Device,
    command_buffer: CommandBuffer,
    src_image: vk::Image,
    dst_image: vk::Image,
    src_extent: vk::Extent2D,
    dst_extent: vk::Extent2D,
) {
    let blit_region = util::standard_copy_image_blit(src_extent, dst_extent);

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

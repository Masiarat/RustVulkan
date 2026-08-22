use crate::error::{RendererError, RendererResult};
use crate::renderer::core::context::SharedContext;
use crate::renderer::resource::descriptor;
use crate::renderer::resource::descriptor::DescAllocator;
use crate::renderer::resource::image::Image;
use ash::vk;
use std::sync::Arc;

pub struct DrawImage {
    pub(crate) descr_set: vk::DescriptorSet,
    pub descr_layout: vk::DescriptorSetLayout,
    pub image: Image,
}

impl DrawImage {
    pub fn new(context: Arc<SharedContext>, extent: vk::Extent2D, allocator: &mut DescAllocator) -> RendererResult<Self>{
        let draw_image = Image::new_display(
            context.clone(),
            extent,
            "draw_image",
        ).map_err(|e| RendererError::ImageAllocationError(e.to_string()))?;

        let mut builder = descriptor::LayoutBuilder::new();
        let device = &context.device;
        builder.add_binding(0, vk::DescriptorType::STORAGE_IMAGE);
        let layout = unsafe {
            builder.build(
                &device,
                vk::ShaderStageFlags::COMPUTE,
                vk::DescriptorSetLayoutCreateFlags::empty(),
            )?
        };
        let draw_image_descr_layout = layout;

        let descriptor = allocator.allocate(draw_image_descr_layout)?;
        let draw_image_descriptor = descriptor;

        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(draw_image.image_view);

        let image_infos = &[image_info];
        let draw_image_write = vk::WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_set(draw_image_descriptor)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(image_infos);

        unsafe { context.device.update_descriptor_sets(&[draw_image_write], &[]) }
        Ok(DrawImage {
            descr_set: draw_image_descriptor,
            descr_layout: draw_image_descr_layout,
            image: draw_image,
        })
    }
}
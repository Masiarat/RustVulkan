use crate::renderer::context::SharedContext;
use crate::renderer::descriptor::{Allocator, LayoutBuilder, PoolSizeRatio};
use crate::renderer::util::standard_sampler_create_info;
use ash::vk;
use std::sync::Arc;

pub struct EguiPainter {
    pub sampler: vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub font_descriptor_set: vk::DescriptorSet,

    pub descriptor_allocator: Allocator,
    context: Arc<SharedContext>,
}

impl EguiPainter {
    pub fn new(context: Arc<SharedContext>) -> Result<Self, vk::Result> {
        let device = &context.device;

        let mut builder = LayoutBuilder::new();
        builder.add_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER);

        let descriptor_set_layout = unsafe {
            builder.build(
                device,
                vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty()
            ).expect("Failed to build egui descriptor layout")
        };

        let sizes = vec![PoolSizeRatio { ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, ratio: 1.0 }];
        let mut descriptor_allocator = Allocator::new(context.clone(), 1, &sizes)
            .expect("Failed to create egui descriptor allocator");

        let font_descriptor_set = descriptor_allocator
            .allocate(descriptor_set_layout)
            .expect("Failed to allocate egui font descriptor set");

        let sampler_info = standard_sampler_create_info();
        let sampler = unsafe {
            device.create_sampler(&sampler_info, None)
                .expect("Failed to create egui sampler")
        };

        Ok(Self {
            sampler,
            descriptor_set_layout,
            font_descriptor_set,
            descriptor_allocator,
            context,
        })
    }
}

impl Drop for EguiPainter {
    fn drop(&mut self) {
        unsafe{ self.context.device.destroy_sampler(self.sampler, None) }
    }
}
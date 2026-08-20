use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::util::standard_sampler_create_info;
use crate::renderer::pipeline::{load_shader_module, PipelineBuilder};
use crate::renderer::resource::descriptor::{Allocator, PoolSizeRatio};
use ash::vk;
use std::ffi::CStr;
use std::sync::Arc;

const DESCRIPTOR_MAX_SETS: u32 = 10;

pub struct EguiPipeline {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub sampler: vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_allocator: Allocator,
    context: Arc<SharedContext>,
}

impl EguiPipeline {
    pub fn new(
        context: Arc<SharedContext>,
        swapchain_format: vk::Format,
    ) -> RendererResult<Self> {
        let device = &context.device;

        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        let sampler_info = standard_sampler_create_info();
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        let descriptor_allocator = Allocator::new(
            context.clone(),
            DESCRIPTOR_MAX_SETS,
            &[PoolSizeRatio {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                ratio: 1.0,
            }],
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
        )?;

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(8);

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let path = concat!(env!("OUT_DIR"), "/egui.spv").as_ref();
        let egui_shader_module = unsafe { load_shader_module(device, path)? };

        let vertex_entry = CStr::from_bytes_with_nul(b"vertexMain\0")
            .map_err(|_| crate::error::RendererError::ShaderError("Invalid vertex entry point".into()))?;
        let fragment_entry = CStr::from_bytes_with_nul(b"fragmentMain\0")
            .map_err(|_| crate::error::RendererError::ShaderError("Invalid fragment entry point".into()))?;

        let vertex_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(egui_shader_module)
            .name(vertex_entry);

        let fragment_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(egui_shader_module)
            .name(fragment_entry);

        let vertex_binding = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<egui::epaint::Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];

        let vertex_attributes = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
                .offset(16),
        ];

        let pipeline = PipelineBuilder::new()
            .set_pipeline_layout(pipeline_layout)
            .set_color_attachment_format(swapchain_format)
            .disable_depthtest()
            .enable_blending_premultiplied_alpha()
            .set_vertex_input(&vertex_binding, &vertex_attributes)
            .add_shader_stage(vertex_stage)
            .add_shader_stage(fragment_stage)
            .build(device)?;

        unsafe {
            device.destroy_shader_module(egui_shader_module, None);
        }

        Ok(Self {
            pipeline_layout,
            pipeline,
            sampler,
            descriptor_set_layout,
            descriptor_allocator,
            context,
        })
    }
}

impl Drop for EguiPipeline {
    fn drop(&mut self) {
        unsafe {
            self.context.device.destroy_pipeline(self.pipeline, None);
            self.context
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

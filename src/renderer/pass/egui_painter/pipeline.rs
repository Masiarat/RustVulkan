use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::util::standard_sampler_create_info;
use crate::renderer::pipeline::{PipelineBuilder, load_shader_module};
use ash::vk;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

pub struct EguiPipeline {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub sampler: vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    context: Arc<SharedContext>,
}

impl EguiPipeline {
    pub fn new(
        context: Arc<SharedContext>,
        swapchain_format: vk::Format,
    ) -> RendererResult<Self> {
        let device = &context.device;

        let path = concat!(env!("OUT_DIR"), "/egui.spv").as_ref();
        let egui_shader_module = unsafe { load_shader_module(device, path)? };

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

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(size_of::<EguiPushConstants>() as u32);

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let pipeline = PipelineBuilder::default()
            .layout(pipeline_layout)
            .color_attachment_format(swapchain_format)
            .disable_depth_test()
            .input_topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE, vk::FrontFace::COUNTER_CLOCKWISE)
            .multisampling_none()
            .enable_blending_premultiplied_alpha()
            .shaders(egui_shader_module)
            .build(device)?;

        // let pipeline = PipelineBuilder::default()
        //     .set_pipeline_layout(pipeline_layout)
        //     .set_color_attachment_format(swapchain_format)
        //     .disable_depthtest()
        //     .enable_blending_premultiplied_alpha()
        //     .set_vertex_input(&vertex_binding, &vertex_attributes)
        //     .add_shader_stage(vertex_stage)
        //     .add_shader_stage(fragment_stage)
        //     .build(device)?;

        unsafe {
            device.destroy_shader_module(egui_shader_module, None);
        }

        Ok(Self {
            pipeline_layout,
            pipeline,
            sampler,
            descriptor_set_layout,
            context,
        })
    }
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
pub struct EguiPushConstants {
    pub screen_size: [f32; 2],
    pub vertex_buffer: vk::DeviceAddress,
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

use ash::vk::ShaderModule;
use ash::{vk, Device};
use crate::error::RendererResult;
use std::fs;
use std::path::Path;

pub struct PipelineBuilder {
    shader_stages: Vec<vk::PipelineShaderStageCreateInfo<'static>>,
    input_assembly: vk::PipelineInputAssemblyStateCreateInfo<'static>,
    rasterizer: vk::PipelineRasterizationStateCreateInfo<'static>,
    color_blend_attachment: vk::PipelineColorBlendAttachmentState,
    multisampling: vk::PipelineMultisampleStateCreateInfo<'static>,
    pipeline_layout: vk::PipelineLayout,
    depth_stencil: vk::PipelineDepthStencilStateCreateInfo<'static>,
    color_attachment_format: vk::Format,
    depth_attachment_format: vk::Format,

    // Dla egui (oraz innych modeli 3D w przyszłości) potrzebujemy opisu wierzchołków
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            shader_stages: Vec::new(),
            input_assembly: vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false),
            rasterizer: vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::CLOCKWISE)
                .depth_clamp_enable(false)
                .depth_bias_enable(false),
            color_blend_attachment: vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .blend_enable(false),
            multisampling: vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .min_sample_shading(1.0)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false),
            pipeline_layout: vk::PipelineLayout::null(),
            depth_stencil: vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::ALWAYS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .front(vk::StencilOpState::default())
                .back(vk::StencilOpState::default())
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0),
            color_attachment_format: vk::Format::UNDEFINED,
            depth_attachment_format: vk::Format::UNDEFINED,
            vertex_bindings: Vec::new(),
            vertex_attributes: Vec::new(),
        }
    }

    pub fn set_pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.pipeline_layout = layout;
        self
    }

    pub fn set_vertex_input(
        mut self,
        bindings: &[vk::VertexInputBindingDescription],
        attributes: &[vk::VertexInputAttributeDescription],
    ) -> Self {
        self.vertex_bindings = bindings.to_vec();
        self.vertex_attributes = attributes.to_vec();
        self
    }

    pub fn set_input_topology(mut self, topology: vk::PrimitiveTopology) -> Self {
        self.input_assembly = self.input_assembly.topology(topology);
        self
    }

    pub fn set_polygon_mode(mut self, mode: vk::PolygonMode) -> Self {
        self.rasterizer = self.rasterizer.polygon_mode(mode);
        self
    }

    pub fn set_cull_mode(mut self, cull_mode: vk::CullModeFlags, front_face: vk::FrontFace) -> Self {
        self.rasterizer = self.rasterizer.cull_mode(cull_mode).front_face(front_face);
        self
    }

    pub fn disable_blending(mut self) -> Self {
        self.color_blend_attachment = self.color_blend_attachment.blend_enable(false);
        self
    }

    // Ta funkcja jest KLUCZOWA dla EGUI!
    pub fn enable_blending_premultiplied_alpha(mut self) -> Self {
        self.color_blend_attachment = self.color_blend_attachment
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE)
            .alpha_blend_op(vk::BlendOp::ADD);
        self
    }

    pub fn set_color_attachment_format(mut self, format: vk::Format) -> Self {
        self.color_attachment_format = format;
        self
    }

    pub fn set_depth_format(mut self, format: vk::Format) -> Self {
        self.depth_attachment_format = format;
        self
    }

    pub fn disable_depthtest(mut self) -> Self {
        self.depth_stencil = self.depth_stencil
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::NEVER);
        self
    }

    pub fn add_shader_stage(mut self, stage: vk::PipelineShaderStageCreateInfo<'static>) -> Self {
        self.shader_stages.push(stage);
        self
    }

    // GŁÓWNA FUNKCJA BUDUJĄCA
    pub fn build(self, device: &Device) -> RendererResult<vk::Pipeline> {
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        let color_blend_attachments = [self.color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments);

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&self.vertex_bindings)
            .vertex_attribute_descriptions(&self.vertex_attributes);

        let color_attachment_formats = [self.color_attachment_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats);

        if self.depth_attachment_format != vk::Format::UNDEFINED {
            rendering_info = rendering_info.depth_attachment_format(self.depth_attachment_format);
        }

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut rendering_info) // Wstrzykujemy Dynamic Rendering!
            .stages(&self.shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&self.input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&self.rasterizer)
            .multisample_state(&self.multisampling)
            .depth_stencil_state(&self.depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(self.pipeline_layout);

        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_info),
                None,
            )
        }.map_err(|e| crate::error::RendererError::PipelineError(format!("Failed to create graphics pipeline: {:?}", e.1)))?[0];

        Ok(pipeline)
    }
}

/// Load a SPIR-V file from the disk and create a `vk::ShaderModule`.
pub unsafe fn load_shader_module(device: &Device, path: &Path) -> RendererResult<ShaderModule> {
    let bytes = fs::read(path)
        .map_err(|e| crate::error::RendererError::ShaderError(format!("Failed to read shader file: {}", e)))?;
    unsafe { load_shader_module_from_bytes(device, &bytes) }
}

/// Build a shader module from a SPIR-V byte slice.
pub unsafe fn load_shader_module_from_bytes(
    device: &Device,
    spirv: &[u8],
) -> RendererResult<vk::ShaderModule> {
    if spirv.len() % std::mem::size_of::<u32>() != 0 {
        return Err(crate::error::RendererError::ShaderError(
            format!("SPIR-V byte length {} is not a multiple of 4", spirv.len()),
        ));
    }
    let code: &[u32] = unsafe {
        std::slice::from_raw_parts(
            spirv.as_ptr().cast::<u32>(),
            spirv.len() / std::mem::size_of::<u32>(),
        )
    };

    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    Ok(unsafe { device.create_shader_module(&create_info, None)? })
}

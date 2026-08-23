use crate::error::RendererResult;
use crate::renderer::core::util::standard_pipeline_shader_stage_create_info;
use ash::vk::{PipelineCache, ShaderModule};
use ash::{Device, vk};
use std::fs;
use std::path::Path;

#[derive(Default)]
pub struct PipelineBuilder {
    shader_stages: Vec<vk::PipelineShaderStageCreateInfo<'static>>,
    input_assembly: vk::PipelineInputAssemblyStateCreateInfo<'static>,
    rasterizer: vk::PipelineRasterizationStateCreateInfo<'static>,
    color_blend_attachment: vk::PipelineColorBlendAttachmentState,
    multisampling: vk::PipelineMultisampleStateCreateInfo<'static>,
    pipeline_layout: vk::PipelineLayout,
    depth_stencil: vk::PipelineDepthStencilStateCreateInfo<'static>,
    render_info: vk::PipelineRenderingCreateInfo<'static>,
    color_attachment_format: vk::Format,
    depth_attachment_format: vk::Format,
}

impl PipelineBuilder {
    pub fn build(self, device: &Device) -> RendererResult<vk::Pipeline> {
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let binding = [self.color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&binding);

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();

        let state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&state);

        // CRITICAL: Keep color_formats in scope for render_info
        let color_formats = [self.color_attachment_format];
        let mut render_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_formats);
        
        if self.depth_attachment_format != vk::Format::UNDEFINED {
            render_info = render_info.depth_attachment_format(self.depth_attachment_format);
        }

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut render_info)
            .stages(&self.shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&self.input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&self.rasterizer)
            .multisample_state(&self.multisampling)
            .color_blend_state(&color_blending)
            .depth_stencil_state(&self.depth_stencil)
            .layout(self.pipeline_layout)
            .dynamic_state(&dynamic_info);

        let pipelines = unsafe {
            device
                .create_graphics_pipelines(PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| crate::error::RendererError::PipelineError(e.to_string()))?
        };
        Ok(pipelines[0])
    }

    pub fn layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.pipeline_layout = layout;
        self
    }

    pub fn shaders(mut self, shader_module: ShaderModule) -> Self {
        self.shader_stages
            .push(standard_pipeline_shader_stage_create_info(
                vk::ShaderStageFlags::VERTEX,
                shader_module,
                c"vertex",
            ));
        self.shader_stages
            .push(standard_pipeline_shader_stage_create_info(
                vk::ShaderStageFlags::FRAGMENT,
                shader_module,
                c"fragment",
            ));
        self
    }

    pub fn input_topology(mut self, topology: vk::PrimitiveTopology) -> Self {
        self.input_assembly = self
            .input_assembly
            .topology(topology)
            .primitive_restart_enable(false);
        self
    }

    pub fn polygon_mode(mut self, mode: vk::PolygonMode) -> Self {
        self.rasterizer = self.rasterizer.polygon_mode(mode).line_width(1.0);
        self
    }

    pub fn cull_mode(mut self, mode: vk::CullModeFlags, front_face: vk::FrontFace) -> Self {
        self.rasterizer = self.rasterizer.cull_mode(mode).front_face(front_face);
        self
    }

    pub fn multisampling_none(mut self) -> Self {
        self.multisampling = self
            .multisampling
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);
        self
    }

    // CRITICAL: For egui's premultiplied alpha blending!
    // egui pre-multiplies colors: RGB is already multiplied by Alpha
    // Correct blend: src = ONE (not SRC_ALPHA, because already premultiplied)
    // dst = ONE_MINUS_SRC_ALPHA
    pub fn enable_blending_premultiplied_alpha(mut self) -> Self {
        self.color_blend_attachment = self
            .color_blend_attachment
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD);
        self
    }

    pub fn disable_blending(mut self) -> Self {
        self.color_blend_attachment = self
            .color_blend_attachment
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        self
    }

    pub fn color_attachment_format(mut self, format: vk::Format) -> Self {
        self.color_attachment_format = format;
        self
    }

    pub fn depth_format(mut self, format: vk::Format) -> Self {
        self.depth_attachment_format = format;
        self
    }

    pub fn disable_depth_test(mut self) -> Self {
        self.depth_stencil = self
            .depth_stencil
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::NEVER)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(vk::StencilOpState::default())
            .back(vk::StencilOpState::default())
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0);
        self
    }
}

/// Load a SPIR-V file from the disk and create a `vk::ShaderModule`.
pub unsafe fn load_shader_module(device: &Device, path: &Path) -> RendererResult<ShaderModule> {
    let bytes = fs::read(path).map_err(|e| {
        crate::error::RendererError::ShaderError(format!("Failed to read shader file: {}", e))
    })?;
    unsafe { load_shader_module_from_bytes(device, &bytes) }
}

/// Build a shader module from a SPIR-V byte slice.
pub unsafe fn load_shader_module_from_bytes(
    device: &Device,
    spirv: &[u8],
) -> RendererResult<vk::ShaderModule> {
    if spirv.len() % std::mem::size_of::<u32>() != 0 {
        return Err(crate::error::RendererError::ShaderError(format!(
            "SPIR-V byte length {} is not a multiple of 4",
            spirv.len()
        )));
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

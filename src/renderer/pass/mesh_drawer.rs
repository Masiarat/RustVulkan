use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::util::{standard_scissor, standard_viewport};
use crate::renderer::pipeline::{PipelineBuilder, load_shader_module};
use ash::vk;
use std::sync::Arc;

pub struct MeshDrawer {
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    context: Arc<SharedContext>,
}

impl MeshDrawer {
    pub fn new(context: Arc<SharedContext>, swapchain_format: vk::Format) -> RendererResult<Self> {
        let shader_path = concat!(env!("OUT_DIR"), "/colored_triangle.spv").as_ref();
        let shader_module = unsafe { load_shader_module(&context.device, shader_path)? };

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe {
            context
                .device
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let pipeline = PipelineBuilder::default()
            .layout(pipeline_layout)
            .shaders(shader_module)
            .input_topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE, vk::FrontFace::CLOCKWISE)
            .multisampling_none()
            .disable_blending()
            .disable_depth_test()
            .color_attachment_format(swapchain_format)
            .depth_format(vk::Format::UNDEFINED)
            .build(&context.device)?;

        unsafe { context.device.destroy_shader_module(shader_module, None) }

        Ok(Self {
            pipeline_layout,
            pipeline,
            context,
        })
    }

    pub fn draw_geometry(
        &self,
        context: &SharedContext,
        cmb: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        unsafe { context.device.cmd_bind_pipeline(cmb, vk::PipelineBindPoint::GRAPHICS, self.pipeline) }

        let viewport = standard_viewport(extent);
        unsafe{ context.device.cmd_set_viewport(cmb, 0, &[viewport]) }

        let scissor = standard_scissor(extent);
        unsafe { context.device.cmd_set_scissor(cmb, 0, &[scissor]) }

        unsafe { context.device.cmd_draw(cmb, 3, 1, 0 ,0) }
    }
}

impl Drop for MeshDrawer {
    fn drop(&mut self) {
        let device = &self.context.device;
        unsafe {
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_pipeline(self.pipeline, None);
        }
    }
}

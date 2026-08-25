mod gpu_mesh_buffers;

use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::immediate_submitter::ImmediateSubmitter;
use crate::renderer::core::util::{standard_scissor, standard_viewport};
use crate::renderer::pass::mesh_drawer::gpu_mesh_buffers::GpuMeshBuffers;
use crate::renderer::pipeline::{PipelineBuilder, load_shader_module};
use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::sync::Arc;

pub struct MeshDrawer {
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    buffers: GpuMeshBuffers,
    context: Arc<SharedContext>,
}

impl MeshDrawer {
    pub fn new(context: Arc<SharedContext>, swapchain_format: vk::Format, immediate_submitter: &ImmediateSubmitter) -> RendererResult<Self> {
        let shader_path = concat!(env!("OUT_DIR"), "/colored_triangle.spv").as_ref();
        let shader_module = unsafe { load_shader_module(&context.device, shader_path)? };

        let push_constants_size = std::mem::size_of::<MeshPushConstants>();
        let pc_range = vk::PushConstantRange::default()
            .size(push_constants_size as u32)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(std::slice::from_ref(&pc_range));

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

        let mut rect_vertices: [MeshVertex; 4] = std::array::from_fn(|u| MeshVertex::default());

        rect_vertices[0].position = [0.5, -0.5, 0.0];
        rect_vertices[1].position = [0.5, 0.5, 0.0];
        rect_vertices[2].position = [-0.5, -0.5, 0.0];
        rect_vertices[3].position = [-0.5, 0.5, 0.0];

        rect_vertices[0].color = [0.0, 0.0, 0.0, 1.0];
        rect_vertices[1].color = [0.5, 0.5, 0.5, 1.0];
        rect_vertices[2].color = [1.0, 0.0, 0.0, 1.0];
        rect_vertices[3].color = [0.0, 1.0, 0.0, 1.0];

        let rect_indices: [u32; 6] = [0, 1, 2, 2, 1, 3];

        let buffers = GpuMeshBuffers::upload(&context, &rect_vertices, &rect_indices, immediate_submitter)?;

        Ok(Self {
            pipeline_layout,
            pipeline,
            context,
            buffers
        })
    }

    pub fn draw_geometry(
        &self,
        context: &SharedContext,
        cmb: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        unsafe {
            context
                .device
                .cmd_bind_pipeline(cmb, vk::PipelineBindPoint::GRAPHICS, self.pipeline)
        }

        let viewport = standard_viewport(extent);
        let scissor = standard_scissor(extent);

        let push_constants = MeshPushConstants{world_matrix: glam::Mat4::IDENTITY, vertex_buffer: self.buffers.vertex_buffer_address, _padding: 0 };
        let pc_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&push_constants));

        unsafe {
            context.device.cmd_set_viewport(cmb, 0, &[viewport]);
            context.device.cmd_set_scissor(cmb, 0, &[scissor]);

            context.device.cmd_push_constants(cmb, self.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, pc_data);
            context.device.cmd_bind_index_buffer(cmb, self.buffers.index_buffer.buffer, 0, vk::IndexType::UINT32);

            context.device.cmd_draw_indexed(cmb, 6, 1, 0, 0, 0);
        }
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

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
struct MeshPushConstants {
    world_matrix: Mat4,
    vertex_buffer: vk::DeviceAddress,
    _padding: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Zeroable, Pod)]
struct MeshVertex {
    position: [f32; 3],
    uv_x: f32,
    normal: [f32; 3],
    uv_y: f32,
    color: [f32; 4],
}

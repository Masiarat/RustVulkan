use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::immediate_submitter::ImmediateSubmitter;
use crate::renderer::pass::mesh_drawer::MeshVertex;
use crate::renderer::resource::buffer::Buffer;
use ash::vk;
use bytemuck::cast_slice;
use gpu_allocator::MemoryLocation;

pub struct GpuMeshBuffers {
    pub index_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub vertex_buffer_address: vk::DeviceAddress,
}

impl GpuMeshBuffers {
    pub fn upload(
        context: &SharedContext,
        vertices: &[MeshVertex],
        indices: &[u32],
        immediate_submitter: &ImmediateSubmitter,
    ) -> RendererResult<Self> {
        let vertices_size = size_of_val(vertices);
        let indices_size = size_of_val(indices);

        let vertex_buffer = Buffer::new(
            context,
            vertices_size as vk::DeviceSize,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST,
            "mesh_vertex_buffer",
            MemoryLocation::GpuOnly,
        )?;
        let index_buffer = Buffer::new(
            context,
            indices_size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            "mesh_index_buffer",
            MemoryLocation::GpuOnly,
        )?;

        let buffer_address_info =
            vk::BufferDeviceAddressInfo::default().buffer(vertex_buffer.buffer);
        let vertex_buffer_address = unsafe {
            context
                .device
                .get_buffer_device_address(&buffer_address_info)
        };

        let mut staging_buffer = Buffer::new(
            context,
            (vertices_size + indices_size) as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            "mesh_staging_buffer",
            MemoryLocation::CpuToGpu,
        )?;

        staging_buffer.copy_from_slice_at(cast_slice(vertices), 0)?;
        staging_buffer.copy_from_slice_at(cast_slice(indices), vertices_size)?;

        immediate_submitter.submit(|cmb| {
            let vertex_copy = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: vertices_size as vk::DeviceSize,
            };
            let index_copy = vk::BufferCopy{
                src_offset: vertices_size as vk::DeviceSize,
                dst_offset: 0,
                size: indices_size as vk::DeviceSize,
            };
            unsafe {
                context.device.cmd_copy_buffer(cmb, staging_buffer.buffer, vertex_buffer.buffer, &[vertex_copy]);
                context.device.cmd_copy_buffer(cmb, staging_buffer.buffer, index_buffer.buffer, &[index_copy]);
            }
        })?;

        Ok(GpuMeshBuffers{
            index_buffer,
            vertex_buffer,
            vertex_buffer_address
        })
    }
}

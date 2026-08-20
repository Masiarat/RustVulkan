use crate::error::RendererResult;
use crate::renderer::resource::resizable_buffer::ResizableBuffer;
use ash::vk;
use std::sync::Arc;

pub struct DrawContext {
    pub vertex_buffer: ResizableBuffer,
    pub index_buffer: ResizableBuffer,
}

impl DrawContext {
    pub fn new(context: Arc<crate::renderer::core::context::SharedContext>) -> RendererResult<Self> {
        let vertex_buffer = ResizableBuffer::new(
            context.clone(),
            2,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            "egui_vertex_buffer",
            gpu_allocator::MemoryLocation::CpuToGpu,
        ).map_err(|e| crate::error::RendererError::BufferAllocationError(e.to_string()))?;
        let index_buffer = ResizableBuffer::new(
            context,
            2,
            vk::BufferUsageFlags::INDEX_BUFFER,
            "egui_index_buffer",
            gpu_allocator::MemoryLocation::CpuToGpu,
        ).map_err(|e| crate::error::RendererError::BufferAllocationError(e.to_string()))?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn update_buffers(
        &mut self,
        primitives: &[egui::ClippedPrimitive],
    ) -> RendererResult<()> {
        let mut total_vertices = 0;
        let mut total_indices = 0;
        for p in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &p.primitive {
                total_vertices += mesh.vertices.len();
                total_indices += mesh.indices.len();
            }
        }

        if total_vertices == 0 || total_indices == 0 {
            return Ok(());
        }

        let mut all_vertices = Vec::with_capacity(total_vertices);
        let mut all_indices = Vec::with_capacity(total_indices);

        for p in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &p.primitive {
                all_vertices.extend_from_slice(&mesh.vertices);
                all_indices.extend_from_slice(&mesh.indices);
            }
        }

        let vertex_size = (all_vertices.len() * std::mem::size_of::<egui::epaint::Vertex>()) as u64;
        let index_size = (all_indices.len() * std::mem::size_of::<u32>()) as u64;

        let vertices_bytes = unsafe {
            std::slice::from_raw_parts(
                all_vertices.as_ptr() as *const u8,
                all_vertices.len() * std::mem::size_of::<egui::epaint::Vertex>(),
            )
        };
        let indices_bytes = unsafe {
            std::slice::from_raw_parts(
                all_indices.as_ptr() as *const u8,
                all_indices.len() * std::mem::size_of::<u32>(),
            )
        };

        self.vertex_buffer.resize(vertex_size)
            .map_err(|e| crate::error::RendererError::BufferAllocationError(e.to_string()))?;
        self.index_buffer.resize(index_size)
            .map_err(|e| crate::error::RendererError::BufferAllocationError(e.to_string()))?;
        self.vertex_buffer.copy_from_slice(vertices_bytes)
            .map_err(|e| crate::error::RendererError::BufferAllocationError(e))?;
        self.index_buffer.copy_from_slice(indices_bytes)
            .map_err(|e| crate::error::RendererError::BufferAllocationError(e))?;

        Ok(())
    }

    pub fn paint(
        &self,
        cmd: vk::CommandBuffer,
        device: &ash::Device,
        primitives: &[egui::ClippedPrimitive],
        screen_size_physical: [u32; 2],
        pixels_per_point: f32,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        texture_descriptor_sets: &std::collections::HashMap<
            egui::TextureId,
            vk::DescriptorSet,
        >,
    ) -> RendererResult<()> {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);

            let screen_size_points = [
                screen_size_physical[0] as f32 / pixels_per_point,
                screen_size_physical[1] as f32 / pixels_per_point,
            ];
            let pc_data = std::slice::from_raw_parts(
                screen_size_points.as_ptr() as *const u8,
                std::mem::size_of::<[f32; 2]>(),
            );
            device.cmd_push_constants(
                cmd,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                pc_data,
            );

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: screen_size_physical[0] as f32,
                height: screen_size_physical[1] as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);

            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.buffer()], &[0]);
            device.cmd_bind_index_buffer(cmd, self.index_buffer.buffer(), 0, vk::IndexType::UINT32);

            let mut vertex_offset = 0;
            let mut index_offset = 0;

            for p in primitives {
                if let egui::epaint::Primitive::Mesh(mesh) = &p.primitive {
                    let clip_min_x = pixels_per_point * p.clip_rect.min.x;
                    let clip_min_y = pixels_per_point * p.clip_rect.min.y;
                    let clip_max_x = pixels_per_point * p.clip_rect.max.x;
                    let clip_max_y = pixels_per_point * p.clip_rect.max.y;

                    let clip_min_x = clip_min_x.clamp(0.0, screen_size_physical[0] as f32);
                    let clip_min_y = clip_min_y.clamp(0.0, screen_size_physical[1] as f32);
                    let clip_max_x = clip_max_x.clamp(clip_min_x, screen_size_physical[0] as f32);
                    let clip_max_y = clip_max_y.clamp(clip_min_y, screen_size_physical[1] as f32);

                    let clip_width = (clip_max_x - clip_min_x).round() as u32;
                    let clip_height = (clip_max_y - clip_min_y).round() as u32;

                    if clip_width == 0 || clip_height == 0 {
                        vertex_offset += mesh.vertices.len() as i32;
                        index_offset += mesh.indices.len() as u32;
                        continue;
                    }

                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D {
                            x: clip_min_x.round() as i32,
                            y: clip_min_y.round() as i32,
                        },
                        extent: vk::Extent2D {
                            width: clip_width,
                            height: clip_height,
                        },
                    };
                    device.cmd_set_scissor(cmd, 0, &[scissor]);

                    if let Some(&descriptor_set) = texture_descriptor_sets.get(&mesh.texture_id) {
                        device.cmd_bind_descriptor_sets(
                            cmd,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layout,
                            0,
                            &[descriptor_set],
                            &[],
                        );

                        device.cmd_draw_indexed(
                            cmd,
                            mesh.indices.len() as u32,
                            1,
                            index_offset,
                            vertex_offset,
                            0,
                        );
                    }

                    vertex_offset += mesh.vertices.len() as i32;
                    index_offset += mesh.indices.len() as u32;
                }
            }
        }

        Ok(())
    }
}

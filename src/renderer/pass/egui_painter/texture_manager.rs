use crate::error::{RendererError, RendererResult};
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::immediate_submitter::ImmediateSubmitter;
use crate::renderer::resource::buffer::Buffer;
use crate::renderer::resource::descriptor::DescAllocator;
use crate::renderer::resource::image::Image;
use ash::vk;
use std::collections::HashMap;

pub struct TextureManager {
    pub textures: HashMap<egui::TextureId, EguiTexture>,
    sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_allocator: DescAllocator,
    texture_free_queue: [Vec<EguiTexture>; crate::renderer::FRAME_COUNT],
}

impl TextureManager {
    pub fn new(
        descriptor_allocator: DescAllocator,
        sampler: vk::Sampler,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Self {
        Self {
            textures: HashMap::new(),
            sampler,
            descriptor_set_layout,
            descriptor_allocator,
            texture_free_queue: Default::default(),
        }
    }

    pub fn update(
        &mut self,
        context: &SharedContext,
        immediate_submitter: &ImmediateSubmitter,
        textures_delta: &egui::TexturesDelta,
        frame_index: usize
    ) -> RendererResult<()> {

        let device = &context.device;

        for texture_id in &textures_delta.free {
            if let Some(egui_texture) = self.textures.remove(texture_id) {
                self.texture_free_queue[frame_index].push(egui_texture);
            }
        }

        for (texture_id, image_delta) in &textures_delta.set {
            let (width, height, pixels) = {
                let egui::ImageData::Color(color_image) = &image_delta[0].image;
                let w = color_image.width() as u32;
                let h = color_image.height() as u32;
                let pixels: Vec<u8> = color_image
                    .pixels
                    .iter()
                    .flat_map(|p| p.to_array())
                    .collect();
                (w, h, pixels)
            };

            let mut staging_buffer = Buffer::new(
                context,
                pixels.len() as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                &format!("egui_staging_buffer_{:?}", texture_id),
                gpu_allocator::MemoryLocation::CpuToGpu,
            )
            .map_err(|e| crate::error::RendererError::BufferAllocationError(e.to_string()))?;

            staging_buffer
                .copy_from_slice(&pixels)
                .map_err(|e| crate::error::RendererError::BufferAllocationError(e))?;

            let extent = vk::Extent2D { width, height };
            let is_new_texture = !self.textures.contains_key(texture_id);

            if is_new_texture {
                let texture = Image::new_texture(
                    context,
                    extent,
                    &format!("egui_texture_{:?}", texture_id),
                )
                .map_err(|e| crate::error::RendererError::ImageAllocationError(e.to_string()))?;

                let image_info = vk::DescriptorImageInfo::default()
                    .sampler(self.sampler)
                    .image_view(texture.image_view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

                let descriptor_set = self
                    .descriptor_allocator
                    .allocate(self.descriptor_set_layout)?;

                let write_info = vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(std::slice::from_ref(&image_info));

                unsafe {
                    device.update_descriptor_sets(&[write_info], &[]);
                }

                self.textures.insert(*texture_id, EguiTexture{texture, descriptor_set});
            }

            let texture = self.textures.get(texture_id).map(|x| &x.texture).ok_or(RendererError::TextureError(format!("Texture not found: {:?}", texture_id)))?;


            immediate_submitter.submit(|cmd| {
                let old_layout = if is_new_texture {
                    vk::ImageLayout::UNDEFINED
                } else {
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
                };
                texture.transition_layout(cmd, old_layout, vk::ImageLayout::TRANSFER_DST_OPTIMAL, context);

                let (offset_x, offset_y) = match image_delta[0].pos {
                    Some([x, y]) => (x as i32, y as i32),
                    None => (0, 0),
                };

                let copy_region = vk::BufferImageCopy::default()
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image_offset(vk::Offset3D {
                        x: offset_x,
                        y: offset_y,
                        z: 0,
                    })
                    .image_extent(vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    });

                unsafe {
                    device.cmd_copy_buffer_to_image(
                        cmd,
                        staging_buffer.buffer,
                        texture.image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_region],
                    );
                }

                texture.transition_layout(
                    cmd,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    context
                );
            })?;
        }
        Ok(())
    }

    pub fn flush_descriptor_sets(&mut self, frame_index: usize) -> RendererResult<()>{
        self.texture_free_queue[frame_index].drain(..).try_for_each(|EguiTexture{descriptor_set, ..}| {
            self.descriptor_allocator
                .remove(descriptor_set)
                .map_err(|e| RendererError::from(e))
        })
    }

    pub fn count(&self) -> usize {
        self.textures.len()
    }
}

pub struct EguiTexture {
    pub descriptor_set: vk::DescriptorSet,
    pub texture: Image,
}

use crate::error::RendererResult;
use crate::renderer::FRAME_COUNT;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::immediate_submitter::ImmediateSubmitter;
use crate::renderer::pass::egui_painter::texture_manager::{EguiTexture, TextureManager};
use crate::renderer::resource::descriptor::{DescAllocator, PoolSizeRatio};
use ash::vk;
use drawing::DrawContext;
use pipeline::EguiPipeline;
use std::sync::Arc;

pub mod drawing;
pub mod pipeline;
pub mod texture_manager;

pub struct EguiPainter {
    // draw_context: DrawContext,
    pipeline: EguiPipeline,
    texture_manager: TextureManager,
    context: Arc<SharedContext>,
    draw_contexts: Vec<DrawContext>,
}

const DESCRIPTOR_MAX_SETS: u32 = 100;

impl EguiPainter {
    pub fn new(context: Arc<SharedContext>, swapchain_format: vk::Format) -> RendererResult<Self> {
        let pipeline = EguiPipeline::new(context.clone(), swapchain_format)?;
        let descriptor_allocator = DescAllocator::new(
            context.clone(),
            DESCRIPTOR_MAX_SETS,
            &[PoolSizeRatio {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                ratio: 1.0,
            }],
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
        )?;
        let texture_manager = TextureManager::new(
            descriptor_allocator,
            pipeline.sampler.sampler,
            pipeline.descriptor_set_layout,
        );
        let draw_contexts = (0..FRAME_COUNT)
            .map(|_| DrawContext::new(&context))
            .collect::<RendererResult<_>>()?;

        Ok(Self {
            pipeline,
            texture_manager,
            context,
            draw_contexts,
        })
    }

    pub fn update_textures(
        &mut self,
        immediate_submitter: &ImmediateSubmitter,
        textures_delta: &egui::TexturesDelta,
        frame_index: usize,
    ) -> RendererResult<()> {
        self.texture_manager.update(
            &self.context,
            immediate_submitter,
            textures_delta,
            frame_index,
        )
    }

    pub fn cleanup(&mut self, frame_index: usize) -> RendererResult<()> {
        self.texture_manager.flush_descriptor_sets(frame_index)
    }

    /// Number of GPU-side textures currently tracked by the `TextureManager`.
    pub fn texture_count(&self) -> usize {
        self.texture_manager.count()
    }

    pub fn paint(
        &mut self,
        cmd: vk::CommandBuffer,
        frame_index: usize,
        primitives: &[egui::ClippedPrimitive],
        screen_size_physical: [u32; 2],
        pixels_per_point: f32,
    ) -> RendererResult<()> {
        let draw_context = &mut self.draw_contexts[frame_index];
        draw_context.update_buffers(&self.context, primitives)?;

        let texture_sets: std::collections::HashMap<_, _> = self
            .texture_manager
            .textures
            .iter()
            .filter_map(
                |(
                    id,
                    EguiTexture {
                        texture: _,
                        descriptor_set,
                    },
                )| Some((*id, *descriptor_set)),
            )
            .collect();

        draw_context.paint(
            cmd,
            &self.context.device,
            primitives,
            screen_size_physical,
            pixels_per_point,
            self.pipeline.pipeline,
            self.pipeline.pipeline_layout,
            &texture_sets,
        )
    }
}

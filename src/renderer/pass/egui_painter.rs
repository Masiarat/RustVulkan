use super::drawing::DrawContext;
use super::pipeline::EguiPipeline;
use super::texture_manager::TextureManager;
use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::immediate_submitter::ImmediateSubmitter;
use crate::renderer::resource::descriptor::{Allocator, PoolSizeRatio};
use ash::vk;
use std::sync::Arc;

pub struct EguiPainter {
    draw_context: DrawContext,
    pipeline: EguiPipeline,
    texture_manager: TextureManager,
    context: Arc<SharedContext>,
}

const DESCRIPTOR_MAX_SETS: u32 = 10;

impl EguiPainter {
    pub fn new(
        context: Arc<SharedContext>,
        swapchain_format: vk::Format,
    ) -> RendererResult<Self> {
        let pipeline = EguiPipeline::new(context.clone(), swapchain_format)?;
        let draw_context = DrawContext::new(context.clone())?;
        let descriptor_allocator = Allocator::new(
            context.clone(),
            DESCRIPTOR_MAX_SETS,
            &[PoolSizeRatio {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                ratio: 1.0,
            }],
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
        )?;
        let texture_manager = TextureManager::new(
            context.clone(),
            descriptor_allocator,
            pipeline.sampler,
            pipeline.descriptor_set_layout,
        );

        Ok(Self {
            draw_context,
            pipeline,
            texture_manager,
            context,
        })
    }

    pub fn update_textures(
        &mut self,
        immediate_submitter: &ImmediateSubmitter,
        textures_delta: &egui::TexturesDelta,
    ) -> RendererResult<()> {
        self.texture_manager.update(immediate_submitter, textures_delta)
    }

    pub fn paint(
        &mut self,
        cmd: vk::CommandBuffer,
        primitives: &[egui::ClippedPrimitive],
        screen_size_physical: [u32; 2],
        pixels_per_point: f32,
    ) -> RendererResult<()> {
        self.draw_context.update_buffers(primitives)?;

        let texture_sets: std::collections::HashMap<_, _> = self
            .texture_manager
            .textures
            .iter()
            .filter_map(|(id, (_, desc_set))| Some((*id, *desc_set)))
            .collect();

        self.draw_context.paint(
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

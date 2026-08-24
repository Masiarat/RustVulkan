use crate::app::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::pipeline::load_shader_module;
use crate::renderer::resource::descriptor::DescAllocator;
use crate::renderer::sync::draw_image::DrawImage;
use crate::renderer::sync::swap_chain::SwapChainImage;
use ash::vk;
use std::cell::Cell;
use std::mem;
use std::sync::Arc;

pub struct ComputeDrawer {
    pub gradient_pipeline: vk::Pipeline,
    gradient_color_top: Cell<[f32; 4]>,
    gradient_color_bottom: Cell<[f32; 4]>,
    gradient_pipeline_layout: vk::PipelineLayout,
    draw_image: DrawImage,
    context: Arc<SharedContext>,
}

impl ComputeDrawer {
    pub fn new(
        context: Arc<SharedContext>,
        image_extent: vk::Extent2D,
        allocator: &mut DescAllocator,
    ) -> RendererResult<Self> {
        let draw_image = DrawImage::new(&context, image_extent, allocator)?;

        let compute_draw_shader = unsafe {
            load_shader_module(
                &context.device,
                concat!(env!("OUT_DIR"), "/gradient.spv").as_ref(),
            )?
        };

        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_draw_shader)
            .name(c"main");

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(mem::size_of::<ComputePushConstants>() as u32);

        let descriptor_layouts = &[draw_image.descr_layout];

        let compute_layout = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(descriptor_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let gradient_pipeline_layout = unsafe {
            context
                .device
                .create_pipeline_layout(&compute_layout, None)?
        };

        let compute_pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(gradient_pipeline_layout);

        // 2. Build the pipeline
        let gradient_pipeline = unsafe {
            context.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&compute_pipeline_info),
                None,
            )
        }
        .map_err(|e| {
            // ash returns (Vec<Pipeline>, vk::Result) on error, so we access e.1
            crate::error::RendererError::PipelineError(format!(
                "Failed to create compute pipeline: {:?}",
                e.1
            ))
        })?[0]; // Grab the first (and only) pipeline from the returned Vec

        // let compute_draw_pipeline_info = vk::ComputePipelineCreateInfo::default()
        //     .stage(stage_info)
        //     .layout(gradient_pipeline_layout);
        //
        // let gradient_pipeline = PipelineBuilder::new()
        //     .add_shader_stage(stage_info)
        //     .set_pipeline_layout(gradient_pipeline_layout)
        //     .build(&context.device)?;

        unsafe {
            context
                .device
                .destroy_shader_module(compute_draw_shader, None)
        };

        Ok(ComputeDrawer {
            gradient_pipeline,
            gradient_color_top: Cell::from([1.0, 0.0, 0.0, 1.0]),
            gradient_color_bottom: Cell::from([0.0, 0.0, 1.0, 1.0]),
            gradient_pipeline_layout,
            draw_image,
            context,
        })
    }

    ///Draws the gradient on the swapchain image. Swapchain image: in layout: UNDEFINED, out layout: TRANSFER_DST_OPTIMAL
    pub fn draw(
        &self,
        cmb: vk::CommandBuffer,
        current_swap_chain_image: &SwapChainImage,
        image_extent: vk::Extent2D,
    ) {
        let device = &self.context.device;

        self.draw_image.image.transition_layout(
            cmb,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            &self.context,
        );

        let mut push_constants = ComputePushConstants::default();
        // push_constants.data1 = [1.0, 0.0, 0.0, 1.0];
        // push_constants.data2 = [0.0, 0.0, 1.0, 1.0];
        push_constants.data1 = self.gradient_color_top.get();
        push_constants.data2 = self.gradient_color_bottom.get();

        unsafe {
            device.cmd_bind_pipeline(cmb, vk::PipelineBindPoint::COMPUTE, self.gradient_pipeline);

            device.cmd_bind_descriptor_sets(
                cmb,
                vk::PipelineBindPoint::COMPUTE,
                self.gradient_pipeline_layout,
                0,
                &[self.draw_image.descr_set],
                &[],
            );

            let pc_data = std::slice::from_raw_parts(
                &push_constants as *const _ as *const u8,
                mem::size_of::<ComputePushConstants>(),
            );

            device.cmd_push_constants(
                cmb,
                self.gradient_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                pc_data,
            );

            device.cmd_dispatch(
                cmb,
                (WINDOW_WIDTH as f32 / 16.0).ceil() as u32,
                (WINDOW_HEIGHT as f32 / 16.0).ceil() as u32,
                1,
            )
        };

        self.draw_image.image.transition_layout(
            cmb,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            &self.context,
        );

        current_swap_chain_image.transition_layout(
            &self.context,
            cmb,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        self.draw_image.image.copy_to(
            cmb,
            current_swap_chain_image.image,
            image_extent,
            &self.context,
        );
    }

    pub fn gradient_color_top_rgb(&mut self) -> &mut [f32; 3] {
        self.gradient_color_top.get_mut().as_rgb()
    }

    pub fn gradient_color_bottom_rgb(&mut self) -> &mut [f32; 3] {
        self.gradient_color_bottom.get_mut().as_rgb()
    }
}

impl Drop for ComputeDrawer {
    fn drop(&mut self) {
        println!("Destroying compute drawer");
        unsafe {
            self.context
                .device
                .destroy_pipeline(self.gradient_pipeline, None);
            self.context
                .device
                .destroy_pipeline_layout(self.gradient_pipeline_layout, None);
        }
    }
}

trait AsRgb<T> {
    fn as_rgb(&mut self) -> &mut T;
}

impl AsRgb<[f32; 3]> for [f32; 4] {
    fn as_rgb(&mut self) -> &mut [f32; 3] {
        <&mut [f32; 3]>::try_from(&mut self[0..3]).unwrap()
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct ComputePushConstants {
    data1: [f32; 4],
    data2: [f32; 4],
    data3: [f32; 4],
    data4: [f32; 4],
}

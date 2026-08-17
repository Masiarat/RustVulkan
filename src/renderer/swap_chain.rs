use crate::renderer::context::SharedContext;
use crate::renderer::image::transition_image;
use crate::renderer::util;
use ash::khr::swapchain;
use ash::vk;
use itertools::izip;
use std::sync::Arc;

pub struct SwapChain {
    pub(crate) swapchain_loader: swapchain::Device,
    pub(crate) swapchain: vk::SwapchainKHR,

    pub(crate) swapchain_images: Vec<SwapChainImage>,

    pub(crate) image_format: vk::Format,
    pub(crate) image_extent: vk::Extent2D,

    context: Arc<SharedContext>
}

impl SwapChain {
    fn create_image_views(context: &SharedContext, images: &[vk::Image], image_format: vk::Format) -> Result<Vec<vk::ImageView>, vk::Result>{
        let image_views: Vec<vk::ImageView> = images.iter().map(|&image| {
            let image_view_create_info = vk::ImageViewCreateInfo::from(
                util::StandardImageViewCreateInfo{format: image_format, image, aspect_flags: vk::ImageAspectFlags::COLOR});
            unsafe { context.device.create_image_view(&image_view_create_info, None) }
        }).collect::<Result<Vec<_>, _>>()?;
        Ok( image_views )
    }

    fn query_create_info(window_width: u32, window_height: u32, context: &SharedContext) -> Result<(vk::SwapchainCreateInfoKHR<'static>, vk::Format, vk::Extent2D), vk::Result> {
        let surface_loader = &context.surface_loader;
        let surface_format = unsafe{
            surface_loader.get_physical_device_surface_formats(context.physical_device, context.surface)?[0]
        };

        let surface_capabilities = unsafe{
            surface_loader.get_physical_device_surface_capabilities(context.physical_device, context.surface)?
        };

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let extent = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window_width.clamp(surface_capabilities.min_image_extent.width, surface_capabilities.max_image_extent.width),
                height: window_height.clamp(surface_capabilities.min_image_extent.height, surface_capabilities.max_image_extent.height)
            },
            _ => surface_capabilities.current_extent,
        };

        let pre_transform =
            if surface_capabilities.supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY) { vk::SurfaceTransformFlagsKHR::IDENTITY }
            else {
                surface_capabilities.current_transform
            };

        let present_modes = unsafe { surface_loader
            .get_physical_device_surface_present_modes(context.physical_device, context.surface)?
        };
        let present_mode = present_modes
            .into_iter()
            .find(|&mode| mode == vk::PresentModeKHR::FIFO_RELAXED)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swap_chain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(extent)
            .image_usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        Ok( (swap_chain_create_info, surface_format.format, extent) )
    }

    pub fn new(window_width: u32, window_height: u32, context: Arc<SharedContext>) -> SwapChain{
       let (swap_chain_create_info, image_format, image_extent) = SwapChain::query_create_info(window_width, window_height, context.as_ref()).expect("Failed to query swap chain info!");

        let swapchain_loader = swapchain::Device::new(&context.instance, &context.device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swap_chain_create_info, None).expect("Failed to create swap chain!") };

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).expect("Failed to get swap chain images!") };
        let image_views = SwapChain::create_image_views(context.as_ref(), &images, image_format).expect("Failed to create image views!") ;

        let render_semaphores = (0..images.len())
            .map(|_| {
                unsafe { context.device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
            }).collect::<Result<Vec<_>, _>>()
            .expect("Failed to create render semaphores!");

        let swapchain_images = izip!(images, image_views, render_semaphores)
            .map(|(image, image_view, render_finished)| {
                SwapChainImage { image, image_view, render_finished }
            }).collect::<Vec<_>>();

        SwapChain{ swapchain_loader, swapchain, swapchain_images, image_format, image_extent, context }
    }
}

impl Drop for SwapChain {
    fn drop(&mut self) {
        unsafe {
            println!("Destroying image_views and semaphores...");
            for image in &self.swapchain_images {
                self.context.device.destroy_image_view(image.image_view, None);
                self.context.device.destroy_semaphore(image.render_finished, None);
            }
            println!("Destroying swapchain...");
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct SwapChainImage {
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) render_finished: vk::Semaphore
}

impl SwapChainImage {
    pub fn transition_image(&self, context: &SharedContext, command_buffer: vk::CommandBuffer, current_layout: vk::ImageLayout, new_layout: vk::ImageLayout){
        transition_image(context, command_buffer, self.image, current_layout, new_layout)
    }
}
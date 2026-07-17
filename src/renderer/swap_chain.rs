use crate::renderer::context::Context;
use ash::khr::swapchain;
use ash::vk;
use itertools::izip;

#[derive(Debug, Copy, Clone)]
pub struct SwapChainImage {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub render_finished: vk::Semaphore
}

impl SwapChainImage {
    pub fn destroy(&self, context: &Context){
        unsafe {
            context.device().destroy_semaphore(self.render_finished, None);
            context.device().destroy_image_view(self.image_view, None);
        }
    }
}

pub struct SwapChain {
    swap_chain_loader: swapchain::Device,
    swap_chain: vk::SwapchainKHR,

    swap_chain_images: Vec<SwapChainImage>,

    image_format: vk::Format,
    image_extent: vk::Extent2D,
}

impl SwapChain {

    pub fn swap_chain_loader(&self) -> &swapchain::Device { &self.swap_chain_loader }
    pub fn swap_chain(&self) -> vk::SwapchainKHR { self.swap_chain }
    pub fn swap_chain_images(&self) -> &[SwapChainImage] { &self.swap_chain_images }

    // pub fn image_format(&self) -> vk::Format { self.image_format }
    // pub fn image_extent(&self) -> vk::Extent2D { self.image_extent }
    fn create_image_views(context: &Context, images: &[vk::Image], image_format: vk::Format) -> Result<Vec<vk::ImageView>, vk::Result>{
        let image_views: Vec<vk::ImageView> = images.iter().map(|&image| {
            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(image_format)
                .components(vk::ComponentMapping::default())
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1)
                );
            unsafe { context.device().create_image_view(&image_view_create_info, None) }
        }).collect::<Result<Vec<_>, _>>()?;
        Ok( image_views )
    }

    fn query_create_info(window_width: u32, window_height: u32, context: &Context) -> Result<(vk::SwapchainCreateInfoKHR<'static>, vk::Format, vk::Extent2D), vk::Result> {
        let surface_format = unsafe{
            context.surface_loader().get_physical_device_surface_formats(context.physical_device(), context.surface())?[0]
        };
        let surface_capabilities = unsafe{
            context.surface_loader().get_physical_device_surface_capabilities(context.physical_device(), context.surface())?
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
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = unsafe { context.surface_loader()
            .get_physical_device_surface_present_modes(context.physical_device(), context.surface())?
        };
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::FIFO_RELAXED)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swap_chain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.surface())
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

    pub fn new(window_width: u32, window_height: u32, context: &Context) -> SwapChain{
       let (swap_chain_create_info, image_format, image_extent) = SwapChain::query_create_info(window_width, window_height, context).expect("Failed to query swap chain info!");

        let swap_chain_loader = swapchain::Device::new(context.instance(), context.device());
        let swap_chain = unsafe { swap_chain_loader.create_swapchain(&swap_chain_create_info, None).expect("Failed to create swap chain!") };

        let images = unsafe { swap_chain_loader.get_swapchain_images(swap_chain).expect("Failed to get swap chain images!") };
        let image_views = SwapChain::create_image_views(context, &images, image_format).expect("Failed to create image views!") ;

        let render_semaphores = (0..images.len()).map(|_| {
            unsafe { context.device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
        }).collect::<Result<Vec<_>, _>>()
            .expect("Failed to create render semaphores!");

        let swap_chain_images = izip!(images, image_views, render_semaphores)
            .map(|(image, image_view, render_finished)| {
                SwapChainImage { image, image_view, render_finished }
            }).collect::<Vec<_>>();

        SwapChain{ swap_chain_loader, swap_chain, swap_chain_images, image_format, image_extent }
    }

    pub fn destroy(&self, context: &Context){
        for image in &self.swap_chain_images {
            image.destroy(context);
        }
        unsafe { self.swap_chain_loader.destroy_swapchain(self.swap_chain, None); }
    }
}
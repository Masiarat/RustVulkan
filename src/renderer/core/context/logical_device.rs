use ash::khr::surface;
use ash::{Device, Instance, vk};

pub fn create_device(instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR, surface_loader: &surface::Instance) -> Result<(Device, vk::Queue, u32), vk::Result> {
    // Queue family
    let queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
    let queue_family_index = queue_families
        .iter()
        .position(|info| info.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .expect("Failed to find a suitable graphics queue family") as u32;

    let queue_priorities = [1.0];
    let queue_create_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&queue_priorities);

    let surface_support = unsafe { surface_loader.get_physical_device_surface_support(physical_device, queue_family_index, surface).expect("Could not get physical device surface!") };
    assert!(surface_support, "Surface does not support queue family");

    // Logical device
    let mut vk_11_features = vk::PhysicalDeviceVulkan11Features::default()
        .shader_draw_parameters(true);
    let mut vk_12_features = vk::PhysicalDeviceVulkan12Features::default()
        .buffer_device_address(true)
        .descriptor_indexing(true);
    let mut vk_13_features = vk::PhysicalDeviceVulkan13Features::default()
        .dynamic_rendering(true)
        .synchronization2(true);
    let vk_10_features = vk::PhysicalDeviceFeatures::default()
        .sampler_anisotropy(true);

    let mut features2 = vk::PhysicalDeviceFeatures2::default()
        .features(vk_10_features)
        .push_next(&mut vk_11_features)
        .push_next(&mut vk_12_features)
        .push_next(&mut vk_13_features);

    let device_extensions = [
        vk::KHR_SWAPCHAIN_NAME.as_ptr()
    ];

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_create_info))
        .enabled_extension_names(&device_extensions)
        .push_next(&mut features2);

    let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
    Ok((device, queue, queue_family_index))
}

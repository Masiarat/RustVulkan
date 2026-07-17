use crate::renderer::debug_messenger::DebugMessenger;
use ash::ext::debug_utils;
use ash::khr::surface;
use ash::{vk, Device, Entry, Instance};
use std::ffi::c_char;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

const DEFAULT_PHYSICAL_DEVICE: usize = 0;

pub(crate) struct Context{
    entry: Entry,
    pub(crate) instance: Instance,
    debug: DebugMessenger,
    pub(crate) surface: vk::SurfaceKHR,
    pub(crate) surface_loader: surface::Instance,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) present_queue: vk::Queue,
    pub(crate) present_queue_family: u32,
    pub(crate) device: Device,
}

fn create_instance(entry: &Entry, window: &Window) -> Result<Instance, vk::Result> {
    let app_info = vk::ApplicationInfo::default()
        .application_name(c"rust + vulkan + winit")
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let layer_names = [c"VK_LAYER_KHRONOS_validation"];
    let layers_names_raw: Vec<*const c_char> = layer_names
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();

    let mut extension_names =
        ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw())?
            .to_vec();
    extension_names.push(debug_utils::NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layers_names_raw);

    Ok( unsafe { entry.create_instance(&create_info, None)? } )
}

fn select_physical_device(instance: &Instance, device_index: usize) -> Result<vk::PhysicalDevice, vk::Result> {
    let devices = unsafe { instance.enumerate_physical_devices()? };

    if device_index >= devices.len() { panic!("Device index {} out of bounds", device_index); }
    let physical_device = devices[device_index];

    let mut device_properties = vk::PhysicalDeviceProperties2::default();
    unsafe { instance.get_physical_device_properties2(physical_device, &mut device_properties) };
    let device_name = unsafe { std::ffi::CStr::from_ptr(device_properties.properties.device_name.as_ptr()) }.to_string_lossy();
    println!("Selected physical device: {}", device_name);

    Ok( physical_device )
}

fn create_device(instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR, surface_loader: &surface::Instance) -> Result<(Device, vk::Queue, u32), vk::Result> {
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

    unsafe { surface_loader.get_physical_device_surface_support(physical_device, queue_family_index, surface).expect("Could not get physical device surface!") };

    // Logical device
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

fn create_surface(entry: &Entry, instance: &Instance, window: &Window) -> Result<(vk::SurfaceKHR, surface::Instance), vk::Result> {
    let window_handle = window.window_handle().expect("Could not get window handle").as_raw();
    let display_handle = window.display_handle().expect("Could not get display handle").as_raw();

    let surface = unsafe { ash_window::create_surface(entry, instance, display_handle, window_handle, None)? };
    let surface_loader = surface::Instance::new(&entry, &instance);

    Ok( (surface, surface_loader) )
}

impl Context{
    pub fn new(window: &Window) -> Context {
        let entry = Entry::linked();
        let instance = create_instance(&entry, &window).expect("Failed to create instance");
        let debug = DebugMessenger::new(&entry, &instance).expect("Failed to create debug messenger");
        let (surface, surface_loader) = create_surface(&entry, &instance, &window).expect("Failed to create surface");
        let physical_device = select_physical_device(&instance, DEFAULT_PHYSICAL_DEVICE).expect("Failed to select physical device");
        let (device, present_queue, present_queue_family) = create_device(&instance, physical_device, surface, &surface_loader).expect("Failed to create device");

        Context{ entry, instance, debug, surface, surface_loader, physical_device, device, present_queue, present_queue_family }
    }
    
    //TODO change to public fields
    
    pub fn destroy(&self){
        unsafe {
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug.destroy();
            self.instance.destroy_instance(None);
        }
    }
}

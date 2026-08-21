use ash::{vk, Instance};

pub fn select_physical_device(instance: &Instance, device_index: usize) -> Result<vk::PhysicalDevice, vk::Result> {
    let devices = unsafe { instance.enumerate_physical_devices()? };

    if device_index >= devices.len() { panic!("Device index {} out of bounds", device_index); }
    let physical_device = devices[device_index];

    let mut device_properties = vk::PhysicalDeviceProperties2::default();
    unsafe { instance.get_physical_device_properties2(physical_device, &mut device_properties) };
    let device_name = unsafe { std::ffi::CStr::from_ptr(device_properties.properties.device_name.as_ptr()) }.to_string_lossy();
    println!("Selected physical device: {}", device_name);

    Ok( physical_device )
}
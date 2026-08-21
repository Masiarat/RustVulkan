use ash::ext::debug_utils;
use ash::{vk, Entry, Instance};
use std::ffi::c_char;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;

pub fn create_instance(entry: &Entry, window: &Window) -> Result<Instance, vk::Result> {
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
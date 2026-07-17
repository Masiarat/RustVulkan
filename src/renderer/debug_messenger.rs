use ash::ext::debug_utils;
use ash::{vk, Entry, Instance};
use colored::Colorize;
use std::borrow::Cow;
use std::ffi;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 { unsafe {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "ERROR".red().bold(),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "WARN".yellow().bold(),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "INFO".blue(),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "TRACE".dimmed(),
        _ => "UNKNOWN".normal(),
    };

    println!(
        "[{}] [{:?}] {}: {}", severity, message_type, message_id_name, message
    );

    vk::FALSE
    }
}

pub struct DebugMessenger {
    debug_loader: debug_utils::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugMessenger {
    pub fn new(entry: &Entry, instance: &Instance) -> Result<DebugMessenger, vk::Result> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_loader = debug_utils::Instance::new(&entry, &instance);
        let debug_messenger = unsafe {
            debug_loader.create_debug_utils_messenger(
                &debug_info,
                None,
            )?
        };
        Ok ( DebugMessenger{debug_loader, debug_messenger } )
    }

    pub unsafe fn destroy(self) { unsafe {
        self.debug_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
    }}
}
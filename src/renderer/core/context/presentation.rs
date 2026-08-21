use ash::khr::surface;
use ash::{vk, Entry, Instance};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

pub fn create_surface(entry: &Entry, instance: &Instance, window: &Window) -> Result<(vk::SurfaceKHR, surface::Instance), vk::Result> {
    let window_handle = window.window_handle().expect("Could not get window handle").as_raw();
    let display_handle = window.display_handle().expect("Could not get display handle").as_raw();

    let surface = unsafe { ash_window::create_surface(entry, instance, display_handle, window_handle, None)? };
    let surface_loader = surface::Instance::new(&entry, &instance);

    Ok( (surface, surface_loader) )
}

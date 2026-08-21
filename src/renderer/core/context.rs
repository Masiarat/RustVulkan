pub mod allocator;
mod bootstrap;
mod debug_messenger;
mod logical_device;
mod presentation;
mod selection;
pub mod destroy_queue;

use crate::error::{RendererError, RendererResult};
use crate::renderer::core::context::allocator::GpuAllocator;
use ash::khr::surface;
use ash::{vk, Device, Entry, Instance};
use debug_messenger::DebugMessenger;
use std::ops::DerefMut;
use std::sync::Mutex;
use winit::window::Window;

const DEFAULT_PHYSICAL_DEVICE: usize = 0;

pub(crate) struct SharedContext {
    entry: Entry,
    pub(crate) instance: Instance,
    debug: DebugMessenger,
    pub(crate) surface: vk::SurfaceKHR,
    pub(crate) surface_loader: surface::Instance,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) present_queue: vk::Queue,
    pub(crate) present_queue_family: u32,
    pub(crate) device: Device,
    allocator: Mutex<GpuAllocator>,
    destroy_queue: Mutex<destroy_queue::DestroyQueue>,
}

impl SharedContext {
    pub fn new(window: &Window) -> SharedContext {
        let entry = Entry::linked();
        let instance =
            bootstrap::create_instance(&entry, &window).expect("Failed to create instance");

        let debug =
            DebugMessenger::new(&entry, &instance).expect("Failed to create debug messenger");

        let (surface, surface_loader) =
            presentation::create_surface(&entry, &instance, &window)
            .expect("Failed to create surface");

        let physical_device =
            selection::select_physical_device(&instance, DEFAULT_PHYSICAL_DEVICE)
            .expect("Failed to select physical device");

        let (device, present_queue, present_queue_family) =
            logical_device::create_device(&instance, physical_device, surface, &surface_loader)
                .expect("Failed to create device");

        let allocator =
            GpuAllocator::new(instance.clone(), device.clone(), physical_device.clone())
                .expect("Failed to create allocator");

        let destroy_queue = destroy_queue::DestroyQueue::new();

        SharedContext {
            entry,
            instance,
            debug,
            surface,
            surface_loader,
            physical_device,
            device,
            present_queue,
            present_queue_family,
            allocator: Mutex::new(allocator),
            destroy_queue: Mutex::new(destroy_queue),
        }
    }

    pub fn with_destroy_queue<R>(&self, f: impl FnOnce(&mut destroy_queue::DestroyQueue) -> R) -> RendererResult<R> {
        let mut destroy_queue = self.destroy_queue.lock().map_err(|_e| {
            RendererError::GpuAllocatorMessage("Destroy queue mutex poisoned".to_string())
        })?;
        Ok(f(&mut destroy_queue))
    }

    pub fn with_allocator<R>(&self, f: impl FnOnce(&mut GpuAllocator) -> R) -> RendererResult<R> {
        let mut allocator = self.allocator.lock().map_err(|_e| {
            RendererError::GpuAllocatorMessage("Allocator mutex poisoned".to_string())
        })?;
        Ok(f(&mut allocator))
    }
}

impl Drop for SharedContext {
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = self.with_destroy_queue(|q| q.flush(self)) {
                println!("Failed to flush destroy queue: {}", e);
            }
            println!("Destroying allocator");
            if let Ok(mut guard) = self.allocator.lock() {
                if let Err(e) = guard.deref_mut().free() {
                    println!("Failed to free allocator: {}", e);
                }
            }
            println!("Destroying device");
            self.device.destroy_device(None);
            println!("Destroying surface");
            self.surface_loader.destroy_surface(self.surface, None);
            println!("Destroying debug messenger");
            self.debug.destroy();
            println!("Destroying instance");
            self.instance.destroy_instance(None);
        }
    }
}

pub mod allocator;
mod bootstrap;
mod debug_messenger;
mod logical_device;
mod presentation;
mod selection;

use crate::error::{RendererError, RendererResult};
use crate::renderer::core::context::allocator::GpuAllocator;
use crate::renderer::core::destroy_queue::ResourceGarbage;
use crate::renderer::sync::frame::Frame;
use ash::khr::surface;
use ash::{Device, Entry, Instance, vk};
use debug_messenger::DebugMessenger;
use std::ops::DerefMut;
use std::sync::{Mutex, mpsc};
use std::time::Duration;
use winit::window::Window;

const DEFAULT_PHYSICAL_DEVICE: usize = 0;

pub(crate) struct SharedContext {
    pub trash_tx: mpsc::Sender<ResourceGarbage>,
    pub trash_rx: mpsc::Receiver<ResourceGarbage>,
    allocator: Mutex<GpuAllocator>,
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

        let (trash_tx, trash_rx) = mpsc::channel();

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
            trash_tx,
            trash_rx,
        }
    }

    pub fn with_allocator<R>(&self, f: impl FnOnce(&mut GpuAllocator) -> R) -> RendererResult<R> {
        let mut allocator = self.allocator.lock().map_err(|_e| {
            RendererError::GpuAllocatorMessage("Allocator mutex poisoned".to_string())
        })?;
        Ok(f(&mut allocator))
    }

    pub fn collect_garbage(&self, frame: &mut Frame) -> RendererResult<()> {
        self.trash_rx
            .try_iter()
            .map(|garbage| {
                frame.with_destroy_queue(|q| q.push(garbage))
            })
            .collect()
    }
    
    pub fn collect_garbage_timeout(&self, frame: &mut Frame, timeout: Duration) -> RendererResult<()> {
        while let Ok(garbage) = self.trash_rx.recv_timeout(timeout) {
            frame.with_destroy_queue(|q| q.push(garbage))?;
        }
        Ok(())
    }
}

impl Drop for SharedContext {
    fn drop(&mut self) {
        unsafe {
            // if let Err(e) = self.with_destroy_queue(|q| q.flush(self)) {
            //     println!("Failed to flush destroy queue: {}", e);
            // }
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

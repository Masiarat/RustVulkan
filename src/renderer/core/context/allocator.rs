use crate::error::{RendererError, RendererResult};
use ash::vk::PhysicalDevice;
use ash::{Device, Instance};
use gpu_allocator::vulkan::AllocatorCreateDesc;

pub struct GpuAllocator {
    allocator: Option<gpu_allocator::vulkan::Allocator>,
}

impl GpuAllocator {
    pub fn new(
        instance: Instance,
        device: Device,
        physical_device: PhysicalDevice,
    ) -> Result<GpuAllocator, gpu_allocator::AllocationError> {
        let allocator_create_desc = AllocatorCreateDesc {
            instance,
            device,
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        };
        Ok(GpuAllocator {
            allocator: Some(gpu_allocator::vulkan::Allocator::new(
                &allocator_create_desc,
            )?),
        })
    }

    pub fn allocate(
        &mut self,
        allocation_create_desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> RendererResult<gpu_allocator::vulkan::Allocation> {
        let allocator = self
            .allocator
            .as_mut()
            .ok_or(RendererError::GpuAllocatorMessage(
                "Allocator was not initialized".to_string(),
            ))?;
        allocator
            .allocate(allocation_create_desc)
            .map_err(|e| RendererError::GpuAllocatorError(e))
    }

    pub fn free_allocation(
        &mut self,
        allocation: gpu_allocator::vulkan::Allocation,
    ) -> RendererResult<()> {
        let allocator = self
            .allocator
            .as_mut()
            .ok_or(RendererError::GpuAllocatorMessage(
                "Allocator was not initialized".to_string(),
            ))?;
        allocator
            .free(allocation)
            .map_err(|e| RendererError::GpuAllocatorError(e))
    }

    pub fn free(&mut self) -> RendererResult<()> {
        drop(
            self.allocator
                .take()
                .ok_or(RendererError::GpuAllocatorMessage(
                    "Allocator was not initialized".to_string(),
                ))?,
        );
        Ok(())
    }

    // pub fn with_allocator<R>(&self, f: impl FnOnce(&gpu_allocator::vulkan::Allocator) -> R) -> RendererResult<R>{
    //     let allocator = self.allocator.as_ref().ok_or(RendererError::GpuAllocatorMessage("Allocator was not initialized".to_string()))?;
    //     Ok( f(allocator) )
    // }
}

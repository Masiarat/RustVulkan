use crate::error::RendererResult;
use crate::renderer::core::context::destroy_queue::Destroyable;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::util::standard_buffer_create_info;
use ash::vk;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::MemoryLocation;
use std::error::Error;
use std::sync::Arc;

pub struct Buffer {
    pub buffer: vk::Buffer,
    allocation: Option<Allocation>,

    context: Arc<SharedContext>,
}

impl Buffer {
    pub fn new(
        context: Arc<SharedContext>,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        name: &str,
        location: MemoryLocation,
    ) -> Result<Buffer, Box<dyn Error>> {
        let buffer_ci = standard_buffer_create_info(size, usage);
        let device = &context.device;
        let buffer = unsafe { device.create_buffer(&buffer_ci, None)? };

        let allocation = Self::allocate(&context, buffer, name, location)?;
        unsafe { device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())? };

        Ok(Buffer {
            buffer,
            allocation: Some(allocation),
            context,
        })
    }

    pub fn allocate(
        context: &SharedContext,
        buffer_info: vk::Buffer,
        name: &str,
        location: MemoryLocation,
    ) -> RendererResult<Allocation> {
        let requirements = unsafe { context.device.get_buffer_memory_requirements(buffer_info) };

        let allocation_desc = gpu_allocator::vulkan::AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedBuffer(
                buffer_info,
            ),
        };

        context.with_allocator(|allocator| allocator.allocate(&allocation_desc))?
    }

    pub fn copy_from_slice(&mut self, data: &[u8]) -> Result<(), String> {
        if let Some(allocation) = &mut self.allocation {
            if let Some(mapped_slice) = allocation.mapped_slice_mut() {
                if data.len() > mapped_slice.len() {
                    return Err(format!(
                        "Dane ({} bajtów) są większe niż zaalokowany bufor ({} bajtów)!",
                        data.len(),
                        mapped_slice.len()
                    ));
                }
                // Bezpieczne kopiowanie danych do zmapowanej pamięci
                mapped_slice[..data.len()].copy_from_slice(data);
                Ok(())
            } else {
                Err(
                    "Pamięć bufora nie jest zmapowana! (Czy użyto MemoryLocation::CpuToGpu?)"
                        .to_string(),
                )
            }
        } else {
            Err("Bufor nie posiada poprawnej alokacji pamięci.".to_string())
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            let retired = RetiredBuffer { buffer: self.buffer, allocation };
            if let Err(e) = self
                .context
                .with_destroy_queue(|queue| queue.push(Box::new(retired)))
            {
                println!("Error queueing buffer cleanup {}", e);
            }
        }else{
            println!("Error queueing buffer cleanup, allocation is None");
        }
    }
}

#[derive(Debug)]
struct RetiredBuffer {
    buffer: vk::Buffer,
    allocation: Allocation,
}

impl Destroyable for RetiredBuffer {
    fn destroy(self: Box<Self>, context: &SharedContext) -> RendererResult<()> {
        unsafe{ context.device.destroy_buffer(self.buffer, None) }
        context.with_allocator(|allocator| allocator.free_allocation(self.allocation))?
    }
}
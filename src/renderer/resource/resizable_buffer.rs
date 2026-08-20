use crate::renderer::core::context::SharedContext;
use crate::renderer::resource::buffer::Buffer;
use ash::vk;
use gpu_allocator::MemoryLocation;
use std::cmp::min;
use std::error::Error;
use std::sync::Arc;

pub struct ResizableBuffer {
    buffer: Buffer,
    capacity: vk::DeviceSize,
    context: Arc<SharedContext>,
    usage_flags: vk::BufferUsageFlags,
    name: String,
    location: MemoryLocation,
}

impl ResizableBuffer {
    pub fn new(
        context: Arc<SharedContext>,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        name: &str,
        location: MemoryLocation,
    ) -> Result<ResizableBuffer, Box<dyn Error>> {
        let name = name.to_string();
        let buffer = Buffer::new(context.clone(), size, usage, &name, location)?;
        Ok( ResizableBuffer {buffer, capacity: size, context, usage_flags: usage, name, location } )
    }

    pub fn resize(&mut self, size: vk::DeviceSize) -> Result<(), Box<dyn Error>>{
        if size <= self.capacity { return Ok(()) }
        let mut new_size = min(self.capacity, 2);
        while new_size < size {
            new_size *= 2;
        };
        let new_buffer = Buffer::new(self.context.clone(), new_size, self.usage_flags, &self.name, self.location)?;
        self.buffer = new_buffer;
        Ok( () )
    }
    pub fn copy_from_slice(&mut self, p0: &[u8]) -> Result<(), String> {
        self.buffer.copy_from_slice(p0)
    }

    pub fn buffer(&self) -> vk::Buffer{
        self.buffer.buffer
    }

}
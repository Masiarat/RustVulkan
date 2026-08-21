use std::ffi::NulError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] ash::vk::Result),

    #[error("Gpu Allocator error: {0}")]
    GpuAllocatorError(#[from] gpu_allocator::AllocationError),

    #[error("Gpu Allocator error: {0}")]
    GpuAllocatorMessage(String),

    #[error("Shader compilation failed: {0}")]
    ShaderError(String),

    #[error("Buffer allocation failed: {0}")]
    BufferAllocationError(String),

    #[error("Image allocation failed: {0}")]
    ImageAllocationError(String),

    #[error("Descriptor allocation failed: {0}")]
    DescriptorAllocationError(String),

    #[error("Pipeline initialization failed: {0}")]
    PipelineError(String),

    #[error("Texture management failed: {0}")]
    TextureError(String),

    #[error("Device initialization failed: {0}")]
    DeviceInitError(String),

    #[error("Swapchain error: {0}")]
    SwapchainError(String),

    #[error("Frame synchronization failed: {0}")]
    FrameSyncError(String),

    #[error("Invalid CString: {0}")]
    CStringError(#[from] NulError),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Resource cleanup failed: {0}")]
    CleanupError(String),
}

pub type RendererResult<T> = Result<T, RendererError>;

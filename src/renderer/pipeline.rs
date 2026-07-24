use ash::vk::ShaderModule;
use ash::{vk, Device};
use std::fs;
use std::io;
use std::path::Path;

/// Load a SPIR-V file from the disk and create a `vk::ShaderModule`.
pub unsafe fn load_shader_module(device: &Device, path: &Path) -> Result<ShaderModule, ShaderLoadError> {
    let bytes = fs::read(path).map_err(|e| ShaderLoadError::Io(path.to_path_buf(), e))?;
    unsafe { load_shader_module_from_bytes(device, &bytes) }
}

/// Build a shader module from a SPIR-V byte slice.
pub unsafe fn load_shader_module_from_bytes(
    device: &Device,
    spirv: &[u8],
) -> Result<vk::ShaderModule, ShaderLoadError> {
    if spirv.len() % std::mem::size_of::<u32>() != 0 {
        return Err( ShaderLoadError::ByteLength(spirv.len()) );
    }
    // SPIR-V is a stream of 32-bit words; Vulkan wants a `&[u32]`.
    let code: &[u32] = unsafe {
        std::slice::from_raw_parts(
            spirv.as_ptr().cast::<u32>(),
            spirv.len() / std::mem::size_of::<u32>(),
        )
    };

    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    unsafe { device.create_shader_module(&create_info, None).map_err(ShaderLoadError::Create) }
}

#[derive(Debug, thiserror::Error)]
pub enum ShaderLoadError {
    #[error("failed to read shader file `{0}`: {1}")]
    Io(std::path::PathBuf, io::Error),
    #[error("SPIR-V byte length {0} is not a multiple of 4")]
    ByteLength(usize),
    #[error("vkCreateShaderModule failed: {0}")]
    Create(vk::Result),
}

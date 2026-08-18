use crate::renderer::buffer::Buffer;
use crate::renderer::context::SharedContext;
use crate::renderer::descriptor::{Allocator, PoolSizeRatio};
use crate::renderer::image::Image;
use crate::renderer::immediate_submitter::ImmediateSubmitter;
use crate::renderer::pipeline::{load_shader_module, PipelineBuilder};
use ash::vk;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::CStr;
use std::sync::Arc;

pub struct EguiPainter {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    pub textures: HashMap<egui::TextureId, (crate::renderer::image::Image, vk::DescriptorSet)>,

    pub sampler: vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub font_descriptor_set: vk::DescriptorSet,

    pub descriptor_allocator: Allocator,
    context: Arc<SharedContext>,
}

impl EguiPainter {
    pub fn new(
        context: Arc<SharedContext>,
        swapchain_format: vk::Format,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let device = &context.device;

        // ==========================================
        // 1. DESCRIPTOR SET LAYOUT (Koszyk na tekstury)
        // ==========================================
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout = unsafe {
            device.create_descriptor_set_layout(&layout_info, None)?
        };

        // ==========================================
        // 2. SAMPLER (Jak tekstury mają być filtrowane)
        // ==========================================
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);

        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        // ==========================================
        // 3. DESCRIPTOR ALLOCATOR & FONT DESCRIPTOR SET
        // ==========================================
        // Inicjalizujemy alokator descriptorów (zakładam, że Twoja struktura Allocator ma metodę new)
        let mut descriptor_allocator = Allocator::new(context.clone(), 1, &[PoolSizeRatio{ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, ratio: 1.0 }])?;

        // Alokujemy domyślny descriptor set (np. dla głównej czcionki egui)
        let font_descriptor_set = descriptor_allocator.allocate(descriptor_set_layout)?;

        // ==========================================
        // 4. PIPELINE LAYOUT (Plan układu pamięci)
        // ==========================================
        // 8 bajtów na Push Constant (2x float32 dla szerokości i wysokości ekranu)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(8);

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_info, None)?
        };

        // ==========================================
        // 5. ŁADOWANIE SHADERÓW Z OUT_DIR (egui.spv)
        // ==========================================
        let path = concat!(env!("OUT_DIR"), "/egui.spv").as_ref();
        let egui_shader_module = unsafe { load_shader_module(device, path)? };

        // Nazwy funkcji z naszego pliku Slang (muszą być zakończone bajtem null \0)
        let vertex_entry = CStr::from_bytes_with_nul(b"vertexMain\0")?;
        let fragment_entry = CStr::from_bytes_with_nul(b"fragmentMain\0")?;

        let vertex_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(egui_shader_module)
            .name(vertex_entry);

        let fragment_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(egui_shader_module)
            .name(fragment_entry);

        // ==========================================
        // 6. OPIS WIERZCHOŁKA (Vertex Input)
        // ==========================================
        let vertex_binding = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<egui::epaint::Vertex>() as u32) // 20 bajtów
            .input_rate(vk::VertexInputRate::VERTEX)];

        let vertex_attributes = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT) // pos X, Y
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT) // uv U, V
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM) // color R,G,B,A
                .offset(16),
        ];

        // ==========================================
        // 7. BUDOWANIE PIPELINE'U!
        // ==========================================
        let pipeline = PipelineBuilder::new()
            .set_pipeline_layout(pipeline_layout)
            .set_color_attachment_format(swapchain_format)
            .disable_depthtest() // Brak osi Z w UI
            .enable_blending_premultiplied_alpha() // Ważne dla przezroczystości!
            .set_vertex_input(&vertex_binding, &vertex_attributes)
            .add_shader_stage(vertex_stage)
            .add_shader_stage(fragment_stage)
            .build(device)?;

        // Moduł shadera jest już wkompilowany w Pipeline, możemy go bezpiecznie zniszczyć
        unsafe {
            device.destroy_shader_module(egui_shader_module, None);
        }

        Ok(Self {
            pipeline_layout,
            pipeline,
            textures: HashMap::new(),
            sampler,
            descriptor_set_layout,
            font_descriptor_set,
            descriptor_allocator,
            context,
        })
    }

    pub fn update_textures(
        &mut self,
        context: Arc<SharedContext>,
        immediate_submitter: &ImmediateSubmitter,
        textures_delta: &egui::TexturesDelta, // <-- Pobieramy pełną deltę z pętli!
    ) -> Result<(), Box<dyn Error>> {
        let device = &context.device;

        // 1. ZWALNIANIE STARYCH TEKSTUR (FREE)
        for texture_id in &textures_delta.free {
            // Usunięcie z mapy automatycznie wywoła nasz świetny Drop na Image!
            self.textures.remove(texture_id);
        }

        // 2. WGRYWANIE / AKTUALIZACJA NOWYCH TEKSTUR (SET)
        for (texture_id, image_delta) in &textures_delta.set {

            // Konwersja pikseli z egui na format RGBA
            let (width, height, pixels) = match &image_delta[0].image {
                egui::ImageData::Color(color_image) => {
                    let w = color_image.width() as u32;
                    let h = color_image.height() as u32;
                    let pixels: Vec<u8> = color_image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    (w, h, pixels)
                }
                // egui::ImageData::Font(font_image) => {
                //     let w = font_image.width() as u32;
                //     let h = font_image.height() as u32;
                //     let pixels: Vec<u8> = font_image.srgba_pixels(None).flat_map(|p| p.to_array()).collect();
                //     (w, h, pixels)
                // }
            };

            // Tworzymy Staging Buffer
            let mut staging_buffer = Buffer::new(
                context.clone(),
                pixels.len() as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                &format!("egui_staging_buffer_{:?}", texture_id),
                gpu_allocator::MemoryLocation::CpuToGpu,
            )?;

            staging_buffer.copy_from_slice(&pixels)?;

            // Tworzymy docelową teksturę Image
            let extent = vk::Extent2D { width, height };
            let texture = Image::new_texture(
                context.clone(),
                extent,
                &format!("egui_texture_{:?}", texture_id) // Dynamiczna nazwa dla alokatora
            )?;

            // Kopiujemy dane przez Immediate Submitter
            immediate_submitter.submit(|cmd| {
                texture.transition_layout(cmd, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

                let copy_region = vk::BufferImageCopy::default()
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image_extent(vk::Extent3D { width, height, depth: 1 });

                unsafe {
                    device.cmd_copy_buffer_to_image(
                        cmd,
                        staging_buffer.buffer,
                        texture.image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_region],
                    );
                }

                texture.transition_layout(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            })?;

            // Tworzymy/Aktualizujemy Descriptor Set dla TEJ konkretnej tekstury
            // Uwaga: Tutaj zakładam na chwilę, że używasz jednego setu,
            // ale w pełni działającym silniku powinieneś tu zaalokować nowy set z Poola
            // Zrobiłem to na razie używając Twojego `self.font_descriptor_set` jako przykładu.
            let image_info = vk::DescriptorImageInfo::default()
                .sampler(self.sampler)
                .image_view(texture.image_view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

            let write_info = vk::WriteDescriptorSet::default()
                .dst_set(self.font_descriptor_set) // docelowo: stworzyć nowy dla każdego ID
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info));

            unsafe { device.update_descriptor_sets(&[write_info], &[]); }

            // Wrzucamy do naszego HashMapy - zastąpi ew. poprzednią wersję
            self.textures.insert(*texture_id, (texture, self.font_descriptor_set));
        }

        Ok(())
    }
}

impl Drop for EguiPainter {
    fn drop(&mut self) {
        unsafe{ self.context.device.destroy_sampler(self.sampler, None) }
    }
}
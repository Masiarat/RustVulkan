use ash::{vk, Device};

pub struct LayoutBuilder {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'static>>
}

impl LayoutBuilder {
    pub fn new() -> LayoutBuilder {
        LayoutBuilder { bindings: Vec::new() }
    }

    pub unsafe fn build(&mut self, device: &Device, shader_stages: vk::ShaderStageFlags, flags: vk::DescriptorSetLayoutCreateFlags) -> Result<vk::DescriptorSetLayout, vk::Result>{
        for binding in self.bindings.iter_mut(){
            binding.stage_flags |= shader_stages;
        }

        let descriptor_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags);

        //TODO make sure to free this
        unsafe{ device.create_descriptor_set_layout(&descriptor_layout_info, None) }
    }

    pub fn add_binding(&mut self, binding: u32, descriptor_type: vk::DescriptorType){
        let new_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(1);
        self.bindings.push(new_binding);
    }

    fn clear(&mut self){
        self.bindings.clear();
    }
}
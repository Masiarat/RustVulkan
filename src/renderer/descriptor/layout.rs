use ash::{vk, Device};

struct DescriptorLayoutBuilder{
    bindings: Vec<vk::DescriptorSetLayoutBinding<'static>>
}

impl DescriptorLayoutBuilder{
    fn new() -> DescriptorLayoutBuilder{
        DescriptorLayoutBuilder { bindings: Vec::new() }
    }

    fn build(&mut self, device: &Device, shader_stages: vk::ShaderStageFlags, flags: vk::DescriptorSetLayoutCreateFlags) -> Result<vk::DescriptorSetLayout, vk::Result>{
        for binding in self.bindings.iter_mut(){
            binding.stage_flags |= shader_stages;
        }

        let descriptor_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags);

        unsafe{ device.create_descriptor_set_layout(&descriptor_layout_info, None) }
    }

    fn add_binding(&mut self, binding: u32, descriptor_type: vk::DescriptorType){
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
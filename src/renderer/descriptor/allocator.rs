use crate::renderer::context::SharedContext;
use ash::vk;
use std::sync::Arc;

pub struct Allocator {
    pool: vk::DescriptorPool,
    allocated_layouts: Vec<vk::DescriptorSetLayout>,

    context: Arc<SharedContext>
}

impl Allocator {
    pub fn new(context: Arc<SharedContext>, max_sets: u32, pool_ratios: &[PoolSizeRatio]) -> Result<Allocator, vk::Result> {
        let pool_sizes = pool_ratios.iter().map(|x| {
            vk::DescriptorPoolSize::default().ty(x.ty).descriptor_count((max_sets as f32 * x.ratio) as u32)
        }).collect::<Vec<_>>();

        let allocated_layouts = Vec::new();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::empty())
            .max_sets(max_sets)
            .pool_sizes(&pool_sizes);

        let pool = unsafe{ context.device.create_descriptor_pool(&pool_info, None)? };
        Ok(Allocator{ context, allocated_layouts, pool })
    }

    pub fn clear(&mut self) -> Result<(), vk::Result>{
        unsafe{ self.context.device.reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty()) }
    }

    pub fn allocate(&mut self, descriptor_set_layout: vk::DescriptorSetLayout) -> Result<vk::DescriptorSet, vk::Result> {
        self.allocated_layouts.push(descriptor_set_layout);
        let layouts = &[descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(layouts);

        Ok ( unsafe{ self.context.device.allocate_descriptor_sets(&alloc_info)?[0] } )
    }
}

impl Drop for Allocator{
    fn drop(&mut self) {
        unsafe {
            println!("Destroying descriptor set layouts");
            let device = &self.context.device;
            for layout in self.allocated_layouts.iter(){
                device.destroy_descriptor_set_layout(*layout, None);
            }
            println!("Destroying descriptor pool");
            device.destroy_descriptor_pool(self.pool, None);
        }
    }
}


pub struct PoolSizeRatio{
    pub(crate) ty: vk::DescriptorType,
    pub(crate) ratio: f32
}
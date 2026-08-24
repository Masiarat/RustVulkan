use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::core::destroy_queue::ResourceGarbage;
use crate::renderer::core::util::standard_sampler_create_info;
use ash::vk;
use std::sync::mpsc::Sender;

pub struct Sampler {
    pub sampler: vk::Sampler,
    trash_tx: Sender<ResourceGarbage>
}

impl Sampler {
    pub fn new(context: &SharedContext) -> RendererResult<Self> {
        let sampler_info = standard_sampler_create_info();
        let sampler = unsafe { context.device.create_sampler(&sampler_info, None)? };
        Ok( Self {
            sampler,
            trash_tx: context.trash_tx.clone()
        } )
    }
}

impl Drop for Sampler{
    fn drop(&mut self){
        if let Err(e) = self.trash_tx.send(ResourceGarbage::Sampler(RetiredSampler(self.sampler))){
            eprintln!("Failed to send Sampler to destroy queue: {:?}", e);
        }
    }
}

pub struct RetiredSampler(pub vk::Sampler);

impl RetiredSampler {
    pub fn destroy(self, context: &SharedContext){
        unsafe { context.device.destroy_sampler(self.0, None) }
    }
}
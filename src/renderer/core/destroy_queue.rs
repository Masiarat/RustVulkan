use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use crate::renderer::resource::buffer::RetiredBuffer;
use crate::renderer::resource::image::RetiredImage;
use crate::renderer::resource::sampler::RetiredSampler;

pub enum ResourceGarbage {
    Buffer(RetiredBuffer),
    Image(RetiredImage),
    Sampler(RetiredSampler)
}

impl ResourceGarbage{
    fn dispose(self, context: &SharedContext) -> RendererResult<()>{
        match self {
            ResourceGarbage::Buffer(retired) => retired.destroy(context),
            ResourceGarbage::Image(retired) => retired.destroy(context),
            ResourceGarbage::Sampler(retired) => Ok(retired.destroy(context))
        }
    }
}

pub struct DestroyQueue {
    queue: Vec<ResourceGarbage>
}

impl DestroyQueue {
    pub fn new() -> DestroyQueue {
        DestroyQueue {
            queue: Vec::new(),
        }
    }

    pub fn push(&mut self, garbage: ResourceGarbage) {
        self.queue.push(garbage);
    }

    pub fn flush(&mut self, context: &SharedContext) -> RendererResult<()> {
        let mut result = RendererResult::Ok(());

        for garbage in self.queue.drain(..) {
            if let Err(e) = garbage.dispose(context) {
                result = Err(e);
            }
        }
        result
    }

}

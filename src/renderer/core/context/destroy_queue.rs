use crate::error::RendererResult;
use crate::renderer::core::context::SharedContext;
use std::fmt::Debug;

pub trait Destroyable: Debug {
    fn destroy(self: Box<Self>, context: &SharedContext) -> RendererResult<()>;
}

pub struct DestroyQueue {
    queue: Vec<Box<dyn Destroyable>>,
}

impl DestroyQueue {
    pub fn new() -> DestroyQueue {
        DestroyQueue {
            queue: Vec::new(),
        }
    }
    pub fn push(&mut self, destroyable: Box<dyn Destroyable>) {
        self.queue.push(destroyable);
    }

    pub fn flush(&mut self, context: &SharedContext) -> RendererResult<()> {
        self.queue
            .drain(..)
            .map(|destroyable| {
                // println!("Destroying {:?}", destroyable);
                destroyable.destroy(context)
            })
            .collect()
    }
}

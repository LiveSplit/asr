use core::iter::FusedIterator;

use super::super::managed::ImageRef;
use super::{Class, Module};
use crate::{future::retry, Address, Process};

/// An image is a .NET DLL that is loaded by the game. The `Assembly-CSharp`
/// image is the main game assembly, and contains all the game logic.
#[derive(Copy, Clone)]
pub struct Image {
    pub(super) image: Address,
}

impl Image {
    /// Iterates over all [.NET classes](struct@Class) in the image.
    pub fn classes<'a>(
        &self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = Class> + 'a {
        let walk = module.walk();

        walk.runtime
            .classes(process, walk.pointer_size, ImageRef::new(self.image))
            .map(|class| Class {
                class: class.address,
            })
            .fuse()
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    pub fn get_class(&self, process: &Process, module: &Module, class_name: &str) -> Option<Class> {
        module
            .walk()
            .find_class(process, ImageRef::new(self.image), class_name)
            .map(|class| Class {
                class: class.address,
            })
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    /// This is the `await`able version of the [`get_class`](Self::get_class)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_get_class(
        &self,
        process: &Process,
        module: &Module,
        class_name: &str,
    ) -> Class {
        retry(|| self.get_class(process, module, class_name)).await
    }
}

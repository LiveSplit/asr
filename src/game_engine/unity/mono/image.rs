use core::iter::{self, FusedIterator};

use super::CSTR;
use super::{Class, Module};
use crate::future::retry;
use crate::{Address, Process};

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
        let class_cache_size = process
            .read::<i32>(
                self.image + module.offsets.image.class_cache + module.offsets.hash_table.size,
            )
            .unwrap_or_default() as _;

        let table_addr = match class_cache_size {
            0 => Address::NULL,
            _ => process
                .read_pointer(
                    self.image + module.offsets.image.class_cache + module.offsets.hash_table.table,
                    module.pointer_size,
                )
                .unwrap_or_default(),
        };

        (0..class_cache_size).flat_map(move |i| {
            let mut table = match table_addr {
                Address::NULL => None,
                addr => process
                    .read_pointer(
                        addr + module.size_of_ptr().wrapping_mul(i),
                        module.pointer_size,
                    )
                    .ok()
                    .filter(|addr| !addr.is_null()),
            };

            iter::from_fn(move || {
                let this_table = table?;
                let class = process.read_pointer(this_table, module.pointer_size).ok()?;

                table = process
                    .read_pointer(
                        this_table + module.offsets.class.next_class_cache,
                        module.pointer_size,
                    )
                    .ok()
                    .filter(|val| !val.is_null());

                Some(Class { class })
            })
            .fuse()
        })
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    pub fn get_class(&self, process: &Process, module: &Module, class_name: &str) -> Option<Class> {
        let name_space_index = class_name.rfind('.');

        self.classes(process, module).find(|class| {
            class.get_name::<CSTR>(process, module).is_ok_and(|name| {
                if let Some(name_space_index) = name_space_index {
                    let class_name_space = &class_name[..name_space_index];
                    let class_name = &class_name[name_space_index + 1..];

                    name.matches(class_name)
                        && class
                            .get_name_space::<CSTR>(process, module)
                            .is_ok_and(|name_space| name_space.matches(class_name_space))
                } else {
                    name.matches(class_name)
                }
            })
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

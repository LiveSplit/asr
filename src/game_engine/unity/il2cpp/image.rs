use super::CSTR;
use super::{Class, Module, Version};
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
    ) -> impl DoubleEndedIterator<Item = Class> + 'a {
        let type_count = process
            .read::<u32>(self.image + module.offsets.image.type_count)
            .unwrap_or_default() as u64;

        let metadata_ptr = match (type_count, module.version) {
            (0, _) => Address::NULL,
            (_, Version::Base | Version::V2019) => {
                self.image + module.offsets.image.matadata_handle
            }
            (_, _) => process
                .read_pointer(
                    self.image + module.offsets.image.matadata_handle,
                    module.pointer_size,
                )
                .unwrap_or_default(),
        };

        let metadata_handle = match metadata_ptr {
            Address::NULL => 0,
            handle => process.read::<u32>(handle).unwrap_or_default(),
        };

        let type_info_definition_table = match metadata_handle {
            0 => Address::NULL,
            _ => process
                .read_pointer(module.type_info_definition_table, module.pointer_size)
                .unwrap_or_default(),
        };

        let ptr = match (metadata_handle, type_info_definition_table) {
            (0, _) | (_, Address::NULL) => Address::NULL,
            _ => {
                type_info_definition_table + module.size_of_ptr().wrapping_mul(metadata_handle as _)
            }
        };

        (0..type_count).filter_map(move |i| {
            process
                .read_pointer(
                    ptr + module.size_of_ptr().wrapping_mul(i),
                    module.pointer_size,
                )
                .ok()
                .filter(|val| !val.is_null())
                .map(|class| Class { class })
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

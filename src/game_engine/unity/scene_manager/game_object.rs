use super::{SceneManager, CSTR};
use crate::{string::ArrayCString, Address, Address32, Address64, Error, PointerSize, Process};
use core::array;
use core::mem::MaybeUninit;

/// Representing a GameObject.
///
/// This contains the information about attached Components and other internals like activeSelf.
///
/// If you have an instance of a C# game object (which you might get via following a path from a
/// static field), the C++ game object is at + pointer_size * 2 (0x8 on 32 bit, 0x10 on 64 bit).
#[derive(Clone, Debug)]
pub struct GameObject {
    pub(super) address: Address,
}

impl GameObject {
    pub fn components<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Address> + 'a, Error> {
        let (number_of_components, main_object): (usize, Address) = match scene_manager.pointer_size
        {
            PointerSize::Bit64 => {
                let array = process
                    .read::<[Address64; 3]>(self.address + scene_manager.offsets.game_object)?;
                (array[2].value() as usize, array[0].into())
            }
            _ => {
                let array = process
                    .read::<[Address32; 3]>(self.address + scene_manager.offsets.game_object)?;
                (array[2].value() as usize, array[0].into())
            }
        };

        if number_of_components == 0 {
            return Err(Error {});
        }

        const ARRAY_SIZE: usize = 128;

        let components: [Address; ARRAY_SIZE] = match scene_manager.pointer_size {
            PointerSize::Bit64 => {
                let mut buf = [MaybeUninit::<[Address64; 2]>::uninit(); ARRAY_SIZE];
                let slice = process
                    .read_into_uninit_slice(main_object, &mut buf[..number_of_components])?;

                let mut iter = slice.iter_mut();
                array::from_fn(|_| {
                    iter.next()
                        .map(|&mut [_, second]| second.into())
                        .unwrap_or_default()
                })
            }
            _ => {
                let mut buf = [MaybeUninit::<[Address32; 2]>::uninit(); ARRAY_SIZE];
                let slice = process
                    .read_into_uninit_slice(main_object, &mut buf[..number_of_components])?;

                let mut iter = slice.iter_mut();
                array::from_fn(|_| {
                    iter.next()
                        .map(|&mut [_, second]| second.into())
                        .unwrap_or_default()
                })
            }
        };

        Ok((1..number_of_components).filter_map(move |m| {
            process
                .read_pointer(
                    components[m] + scene_manager.offsets.klass,
                    scene_manager.pointer_size,
                )
                .ok()
                .filter(|val| !val.is_null())
        }))
    }

    /// Tries to find the base address of a class in the current `GameObject`.
    pub fn get_class(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
    ) -> Result<Address, Error> {
        self.components(process, scene_manager)?
            .find(|&addr| {
                let val: Result<ArrayCString<CSTR>, Error> = match scene_manager.is_il2cpp {
                    true => process.read_pointer_path(
                        addr,
                        scene_manager.pointer_size,
                        &[0x0, scene_manager.size_of_ptr().wrapping_mul(2), 0x0],
                    ),
                    false => process.read_pointer_path(
                        addr,
                        scene_manager.pointer_size,
                        &[0x0, 0x0, scene_manager.offsets.klass_name as u64, 0x0],
                    ),
                };

                val.is_ok_and(|class_name| class_name.matches(name))
            })
            .ok_or(Error {})
    }

    pub fn is_active_in_hierarchy(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<bool, Error> {
        process.read::<bool>(self.address + scene_manager.offsets.game_object_activeinhierarchy)
    }
}

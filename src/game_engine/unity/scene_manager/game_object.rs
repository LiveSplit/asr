use super::{SceneManager, CSTR};
use crate::game_engine::unity::{il2cpp, mono};
use crate::string::ArrayCString;
use crate::{Address, Address32, Address64, Error, PointerSize, Process};
use core::array;
use core::mem::MaybeUninit;

/// Representing a GameObject. From a GameObject, you can get the attached components (includes the
/// C# scripts).
#[derive(Clone, Debug)]
pub struct GameObject {
    pub(super) address: Address,
}

impl GameObject {
    /// Get the name of the GameObject.
    pub fn get_name<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        process.read_pointer_path(
            self.address,
            scene_manager.pointer_size,
            &[scene_manager.offsets.game_object_name as u64, 0x0],
        )
    }

    /// Traverse the classes associated with the Components attached to this game object.
    pub fn classes<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Address> + 'a, Error> {
        let (number_of_components, component_pair_array): (usize, Address) =
            match scene_manager.pointer_size {
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
                let slice = process.read_into_uninit_slice(
                    component_pair_array,
                    &mut buf[..number_of_components],
                )?;

                let mut iter = slice.iter_mut();
                array::from_fn(|_| {
                    iter.next()
                        .map(|&mut [_, second]| second.into())
                        .unwrap_or_default()
                })
            }
            _ => {
                let mut buf = [MaybeUninit::<[Address32; 2]>::uninit(); ARRAY_SIZE];
                let slice = process.read_into_uninit_slice(
                    component_pair_array,
                    &mut buf[..number_of_components],
                )?;

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

    // TODO it's really dumb i have to split this by mono/il2cpp

    /// Tries to find the base address of a class in the current `GameObject` by name.
    ///
    /// Mono only.
    pub fn get_class_mono(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &mono::Module,
        name: &str,
    ) -> Result<Address, Error> {
        if scene_manager.is_il2cpp {
            return Err(Error {});
        }

        self.classes(process, scene_manager)?
            .find(|&addr| {
                let val = mono::Class::get_from_component(process, module, addr)
                    .and_then(|c| c.get_name::<CSTR>(process, module));

                val.is_ok_and(|class_name| class_name.matches(name))
            })
            .ok_or(Error {})
    }

    /// Tries to find the base address of a class in the current `GameObject` by name.
    ///
    /// IL2CPP only.
    pub fn get_class_il2cpp(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &il2cpp::Module,
        name: &str,
    ) -> Result<Address, Error> {
        if !scene_manager.is_il2cpp {
            return Err(Error {});
        }

        self.classes(process, scene_manager)?
            .find(|&addr| {
                let val = il2cpp::Class::get_from_component(process, module, addr)
                    .and_then(|c| c.get_name::<CSTR>(process, module));

                val.is_ok_and(|class_name| class_name.matches(name))
            })
            .ok_or(Error {})
    }

    /// Returns whether the game object is considered "active" by the scene (if it or any of its
    /// parents are inactive, then the game object is inactive)
    pub fn is_active_in_hierarchy(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<bool, Error> {
        process.read::<bool>(self.address + scene_manager.offsets.game_object_activeinhierarchy)
    }

    /// Returns whether the game object is considered "active" by itself (irrespective of any of its
    /// parents)
    pub fn is_active_in_self(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<bool, Error> {
        process.read::<bool>(self.address + scene_manager.offsets.game_object_activeself)
    }
}

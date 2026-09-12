use super::{Component, SceneManager, CSTR};
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

    /// Traverse the components attached to this game object.
    pub fn components<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Component> + 'a, Error> {
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

        // 0 is always Transform
        // FIXME: It would be good to be able to read those components here too
        Ok((1..number_of_components).map(move |m| Component {
            address: components[m],
        }))
    }

    /// Get a Component attached to the current `GameObject` by name of it's class.
    ///
    /// For Mono.
    pub fn get_component_mono(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &mono::Module,
        name: &str,
    ) -> Result<Component, Error> {
        if scene_manager.is_il2cpp {
            return Err(Error {});
        }

        self.components(process, scene_manager)?
            .find(|component| {
                let val = component
                    .get_mono_object(process, scene_manager, module)
                    .and_then(|object| object.get_class(process, module))
                    .and_then(|c| c.get_name::<CSTR>(process, module));

                val.is_ok_and(|class_name| class_name.matches(name))
            })
            .ok_or(Error {})
    }

    /// Get the Mono object of a  Component attached to the current `GameObject`
    /// by name of it's class.
    ///
    /// For Mono.
    pub fn get_component_mono_object(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &mono::Module,
        name: &str,
    ) -> Result<mono::Object, Error> {
        if scene_manager.is_il2cpp {
            return Err(Error {});
        }

        self.components(process, scene_manager)?
            .filter_map(|component| {
                component
                    .get_mono_object(process, scene_manager, module)
                    .ok()
            })
            .find(|object| {
                let val = object
                    .get_class(process, module)
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
    ) -> Result<Component, Error> {
        if !scene_manager.is_il2cpp {
            return Err(Error {});
        }

        self.components(process, scene_manager)?
            .find(|comp| {
                let val = il2cpp::Class::get_from_component(process, module, comp.address)
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

    /// Returns whether the game object is considered "active" by itself (irrespective of its
    /// parents)
    pub fn is_active_self(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<bool, Error> {
        process.read::<bool>(self.address + scene_manager.offsets.game_object_activeself)
    }
}

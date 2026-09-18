use super::{offsets::ReferenceShape, SceneManager, CSTR};
use crate::game_engine::unity::{il2cpp, mono};
use crate::{string::ArrayCString, Address, Address32, Address64, Error, PointerSize, Process};
use core::{array, mem::MaybeUninit};

/// A `Transform` is a base class for all entities used in a Unity scene. All
/// classes of interest useful for an auto splitter can be found starting from
/// the addresses of the root `Transform`s linked in each scene.
pub struct Transform {
    pub(super) address: Address,
}

impl Transform {
    /// Tries to return the name of the current `Transform`.
    pub fn get_name<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        process.read_pointer_path(
            self.address,
            scene_manager.pointer_size,
            &[
                scene_manager.profile.transform.game_object as u64,
                scene_manager.profile.game_object.name as u64,
                0x0,
            ],
        )
    }

    /// Iterates over the managed objects of the components of the current
    /// `Transform`'s game object, leaving out the transform itself.
    pub fn components<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Address> + 'a, Error> {
        let game_object = process.read_pointer(
            self.address + scene_manager.profile.transform.game_object,
            scene_manager.pointer_size,
        )?;

        let (number_of_components, main_object): (usize, Address) = match scene_manager.pointer_size
        {
            PointerSize::Bit64 => {
                let array = process.read::<[Address64; 3]>(
                    game_object + scene_manager.profile.game_object.components,
                )?;
                (array[2].value() as usize, array[0].into())
            }
            _ => {
                let array = process.read::<[Address32; 3]>(
                    game_object + scene_manager.profile.game_object.components,
                )?;
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

        let reference = scene_manager.profile.object.managed_reference;
        let read = move |at: Address| {
            process
                .read_pointer(at, scene_manager.pointer_size)
                .ok()
                .filter(|val| !val.is_null())
        };
        Ok((1..number_of_components).filter_map(move |m| {
            let at = components[m] + reference;
            match scene_manager.profile.reference {
                ReferenceShape::CachedObject => read(at),
                ReferenceShape::RootSlot => read(read(at)?),
            }
        }))
    }

    /// Tries to find the managed object of a component of the current
    /// `GameObject` by the name of its class, in a game using the Mono
    /// backend.
    pub fn get_component_mono(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &mono::Module,
        name: &str,
    ) -> Result<Address, Error> {
        self.find_component(process, scene_manager, name, |object| {
            module.object_class_name(process, object)
        })
    }

    /// Tries to find the managed object of a component of the current
    /// `GameObject` by the name of its class, in a game using the IL2CPP
    /// backend.
    pub fn get_component_il2cpp(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &il2cpp::Module,
        name: &str,
    ) -> Result<Address, Error> {
        self.find_component(process, scene_manager, name, |object| {
            module.object_class_name(process, object)
        })
    }

    pub(super) fn find_component(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
        class_name_of: impl Fn(Address) -> Option<ArrayCString<CSTR>>,
    ) -> Result<Address, Error> {
        self.components(process, scene_manager)?
            .find(|&object| {
                class_name_of(object).is_some_and(|class_name| class_name.matches(name))
            })
            .ok_or(Error {})
    }

    /// Iterates over children `Transform`s referred by the current one
    pub fn children<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Self> + 'a, Error> {
        let (child_count, child_pointer): (usize, Address) = match scene_manager.pointer_size {
            PointerSize::Bit64 => {
                let [first, _, third] = process
                    .read::<[u64; 3]>(self.address + scene_manager.profile.transform.children)?;
                (third as usize, Address::new(first))
            }
            _ => {
                let [first, _, third] = process
                    .read::<[u32; 3]>(self.address + scene_manager.profile.transform.children)?;
                (third as usize, Address::new(first as _))
            }
        };

        // Define an empty array and fill it later with the addresses of all child classes found for the current Transform.
        // Reading the whole array of pointers is (slightly) faster than reading each address in a loop
        const ARRAY_SIZE: usize = 128;

        if child_count == 0 || child_count > ARRAY_SIZE {
            return Err(Error {});
        }

        let children: [Address; ARRAY_SIZE] = match scene_manager.pointer_size {
            PointerSize::Bit64 => {
                let mut buf = [MaybeUninit::<Address64>::uninit(); ARRAY_SIZE];
                let slice =
                    process.read_into_uninit_slice(child_pointer, &mut buf[..child_count])?;

                let mut iter = slice.iter_mut();
                array::from_fn(|_| iter.next().copied().map(Into::into).unwrap_or_default())
            }
            _ => {
                let mut buf = [MaybeUninit::<Address32>::uninit(); ARRAY_SIZE];
                let slice =
                    process.read_into_uninit_slice(child_pointer, &mut buf[..child_count])?;

                let mut iter = slice.iter_mut();
                array::from_fn(|_| iter.next().copied().map(Into::into).unwrap_or_default())
            }
        };

        Ok((0..child_count).map(move |f| Self {
            address: children[f],
        }))
    }

    /// Tries to find a child `Transform` from the current one.
    pub fn get_child(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
    ) -> Result<Self, Error> {
        self.children(process, scene_manager)?
            .find(|p| {
                p.get_name::<CSTR>(process, scene_manager)
                    .is_ok_and(|obj_name| obj_name.matches(name))
            })
            .ok_or(Error {})
    }
}

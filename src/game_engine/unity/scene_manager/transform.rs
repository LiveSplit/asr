use super::{SceneManager, CSTR};
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
                scene_manager.offsets.game_object as u64,
                scene_manager.offsets.game_object_name as u64,
                0x0,
            ],
        )
    }

    /// Iterates over the classes referred to in the current `Transform`.
    pub fn classes<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Address> + 'a, Error> {
        let game_object = process.read_pointer(
            self.address + scene_manager.offsets.game_object,
            scene_manager.pointer_size,
        )?;

        let (number_of_components, main_object): (usize, Address) = match scene_manager.pointer_size
        {
            PointerSize::Bit64 => {
                let array = process
                    .read::<[Address64; 3]>(game_object + scene_manager.offsets.game_object)?;
                (array[2].value() as usize, array[0].into())
            }
            _ => {
                let array = process
                    .read::<[Address32; 3]>(game_object + scene_manager.offsets.game_object)?;
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
        self.classes(process, scene_manager)?
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

    /// Iterates over children `Transform`s referred by the current one
    pub fn children<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Self> + 'a, Error> {
        let (child_count, child_pointer): (usize, Address) = match scene_manager.pointer_size {
            PointerSize::Bit64 => {
                let [first, _, third] = process
                    .read::<[u64; 3]>(self.address + scene_manager.offsets.children_pointer)?;
                (third as usize, Address::new(first))
            }
            _ => {
                let [first, _, third] = process
                    .read::<[u32; 3]>(self.address + scene_manager.offsets.children_pointer)?;
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

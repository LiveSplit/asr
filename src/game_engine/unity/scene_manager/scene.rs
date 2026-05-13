use super::{SceneManager, Transform, CSTR};
use crate::{string::ArrayCString, Address, Address32, Address64, Error, PointerSize, Process};
use core::iter;
use core::iter::FusedIterator;

/// A scene loaded in the attached game.
pub struct Scene {
    pub(super) address: Address,
}

impl Scene {
    /// Returns the address of the scene in the attached game.
    pub const fn address(&self) -> Address {
        self.address
    }

    /// Returns [`true`] if the address of the scene still points to valid
    /// memory.
    pub fn is_valid(&self, process: &Process) -> bool {
        process.read::<u8>(self.address).is_ok()
    }

    /// Returns the build index of the scene. This index is unique to each
    /// scene in the game.
    pub fn index(&self, process: &Process, scene_manager: &SceneManager) -> Result<i32, Error> {
        process.read(self.address + scene_manager.offsets.build_index)
    }

    /// Returns the full path to the scene.
    pub fn path<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.address + scene_manager.offsets.asset_path,
                scene_manager.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    /// Iterates over all root [`Transform`]s declared for the
    /// specified scene.
    ///
    /// Each Unity scene normally has a linked list of [`Transform`]s.
    /// Each one can, recursively, have one or more children [`Transform`]s
    /// (and so on), as well as a list of `Component`s, which are classes (eg.
    /// `MonoBehaviour`) containing data we might want to retrieve for the auto
    /// splitter logic.
    fn root_game_objects<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> impl FusedIterator<Item = Transform> + 'a {
        let list_first = process
            .read_pointer(
                self.address + scene_manager.offsets.root_storage_container,
                scene_manager.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null());

        let mut current_list = list_first;

        iter::from_fn(move || {
            let [_prev, next, current]: [Address; 3] = match scene_manager.pointer_size {
                PointerSize::Bit64 => process
                    .read::<[Address64; 3]>(current_list?)
                    .ok()
                    .filter(|[_prev, next, current]| !next.is_null() && !current.is_null())?
                    .map(|a| a.into()),
                _ => process
                    .read::<[Address32; 3]>(current_list?)
                    .ok()
                    .filter(|[_prev, next, current]| !next.is_null() && !current.is_null())?
                    .map(|a| a.into()),
            };

            if next == list_first? {
                current_list = None;
            } else {
                current_list = Some(next);
            }

            Some(Transform { address: current })
        })
        .fuse()
    }

    /// Tries to find the specified root [`Transform`] from the currently
    /// active Unity scene.
    pub fn get_root_game_object(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
    ) -> Result<Transform, Error> {
        self.root_game_objects(process, scene_manager)
            .find(|obj| {
                obj.get_name::<CSTR>(process, scene_manager)
                    .is_ok_and(|obj_name| obj_name.matches(name))
            })
            .ok_or(Error {})
    }

    pub fn find_transform(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        root_object_name: &str,
        child_path: &[&str],
    ) -> Result<Transform, Error> {
        let mut current_transform =
            self.get_root_game_object(process, scene_manager, root_object_name)?;

        for object_name in child_path {
            current_transform = current_transform.get_child(process, scene_manager, object_name)?;
        }

        Ok(current_transform)
    }
}

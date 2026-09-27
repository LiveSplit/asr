use core::iter::{self, FusedIterator};

use super::{transform::Transform, Scene, SceneManager, CSTR};
use crate::{Address, Address32, Address64, Error, PointerSize, Process};

impl SceneManager {
    /// Iterates over all root [`Transform`]s declared for the
    /// specified scene.
    ///
    /// Each Unity scene normally has a linked list of [`Transform`]s.
    /// Each one can, recursively, have one or more children [`Transform`]s
    /// (and so on), as well as a list of `Component`s, which are classes (eg.
    /// `MonoBehaviour`) containing data we might want to retrieve for the auto
    /// splitter logic.
    pub(super) fn root_game_objects<'a>(
        &'a self,
        process: &'a Process,
        scene: &Scene,
    ) -> impl FusedIterator<Item = Transform> + 'a {
        let head = scene.address + self.profile.scene.roots;
        let mut current = process
            .read_pointer(head + self.size_of_ptr(), self.pointer_size)
            .ok()
            .filter(|node| !node.is_null() && *node != head);

        iter::from_fn(move || {
            let node = current?;
            let [_, next, transform]: [Address; 3] = match self.pointer_size {
                PointerSize::Bit64 => process
                    .read::<[Address64; 3]>(node)
                    .ok()
                    .filter(|[_, _, transform]| !transform.is_null())?
                    .map(|a| a.into()),
                _ => process
                    .read::<[Address32; 3]>(node)
                    .ok()
                    .filter(|[_, _, transform]| !transform.is_null())?
                    .map(|a| a.into()),
            };

            current = Some(next).filter(|next| !next.is_null() && *next != head);

            Some(Transform { address: transform })
        })
        .fuse()
    }

    /// Tries to find the specified root [`Transform`] from the currently
    /// active Unity scene.
    pub fn get_root_game_object(&self, process: &Process, name: &str) -> Result<Transform, Error> {
        self.root_game_objects(process, &self.get_current_scene(process)?)
            .find(|obj| {
                obj.get_name::<CSTR>(process, self)
                    .is_ok_and(|obj_name| obj_name.matches(name))
            })
            .ok_or(Error {})
    }

    /// Tries to find the specified root [`Transform`] from the
    /// `DontDestroyOnLoad` Unity scene.
    pub fn get_game_object_from_dont_destroy_on_load(
        &self,
        process: &Process,
        name: &str,
    ) -> Result<Transform, Error> {
        self.root_game_objects(process, &self.get_dont_destroy_on_load_scene())
            .find(|obj| {
                obj.get_name::<CSTR>(process, self)
                    .is_ok_and(|obj_name| obj_name.matches(name))
            })
            .ok_or(Error {})
    }
}

use super::{SceneManager, CSTR};
use crate::{string::ArrayCString, Address, Error, Process};

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

    /// Returns the full asset path of the scene.
    ///
    /// Usually looks like "`Assets/some/path/scene.unity`".
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

    /// Returns the full path of the scene, as a [String](alloc::string::String).
    pub fn path_as_string(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<alloc::string::String, Error> {
        let path = self.path::<CSTR>(process, scene_manager)?;
        let str = path.validate_utf8().map_err(|_| Error {})?;

        Ok(str.into())
    }

    /// Returns the name of the scene, as a [String](alloc::string::String).
    pub fn name(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<alloc::string::String, Error> {
        // The name is also stored in memory, but it's just easier to interpret the path
        let path = self.path_as_string(process, scene_manager)?;
        // if for some reason the path has no /, or doesn't end in a .unity, just safely default
        let cs = path.rsplit_once('/').unwrap_or(("", &path)).1;
        Ok(cs.split_once('.').unwrap_or((cs, "")).0.into())
    }
}

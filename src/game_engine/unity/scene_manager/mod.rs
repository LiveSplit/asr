//! Support for identifying the current scene in Unity games.

// References:
// https://gist.githubusercontent.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e/raw/8aa3e6b8da01fd03ff2ff0c03cbd018e522ef988/UnityScene.hpp
//
// The offsets come from the measured builds in `builds.rs`. The logic for
// Transforms and GameObjects is taken from https://github.com/Micrologist/UnityInstanceDumper

use crate::{
    file_format::{elf, macho, pe},
    future::retry,
    print_limited,
    string::ArrayCString,
    Address, Error, PointerSize, Process,
};

mod builds;

mod game_objects;

#[cfg(all(test, not(target_family = "wasm")))]
mod attach_tests;

#[cfg(all(test, not(target_family = "wasm")))]
mod scene_tests;

#[cfg(all(test, not(target_family = "wasm")))]
mod roots_tests;

#[cfg(all(test, not(target_family = "wasm")))]
mod components_tests;

mod offsets;

mod transform;
pub use transform::Transform;

use offsets::Profile;

mod scene;
pub use scene::Scene;

use super::{BinaryFormat, CSTR};

/// The scene manager allows you to easily identify the current scene loaded in
/// the attached Unity game.
///
/// It can be useful to identify splitting conditions or as an alternative to
/// the traditional class lookup in games with no useful static references.
pub struct SceneManager {
    pointer_size: PointerSize,
    address: Address,
    profile: &'static Profile,
}

impl SceneManager {
    /// Attaches to the scene manager in the given process. On Windows, the
    /// Unity version of the game picks the measured build whose offsets the
    /// scene manager reads through: the build of that version, or else the
    /// newest build below it.
    pub fn attach(process: &Process) -> Option<Self> {
        let (unity_player, format) = Self::engine_module(process)?;

        let profile = match format {
            BinaryFormat::PE => {
                let pointer_size =
                    pe::MachineType::read(process, unity_player.0)?.pointer_size()?;
                let unity = Self::unity_version(process, unity_player.0)?;
                let build = builds::nearest(unity, pointer_size)?;
                print_limited::<128>(&format_args!(
                    "scene manager: unity {}.{}.{}.{} takes the build measured on {}.{}.{}.{}",
                    unity.0,
                    unity.1,
                    unity.2,
                    unity.3,
                    build.unity.0,
                    build.unity.1,
                    build.unity.2,
                    build.unity.3,
                ));
                &build.profile
            }
            BinaryFormat::ELF => match elf::pointer_size(process, unity_player.0)? {
                PointerSize::Bit64 => &builds::ELF_AND_MACHO_X64,
                _ => return None,
            },
            BinaryFormat::MachO => match macho::pointer_size(process, unity_player)? {
                PointerSize::Bit64 => &builds::ELF_AND_MACHO_X64,
                _ => return None,
            },
        };

        Self::attach_with(process, unity_player, profile)
    }

    /// Attaches to the scene manager in the given process.
    ///
    /// This is the `await`able version of the [`attach`](Self::attach)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_attach(process: &Process) -> SceneManager {
        retry(|| Self::attach(process)).await
    }

    /// Finds the module that holds the engine: `UnityPlayer.dll` and its
    /// Linux and Mac siblings, or the game's own executable on Unity 5.6,
    /// which linked the engine in. Finding the executable needs its name,
    /// so that part needs the `alloc` feature.
    fn engine_module(process: &Process) -> Option<((Address, u64), BinaryFormat)> {
        let player = [
            ("UnityPlayer.dll", BinaryFormat::PE),
            ("UnityPlayer.so", BinaryFormat::ELF),
            ("UnityPlayer.dylib", BinaryFormat::MachO),
        ]
        .into_iter()
        .find_map(|(name, format)| match format {
            BinaryFormat::PE => {
                let address = process.get_module_address(name).ok()?;
                Some((
                    (address, pe::read_size_of_image(process, address)? as u64),
                    format,
                ))
            }
            _ => Some((process.get_module_range(name).ok()?, format)),
        });

        #[cfg(feature = "alloc")]
        let player = player.or_else(|| {
            let executable = process.get_main_module_range().ok()?;
            pe::MachineType::read(process, executable.0)?;
            Some((executable, BinaryFormat::PE))
        });

        player
    }

    /// Reads the four parts of the file version of the engine module, which
    /// name the Unity version of the game.
    fn unity_version(process: &Process, unity_player: Address) -> Option<(u16, u16, u16, u16)> {
        let file_version = pe::FileVersion::read(process, unity_player)?;
        Some((
            file_version.major_version,
            file_version.minor_version,
            file_version.build_part,
            file_version.private_part,
        ))
    }

    /// Finds the scene manager in the engine module through the anchor of
    /// the profile. Every match of the anchor must name the same global,
    /// and the global must hold the manager.
    fn attach_with(
        process: &Process,
        unity_player: (Address, u64),
        profile: &'static Profile,
    ) -> Option<Self> {
        let pointer_size = profile.pointer_size;
        let displacement = profile.anchor.displacement as u64;

        // The load of the global is relative to the next instruction on
        // x64, and absolute on x86.
        let global = |at: Address| -> Option<Address> {
            match pointer_size {
                PointerSize::Bit64 => {
                    let offset = process.read::<i32>(at + displacement).ok()?;
                    Some(at + displacement + 4 + offset)
                }
                _ => Some(process.read::<u32>(at + displacement).ok()?.into()),
            }
        };

        let mut globals = profile
            .anchor
            .signature
            .scan_iter(process, unity_player)
            .map(global);
        let global = globals.next()??;
        if globals.any(|other| other != Some(global)) {
            return None;
        }

        // Dereferencing one level because this pointer never changes as long as the game is open.
        // It might not seem a lot, but it helps make things a bit faster when querying for scene stuff.
        let address = process
            .read_pointer(global, pointer_size)
            .ok()
            .filter(|val| !val.is_null())?;

        Some(Self {
            pointer_size,
            address,
            profile,
        })
    }

    #[inline]
    const fn size_of_ptr(&self) -> u64 {
        self.pointer_size as u64
    }

    /// Tries to retrieve the current active scene.
    pub fn get_current_scene(&self, process: &Process) -> Result<Scene, Error> {
        process
            .read_pointer(
                self.address + self.profile.manager.active_scene,
                self.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
            .map(|address| Scene { address })
            .ok_or(Error {})
    }

    /// `DontDestroyOnLoad` is a special Unity scene containing game objects
    /// that must be preserved when switching between different scenes (eg. a
    /// `scene1` starting some background music that continues when `scene2`
    /// loads).
    pub fn get_dont_destroy_on_load_scene(&self) -> Scene {
        Scene {
            address: self.address + self.profile.manager.dont_destroy_on_load_scene,
        }
    }

    /// Returns the current scene index.
    ///
    /// The value returned is a [`i32`] because some games will show `-1` as their
    /// current scene until fully initialized.
    pub fn get_current_scene_index(&self, process: &Process) -> Result<i32, Error> {
        self.get_current_scene(process)
            .and_then(|scene| scene.index(process, self))
    }

    /// Returns the full path to the current scene. Use [`get_scene_name`]
    /// afterwards to get the scene name.
    pub fn get_current_scene_path<const N: usize>(
        &self,
        process: &Process,
    ) -> Result<ArrayCString<N>, Error> {
        self.get_current_scene(process)
            .and_then(|scene| scene.path(process, self))
    }

    /// Returns the number of currently loaded scenes in the attached game.
    pub fn get_scene_count(&self, process: &Process) -> Result<u32, Error> {
        process.read(self.loaded_scenes() + self.size_of_ptr().wrapping_mul(2))
    }

    // The loaded scenes are a dynamic array: the pointer to the scenes
    // first, the allocation label next, then the count.
    fn loaded_scenes(&self) -> Address {
        self.address + self.profile.manager.scenes
    }

    /// Iterates over all the currently loaded scenes in the attached game.
    pub fn scenes<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Scene> + 'a {
        let scenes = process
            .read_pointer(self.loaded_scenes(), self.pointer_size)
            .unwrap_or_default();
        let count = self.get_scene_count(process).unwrap_or_default() as usize;

        (0..count).filter_map(move |index| {
            process
                .read_pointer(
                    scenes + (index as u64).wrapping_mul(self.size_of_ptr()),
                    self.pointer_size,
                )
                .ok()
                .filter(|val| !val.is_null())
                .map(|address| Scene { address })
        })
    }
}

/// Returns the name of the scene from the given scene path. This path is
/// usually retrieved from [`SceneManager::get_current_scene_path`].
pub fn get_name(scene_path: &[u8]) -> &[u8] {
    scene_path
        .rsplit(|&b| b == b'/')
        .next()
        .unwrap_or_default()
        .split(|&b| b == b'.')
        .next()
        .unwrap_or_default()
}

//! Support for identifying the current scene in Unity games.

// References:
// https://gist.githubusercontent.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e/raw/8aa3e6b8da01fd03ff2ff0c03cbd018e522ef988/UnityScene.hpp
// (some offsets seem to be wrong anyway, but it's a very good starting point)
//
// Offsets and logic for Transforms and GameObjects taken from https://github.com/Micrologist/UnityInstanceDumper

use crate::{
    file_format::{elf, pe},
    future::retry,
    signature::Signature,
    string::ArrayCString,
    Address, Address32, Error, PointerSize, Process,
};

mod game_objects;

mod offsets;

mod transform;
pub use transform::Transform;

use offsets::Offsets;

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
    is_il2cpp: bool,
    address: Address,
    offsets: &'static Offsets,
}

impl SceneManager {
    /// Attaches to the scene manager in the given process.
    pub fn attach(process: &Process) -> Option<Self> {
        const SIG_64_BIT_PE: Signature<13> =
            Signature::new("48 83 EC 20 4C 8B ?5 ?? ?? ?? ?? 33 F6");
        const SIG_64_BIT_ELF: Signature<13> =
            Signature::new("41 54 53 50 4C 8B ?5 ?? ?? ?? ?? 41 83");
        const SIG_32_1: Signature<12> = Signature::new("55 8B EC 51 A1 ?? ?? ?? ?? 53 33 DB");
        const SIG_32_2: Signature<6> = Signature::new("53 8D 41 ?? 33 DB");
        const SIG_32_3: Signature<14> = Signature::new("55 8B EC 83 EC 18 A1 ?? ?? ?? ?? 33 C9 53");

        let (unity_player, format) = [
            ("UnityPlayer.dll", BinaryFormat::PE),
            ("UnityPlayer.so", BinaryFormat::ELF),
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
        })?;

        let pointer_size = match format {
            BinaryFormat::PE => pe::MachineType::read(process, unity_player.0)?.pointer_size()?,
            BinaryFormat::ELF => elf::pointer_size(process, unity_player.0)?,
        };

        let is_il2cpp = process.get_module_address("GameAssembly.dll").is_ok();

        // There are multiple signatures that can be used, depending on the version of Unity
        // used in the target game.
        let base_address: Address = match (pointer_size, format) {
            (PointerSize::Bit64, BinaryFormat::PE) => {
                let addr = SIG_64_BIT_PE.scan_process_range(process, unity_player)? + 7;
                addr + 0x4 + process.read::<i32>(addr).ok()?
            }
            (PointerSize::Bit64, BinaryFormat::ELF) => {
                let addr = SIG_64_BIT_ELF.scan_process_range(process, unity_player)? + 7;
                addr + 0x4 + process.read::<i32>(addr).ok()?
            }
            (PointerSize::Bit32, BinaryFormat::PE) => {
                if let Some(addr) = SIG_32_1.scan_process_range(process, unity_player) {
                    process.read::<Address32>(addr + 5).ok()?.into()
                } else if let Some(addr) = SIG_32_2.scan_process_range(process, unity_player) {
                    process.read::<Address32>(addr.add_signed(-4)).ok()?.into()
                } else if let Some(addr) = SIG_32_3.scan_process_range(process, unity_player) {
                    process.read::<Address32>(addr + 7).ok()?.into()
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        };

        let offsets = Offsets::new(pointer_size)?;

        // Dereferencing one level because this pointer never changes as long as the game is open.
        // It might not seem a lot, but it helps make things a bit faster when querying for scene stuff.
        let address = process
            .read_pointer(base_address, pointer_size)
            .ok()
            .filter(|val| !val.is_null())?;

        Some(Self {
            pointer_size,
            is_il2cpp,
            address,
            offsets,
        })
    }

    /// Attaches to the scene manager in the given process.
    ///
    /// This is the `await`able version of the [`attach`](Self::attach)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_attach(process: &Process) -> SceneManager {
        retry(|| Self::attach(process)).await
    }

    #[inline]
    const fn size_of_ptr(&self) -> u64 {
        self.pointer_size as u64
    }

    /// Tries to retrieve the current active scene.
    pub fn get_current_scene(&self, process: &Process) -> Result<Scene, Error> {
        process
            .read_pointer(self.address + self.offsets.active_scene, self.pointer_size)
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
            address: self.address + self.offsets.dont_destroy_on_load_scene,
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
        process.read(self.address + self.offsets.scene_count)
    }

    /// Iterates over all the currently loaded scenes in the attached game.
    pub fn scenes<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Scene> + 'a {
        let (num_scenes, addr): (usize, Address) = match self.pointer_size {
            PointerSize::Bit64 => {
                let [first, _, third] = process
                    .read::<[u64; 3]>(self.address + self.offsets.scene_count)
                    .unwrap_or_default();
                (first as usize, Address::new(third))
            }
            _ => {
                let [first, _, third] = process
                    .read::<[u32; 3]>(self.address + self.offsets.scene_count)
                    .unwrap_or_default();
                (first as usize, Address::new(third as _))
            }
        };

        (0..num_scenes).filter_map(move |index| {
            process
                .read_pointer(
                    addr + (index as u64).wrapping_mul(self.size_of_ptr()),
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

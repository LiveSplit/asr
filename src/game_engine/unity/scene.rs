//! Support for identifying the current scene in Unity games.

// References:
// https://gist.githubusercontent.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e/raw/8aa3e6b8da01fd03ff2ff0c03cbd018e522ef988/UnityScene.hpp

use crate::{
    file_format::pe, future::retry, signature::Signature, string::ArrayCString, Address, Address32,
    Address64, Error, Process,
};

/// The scene manager allows you to easily identify the current scene loaded in
/// the attached Unity game.
///
/// It can be useful to identify splitting conditions or as an alternative to
/// the traditional class lookup in games with no useful static references.
pub struct SceneManager {
    is_64_bit: bool,
    address: Address,
    offsets: &'static Offsets,
}

impl SceneManager {
    /// Attaches to the scene manager in the given process.
    pub fn attach(process: &Process) -> Option<Self> {
        const SIG_64_BIT: Signature<13> = Signature::new("48 83 EC 20 4C 8B ?5 ???????? 33 F6");
        const SIG_32_1: Signature<12> = Signature::new("55 8B EC 51 A1 ???????? 53 33 DB");
        const SIG_32_2: Signature<6> = Signature::new("53 8D 41 ?? 33 DB");
        const SIG_32_3: Signature<14> = Signature::new("55 8B EC 83 EC 18 A1 ???????? 33 C9 53");

        let unity_player = process.get_module_range("UnityPlayer.dll").ok()?;

        let is_64_bit = pe::MachineType::read(process, unity_player.0)? == pe::MachineType::X86_64;

        let address = if is_64_bit {
            let addr = SIG_64_BIT.scan_process_range(process, unity_player)? + 7;
            addr + 0x4 + process.read::<i32>(addr).ok()?
        } else if let Some(addr) = SIG_32_1.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr + 5).ok()?.into()
        } else if let Some(addr) = SIG_32_2.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr.add_signed(-4)).ok()?.into()
        } else if let Some(addr) = SIG_32_3.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr + 7).ok()?.into()
        } else {
            return None;
        };

        let offsets = Offsets::new(is_64_bit);

        Some(Self {
            is_64_bit,
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

    fn read_pointer(&self, process: &Process, address: Address) -> Result<Address, Error> {
        Ok(match self.is_64_bit {
            true => process.read::<Address64>(address)?.into(),
            false => process.read::<Address32>(address)?.into(),
        })
    }

    fn get_current_scene_address(&self, process: &Process) -> Result<Address, Error> {
        let addr = self.read_pointer(process, self.address)?;
        self.read_pointer(process, addr + self.offsets.active_scene)
    }

    /// Returns the current scene index.
    ///
    /// The value returned is a [`i32`] because some games will show `-1` as their
    /// current scene until fully initialized.
    pub fn get_current_scene_index(&self, process: &Process) -> Result<i32, Error> {
        process.read(self.get_current_scene_address(process)? + self.offsets.build_index)
    }

    /// Returns the full path to the current scene. Use
    /// [`get_current_scene_path`](Self::get_current_scene_path) afterwards to
    /// get the scene name.
    pub fn get_current_scene_path<const N: usize>(
        &self,
        process: &Process,
    ) -> Result<ArrayCString<N>, Error> {
        let addr = self.read_pointer(
            process,
            self.get_current_scene_address(process)? + self.offsets.asset_path,
        )?;
        process.read(addr)
    }

    /// Returns the number of total scenes in the attached game.
    pub fn get_scene_count(&self, process: &Process) -> Result<u32, Error> {
        process.read(self.address + self.offsets.scene_count)
    }

    /// Iterates over all the currently loaded scenes in the attached game.
    pub fn scenes<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Scene> + 'a {
        let fptr = self.read_pointer(process, self.address).unwrap_or_default();
        let addr = self
            .read_pointer(process, fptr + self.offsets.loaded_scenes)
            .unwrap_or_default();

        let pointer_size = if self.is_64_bit { 8 } else { 4 };

        (0..16)
            .map(move |index| Scene {
                address: {
                    self.read_pointer(process, addr + (index as u64).wrapping_mul(pointer_size))
                        .unwrap_or_default()
                },
            })
            .filter(move |p| !fptr.is_null() && p.is_valid(process))
    }
}

struct Offsets {
    scene_count: u8,
    loaded_scenes: u8,
    active_scene: u8,
    asset_path: u8,
    build_index: u8,
}

impl Offsets {
    pub const fn new(is_64_bit: bool) -> &'static Self {
        match is_64_bit {
            true => &Self {
                scene_count: 0x18,
                loaded_scenes: 0x28,
                active_scene: 0x48,
                asset_path: 0x10,
                build_index: 0x98,
            },
            false => &Self {
                scene_count: 0xC,
                loaded_scenes: 0x18,
                active_scene: 0x28,
                asset_path: 0xC,
                build_index: 0x70,
            },
        }
    }
}

/// A scene loaded in the attached game.
pub struct Scene {
    address: Address,
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
    pub fn index(&self, process: &Process, scene_manager: &SceneManager) -> Result<u32, Error> {
        process.read(self.address + scene_manager.offsets.build_index)
    }

    /// Returns the full path to the scene.
    pub fn path<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        let addr =
            scene_manager.read_pointer(process, self.address + scene_manager.offsets.asset_path)?;
        process.read(addr)
    }
}

/// Returns the name of the scene from the given scene path. This path is
/// usually retrieved from [`SceneManager::get_current_scene_path`].
pub fn get_scene_name(scene_path: &[u8]) -> &[u8] {
    scene_path
        .rsplit(|&b| b == b'/')
        .next()
        .unwrap_or_default()
        .split(|&b| b == b'.')
        .next()
        .unwrap_or_default()
}

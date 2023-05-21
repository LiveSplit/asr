use crate::{
    file_format::pe, future::retry, signature::Signature, Address, Address32, Address64, Error,
    Process,
};
use arrayvec::ArrayString;

/// SceneManager allows to easily identify the current scene loaded in the attached Unity game.
///
/// It can be useful to identify splitting conditions or as an alternative to the traditional
/// class lookup in games with no useful static references.
pub struct SceneManager<'a> {
    process: &'a Process,
    x64_32: bool,
    address: Address,
    offsets: SceneManagerOffsets,
}

impl<'a> SceneManager<'a> {
    /// Creates a new instance of `SceneManager`
    pub fn new(process: &'a Process) -> Option<Self> {
        const SIG_64_BIT: Signature<13> = Signature::new("48 83 EC 20 4C 8B ?5 ???????? 33 F6");
        const SIG_32_1: Signature<12> = Signature::new("55 8B EC 51 A1 ???????? 53 33 DB");
        const SIG_32_2: Signature<6> = Signature::new("53 8D 41 ?? 33 DB");
        const SIG_32_3: Signature<14> = Signature::new("55 8B EC 83 EC 18 A1 ???????? 33 C9 53");

        let unity_player = process.get_module_range("UnityPlayer.dll").ok()?;

        let x64_32 = pe::MachineType::read(process, unity_player.0)? == pe::MachineType::X86_64;

        let address: Address = if x64_32 {
            let addr = SIG_64_BIT.scan_process_range(process, unity_player)? + 7;
            addr + 0x4 + process.read::<i32>(addr).ok()?
        } else if let Some(addr) = SIG_32_1.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr + 5).ok()?.into()
        } else if let Some(addr) = SIG_32_2.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr.add_signed(4)).ok()?.into()
        } else if let Some(addr) = SIG_32_3.scan_process_range(process, unity_player) {
            process.read::<Address32>(addr + 7).ok()?.into()
        } else {
            return None;
        };

        let offsets = SceneManagerOffsets::new(x64_32);

        Some(Self {
            process,
            x64_32,
            address,
            offsets,
        })
    }

    /// Creates a new instance of `SceneManager`
    pub async fn wait_new(process: &'a Process) -> SceneManager<'_> {
        retry(|| Self::new(process)).await
    }

    fn read_pointer(&self, address: Address) -> Result<Address, Error> {
        if self.x64_32 {
            Ok(self.process.read::<Address64>(address)?.into())
        } else {
            Ok(self.process.read::<Address32>(address)?.into())
        }
    }

    fn get_current_scene_address(&self) -> Result<Address, Error> {
        let addr = self.read_pointer(self.address)?;
        self.read_pointer(addr + self.offsets.active_scene)
    }

    /// Returns the current scene index.
    ///
    /// The value returned is a `i32` because some games will show `-1` as their
    /// current scene until fully initialized.
    pub fn get_current_scene_index(&self) -> Result<i32, Error> {
        self.process
            .read(self.get_current_scene_address()? + self.offsets.build_index)
    }

    /// Returns the full path to the current scene
    pub fn get_current_scene_path<const N: usize>(&self) -> Result<ArrayString<N>, Error> {
        let addr =
            self.read_pointer(self.get_current_scene_address()? + self.offsets.asset_path)?;
        let path = self.process.read::<[u8; N]>(addr)?;
        let mut param: ArrayString<N> = ArrayString::new();
        for val in path {
            let success = param.try_push(val as char);
            if success.is_err() {
                return Err(Error {});
            }
        }
        Ok(param)
    }

    /// Returns the name associated with the current scene
    pub fn get_current_scene_name<const N: usize>(&self) -> Result<ArrayString<N>, Error> {
        let addr =
            self.read_pointer(self.get_current_scene_address()? + self.offsets.asset_path)?;
        let path = self.process.read::<[u8; N]>(addr)?;
        let Some(name) = path.split(|&b| b == b'/').last() else { return Err(Error {}) };
        let Some(name) = name.split(|&b| b == b'.').next() else { return Err(Error {}) };
        let mut param: ArrayString<N> = ArrayString::new();
        for &val in name {
            let success = param.try_push(val as char);
            if success.is_err() {
                return Err(Error {});
            }
        }
        Ok(param)
    }

    /// Returns the number of total scenes in the attached game
    pub fn get_scene_count(&self) -> Result<u32, Error> {
        self.process
            .read::<u32>(self.address + self.offsets.scene_count)
    }

    /// Iterator for the currently loaded scenes
    pub fn scenes(&self) -> impl DoubleEndedIterator<Item = Scene<'_>> {
        let fptr = self.read_pointer(self.address).unwrap_or_default();
        let addr = self
            .read_pointer(fptr + self.offsets.loaded_scenes as u64)
            .unwrap_or_default();

        (0..16)
            .map(move |index| Scene {
                scene_manager: self,
                address: {
                    let i = if self.x64_32 { 8 } else { 4 };
                    self.read_pointer(addr + index as u64 * i)
                        .unwrap_or_default()
                },
            })
            .filter(move |p| !fptr.is_null() && p.is_valid())
    }
}

struct SceneManagerOffsets {
    scene_count: u32,
    loaded_scenes: u32,
    active_scene: u32,
    asset_path: u32,
    build_index: u32,
}

impl SceneManagerOffsets {
    pub const fn new(is_64_bit: bool) -> Self {
        match is_64_bit {
            true => Self {
                scene_count: 0x18,
                loaded_scenes: 0x28,
                active_scene: 0x48,
                asset_path: 0x10,
                build_index: 0x98,
            },
            false => Self {
                scene_count: 0xC,
                loaded_scenes: 0x18,
                active_scene: 0x28,
                asset_path: 0xC,
                build_index: 0x70,
            },
        }
    }
}

pub struct Scene<'a> {
    scene_manager: &'a SceneManager<'a>,
    address: Address,
}

impl Scene<'_> {
    pub const fn address(&self) -> Result<Address, Error> {
        Ok(self.address)
    }

    pub fn is_valid(&self) -> bool {
        self.scene_manager.process.read::<u8>(self.address).is_ok()
    }

    pub fn index(&self) -> Result<u32, Error> {
        self.scene_manager
            .process
            .read::<u32>(self.address + self.scene_manager.offsets.build_index)
    }

    pub fn scene_path<const N: usize>(&self) -> Result<ArrayString<N>, Error> {
        let addr = self
            .scene_manager
            .read_pointer(self.address + self.scene_manager.offsets.asset_path)?;
        let name = self.scene_manager.process.read::<[u8; N]>(addr)?;
        let mut param: ArrayString<N> = ArrayString::new();
        for val in name {
            param.push(val as char);
        }
        Ok(param)
    }

    pub fn scene_name<const N: usize>(&self) -> Result<ArrayString<N>, Error> {
        let addr = self
            .scene_manager
            .read_pointer(self.address + self.scene_manager.offsets.asset_path)?;
        let path = self.scene_manager.process.read::<[u8; N]>(addr)?;
        let Some(name) = path.split(|&b| b == b'/').last() else { return Err(Error {}) };
        let Some(name) = name.split(|&b| b == b'.').next() else { return Err(Error {}) };
        let mut param: ArrayString<N> = ArrayString::new();
        for &val in name {
            let success = param.try_push(val as char);
            if success.is_err() {
                return Err(Error {});
            }
        }
        Ok(param)
    }
}

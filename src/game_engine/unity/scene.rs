//! Support for identifying the current scene in Unity games.

// References:
// https://gist.githubusercontent.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e/raw/8aa3e6b8da01fd03ff2ff0c03cbd018e522ef988/UnityScene.hpp
// Offsets and logic for the GameObject functions taken from https://github.com/Micrologist/UnityInstanceDumper

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
    is_il2cpp: bool,
    address: Address,
    offsets: &'static Offsets,
    pointer_size: u8,
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
        let is_il2cpp = process.get_module_address("GameAssembly.dll").is_ok();

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
        let pointer_size = if is_64_bit { 0x8 } else { 0x4 };

        Some(Self {
            is_64_bit,
            is_il2cpp,
            address,
            offsets,
            pointer_size,
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

    /// Iterates over all root `Transform`s / `GameObject`s declared for the current scene.
    ///
    /// Each Unity scene normally has a linked list of `Transform`s (each one is a `GameObject`).
    /// Each one can, recursively, have a child `Transform` (and so on), and has a list of `Component`s, which are
    /// classes (eg. `MonoBehaviour`) containing data we might want to retreieve for the autosplitter logic.
    fn root_game_objects<'a>(
        &'a self,
        process: &'a Process,
    ) -> Result<impl DoubleEndedIterator<Item = GameObject> + 'a, Error> {
        let current_scene_address = self.get_current_scene_address(process)?;
        let first_game_object = self.read_pointer(
            process,
            current_scene_address + self.offsets.root_storage_container,
        )?;

        let number_of_root_game_objects = {
            let mut index: usize = 0;
            let mut temp_tr = first_game_object;

            while temp_tr != current_scene_address + self.offsets.root_storage_container {
                index += 1;
                temp_tr = self.read_pointer(process, temp_tr)?;
            }

            index
        };

        let mut current_game_object = first_game_object;

        Ok((0..number_of_root_game_objects).filter_map(move |n| {
            let buf: [Address; 3] = match self.is_64_bit {
                true => process
                    .read::<[Address64; 3]>(current_game_object)
                    .ok()?
                    .map(|a| a.into()),
                false => process
                    .read::<[Address32; 3]>(current_game_object)
                    .ok()?
                    .map(|a| a.into()),
            };

            let game_object = self
                .read_pointer(process, buf[2] + self.offsets.game_object)
                .ok()?;

            // Load the next game object before looping, except at the last iteration of the loop
            if n + 1 != number_of_root_game_objects {
                current_game_object = buf[0];
            }

            Some(GameObject {
                address: game_object,
            })
        }))
    }

    /// Tries to find the specified root `GameObject` in the current Unity scene.
    pub fn get_root_game_object(&self, process: &Process, name: &str) -> Result<GameObject, Error> {
        self.root_game_objects(process)?
            .find(|obj| {
                obj.get_name::<128>(process, self)
                    .unwrap_or_default()
                    .as_bytes()
                    == name.as_bytes()
            })
            .ok_or(Error {})
    }
}

/// A `GameObject` is a base class for all entities used in a Unity scene.
/// All classes of interest useful for an autosplitter can be found starting from the addresses of the root `GameObject`s linked in each scene.
pub struct GameObject {
    address: Address,
}

impl GameObject {
    /// Tries to return the name of the current `GameObject`
    pub fn get_name<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        let name_ptr = scene_manager.read_pointer(
            process,
            self.address + scene_manager.offsets.game_object_name,
        )?;
        process.read(name_ptr)
    }

    /// Iterates over the classes referred to in the current `GameObject`
    pub fn classes<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Address> + 'a, Error> {
        let number_of_components = process
            .read::<u32>(self.address + scene_manager.offsets.number_of_object_components)?
            as usize;

        if number_of_components == 0 {
            return Err(Error {});
        }
        
        let main_object = scene_manager
            .read_pointer(process, self.address + scene_manager.offsets.game_object)?;

        const ARRAY_SIZE: usize = 128;
        let mut components = [Address::NULL; ARRAY_SIZE];

        if scene_manager.is_64_bit {
            let slice = &mut [Address64::NULL; ARRAY_SIZE * 2][0..number_of_components * 2];
            process.read_into_slice(main_object, slice)?;

            for val in 0..number_of_components {
                components[val] = slice[val * 2 + 1].into();
            }
        } else {
            let slice = &mut [Address32::NULL; ARRAY_SIZE * 2][0..number_of_components * 2];
            process.read_into_slice(main_object, slice)?;

            for val in 0..number_of_components {
                components[val] = slice[val * 2 + 1].into();
            }
        }

        Ok((0..number_of_components).filter_map(move |m| {
            scene_manager
                .read_pointer(process, components[m] + scene_manager.offsets.klass)
                .ok()
        }))
    }

    /// Tries to find the base address of a class in the current `GameObject`
    pub fn get_class(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
    ) -> Result<Address, Error> {
        self.classes(process, scene_manager)?.find(|&c| {
            let Ok(vtable) = scene_manager.read_pointer(process, c) else { return false };

            let name_ptr = {
                match scene_manager.is_il2cpp {
                    true => {
                        let Ok(name_ptr) = scene_manager.read_pointer(process, vtable + scene_manager.pointer_size as u32 * 2) else { return false };
                        name_ptr
                    },
                    false => {
                        let Ok(vtable) = scene_manager.read_pointer(process, vtable) else { return false };
                        let Ok(name_ptr) = scene_manager.read_pointer(process, vtable + scene_manager.offsets.klass_name) else { return false };
                        name_ptr
                    }
                }
            };

            let Ok(class_name) = process.read::<ArrayCString<128>>(name_ptr) else { return false };
            class_name.as_bytes() == name.as_bytes()
        }).ok_or(Error {})
    }

    /// Iterates over children `GameObject`s referred by the current one
    pub fn children<'a>(
        &'a self,
        process: &'a Process,
        scene_manager: &'a SceneManager,
    ) -> Result<impl Iterator<Item = Self> + 'a, Error> {
        let main_object = scene_manager
            .read_pointer(process, self.address + scene_manager.offsets.game_object)?;

        let transform =
            scene_manager.read_pointer(process, main_object + scene_manager.pointer_size)?;

        let child_count =
            process.read::<u32>(transform + scene_manager.offsets.children_count)? as usize;

        if child_count == 0 {
            return Err(Error {});
        }

        let child_pointer = scene_manager
            .read_pointer(process, transform + scene_manager.offsets.children_pointer)?;

        // Define an empty array and fill it later with the addresses of all child classes found for the current GameObject.
        // Reading the whole array of pointers is (slightly) faster than reading each address in a loop
        const ARRAY_SIZE: usize = 128;
        let mut children = [Address::NULL; ARRAY_SIZE];

        if scene_manager.is_64_bit {
            let slice = &mut [Address64::NULL; ARRAY_SIZE][0..child_count];
            process.read_into_slice(child_pointer, slice)?;

            for val in 0..child_count {
                children[val] = slice[val].into();
            }
        } else {
            let slice = &mut [Address32::NULL; ARRAY_SIZE][0..child_count];
            process.read_into_slice(child_pointer, slice)?;

            for val in 0..child_count {
                children[val] = slice[val].into();
            }
        }

        Ok((0..child_count).filter_map(move |f| {
            let game_object = scene_manager
                .read_pointer(process, children[f] + scene_manager.offsets.game_object)
                .ok()?;

            Some(Self {
                address: game_object,
            })
        }))
    }

    /// Tries to find a child `GameObject` from the current one.
    pub fn get_child(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        name: &str,
    ) -> Result<Self, Error> {
        self.children(process, scene_manager)?
            .find(|p| {
                let Ok(obj_name) = p.get_name::<128>(process, scene_manager) else { return false };
                obj_name.as_bytes() == name.as_bytes()
            })
            .ok_or(Error {})
    }
}

struct Offsets {
    scene_count: u8,
    loaded_scenes: u8,
    active_scene: u8,
    asset_path: u8,
    build_index: u8,
    root_storage_container: u8,
    game_object: u8,
    game_object_name: u8,
    number_of_object_components: u8,
    klass: u8,
    klass_name: u8,
    children_count: u8,
    children_pointer: u8,
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
                root_storage_container: 0xB0,
                game_object: 0x30,
                game_object_name: 0x60,
                number_of_object_components: 0x40,
                klass: 0x28,
                klass_name: 0x48,
                children_count: 0x80,
                children_pointer: 0x70,
            },
            false => &Self {
                scene_count: 0xC,
                loaded_scenes: 0x18,
                active_scene: 0x28,
                asset_path: 0xC,
                build_index: 0x70,
                root_storage_container: 0x88,
                game_object: 0x1C,
                game_object_name: 0x3C,
                number_of_object_components: 0x24,
                klass: 0x18,
                klass_name: 0x2C,
                children_count: 0x58,
                children_pointer: 0x50,
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

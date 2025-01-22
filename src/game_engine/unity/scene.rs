//! Support for identifying the current scene in Unity games.

// References:
// https://gist.githubusercontent.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e/raw/8aa3e6b8da01fd03ff2ff0c03cbd018e522ef988/UnityScene.hpp
// (some offsets seem to be wrong anyway, but it's a very good starting point)
//
// Offsets and logic for Transforms and GameObjects taken from https://github.com/Micrologist/UnityInstanceDumper

use core::{
    array,
    iter::{self, FusedIterator},
    mem::MaybeUninit,
};

use crate::{
    file_format::pe, future::retry, signature::Signature, string::ArrayCString, Address, Address32,
    Address64, Error, PointerSize, Process,
};

const CSTR: usize = 128;

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
        const SIG_64_BIT: Signature<13> = Signature::new("48 83 EC 20 4C 8B ?5 ???????? 33 F6");
        const SIG_32_1: Signature<12> = Signature::new("55 8B EC 51 A1 ???????? 53 33 DB");
        const SIG_32_2: Signature<6> = Signature::new("53 8D 41 ?? 33 DB");
        const SIG_32_3: Signature<14> = Signature::new("55 8B EC 83 EC 18 A1 ???????? 33 C9 53");

        let unity_player = process
            .get_module_address("UnityPlayer.dll")
            .ok()
            .and_then(|address| {
                Some((address, pe::read_size_of_image(process, address)? as u64))
            })?;

        let pointer_size = match pe::MachineType::read(process, unity_player.0)? {
            pe::MachineType::X86_64 => PointerSize::Bit64,
            _ => PointerSize::Bit32,
        };

        let is_il2cpp = process.get_module_address("GameAssembly.dll").is_ok();

        // There are multiple signatures that can be used, depending on the version of Unity
        // used in the target game.
        let base_address: Address = if pointer_size == PointerSize::Bit64 {
            let addr = SIG_64_BIT.scan_once(process, unity_player)? + 7;
            addr + 0x4 + process.read::<i32>(addr).ok()?
        } else if let Some(addr) = SIG_32_1.scan_once(process, unity_player) {
            process.read::<Address32>(addr + 5).ok()?.into()
        } else if let Some(addr) = SIG_32_2.scan_once(process, unity_player) {
            process.read::<Address32>(addr.add_signed(-4)).ok()?.into()
        } else if let Some(addr) = SIG_32_3.scan_once(process, unity_player) {
            process.read::<Address32>(addr + 7).ok()?.into()
        } else {
            return None;
        };

        let offsets = Offsets::new(pointer_size);

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
    fn get_current_scene(&self, process: &Process) -> Result<Scene, Error> {
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
    fn get_dont_destroy_on_load_scene(&self) -> Scene {
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
        scene: &Scene,
    ) -> impl FusedIterator<Item = Transform> + 'a {
        let list_first = process
            .read_pointer(
                scene.address + self.offsets.root_storage_container,
                self.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null());

        let mut current_list = list_first;

        iter::from_fn(move || {
            let [first, _, third]: [Address; 3] = match self.pointer_size {
                PointerSize::Bit64 => process
                    .read::<[Address64; 3]>(current_list?)
                    .ok()
                    .filter(|[first, _, third]| !first.is_null() && !third.is_null())?
                    .map(|a| a.into()),
                _ => process
                    .read::<[Address32; 3]>(current_list?)
                    .ok()
                    .filter(|[first, _, third]| !first.is_null() && !third.is_null())?
                    .map(|a| a.into()),
            };

            if first == list_first? {
                current_list = None;
            } else {
                current_list = Some(first);
            }

            Some(Transform { address: third })
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

/// A `Transform` is a base class for all entities used in a Unity scene. All
/// classes of interest useful for an auto splitter can be found starting from
/// the addresses of the root `Transform`s linked in each scene.
pub struct Transform {
    address: Address,
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

struct Offsets {
    scene_count: u8,
    active_scene: u8,
    dont_destroy_on_load_scene: u8,
    asset_path: u8,
    build_index: u8,
    root_storage_container: u8,
    game_object: u8,
    game_object_name: u8,
    klass: u8,
    klass_name: u8,
    children_pointer: u8,
}

impl Offsets {
    pub const fn new(pointer_size: PointerSize) -> &'static Self {
        match pointer_size {
            PointerSize::Bit64 => &Self {
                scene_count: 0x18,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
                asset_path: 0x10,
                build_index: 0x98,
                root_storage_container: 0xB0,
                game_object: 0x30,
                game_object_name: 0x60,
                klass: 0x28,
                klass_name: 0x48,
                children_pointer: 0x70,
            },
            _ => &Self {
                scene_count: 0x10,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
                asset_path: 0xC,
                build_index: 0x70,
                root_storage_container: 0x88,
                game_object: 0x1C,
                game_object_name: 0x3C,
                klass: 0x18,
                klass_name: 0x2C,
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

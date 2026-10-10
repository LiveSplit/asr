//! Support for attaching to Unity games that are using the standard Mono
//! backend.
//!
//! Automatic attachment requires the `alloc` feature so that a game without
//! a separate player module can be identified through its executable. A
//! `no_std` auto splitter using allocation must provide a global allocator. Use
//! [`Module::attach`] or [`Module::wait_attach`] with a known [`Profile`] to
//! attach without allocation on Windows and Linux. macOS support requires
//! `alloc` for Mach-O symbol lookup even with an explicit profile.

#[cfg(feature = "alloc")]
use crate::file_format::macho;
use arrayvec::ArrayVec;
use bytemuck::CheckedBitPattern;
#[cfg(feature = "alloc")]
use core::fmt;

#[cfg(feature = "alloc")]
use crate::print_limited;

use crate::{
    file_format::{elf, pe},
    future::retry,
    signature::Signature,
    string::ArrayCString,
    Address, Address32, Error, PointerSize, Process,
};

// The tables also supply the allocation-free profile constants. Their lookup
// helpers are only used by automatic attachment, which requires `alloc`.
#[cfg_attr(not(feature = "alloc"), allow(dead_code))]
mod builds;
mod image;
#[cfg_attr(not(feature = "alloc"), allow(dead_code))]
mod linux_builds;
#[cfg(feature = "alloc")]
mod mac_builds;
pub use image::Image;
mod class;
pub use class::Class;
mod pointer;
pub mod profiles;
pub use pointer::UnityPointer;
mod offsets;
pub use offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoVTableOffsets, Profile, TypeOffsets,
};
#[cfg(all(test, not(target_family = "wasm")))]
mod attachment_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod collections_tests;
#[cfg(all(test, feature = "alloc", not(target_family = "wasm")))]
mod identity_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod profiles_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod readers_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod version_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod walk_tests;

use super::{managed, BinaryFormat, DictionaryOffsets, HashSetOffsets, ListOffsets, ManagedString};

/// Represents access to a Unity game that is using the standard Mono backend.
pub struct Module {
    assemblies: Address,
    profile: Profile,
    pointer_size: PointerSize,
}

/// The Mono runtime a game runs. Unity has shipped two.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Library {
    /// The old runtime: `mono.dll`, `libmono.so` or `libmono.0.dylib`. It
    /// keeps a class's statics in the data slot of the vtable.
    Mono,
    /// The newer runtime named after its garbage collector:
    /// `mono-2.0-bdwgc.dll`, `libmonobdwgc-2.0.so` or
    /// `libmonobdwgc-2.0.dylib`.
    MonoBdwgc,
}

/// The identity of one exact runtime binary, and the module it was read
/// from, so a build nobody has measured is reported as the file to look at.
#[cfg(feature = "alloc")]
enum Identity {
    Debug(pe::DebugId),
    Build(elf::BuildId, &'static str),
    #[cfg(feature = "alloc")]
    Uuid(macho::Uuid),
}

#[cfg(feature = "alloc")]
impl Identity {
    /// Reads what names the Mono library's build. On ELF the library's own
    /// build ID answers whenever it has one, and `UnityPlayer.so`'s answers
    /// only for a library that has none: a library naming itself is the one
    /// that counts, known or not, where reaching past it would pair it with
    /// another file's offsets.
    fn read(
        process: &Process,
        runtime: ((Address, u64), &'static str),
        player: Option<(Address, u64)>,
        format: BinaryFormat,
    ) -> Option<Self> {
        let (runtime, runtime_name) = runtime;

        match format {
            BinaryFormat::PE => pe::DebugId::read(process, runtime.0).map(Self::Debug),
            BinaryFormat::ELF => match elf::build_id(process, runtime) {
                Some(build_id) => Some(Self::Build(build_id, runtime_name)),
                None => Some(Self::Build(
                    elf::build_id(process, player?)?,
                    "UnityPlayer.so",
                )),
            },
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => macho::uuid(process, runtime).map(Self::Uuid),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// The profile measured from this build, when it is one that was
    /// measured at the pointer size the target runs at.
    fn find(&self, pointer_size: PointerSize) -> Option<Profile> {
        let profile = match self {
            Self::Debug(debug_id) => builds::find(debug_id)?.profile,
            Self::Build(build_id, _) => linux_builds::find(build_id.as_bytes())?.profile,
            #[cfg(feature = "alloc")]
            Self::Uuid(uuid) => mac_builds::find(&uuid.bytes)?.profile,
        };
        (profile.pointer_size == pointer_size).then_some(profile)
    }
}

#[cfg(feature = "alloc")]
impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Debug(debug_id) => write!(f, "{debug_id:?}"),
            Self::Build(build_id, module) => write!(f, "{build_id:?} in {module}"),
            #[cfg(feature = "alloc")]
            Self::Uuid(uuid) => write!(f, "{uuid:?}"),
        }
    }
}

impl Module {
    /// Tries attaching to a Unity game that is using the standard Mono backend.
    /// If the Mono runtime is a known build, this function uses the offsets
    /// measured for that exact build. Otherwise it uses the newest build of
    /// the same runtime library at or below the game's Unity version. On
    /// Windows, a game older than every build doesn't attach. On Linux and
    /// Mac, it gets the oldest build.
    ///
    /// This requires the `alloc` feature. A game without a separate player
    /// module carries its version in its executable, and finding that
    /// executable allocates its path. No fixed-size path buffer is used.
    /// A `no_std` auto splitter must provide a global allocator.
    /// If you know the build in advance, use [`attach`](Self::attach) with
    /// its profile instead; on Windows and Linux, explicit-profile attachment
    /// does not allocate.
    #[cfg(feature = "alloc")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let (module_range, format, name, library) = Self::find_runtime_module(process)?;
        let pointer_size = Self::pointer_size(process, module_range, format)?;

        // The player names the build for a Mono library that has no ID of its
        // own, which is every Linux one past a point.
        let player = match format {
            BinaryFormat::ELF => process.get_module_range("UnityPlayer.so").ok(),
            _ => None,
        };
        let identity = Identity::read(process, (module_range, name), player, format);

        if let Some(identity) = &identity {
            if let Some(profile) = identity.find(pointer_size) {
                if let Some(module) = Self::attach_with(process, module_range, format, profile) {
                    print_limited::<128>(&format_args!("known mono build: {identity:?}"));
                    return Some(module);
                }
            }
        }

        let unity = Self::unity_version(process, format)?;
        let (built, profile) = match format {
            BinaryFormat::PE => builds::nearest(unity, library, pointer_size)
                .map(|build| (build.unity, build.profile))?,
            BinaryFormat::ELF => linux_builds::nearest(unity, library, pointer_size)
                .map(|build| (build.unity, build.profile))?,
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => mac_builds::nearest(unity, library, pointer_size)
                .map(|build| (build.unity, build.profile))?,
            #[allow(unreachable_patterns)]
            _ => return None,
        };

        let module = Self::attach_with(process, module_range, format, profile)?;
        if let Some(identity) = &identity {
            print_limited::<128>(&format_args!("unknown mono build: {identity:?}"));
        }
        print_limited::<128>(&format_args!(
            "mono: unity {}.{}.{}.{} takes the build measured on {}.{}.{}.{}",
            unity.0, unity.1, unity.2, unity.3, built.0, built.1, built.2, built.3,
        ));
        Some(module)
    }

    /// Tries attaching to a Unity game that is using the standard Mono backend
    /// with the provided measured [`Profile`]. The profile's pointer width
    /// and runtime library must match the target process. Use a built-in
    /// [`profiles`] constant for a known player, or construct a custom
    /// profile for another measured layout. On Windows and Linux, this does
    /// not allocate or require the `alloc` feature. macOS requires `alloc`
    /// for Mach-O symbol lookup. If the target is not known in advance,
    /// enable `alloc` and use `Module::attach_auto_detect` instead.
    pub fn attach(process: &Process, profile: Profile) -> Option<Self> {
        let (module_range, format, _, library) = Self::find_runtime_module(process)?;
        let pointer_size = Self::pointer_size(process, module_range, format)?;
        if library != profile.library || pointer_size != profile.pointer_size {
            return None;
        }

        Self::attach_with(process, module_range, format, profile)
    }

    fn find_runtime_module(
        process: &Process,
    ) -> Option<((Address, u64), BinaryFormat, &'static str, Library)> {
        [
            ("mono.dll", BinaryFormat::PE, Library::Mono),
            ("libmono.so", BinaryFormat::ELF, Library::Mono),
            #[cfg(feature = "alloc")]
            ("libmono.0.dylib", BinaryFormat::MachO, Library::Mono),
            ("mono-2.0-bdwgc.dll", BinaryFormat::PE, Library::MonoBdwgc),
            ("libmonobdwgc-2.0.so", BinaryFormat::ELF, Library::MonoBdwgc),
            #[cfg(feature = "alloc")]
            (
                "libmonobdwgc-2.0.dylib",
                BinaryFormat::MachO,
                Library::MonoBdwgc,
            ),
        ]
        .into_iter()
        .find_map(|(name, format, library)| {
            Some((process.get_module_range(name).ok()?, format, name, library))
        })
    }

    /// Reads the Unity version of the game. On Windows it is the file version
    /// of `UnityPlayer.dll`. On Linux and Mac the player carries it as a
    /// string. A game without a player module keeps both in its executable,
    /// which only the `alloc` feature can name.
    #[cfg(feature = "alloc")]
    fn unity_version(process: &Process, format: BinaryFormat) -> Option<(u16, u16, u16, u16)> {
        let player = match format {
            BinaryFormat::PE => process.get_module_range("UnityPlayer.dll").ok(),
            BinaryFormat::ELF => process.get_module_range("UnityPlayer.so").ok(),
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => process.get_module_range("UnityPlayer.dylib").ok(),
            #[allow(unreachable_patterns)]
            _ => None,
        };
        let player = player.or_else(|| process.get_main_module_range().ok());
        let player = player?;

        match format {
            BinaryFormat::PE => {
                let file_version = pe::FileVersion::read(process, player.0)?;
                Some((
                    file_version.major_version,
                    file_version.minor_version,
                    file_version.build_part,
                    file_version.private_part,
                ))
            }
            _ => super::version_string(process, player),
        }
    }

    fn pointer_size(
        process: &Process,
        module_range: (Address, u64),
        format: BinaryFormat,
    ) -> Option<PointerSize> {
        match format {
            BinaryFormat::PE => pe::MachineType::read(process, module_range.0)?.pointer_size(),
            BinaryFormat::ELF => elf::pointer_size(process, module_range.0),
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => macho::pointer_size(process, module_range),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    fn attach_with(
        process: &Process,
        module_range: (Address, u64),
        format: BinaryFormat,
        profile: Profile,
    ) -> Option<Self> {
        let (mono_module, _) = module_range;
        let pointer_size = profile.pointer_size;

        let root_domain_function_address = match format {
            BinaryFormat::PE => {
                pe::symbols(process, mono_module)
                    .find(|symbol| {
                        symbol
                            .get_name::<22>(process)
                            .is_ok_and(|name| name.matches("mono_assembly_foreach"))
                    })?
                    .address
            }
            BinaryFormat::ELF => {
                elf::symbols(process, mono_module)
                    .find(|symbol| {
                        symbol
                            .get_name::<22>(process)
                            .is_ok_and(|name| name.matches("mono_assembly_foreach"))
                    })?
                    .address
            }
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => {
                macho::symbols(process, module_range)
                    .find(|symbol| {
                        symbol
                            .get_name::<26>(process)
                            .is_ok_and(|name| name.matches("_mono_assembly_foreach"))
                    })?
                    .address
            }
            #[allow(unreachable_patterns)]
            _ => return None,
        };

        let assemblies: Address = match (pointer_size, format) {
            (PointerSize::Bit64, BinaryFormat::PE) => {
                const SIG_MONO_64: Signature<3> = Signature::new("48 8B 0D");
                SIG_MONO_64
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| Some(addr + 0x4 + process.read::<i32>(addr).ok()?))?
            }
            (PointerSize::Bit64, BinaryFormat::ELF) => {
                const SIG_MONO_64_ELF: Signature<3> = Signature::new("48 8B 3D");
                SIG_MONO_64_ELF
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                    .map(|addr| addr + 3)
                    .and_then(|addr| Some(addr + 0x4 + process.read::<i32>(addr).ok()?))?
            }
            #[cfg(feature = "alloc")]
            (PointerSize::Bit64, BinaryFormat::MachO) => {
                const SIG_MONO_X86_64_MACHO: Signature<3> = Signature::new("48 8B 3D");
                // 57 0f 00 d0   adrp  x23,(page + 0x1ea000)
                // e0 da 47 f9   ldr   x0,[x23, #0xfb0]=>(page + 0x1eafb0)
                // adrp                                ldr
                // 57       0f       00       d0       e0       da       47       f9
                // ???10111 ???????? ???????? 1??10000 11100000 ??????10 01?????? 11111001
                // hi0      hi1      hi2       lo               i0         i1
                const SIG_MONO_ARM_64_MACHO: Signature<8> = Signature::masked(
                    [
                        0b00010111, 0, 0, 0b10010000, 0xE0, 0b00000010, 0b01000000, 0xF9,
                    ],
                    [
                        0b00011111, 0, 0, 0b10011111, 0xFF, 0b00000011, 0b11000000, 0xFF,
                    ],
                );
                if let Some(scan_address) = SIG_MONO_X86_64_MACHO
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                    .map(|a| a + 3)
                {
                    scan_address + 0x4 + process.read::<i32>(scan_address).ok()?
                } else if let Some(scan_address) = SIG_MONO_ARM_64_MACHO
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                {
                    let page = scan_address.value() & 0xfffffffffffff000;
                    let bs = process.read::<[u8; 8]>(scan_address).ok()?;
                    // adrp
                    let lo = ((bs[3] >> 5) & 0b11) as u64;
                    let hi0 = (bs[0] >> 5) as u64;
                    let hi1 = bs[1] as u64;
                    let hi2 = bs[2] as u64;
                    let adrp = (lo << 12) | (hi0 << 14) | (hi1 << 17) | (hi2 << 25);
                    // ldr
                    let i0 = (bs[5] >> 2) as u64;
                    let i1 = (bs[6] & 0b111111) as u64;
                    let ldr = (i0 << 3) | (i1 << 9);
                    (page + adrp + ldr).into()
                } else {
                    return None;
                }
            }
            (PointerSize::Bit32, BinaryFormat::PE) => {
                const SIG_32_1: Signature<2> = Signature::new("FF 35");
                const SIG_32_2: Signature<2> = Signature::new("8B 0D");

                let ptr = [SIG_32_1, SIG_32_2].iter().find_map(|sig| {
                    sig.scan_process_range(process, (root_domain_function_address, 0x100))
                })? + 2;

                process.read::<Address32>(ptr).ok()?.into()
            }
            _ => return None,
        };

        Some(Self {
            assemblies,
            profile,
            pointer_size,
        })
    }

    /// Retrieve the [pointer size](PointerSize) of the process/module.
    pub fn get_pointer_size(&self) -> PointerSize {
        self.pointer_size
    }

    /// Reads the name of the class of a live object. The scene manager uses
    /// this to tell the components of a game object apart.
    pub(super) fn object_class_name<const N: usize>(
        &self,
        process: &Process,
        object: Address,
    ) -> Option<ArrayCString<N>> {
        let walk = self.walk();
        let class = walk.object_class(process, object)?;
        walk.class_name(process, class)
    }

    fn walk(&self) -> managed::Walk {
        managed::Walk {
            runtime: managed::Runtime::Mono(managed::MonoRuntime {
                assemblies: self.assemblies,
                class_cache: self.profile.image.class_cache,
                hash_table_size: self.profile.hash_table.size.into(),
                hash_table_table: self.profile.hash_table.table.into(),
                next_class_cache: self.profile.class.next_class_cache,
                field_count: self.profile.class.field_count,
                class_kind: self.profile.class.class_kind,
                generic_class: self.profile.generic.generic_class,
                container_class: self.profile.generic.container_class,
                type_data: self.profile.type_words.data,
                type_kind: self.profile.type_words.kind,
                runtime_info: self.profile.class.runtime_info,
                vtable_size: self.profile.class.vtable_size.into(),
                vtable: self.profile.v_table.vtable.into(),
                statics_in_vtable_data: self.profile.library == Library::Mono,
            }),
            offsets: managed::WalkOffsets {
                assembly: managed::AssemblyOffsets {
                    name_in_image: self.profile.image.assembly_name.map(u16::from),
                    name_in_assembly: self.profile.assembly.aname.map(u16::from),
                    image: self.profile.assembly.image.into(),
                },
                class: managed::ClassOffsets {
                    name: self.profile.class.name.into(),
                    namespace: self.profile.class.namespace.into(),
                    parent: self.profile.class.parent.into(),
                    declaring: self.profile.class.nested_in,
                    instance_size: self.profile.class.instance_size,
                    fields: self.profile.class.fields.into(),
                },
                field: managed::FieldOffsets {
                    name: self.profile.field.name.into(),
                    type_: self.profile.field.type_,
                    offset: self.profile.field.offset.into(),
                    stride: self.profile.field.stride.into(),
                },
            },
            stop: managed::ClimbStop::UNITY,
            pointer_size: self.pointer_size,
        }
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`get_default_image`](Self::get_default_image) function is a shorthand
    /// for this function that accesses the `Assembly-CSharp` [image](Image).
    pub fn get_image(&self, process: &Process, assembly_name: &str) -> Option<Image> {
        self.walk()
            .find_image(process, assembly_name)
            .map(|image| Image {
                image: image.address,
            })
    }

    /// Looks for the `Assembly-CSharp` binary [image](Image) inside the target
    /// process. An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main
    /// game assembly, and contains all the game logic. This function is a
    /// shorthand for [`get_image`](Self::get_image) that accesses the
    /// `Assembly-CSharp` [image](Image).
    pub fn get_default_image(&self, process: &Process) -> Option<Image> {
        self.get_image(process, "Assembly-CSharp")
    }

    /// Reads a managed string through the reference stored at the given
    /// address, such as the end of a pointer path or a slot in a static
    /// table. The string carries its own character count, so no length is
    /// passed, and the returned [`ManagedString`] holds exactly that many
    /// UTF-16 units, a nul character among them like any other. `N` bounds
    /// how many units the string holds, and a string claiming more than that
    /// fails rather than truncates, as do a negative count and a null
    /// reference. A string already held by its object address reads with
    /// [`read_string_object`](Self::read_string_object).
    pub fn read_string<const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ManagedString<N>, Error> {
        managed::read_string(process, self.pointer_size, at)
    }

    /// Reads a managed string at its object address rather than through a
    /// reference: the form for an element
    /// [`read_reference_array`](Self::read_reference_array) hands back. A
    /// null address fails, so a null element refuses at the element with
    /// its index in hand. The returned [`ManagedString`] holds exactly the
    /// string's count of UTF-16 units, a nul character among them like any
    /// other. `N` bounds how many units the string holds, and a string
    /// claiming more than that fails rather than truncates, as does a
    /// negative count.
    pub fn read_string_object<const N: usize>(
        &self,
        process: &Process,
        object: Address,
    ) -> Result<ManagedString<N>, Error> {
        managed::read_string_object(process, self.pointer_size, object)
    }

    /// Reads a managed array of value elements through the reference stored
    /// at the given address. The array carries its own length, so no count
    /// is passed; `N` bounds how many elements the returned [`ArrayVec`]
    /// holds, and an array claiming more than that fails rather than
    /// truncates, as does a null reference. The element type is the caller's
    /// claim and has to match the target's own element layout: a managed
    /// `char` is a `u16` here, a `bool` a single byte, and Rust's `char` and
    /// `usize` never match. Reference elements read with
    /// [`read_reference_array`](Self::read_reference_array).
    pub fn read_array<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_array(process, self.pointer_size, at)
    }

    /// Reads a managed array of reference elements through the reference
    /// stored at the given address, handing back the elements' object
    /// addresses read at the target's own pointer width. A null array
    /// reference fails; null elements are data and come back as null
    /// addresses at their positions, since positions carry meaning and
    /// filtering is the caller's choice. `N` bounds how many elements the
    /// returned vector holds, and an array claiming more than that fails
    /// rather than truncates. A `string` element reads with
    /// [`read_string_object`](Self::read_string_object); any other object's
    /// fields read at its address directly.
    pub fn read_reference_array<const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ArrayVec<Address, N>, Error> {
        managed::read_reference_array(process, self.pointer_size, at)
    }

    /// Resolves where a `List` keeps its backing array and live count, finding
    /// `System.Collections.Generic.List` in the class hierarchy of the object
    /// at the given address. The answer is a small `Copy` value worth storing,
    /// like a field offset: resolution walks class metadata, where the read
    /// itself is a handful of reads. An object that is not a list misses.
    pub fn get_list_offsets(&self, process: &Process, at: Address) -> Option<ListOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().list_offsets(process, object)
    }

    /// Resolves where a `Dictionary` keeps its backing entries and live
    /// counts, and how one entry lays out. The address must store a managed
    /// reference to a `System.Collections.Generic.Dictionary`; subclasses
    /// are supported by finding that class in the object's hierarchy.
    ///
    /// The answer is a small `Copy` value worth storing, like a field offset:
    /// resolution walks class metadata, while reading it later costs only a
    /// handful of process reads. The entry layout depends on the dictionary's
    /// concrete key and value types, so the result must only be reused for the
    /// same dictionary type.
    ///
    /// This returns `None` for objects that are not dictionaries, targets
    /// still initializing their metadata, and runtime profiles that lack the
    /// additional layout metadata required for dictionary resolution.
    pub fn get_dictionary_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> Option<DictionaryOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().dictionary_offsets(process, object)
    }

    /// Resolves where a `HashSet` keeps its backing slots, live count, and
    /// high-water mark, and how one slot lays out. The address must store a
    /// managed reference to a `System.Collections.Generic.HashSet`;
    /// subclasses are supported by finding that class in the object's
    /// hierarchy.
    ///
    /// The answer is a small `Copy` value worth storing, like a field offset:
    /// resolution walks class metadata, while reading it later costs only a
    /// handful of process reads. The slot layout depends on the hash set's
    /// concrete value type, so the result must only be reused for the same
    /// hash set type.
    ///
    /// This returns `None` for objects that are not hash sets, targets still
    /// initializing their metadata, and runtime profiles that lack the
    /// additional layout metadata required for hash set resolution.
    pub fn get_hash_set_offsets(&self, process: &Process, at: Address) -> Option<HashSetOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().hash_set_offsets(process, object)
    }

    /// Reads a managed `HashSet`'s live values through the reference stored
    /// at the given address, with the offsets
    /// [`get_hash_set_offsets`](Self::get_hash_set_offsets) resolved. The
    /// walk runs to the high-water mark rather than the live count, since
    /// freed slots sit inside it; `N` bounds the live values, and the live
    /// tally has to balance against the count exactly or the read fails. The
    /// value type is the caller's claim, as with
    /// [`read_array`](Self::read_array). Managed references are read as
    /// [`Address32`] or [`Address64`](crate::Address64), according to
    /// [`get_pointer_size`](Self::get_pointer_size).
    pub fn read_hash_set<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: HashSetOffsets,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_hash_set(process, self.pointer_size, offsets, at)
    }

    /// Reads a managed `Dictionary`'s live pairs through the reference
    /// stored at the given address, with the offsets
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) resolved.
    /// `N` bounds the live pairs, never the counted entries or the backing
    /// capacity; freed entries are skipped by their marks, and a live tally
    /// that cannot balance against the counts fails rather than answering
    /// wrong pairs. The key and value types are the caller's claims, as
    /// with [`read_array`](Self::read_array), refused where a claim
    /// outgrows the room its member has inside one entry. Managed references
    /// are read as [`Address32`] or
    /// [`Address64`](crate::Address64), according to
    /// [`get_pointer_size`](Self::get_pointer_size).
    pub fn read_dictionary<K: CheckedBitPattern, V: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: DictionaryOffsets,
        at: Address,
    ) -> Result<ArrayVec<(K, V), N>, Error> {
        managed::read_dictionary(process, self.pointer_size, offsets, at)
    }

    /// Reads a managed `List` of value elements through the reference stored
    /// at the given address, with the offsets
    /// [`get_list_offsets`](Self::get_list_offsets) resolved. The list's
    /// live count is read, never its backing capacity; `N` bounds the
    /// count, and a count past the buffer or past the backing array's own
    /// length fails rather than truncates, as does a null reference. The
    /// element type is the caller's claim, as with
    /// [`read_array`](Self::read_array).
    pub fn read_list<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: ListOffsets,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_list(process, self.pointer_size, offsets, at)
    }

    /// Reads a managed `List` of reference elements through the reference
    /// stored at the given address, with the offsets
    /// [`get_list_offsets`](Self::get_list_offsets) resolved, handing back
    /// the elements' object addresses read at the target's own pointer
    /// width. The list's live count is read, never its backing capacity;
    /// `N` bounds the count, and a count past the buffer or past the
    /// backing array's own length fails rather than truncates, as does a
    /// null list reference. Null elements are data and come back as null
    /// addresses at their positions.
    pub fn read_reference_list<const N: usize>(
        &self,
        process: &Process,
        offsets: ListOffsets,
        at: Address,
    ) -> Result<ArrayVec<Address, N>, Error> {
        managed::read_reference_list(process, self.pointer_size, offsets, at)
    }

    /// Attaches to a Unity game that is using the standard Mono backend. A
    /// known build gets the offsets measured for it, and any other game gets
    /// the offsets of the measured build nearest to its Unity version.
    ///
    /// This is the `await`able version of the
    /// [`attach_auto_detect`](Self::attach_auto_detect) function, yielding back
    /// to the runtime between each try. It requires the `alloc` feature so
    /// that the executable's path can be allocated when no player module is
    /// present. For allocation-free attachment, use [`wait_attach`](Self::wait_attach)
    /// with a known profile instead.
    #[cfg(feature = "alloc")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
    pub async fn wait_attach_auto_detect(process: &Process) -> Module {
        retry(|| Self::attach_auto_detect(process)).await
    }

    /// Attaches to a Unity game that is using the standard Mono backend with
    /// the provided measured [`Profile`]. The profile's pointer width and
    /// runtime library must match the target process. On Windows and Linux,
    /// this does not allocate or require the `alloc` feature. macOS requires
    /// `alloc` for Mach-O symbol lookup.
    ///
    /// This is the `await`able version of [`attach`](Self::attach), yielding
    /// back to the runtime between each try.
    pub async fn wait_attach(process: &Process, profile: Profile) -> Self {
        retry(|| Self::attach(process, profile)).await
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`wait_get_default_image`](Self::wait_get_default_image) function is a
    /// shorthand for this function that accesses the `Assembly-CSharp`
    /// [image](Image).
    ///
    /// This is the `await`able version of the [`get_image`](Self::get_image)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_get_image(&self, process: &Process, assembly_name: &str) -> Image {
        retry(|| self.get_image(process, assembly_name)).await
    }

    /// Looks for the `Assembly-CSharp` binary [image](Image) inside the target
    /// process. An [image](Image) is a .NET DLL that
    /// is loaded by the game. The `Assembly-CSharp` [image](Image) is the main
    /// game assembly, and contains all the game logic. This function is a
    /// shorthand for [`wait_get_image`](Self::wait_get_image) that accesses the
    /// `Assembly-CSharp` [image](Image).
    ///
    /// This is the `await`able version of the
    /// [`get_default_image`](Self::get_default_image) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_get_default_image(&self, process: &Process) -> Image {
        retry(|| self.get_default_image(process)).await
    }

    /// Resolves where a `List` keeps its backing array and live count from the
    /// class hierarchy of the list object at the given address.
    ///
    /// This is the `await`able version of the
    /// [`get_list_offsets`](Self::get_list_offsets) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_get_list_offsets(&self, process: &Process, at: Address) -> ListOffsets {
        retry(|| self.get_list_offsets(process, at)).await
    }

    /// Resolves where a `Dictionary` keeps its backing entries and live
    /// counts, and how one entry lays out from the dictionary's class
    /// hierarchy.
    ///
    /// This is the `await`able version of the
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) function,
    /// yielding back to the runtime between each try. This waits indefinitely
    /// if the active runtime profile lacks the layout metadata needed for
    /// dictionary resolution.
    pub async fn wait_get_dictionary_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> DictionaryOffsets {
        retry(|| self.get_dictionary_offsets(process, at)).await
    }

    /// Resolves where a `HashSet` keeps its backing slots, live count, and
    /// high-water mark, and how one slot lays out from the hash set's class
    /// hierarchy.
    ///
    /// This is the `await`able version of the
    /// [`get_hash_set_offsets`](Self::get_hash_set_offsets) function,
    /// yielding back to the runtime between each try. This waits indefinitely
    /// if the active runtime profile lacks the layout metadata needed for
    /// hash set resolution.
    pub async fn wait_get_hash_set_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> HashSetOffsets {
        retry(|| self.get_hash_set_offsets(process, at)).await
    }
}

//! Support for attaching to Unity games that are using the standard Mono
//! backend.

#[cfg(feature = "alloc")]
use crate::file_format::macho;
use crate::{
    file_format::{elf, pe},
    future::retry,
    print_limited,
    signature::Signature,
    Address, Address32, PointerSize, Process,
};

mod builds;
mod image;
pub use image::Image;
mod class;
pub use class::Class;
mod version;
pub use version::Version;
mod pointer;
pub use pointer::UnityPointer;
mod offsets;
use offsets::MonoOffsets;
#[cfg(all(test, not(target_family = "wasm")))]
mod walk_tests;

use super::{managed, BinaryFormat};

/// Represents access to a Unity game that is using the standard Mono backend.
pub struct Module {
    assemblies: Address,
    version: Version,
    offsets: &'static MonoOffsets,
    pointer_size: PointerSize,
}

impl Module {
    /// Tries attaching to a Unity game that is using the standard Mono backend.
    /// If the mono runtime is a known build, this function uses the offsets
    /// from its PDB. Otherwise this function detects the [Mono version](Version).
    /// If you know the version in advance or it fails detecting it, use
    /// [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let (module_range, format) = Self::find_runtime_module(process)?;
        let pointer_size = Self::pointer_size(process, module_range, format)?;

        let debug_id = match format {
            BinaryFormat::PE => pe::DebugId::read(process, module_range.0),
            _ => None,
        };

        if let Some(debug_id) = &debug_id {
            if let Some(build) =
                builds::find(debug_id).filter(|build| build.pointer_size == pointer_size)
            {
                if let Some(module) = Self::attach_with(
                    process,
                    module_range,
                    format,
                    pointer_size,
                    build.version,
                    &build.offsets,
                ) {
                    print_limited::<128>(&format_args!("known mono build: {debug_id:?}"));
                    return Some(module);
                }
            }
        }

        let version = Version::detect(process)?;
        let module = Self::attach(process, version)?;

        if let Some(debug_id) = debug_id {
            if builds::find(&debug_id).is_none() {
                print_limited::<128>(&format_args!("unknown mono build: {debug_id:?}"));
            }
        }

        Some(module)
    }

    /// Tries attaching to a Unity game that is using the standard Mono backend
    /// with the [Mono version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let (module_range, format) = Self::find_runtime_module(process)?;
        let pointer_size = Self::pointer_size(process, module_range, format)?;
        let offsets = MonoOffsets::new(version, pointer_size, format)?;

        Self::attach_with(
            process,
            module_range,
            format,
            pointer_size,
            version,
            offsets,
        )
    }

    fn find_runtime_module(process: &Process) -> Option<((Address, u64), BinaryFormat)> {
        [
            ("mono.dll", BinaryFormat::PE),
            ("libmono.so", BinaryFormat::ELF),
            #[cfg(feature = "alloc")]
            ("libmono.0.dylib", BinaryFormat::MachO),
            ("mono-2.0-bdwgc.dll", BinaryFormat::PE),
            ("libmonobdwgc-2.0.so", BinaryFormat::ELF),
            #[cfg(feature = "alloc")]
            ("libmonobdwgc-2.0.dylib", BinaryFormat::MachO),
        ]
        .into_iter()
        .find_map(|(name, format)| Some((process.get_module_range(name).ok()?, format)))
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
        pointer_size: PointerSize,
        version: Version,
        offsets: &'static MonoOffsets,
    ) -> Option<Self> {
        let (mono_module, _) = module_range;

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
                const SIG_MONO_ARM_64_MACHO: Signature<8> = Signature::Complex {
                    needle: [
                        0b00010111, 0, 0, 0b10010000, 0xE0, 0b00000010, 0b01000000, 0xF9,
                    ],
                    mask: [
                        0b00011111, 0, 0, 0b10011111, 0xFF, 0b00000011, 0b11000000, 0xFF,
                    ],
                    anchor_pos: Some(4),
                    anchor_byte: 0xE0,
                    check_pos: Some(7),
                    check_byte: 0xF9,
                };
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
            version,
            offsets,
            pointer_size,
        })
    }

    /// Retrieve the [Mono version](Version) of the module.
    pub fn get_version(&self) -> Version {
        self.version
    }

    /// Retrieve the [pointer size](PointerSize) of the process/module.
    pub fn get_pointer_size(&self) -> PointerSize {
        self.pointer_size
    }

    fn walk(&self) -> managed::Walk {
        managed::Walk {
            runtime: managed::Runtime::Mono(managed::MonoRuntime {
                assemblies: self.assemblies,
                class_cache: self.offsets.image.class_cache,
                hash_table_size: self.offsets.hash_table.size.into(),
                hash_table_table: self.offsets.hash_table.table.into(),
                next_class_cache: self.offsets.class.next_class_cache,
                field_count: self.offsets.class.field_count,
                runtime_info: self.offsets.class.runtime_info,
                vtable_size: self.offsets.class.vtable_size.into(),
                vtable: self.offsets.v_table.vtable.into(),
                statics_in_vtable_data: matches!(self.version, Version::V1 | Version::V1Cattrs),
            }),
            offsets: managed::WalkOffsets {
                assembly: managed::AssemblyOffsets {
                    name_in_image: self.offsets.image.assembly_name.map(u16::from),
                    name_in_assembly: self.offsets.assembly.aname.map(u16::from),
                    image: self.offsets.assembly.image.into(),
                },
                class: managed::ClassOffsets {
                    name: self.offsets.class.name.into(),
                    namespace: self.offsets.class.namespace.into(),
                    parent: self.offsets.class.parent.into(),
                    fields: self.offsets.class.fields.into(),
                },
                field: managed::FieldOffsets {
                    name: self.offsets.field.name.into(),
                    offset: self.offsets.field.offset.into(),
                    stride: self.offsets.field.alignment.into(),
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

    /// Attaches to a Unity game that is using the standard Mono backend. This
    /// function automatically detects the [Mono version](Version). If you
    /// know the version in advance or it fails detecting it, use
    /// [`wait_attach`](Self::wait_attach) instead.
    ///
    /// This is the `await`able version of the
    /// [`attach_auto_detect`](Self::attach_auto_detect) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_attach_auto_detect(process: &Process) -> Module {
        retry(|| Self::attach_auto_detect(process)).await
    }

    /// Attaches to a Unity game that is using the standard Mono backend with the
    /// [Mono version](Version) provided. The version needs to be correct
    /// for this function to work. If you don't know the version in advance, use
    /// [`wait_attach_auto_detect`](Self::wait_attach_auto_detect) instead.
    ///
    /// This is the `await`able version of the [`attach`](Self::attach)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_attach(process: &Process, version: Version) -> Self {
        retry(|| Self::attach(process, version)).await
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
}

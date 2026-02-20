//! Support for attaching to Unity games that are using the standard Mono
//! backend.

#[cfg(feature = "alloc")]
use crate::file_format::macho;
use crate::{
    file_format::{elf, pe},
    future::retry,
    signature::Signature,
    Address, Address32, Address64, PointerSize, Process,
};
use core::iter::{self, FusedIterator};

mod assembly;
use assembly::Assembly;
mod image;
pub use image::Image;
mod class;
pub use class::Class;
mod field;
use field::Field;
mod version;
pub use version::Version;
mod pointer;
pub use pointer::UnityPointer;
mod offsets;
use offsets::MonoOffsets;

use super::{BinaryFormat, CSTR};

/// Represents access to a Unity game that is using the standard Mono backend.
pub struct Module {
    assemblies: Address,
    version: Version,
    offsets: &'static MonoOffsets,
    pointer_size: PointerSize,
}

impl Module {
    /// Tries attaching to a Unity game that is using the standard Mono backend.
    /// This function automatically detects the [Mono version](Version). If you
    /// know the version in advance or it fails detecting it, use
    /// [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let version = Version::detect(process)?;
        Self::attach(process, version)
    }

    /// Tries attaching to a Unity game that is using the standard Mono backend
    /// with the [Mono version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let (module_range, format) = [
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
        .find_map(|(name, format)| Some((process.get_module_range(name).ok()?, format)))?;

        let (mono_module, _) = module_range;

        let pointer_size = match format {
            BinaryFormat::PE => pe::MachineType::read(process, mono_module)?.pointer_size()?,
            BinaryFormat::ELF => elf::pointer_size(process, mono_module)?,
            #[cfg(feature = "alloc")]
            BinaryFormat::MachO => macho::pointer_size(process, module_range)?,
            #[allow(unreachable_patterns)]
            _ => return None,
        };

        let offsets = MonoOffsets::new(version, pointer_size, format)?;

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
                const SIG_MONO_ARM_64_MACHO: Signature<8> =
                    Signature::new("57 0F 00 D0 E0 DA 47 F9");
                if let Some(scan_address) = SIG_MONO_X86_64_MACHO
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                    .map(|a| a + 3)
                {
                    scan_address + 0x4 + process.read::<i32>(scan_address).ok()?
                } else if let Some(scan_address) = SIG_MONO_ARM_64_MACHO
                    .scan_process_range(process, (root_domain_function_address, 0x100))
                {
                    let page = scan_address.value() & 0xfffffffffffff000;
                    (page + 0x1eafb0).into()
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

    fn assemblies<'a>(&'a self, process: &'a Process) -> impl FusedIterator<Item = Assembly> + 'a {
        let mut assembly = process
            .read_pointer(self.assemblies, self.pointer_size)
            .ok()
            .filter(|val| !val.is_null());

        iter::from_fn(move || {
            let [data, next_assembly]: [Address; 2] = match self.pointer_size {
                PointerSize::Bit64 => process
                    .read::<[Address64; 2]>(assembly?)
                    .ok()?
                    .map(|item| item.into()),
                _ => process
                    .read::<[Address32; 2]>(assembly?)
                    .ok()?
                    .map(|item| item.into()),
            };

            assembly = Some(next_assembly);

            Some(Assembly { assembly: data })
        })
        .fuse()
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`get_default_image`](Self::get_default_image) function is a shorthand
    /// for this function that accesses the `Assembly-CSharp` [image](Image).
    pub fn get_image(&self, process: &Process, assembly_name: &str) -> Option<Image> {
        self.assemblies(process)
            .find(|assembly| {
                assembly
                    .get_name::<CSTR>(process, self)
                    .is_ok_and(|name| name.matches(assembly_name))
            })
            .and_then(|assembly| assembly.get_image(process, self))
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

    #[inline]
    const fn size_of_ptr(&self) -> u64 {
        self.pointer_size as u64
    }
}

//! Support for attaching to Unity games that are using the IL2CPP backend.

use crate::{file_format::pe, future::retry, signature::Signature, Address, PointerSize, Process};

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
use offsets::IL2CPPOffsets;

const CSTR: usize = 128;

/// Represents access to a Unity game that is using the IL2CPP backend.
pub struct Module {
    assemblies: Address,
    type_info_definition_table: Address,
    version: Version,
    offsets: &'static IL2CPPOffsets,
    pointer_size: PointerSize,
}

impl Module {
    /// Tries attaching to a Unity game that is using the IL2CPP backend. This
    /// function automatically detects the [IL2CPP version](Version). If you
    /// know the version in advance or it fails detecting it, use
    /// [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let version = Version::detect(process)?;
        Self::attach(process, version)
    }

    /// Tries attaching to a Unity game that is using the IL2CPP backend with
    /// the [IL2CPP version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let il2cpp_module = {
            let address = process.get_module_address("GameAssembly.dll").ok()?;
            let size = pe::read_size_of_image(process, address)? as u64;
            (address, size)
        };

        let pointer_size = pe::MachineType::read(process, il2cpp_module.0)?.pointer_size()?;
        let offsets = IL2CPPOffsets::new(version, pointer_size)?;

        let assemblies: Address = {
            const ASSEMBLIES: Signature<12> = Signature::new("75 ?? 48 8B 1D ?? ?? ?? ?? 48 3B 1D");
            ASSEMBLIES
                .scan_process_range(process, il2cpp_module)
                .map(|addr| addr + 5)
                .and_then(|addr| Some(addr + 0x4 + process.read::<i32>(addr).ok()?))?
        };

        let type_info_definition_table: Address = {
            const GLOBAL_METADATA: Signature<20> =
                Signature::new("67 6C 6F 62 61 6C 2D 6D 65 74 61 64 61 74 61 2E 64 61 74 00");
            let s_metadata = GLOBAL_METADATA.scan_process_range(process, il2cpp_module)?;

            const LEA: Signature<3> = Signature::new("48 8D 0D");
            let lea: Address = LEA
                .scan_iter(process, il2cpp_module)
                .map(|addr| addr + 3)
                .find(|&addr| {
                    let Ok(offset) = process.read::<i32>(addr) else {
                        return false;
                    };

                    addr + 0x4 + offset == s_metadata
                })?;

            const SHR: Signature<3> = Signature::new("48 C1 E9");
            let shr: Address = SHR
                .scan_process_range(process, (lea, 0x200))
                .map(|addr| addr + 3)?;

            const RAX: Signature<3> = Signature::new("48 89 05");
            RAX.scan_process_range(process, (shr, 0x100))
                .map(|addr| addr + 3)
                .and_then(|addr| Some(addr + 0x4 + process.read::<i32>(addr).ok()?))?
        };

        Some(Self {
            assemblies,
            type_info_definition_table,
            version,
            offsets,
            pointer_size,
        })
    }

    fn assemblies<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Assembly> + 'a {
        let (assemblies, nr_of_assemblies): (Address, u64) = {
            let [first, limit] = process
                .read::<[u64; 2]>(self.assemblies)
                .unwrap_or_default();
            let count = limit
                .saturating_sub(first)
                .saturating_div(self.size_of_ptr());
            (Address::new(first), count)
        };

        (0..nr_of_assemblies).filter_map(move |i| {
            process
                .read_pointer(
                    assemblies + self.size_of_ptr().wrapping_mul(i),
                    self.pointer_size,
                )
                .ok()
                .filter(|addr| !addr.is_null())
                .map(|assembly| Assembly { assembly })
        })
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

    /// Attaches to a Unity game that is using the IL2CPP backend. This function
    /// automatically detects the [IL2CPP version](Version). If you know the
    /// version in advance or it fails detecting it, use
    /// [`wait_attach`](Self::wait_attach) instead.
    ///
    /// This is the `await`able version of the
    /// [`attach_auto_detect`](Self::attach_auto_detect) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_attach_auto_detect(process: &Process) -> Module {
        retry(|| Self::attach_auto_detect(process)).await
    }

    /// Attaches to a Unity game that is using the IL2CPP backend with the
    /// [IL2CPP version](Version) provided. The version needs to be correct
    /// for this function to work. If you don't know the version in advance, use
    /// [`wait_attach_auto_detect`](Self::wait_attach_auto_detect) instead.
    ///
    /// This is the `await`able version of the [`attach`](Self::attach)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_attach(process: &Process, version: Version) -> Module {
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

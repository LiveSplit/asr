//! Support for attaching to Unity games that are using the IL2CPP backend.

use core::cmp::Ordering;

use crate::{
    file_format::pe, future::retry, signature::Signature, string::ArrayCString, Address, Address32,
    Address64, Error, Process,
};

use bytemuck::CheckedBitPattern;

#[cfg(feature = "derive")]
pub use asr_derive::Il2cppClass as Class;

/// Represents access to a Unity game that is using the IL2CPP backend.
pub struct Module {
    is_64_bit: bool,
    version: Version,
    offsets: &'static Offsets,
    assemblies: Address,
    type_info_definition_table: Address,
}

impl Module {
    /// Tries attaching to a Unity game that is using the IL2CPP backend. This
    /// function automatically detects the [IL2CPP version](Version). If you
    /// know the version in advance or it fails detecting it, use
    /// [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let version = detect_version(process)?;
        Self::attach(process, version)
    }

    /// Tries attaching to a Unity game that is using the IL2CPP backend with
    /// the [IL2CPP version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let mono_module = process.get_module_range("GameAssembly.dll").ok()?;
        let is_64_bit = pe::MachineType::read(process, mono_module.0)? == pe::MachineType::X86_64;

        let assemblies_trg_addr = if is_64_bit {
            const ASSEMBLIES_TRG_SIG: Signature<12> =
                Signature::new("48 FF C5 80 3C ?? 00 75 ?? 48 8B 1D");

            let addr = ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 12;
            addr + 0x4 + process.read::<i32>(addr).ok()?
        } else {
            const ASSEMBLIES_TRG_SIG: Signature<9> = Signature::new("8A 07 47 84 C0 75 ?? 8B 35");

            let addr = ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 9;
            process.read::<Address32>(addr).ok()?.into()
        };

        let type_info_definition_table_trg_addr = if is_64_bit {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");

            let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG
                .scan_process_range(process, mono_module)?
                .add_signed(-4);

            addr + 0x4 + process.read::<i32>(addr).ok()?
        } else {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("C3 A1 ?? ?? ?? ?? 83 3C ?? 00");

            let addr =
                TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan_process_range(process, mono_module)? + 2;

            process.read::<Address32>(addr).ok()?.into()
        };

        Some(Self {
            is_64_bit,
            version,
            offsets: Offsets::new(version, is_64_bit)?,
            assemblies: assemblies_trg_addr,
            type_info_definition_table: type_info_definition_table_trg_addr,
        })
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image), also called an assembly, is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`get_default_image`](Self::get_default_image) function is a shorthand
    /// for this function that accesses the `Assembly-CSharp` [image](Image).
    pub fn get_image(&self, process: &Process, assembly_name: &str) -> Option<Image> {
        let mut assemblies = self.read_pointer(process, self.assemblies).ok()?;

        let image = loop {
            let mono_assembly = self.read_pointer(process, assemblies).ok()?;
            if mono_assembly.is_null() {
                return None;
            }

            let name_addr = self
                .read_pointer(
                    process,
                    mono_assembly
                        + self.offsets.monoassembly_aname
                        + self.offsets.monoassemblyname_name,
                )
                .ok()?;

            let name = process.read::<ArrayCString<128>>(name_addr).ok()?;

            if name.matches(assembly_name) {
                break self
                    .read_pointer(process, mono_assembly + self.offsets.monoassembly_image)
                    .ok()?;
            }
            assemblies = assemblies + self.size_of_ptr();
        };

        Some(Image { image })
    }

    /// Looks for the `Assembly-CSharp` binary [image](Image) inside the target
    /// process. An [image](Image), also called an assembly, is a .NET DLL that
    /// is loaded by the game. The `Assembly-CSharp` [image](Image) is the main
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
    /// An [image](Image), also called an assembly, is a .NET DLL that is loaded
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
    /// process. An [image](Image), also called an assembly, is a .NET DLL that
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
        if self.is_64_bit {
            8
        } else {
            4
        }
    }

    fn read_pointer(&self, process: &Process, address: Address) -> Result<Address, Error> {
        Ok(if self.is_64_bit {
            process.read::<Address64>(address)?.into()
        } else {
            process.read::<Address32>(address)?.into()
        })
    }
}

/// An image is a .NET DLL that is loaded by the game. The `Assembly-CSharp`
/// image is the main game assembly, and contains all the game logic.
pub struct Image {
    image: Address,
}

impl Image {
    /// Iterates over all [.NET classes](struct@Class) in the image.
    pub fn classes<'a>(
        &self,
        process: &'a Process,
        module: &'a Module,
    ) -> Result<impl DoubleEndedIterator<Item = Class> + 'a, Error> {
        let type_count = process.read::<u32>(self.image + module.offsets.monoimage_typecount)?;

        let metadata_handle = process.read::<i32>(match module.version {
            Version::V2020 => module.read_pointer(
                process,
                self.image + module.offsets.monoimage_metadatahandle,
            )?,
            _ => self.image + module.offsets.monoimage_metadatahandle,
        })? as u64;

        let ptr = module.read_pointer(process, module.type_info_definition_table)?
            + metadata_handle.wrapping_mul(module.size_of_ptr());

        Ok((0..type_count).filter_map(move |i| {
            let class = module
                .read_pointer(process, ptr + (i as u64).wrapping_mul(module.size_of_ptr()))
                .ok()?;

            if !class.is_null() {
                Some(Class { class })
            } else {
                None
            }
        }))
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    pub fn get_class(&self, process: &Process, module: &Module, class_name: &str) -> Option<Class> {
        self.classes(process, module).ok()?.find(|c| {
            let Ok(name_addr) =
                module.read_pointer(process, c.class + module.offsets.monoclass_name)
            else {
                return false;
            };

            let Ok(name) = process.read::<ArrayCString<128>>(name_addr) else {
                return false;
            };

            name.matches(class_name)
        })
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    /// This is the `await`able version of the [`get_class`](Self::get_class)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_get_class(
        &self,
        process: &Process,
        module: &Module,
        class_name: &str,
    ) -> Class {
        retry(|| self.get_class(process, module, class_name)).await
    }
}

/// A .NET class that is part of an [`Image`](Image).
pub struct Class {
    class: Address,
}

impl Class {
    fn fields(
        &self,
        process: &Process,
        module: &Module,
    ) -> impl DoubleEndedIterator<Item = Address> {
        let field_count = process
            .read::<u16>(self.class + module.offsets.monoclass_field_count)
            .unwrap_or_default() as u64;

        let fields = module
            .read_pointer(process, self.class + module.offsets.monoclass_fields)
            .unwrap_or_default();

        let monoclassfield_structsize = module.offsets.monoclassfield_structsize as u64;
        (0..field_count).map(move |i| fields + i.wrapping_mul(monoclassfield_structsize))
    }

    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
    /// table.
    pub fn get_field(&self, process: &Process, module: &Module, field_name: &str) -> Option<u32> {
        let found_field = self.fields(process, module).find(|&field| {
            let Ok(name_addr) =
                module.read_pointer(process, field + module.offsets.monoclassfield_name)
            else {
                return false;
            };

            let Ok(name) = process.read::<ArrayCString<128>>(name_addr) else {
                return false;
            };

            name.matches(field_name)
        })?;

        process
            .read(found_field + module.offsets.monoclassfield_offset)
            .ok()
    }

    /// Tries to find the address of a static instance of the class based on its
    /// field name. This waits until the field is not null.
    pub async fn wait_get_static_instance(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Address {
        let static_table = self.wait_get_static_table(process, module).await;
        let field_offset = self.wait_get_field(process, module, field_name).await;
        let singleton_location = static_table + field_offset;

        retry(|| {
            let addr = module.read_pointer(process, singleton_location).ok()?;

            if addr.is_null() {
                None
            } else {
                Some(addr)
            }
        })
        .await
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields.
    pub fn get_static_table(&self, process: &Process, module: &Module) -> Option<Address> {
        module
            .read_pointer(process, self.class + module.offsets.monoclass_static_fields)
            .ok()
            .filter(|a| !a.is_null())
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        let parent = module
            .read_pointer(process, self.class + module.offsets.monoclass_parent)
            .ok()?;

        Some(Class { class: parent })
    }

    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
    /// table. This is the `await`able version of the
    /// [`get_field`](Self::get_field) function.
    pub async fn wait_get_field(&self, process: &Process, module: &Module, name: &str) -> u32 {
        retry(|| self.get_field(process, module, name)).await
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields. This is the `await`able version of the
    /// [`get_static_table`](Self::get_static_table) function.
    pub async fn wait_get_static_table(&self, process: &Process, module: &Module) -> Address {
        retry(|| self.get_static_table(process, module)).await
    }

    /// Tries to find the parent class. This is the `await`able version of the
    /// [`get_parent`](Self::get_parent) function.
    pub async fn wait_get_parent(&self, process: &Process, module: &Module) -> Class {
        retry(|| self.get_parent(process, module)).await
    }
}

/// An IL2CPP-specific implementation for automatic pointer path resolution
pub struct Pointer<const N: usize> {
    static_table: Option<Address>,
    offsets: [u32; N],
    is_64_bit: bool,
}

impl<const N: usize> Pointer<N> {
    /// Creates a new instance of the Pointer struct
    pub const fn new() -> Self {
        Self {
            static_table: None,
            offsets: [0; N],
            is_64_bit: false,
        }
    }

    /// Tries to resolve the internally stored address of the Mono class
    /// Returns `true` if the pointer path has been resolved, `false` otherwise.
    pub fn try_find(
        &mut self,
        process: &Process,
        module: &Module,
        image: &Image,
        class_name: &str,
        no_of_parents: u32,
        fields: &[&str],
    ) -> bool {
        match self.static_table {
            None => self.force_find(process, module, image, class_name, no_of_parents, fields),
            _ => true,
        }
    }

    /// Tries to resolve the pointer path for the `IL2CPP` class specified, even if a pointer path has been already found.
    ///
    /// Returns `true` if the pointer path has been resolved, `false` otherwise.
    pub fn force_find(
        &mut self,
        process: &Process,
        module: &Module,
        image: &Image,
        class_name: &str,
        no_of_parents: u32,
        fields: &[&str],
    ) -> bool {
        assert!(fields.len() == N);

        // If this function runs, for whatever reason, the address of the static table must be invalidated
        self.static_table = None;

        // Finding the first class in the hierarchy from which we will build our pointer path
        let Some(mut current_class) = image.get_class(process, module, class_name) else {
            return false;
        };

        // Looping through all the needed parent classes, according to the number specified in the function argument
        for _ in 0..no_of_parents {
            let Some(parent_class) = current_class.get_parent(process, module) else {
                return false;
            };
            current_class = parent_class;
        }

        let Some(static_table) = current_class.get_static_table(process, module) else {
            return false;
        };

        let mut new_offsets = [0; N];

        for i in 0..N {
            // Try to parse the offset, passed as a string, as an actual hex or decimal value
            let offset_from_string = {
                let mut temp_val = None;

                if fields[i].starts_with("0x") && fields[i].len() > 2 {
                    if let Some(hex_val) = fields[i].get(2..fields[i].len()) {
                        if let Ok(val) = u32::from_str_radix(hex_val, 16) {
                            temp_val = Some(val)
                        }
                    }
                } else if let Ok(val) = fields[i].parse::<u32>() {
                    temp_val = Some(val)
                }
                temp_val
            };

            // Then we try finding the MonoClassField of interest, which is needed if we only provided the name of the field,
            // and will be needed anyway when looking for the next offset.
            let Some(target_field) = current_class.fields(process, module).find(|&field| {
                if let Some(val) = offset_from_string {
                    process
                        .read::<u32>(field + module.offsets.monoclassfield_offset)
                        .is_ok_and(|value| value == val)
                } else {
                    module
                        .read_pointer(process, field + module.offsets.monoclassfield_name)
                        .is_ok_and(|name_addr|
                            process.read::<ArrayCString<128>>(name_addr)
                            .is_ok_and(|name|
                                name.matches(fields[i])))
                }
            }) else { return false };

            new_offsets[i] = if let Some(val) = offset_from_string {
                val
            } else if let Ok(val) =
                process.read::<u32>(target_field + module.offsets.monoclassfield_offset)
            {
                val
            } else {
                return false;
            };

            // In every iteration of the loop, except the last one, we then need to find the Class address for the next offset
            if i != N - 1 {
                let Ok(r#type) = module.read_pointer(process, target_field + module.size_of_ptr()) else {
                    return false;
                };
                let Ok(type_definition) = module.read_pointer(process, r#type) else {
                    return false;
                };

                let Ok(mut classes) = image.classes(process, module) else {
                    return false;
                };

                let Some(new_class) = classes.find(|c| {
                        module
                            .read_pointer(
                                process,
                                c.class + module.offsets.monoclass_type_definition,
                            )
                            .is_ok_and(|val| val == type_definition)
                    }
                ) else {
                    return false;
                };

                current_class = new_class;
            }
        }

        self.is_64_bit = module.is_64_bit;
        self.offsets = new_offsets;
        self.static_table = Some(static_table);
        true
    }

    /// Reads a value from the cached pointer path
    pub fn read<T: CheckedBitPattern>(&self, process: &Process) -> Result<T, Error> {
        let Some(mut address) = self.static_table else { return Err(Error {}) };
        let depth = self.offsets.len();

        if depth == 0 {
            return process.read(address);
        }

        for offset in 0..depth {
            if offset != depth - 1 {
                address = match self.is_64_bit {
                    true => process
                        .read::<Address64>(address + self.offsets[offset])?
                        .into(),
                    false => process
                        .read::<Address32>(address + self.offsets[offset])?
                        .into(),
                };
            }
        }
        process.read(address + self.offsets[depth - 1])
    }
}

struct Offsets {
    monoassembly_image: u8,
    monoassembly_aname: u8,
    monoassemblyname_name: u8,
    monoimage_typecount: u8,
    monoimage_metadatahandle: u8,
    monoclass_name: u8,
    monoclass_type_definition: u8,
    monoclass_fields: u8,
    monoclass_field_count: u16,
    monoclass_static_fields: u8,
    monoclass_parent: u8,
    monoclassfield_structsize: u8,
    monoclassfield_name: u8,
    monoclassfield_offset: u8,
}

impl Offsets {
    const fn new(version: Version, is_64_bit: bool) -> Option<&'static Self> {
        if !is_64_bit {
            // Il2Cpp on 32-bit is unsupported. Although there are some games
            // using Il2Cpp_base, there are known issues with its offsets.
            return None;
        }

        Some(match version {
            Version::Base => &Self {
                monoassembly_image: 0x0,
                monoassembly_aname: 0x18,
                monoassemblyname_name: 0x0,
                monoimage_typecount: 0x1C,
                monoimage_metadatahandle: 0x18, // MonoImage.typeStart
                monoclass_name: 0x10,
                monoclass_type_definition: 0x68,
                monoclass_fields: 0x80,
                monoclass_field_count: 0x114,
                monoclass_static_fields: 0xB8,
                monoclass_parent: 0x58,
                monoclassfield_structsize: 0x20,
                monoclassfield_name: 0x0,
                monoclassfield_offset: 0x18,
            },
            Version::V2019 => &Self {
                monoassembly_image: 0x0,
                monoassembly_aname: 0x18,
                monoassemblyname_name: 0x0,
                monoimage_typecount: 0x1C,
                monoimage_metadatahandle: 0x18, // MonoImage.typeStart
                monoclass_name: 0x10,
                monoclass_type_definition: 0x68,
                monoclass_fields: 0x80,
                monoclass_field_count: 0x11C,
                monoclass_static_fields: 0xB8,
                monoclass_parent: 0x58,
                monoclassfield_structsize: 0x20,
                monoclassfield_name: 0x0,
                monoclassfield_offset: 0x18,
            },
            Version::V2020 => &Self {
                monoassembly_image: 0x0,
                monoassembly_aname: 0x18,
                monoassemblyname_name: 0x0,
                monoimage_typecount: 0x18,
                monoimage_metadatahandle: 0x28,
                monoclass_name: 0x10,
                monoclass_type_definition: 0x68,
                monoclass_fields: 0x80,
                monoclass_field_count: 0x120,
                monoclass_static_fields: 0xB8,
                monoclass_parent: 0x58,
                monoclassfield_structsize: 0x20,
                monoclassfield_name: 0x0,
                monoclassfield_offset: 0x18,
            },
        })
    }
}

/// The version of IL2CPP that was used for the game.
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    /// The base version.
    Base,
    /// The version used in 2019.
    V2019,
    /// The version used in 2020.
    V2020,
}

fn detect_version(process: &Process) -> Option<Version> {
    let unity_module = process.get_module_range("UnityPlayer.dll").ok()?;

    if pe::MachineType::read(process, unity_module.0)? == pe::MachineType::X86 {
        return Some(Version::Base);
    }

    const SIG: Signature<25> = Signature::new(
        "55 00 6E 00 69 00 74 00 79 00 20 00 56 00 65 00 72 00 73 00 69 00 6F 00 6E",
    );
    const ZERO: u16 = b'0' as u16;
    const NINE: u16 = b'9' as u16;

    let addr = SIG.scan_process_range(process, unity_module)? + 0x1E;
    let version_string = process.read::<[u16; 6]>(addr).ok()?;
    let mut ver = version_string.split(|&b| b == b'.' as u16);

    let version = ver.next()?;
    let mut il2cpp: u32 = 0;
    for &val in version {
        match val {
            ZERO..=NINE => il2cpp = il2cpp * 10 + (val - ZERO) as u32,
            _ => break,
        }
    }

    Some(match il2cpp.cmp(&2019) {
        Ordering::Less => Version::Base,
        Ordering::Equal => Version::V2019,
        Ordering::Greater => {
            const SIG_METADATA: Signature<9> = Signature::new("4C 8B 05 ?? ?? ?? ?? 49 63");
            let game_assembly = process.get_module_range("GameAssembly.dll").ok()?;

            let Some(addr) = SIG_METADATA.scan_process_range(process, game_assembly) else {
                return Some(Version::V2019);
            };
            let addr: Address = addr + 3;
            let addr: Address = addr + 0x4 + process.read::<i32>(addr).ok()?;
            let version = process.read::<i32>(addr + 4).ok()?;

            if version >= 27 {
                Version::V2020
            } else {
                Version::V2019
            }
        }
    })
}

//! Support for attaching to Unity games that are using the IL2CPP backend.

use core::cell::OnceCell;

use crate::{
    deep_pointer::{DeepPointer, DerefType},
    file_format::pe,
    future::retry,
    signature::Signature,
    string::ArrayCString,
    Address, Address32, Address64, Error, Process,
};
use arrayvec::{ArrayString, ArrayVec};
#[cfg(feature = "derive")]
pub use asr_derive::Il2cppClass as Class;
use bytemuck::CheckedBitPattern;

const CSTR: usize = 128;

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
        let mono_module = {
            let address = process.get_module_address("GameAssembly.dll").ok()?;
            let size = pe::read_size_of_image(process, address)? as u64;
            (address, size)
        };

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

        let type_info_definition_table_trg_addr: Address = if is_64_bit {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");

            let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG
                .scan_process_range(process, mono_module)?
                .add_signed(-4);

            process
                .read::<Address64>(addr + 0x4 + process.read::<i32>(addr).ok()?)
                .ok()?
                .into()
        } else {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("C3 A1 ?? ?? ?? ?? 83 3C ?? 00");

            let addr =
                TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan_process_range(process, mono_module)? + 2;

            process
                .read::<Address32>(process.read::<Address32>(addr).ok()?)
                .ok()?
                .into()
        };

        if type_info_definition_table_trg_addr.is_null() {
            None
        } else {
            Some(Self {
                is_64_bit,
                version,
                offsets: Offsets::new(version, is_64_bit)?,
                assemblies: assemblies_trg_addr,
                type_info_definition_table: type_info_definition_table_trg_addr,
            })
        }
    }

    fn assemblies<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Assembly> + 'a {
        let (assemblies, nr_of_assemblies): (Address, u64) = if self.is_64_bit {
            let [first, limit] = process
                .read::<[u64; 2]>(self.assemblies)
                .unwrap_or_default();
            let count = limit.saturating_sub(first) / self.size_of_ptr();
            (Address::new(first), count)
        } else {
            let [first, limit] = process
                .read::<[u32; 2]>(self.assemblies)
                .unwrap_or_default();
            let count = limit.saturating_sub(first) as u64 / self.size_of_ptr();
            (Address::new(first as _), count)
        };

        (0..nr_of_assemblies).filter_map(move |i| {
            Some(Assembly {
                assembly: self
                    .read_pointer(process, assemblies + i.wrapping_mul(self.size_of_ptr()))
                    .ok()?,
            })
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
            })?
            .get_image(process, self)
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
        match self.is_64_bit {
            true => 8,
            false => 4,
        }
    }

    fn read_pointer(&self, process: &Process, address: Address) -> Result<Address, Error> {
        Ok(match self.is_64_bit {
            true => process.read::<Address64>(address)?.into(),
            false => process.read::<Address32>(address)?.into(),
        })
    }
}

struct Assembly {
    assembly: Address,
}

impl Assembly {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process.read(module.read_pointer(
            process,
            self.assembly
                + module.offsets.monoassembly_aname
                + module.offsets.monoassemblyname_name,
        )?)
    }

    fn get_image(&self, process: &Process, module: &Module) -> Option<Image> {
        Some(Image {
            image: module
                .read_pointer(process, self.assembly + module.offsets.monoassembly_image)
                .ok()?,
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
    ) -> impl DoubleEndedIterator<Item = Class> + 'a {
        let type_count = process.read::<u32>(self.image + module.offsets.monoimage_typecount);

        let metadata_ptr = match type_count {
            Ok(_) => match module.version {
                Version::V2020 => module.read_pointer(
                    process,
                    self.image + module.offsets.monoimage_metadatahandle,
                ),
                _ => Ok(self.image + module.offsets.monoimage_metadatahandle),
            },
            _ => Err(Error {}),
        };

        let metadata_handle = match type_count {
            Ok(0) => None,
            Ok(_) => match metadata_ptr {
                Ok(x) => process.read::<u32>(x).ok(),
                _ => None,
            },
            _ => None,
        };

        let ptr = metadata_handle.map(|val| {
            module.type_info_definition_table + (val as u64).wrapping_mul(module.size_of_ptr())
        });

        (0..type_count.unwrap_or_default() as u64).filter_map(move |i| {
            let class = module
                .read_pointer(process, ptr? + i.wrapping_mul(module.size_of_ptr()))
                .ok()?;

            match class.is_null() {
                false => Some(Class { class }),
                true => None,
            }
        })
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    pub fn get_class(&self, process: &Process, module: &Module, class_name: &str) -> Option<Class> {
        self.classes(process, module).find(|class| {
            class
                .get_name::<CSTR>(process, module)
                .is_ok_and(|name| name.matches(class_name))
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

/// A .NET class that is part of an [`Image`].
pub struct Class {
    class: Address,
}

impl Class {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process.read(module.read_pointer(process, self.class + module.offsets.monoclass_name)?)
    }

    fn fields(&self, process: &Process, module: &Module) -> impl DoubleEndedIterator<Item = Field> {
        let field_count = process.read::<u16>(self.class + module.offsets.monoclass_field_count);

        let fields = match field_count {
            Ok(_) => module
                .read_pointer(process, self.class + module.offsets.monoclass_fields)
                .ok(),
            _ => None,
        };

        let monoclassfield_structsize = module.offsets.monoclassfield_structsize as u64;

        (0..field_count.unwrap_or_default() as u64).filter_map(move |i| {
            Some(Field {
                field: fields? + i.wrapping_mul(monoclassfield_structsize),
            })
        })
    }

    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
    /// table.
    pub fn get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Option<u32> {
        self.fields(process, module)
            .find(|field| {
                field
                    .get_name::<CSTR>(process, module)
                    .is_ok_and(|name| name.matches(field_name))
            })?
            .get_offset(process, module)
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
        let field_offset = self
            .wait_get_field_offset(process, module, field_name)
            .await;
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
    /// [`get_field_offset`](Self::get_field_offset) function.
    pub async fn wait_get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        name: &str,
    ) -> u32 {
        retry(|| self.get_field_offset(process, module, name)).await
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

struct Field {
    field: Address,
}

impl Field {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process.read(module.read_pointer(process, self.field + module.offsets.monoclassfield_name)?)
    }

    fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process
            .read(self.field + module.offsets.monoclassfield_offset)
            .ok()
    }
}

/// An IL2CPP-specific implementation for automatic pointer path resolution
pub struct UnityPointer<const CAP: usize> {
    deep_pointer: OnceCell<DeepPointer<CAP>>,
    class_name: ArrayString<CSTR>,
    nr_of_parents: u8,
    fields: ArrayVec<ArrayString<CSTR>, CAP>,
}

impl<const CAP: usize> UnityPointer<CAP> {
    /// Creates a new instance of the Pointer struct
    ///
    /// `CAP` must be higher or equal to the number of offsets defined in `fields`.
    ///
    /// If `CAP` is set to a value lower than the number of the offsets to be dereferenced, this function will ***Panic***
    pub fn new(class_name: &str, nr_of_parents: u8, fields: &[&str]) -> Self {
        assert!(!fields.is_empty() && fields.len() <= CAP);

        Self {
            deep_pointer: OnceCell::new(),
            class_name: ArrayString::from(class_name).unwrap_or_default(),
            nr_of_parents,
            fields: fields
                .iter()
                .map(|&val| ArrayString::from(val).unwrap_or_default())
                .collect(),
        }
    }

    /// Tries to resolve the pointer path for the `IL2CPP` class specified
    fn find_offsets(&self, process: &Process, module: &Module, image: &Image) -> Result<(), Error> {
        // If the pointer path has already been found, there's no need to continue
        if self.deep_pointer.get().is_some() {
            return Ok(());
        }

        let mut current_class = image
            .get_class(process, module, &self.class_name)
            .ok_or(Error {})?;

        for _ in 0..self.nr_of_parents {
            current_class = current_class.get_parent(process, module).ok_or(Error {})?;
        }

        let static_table = current_class
            .get_static_table(process, module)
            .ok_or(Error {})?;
        let mut current_instance = static_table;
        let mut offsets: ArrayVec<u64, CAP> = ArrayVec::new();

        for (i, &field_name) in self.fields.iter().enumerate() {
            // Try to parse the offset, passed as a string, as an actual hex or decimal value
            let offset_from_string = {
                let mut temp_val = None;

                if field_name.starts_with("0x") && field_name.len() > 2 {
                    if let Some(hex_val) = field_name.get(2..field_name.len()) {
                        if let Ok(val) = u32::from_str_radix(hex_val, 16) {
                            temp_val = Some(val)
                        }
                    }
                } else if let Ok(val) = field_name.parse::<u32>() {
                    temp_val = Some(val)
                }
                temp_val
            };

            // Then we try finding the MonoClassField of interest, which is needed if we only provided the name of the field,
            // and will be needed anyway when looking for the next offset.
            let target_field = current_class
                .fields(process, module)
                .find(|field| {
                    if let Some(val) = offset_from_string {
                        field
                            .get_offset(process, module)
                            .is_some_and(|offset| offset == val)
                    } else {
                        field
                            .get_name::<CSTR>(process, module)
                            .is_ok_and(|name| name.matches(field_name.as_ref()))
                    }
                })
                .ok_or(Error {})?;

            let current_offset = match offset_from_string {
                Some(val) => val,
                _ => target_field.get_offset(process, module).ok_or(Error {})?,
            } as _;

            offsets.push(current_offset);

            // In every iteration of the loop, except the last one, we then need to find the Class address for the next offset
            if i != self.fields.len() - 1 {
                current_instance =
                    module.read_pointer(process, current_instance + current_offset)?;

                current_class = Class {
                    class: module.read_pointer(process, current_instance)?,
                };
            }
        }

        let pointer = DeepPointer::new(
            static_table,
            if module.is_64_bit {
                DerefType::Bit64
            } else {
                DerefType::Bit32
            },
            &offsets,
        );
        let _ = self.deep_pointer.set(pointer);
        Ok(())
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<Address, Error> {
        self.find_offsets(process, module, image)?;
        self.deep_pointer
            .get()
            .ok_or(Error {})?
            .deref_offsets(process)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<T, Error> {
        self.find_offsets(process, module, image)?;
        self.deep_pointer.get().ok_or(Error {})?.deref(process)
    }

    /// Recovers the `DeepPointer` struct contained inside this `UnityPointer`,
    /// if the offsets have been found
    pub fn get_deep_pointer(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Option<DeepPointer<CAP>> {
        self.find_offsets(process, module, image).ok()?;
        self.deep_pointer.get().cloned()
    }
}

struct Offsets {
    monoassembly_image: u8,
    monoassembly_aname: u8,
    monoassemblyname_name: u8,
    monoimage_typecount: u8,
    monoimage_metadatahandle: u8,
    monoclass_name: u8,
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
    let unity_module = {
        let address = process.get_module_address("UnityPlayer.dll").ok()?;
        let size = pe::read_size_of_image(process, address)? as u64;
        (address, size)
    };

    if pe::MachineType::read(process, unity_module.0)? == pe::MachineType::X86 {
        return Some(Version::Base);
    }

    const SIG_202X: Signature<6> = Signature::new("00 32 30 32 ?? 2E");
    const SIG_2019: Signature<6> = Signature::new("00 32 30 31 39 2E");

    if SIG_202X.scan_process_range(process, unity_module).is_some() {
        let il2cpp_version = {
            const SIG: Signature<14> = Signature::new("48 2B ?? 48 2B ?? ?? ?? ?? ?? 48 F7 ?? 48");
            let address = process.get_module_address("GameAssembly.dll").ok()?;
            let size = pe::read_size_of_image(process, address)? as u64;

            let ptr = {
                let addr = SIG.scan_process_range(process, (address, size))? + 6;
                addr + 0x4 + process.read::<i32>(addr).ok()?
            };

            let addr = process.read::<Address64>(ptr).ok()?;
            process.read::<u32>(addr + 0x4).ok()?
        };

        Some(if il2cpp_version >= 27 {
            Version::V2020
        } else {
            Version::V2019
        })
    } else if SIG_2019.scan_process_range(process, unity_module).is_some() {
        Some(Version::V2019)
    } else {
        Some(Version::Base)
    }
}

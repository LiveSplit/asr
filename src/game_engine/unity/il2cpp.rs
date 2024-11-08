//! Support for attaching to Unity games that are using the IL2CPP backend.

use core::{
    array,
    cell::RefCell,
    iter::{self, FusedIterator},
};

use crate::{
    file_format::pe,
    future::retry,
    signature::{Signature, SignatureScanner},
    string::ArrayCString,
    Address, Address64, Error, PointerSize, Process,
};

#[cfg(feature = "derive")]
pub use asr_derive::Il2cppClass as Class;
use bytemuck::CheckedBitPattern;

const CSTR: usize = 128;

/// Represents access to a Unity game that is using the IL2CPP backend.
pub struct Module {
    pointer_size: PointerSize,
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

        let pointer_size = match pe::MachineType::read(process, mono_module.0)? {
            pe::MachineType::X86_64 => PointerSize::Bit64,
            _ => PointerSize::Bit32,
        };

        let offsets = Offsets::new(version, pointer_size)?;

        let assemblies = match pointer_size {
            PointerSize::Bit64 => {
                const ASSEMBLIES_TRG_SIG: Signature<12> =
                    Signature::new("48 FF C5 80 3C ?? 00 75 ?? 48 8B 1D");

                let addr = ASSEMBLIES_TRG_SIG.scan(process, mono_module)? + 12;
                addr + 0x4 + process.read::<i32>(addr).ok()?
            }
            PointerSize::Bit32 => {
                const ASSEMBLIES_TRG_SIG: Signature<9> =
                    Signature::new("8A 07 47 84 C0 75 ?? 8B 35");

                let addr = ASSEMBLIES_TRG_SIG.scan(process, mono_module)? + 9;
                process.read_pointer(addr, pointer_size).ok()?
            }
            _ => return None,
        };

        let type_info_definition_table = if pointer_size == PointerSize::Bit64 {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");

            let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG
                .scan(process, mono_module)?
                .add_signed(-4);

            process
                .read_pointer(addr + 0x4 + process.read::<i32>(addr).ok()?, pointer_size)
                .ok()
                .filter(|val| !val.is_null())?
        } else {
            const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> =
                Signature::new("C3 A1 ?? ?? ?? ?? 83 3C ?? 00");

            let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan(process, mono_module)? + 2;

            process
                .read_pointer(process.read_pointer(addr, pointer_size).ok()?, pointer_size)
                .ok()
                .filter(|val| !val.is_null())?
        };

        Some(Self {
            pointer_size,
            version,
            offsets,
            assemblies,
            type_info_definition_table,
        })
    }

    fn assemblies<'a>(
        &'a self,
        process: &'a Process,
    ) -> impl DoubleEndedIterator<Item = Assembly> + 'a {
        let (assemblies, nr_of_assemblies): (Address, u64) = match self.pointer_size {
            PointerSize::Bit64 => {
                let [first, limit] = process
                    .read::<[u64; 2]>(self.assemblies)
                    .unwrap_or_default();
                let count = limit.saturating_sub(first) / self.size_of_ptr();
                (Address::new(first), count)
            }
            _ => {
                let [first, limit] = process
                    .read::<[u32; 2]>(self.assemblies)
                    .unwrap_or_default();
                let count = limit.saturating_sub(first) as u64 / self.size_of_ptr();
                (Address::new(first as _), count)
            }
        };

        (0..nr_of_assemblies).filter_map(move |i| {
            process
                .read_pointer(
                    assemblies + i.wrapping_mul(self.size_of_ptr()),
                    self.pointer_size,
                )
                .ok()
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

#[derive(Copy, Clone)]
struct Assembly {
    assembly: Address,
}

impl Assembly {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.assembly
                    + module.offsets.monoassembly_aname
                    + module.offsets.monoassemblyname_name,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    fn get_image(&self, process: &Process, module: &Module) -> Option<Image> {
        process
            .read_pointer(
                self.assembly + module.offsets.monoassembly_image,
                module.pointer_size,
            )
            .ok()
            .map(|image| Image { image })
    }
}

/// An image is a .NET DLL that is loaded by the game. The `Assembly-CSharp`
/// image is the main game assembly, and contains all the game logic.
#[derive(Copy, Clone)]
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
        let type_count = process
            .read::<u32>(self.image + module.offsets.monoimage_typecount)
            .ok()
            .filter(|val| !val.eq(&0));

        let metadata_ptr = type_count.and_then(|_| match module.version {
            Version::V2020 => process
                .read_pointer(
                    self.image + module.offsets.monoimage_metadatahandle,
                    module.pointer_size,
                )
                .ok(),
            _ => Some(self.image + module.offsets.monoimage_metadatahandle),
        });

        let metadata_handle = type_count
            .and_then(|_| metadata_ptr)
            .and_then(|x| process.read::<u32>(x).ok());

        let ptr = metadata_handle.map(|val| {
            module.type_info_definition_table + (val as u64).wrapping_mul(module.size_of_ptr())
        });

        (0..type_count.unwrap_or_default() as u64).filter_map(move |i| {
            ptr.and_then(|ptr| {
                process
                    .read_pointer(
                        ptr + i.wrapping_mul(module.size_of_ptr()),
                        module.pointer_size,
                    )
                    .ok()
            })
            .filter(|val| !val.is_null())
            .map(|class| Class { class })
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

/// A .NET class that is part of an [`Image`](Image).
#[derive(Copy, Clone)]
pub struct Class {
    class: Address,
}

impl Class {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.class + module.offsets.monoclass_name,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    fn get_name_space<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.class + module.offsets.monoclass_name_space,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    fn fields<'a>(
        &'a self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = Field> + 'a {
        let mut this_class = Some(*self);

        iter::from_fn(move || {
            if this_class?
                .get_name::<CSTR>(process, module)
                .is_ok_and(|name| name.matches("Object"))
                || this_class?
                    .get_name_space::<CSTR>(process, module)
                    .is_ok_and(|name| name.matches("UnityEngine"))
            {
                None
            } else {
                let field_count =
                    process.read::<u16>(this_class?.class + module.offsets.monoclass_field_count);

                let fields = field_count.as_ref().ok().and_then(|_| {
                    process
                        .read_pointer(
                            this_class?.class + module.offsets.monoclass_fields,
                            module.pointer_size,
                        )
                        .ok()
                });

                this_class = this_class?.get_parent(process, module);

                Some(
                    (0..field_count.unwrap_or_default() as u64).filter_map(move |i| {
                        fields.map(|fields| Field {
                            field: fields
                                + i.wrapping_mul(module.offsets.monoclassfield_structsize as u64),
                        })
                    }),
                )
            }
        })
        .fuse()
        .flatten()
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
            })
            .and_then(|field| field.get_offset(process, module))
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
            process
                .read_pointer(singleton_location, module.pointer_size)
                .ok()
                .filter(|val| !val.is_null())
        })
        .await
    }

    fn get_static_table_pointer(&self, module: &Module) -> Address {
        self.class + module.offsets.monoclass_static_fields
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields.
    pub fn get_static_table(&self, process: &Process, module: &Module) -> Option<Address> {
        process
            .read_pointer(self.get_static_table_pointer(module), module.pointer_size)
            .ok()
            .filter(|val| !val.is_null())
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        process
            .read_pointer(
                self.class + module.offsets.monoclass_parent,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
            .map(|class| Class { class })
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

#[derive(Copy, Clone)]
struct Field {
    field: Address,
}

impl Field {
    fn get_name<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        process
            .read_pointer(
                self.field + module.offsets.monoclassfield_name,
                module.pointer_size,
            )
            .and_then(|addr| process.read(addr))
    }

    fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process
            .read(self.field + module.offsets.monoclassfield_offset)
            .ok()
    }
}

/// An IL2CPP-specific implementation for automatic pointer path resolution
#[derive(Clone)]
pub struct UnityPointer<const CAP: usize> {
    cache: RefCell<UnityPointerCache<CAP>>,
    class_name: &'static str,
    nr_of_parents: usize,
    fields: [&'static str; CAP],
    depth: usize,
}

#[derive(Clone, Copy)]
struct UnityPointerCache<const CAP: usize> {
    base_address: Address,
    offsets: [u64; CAP],
    resolved_offsets: usize,
    starting_class: Option<Class>,
}

impl<const CAP: usize> UnityPointer<CAP> {
    /// Creates a new instance of the Pointer struct
    ///
    /// `CAP` should be higher or equal to the number of offsets defined in `fields`.
    ///
    /// If a higher number of offsets is provided, the pointer path will be truncated
    /// according to the value of `CAP`.
    pub fn new(class_name: &'static str, nr_of_parents: usize, fields: &[&'static str]) -> Self {
        let this_fields: [&str; CAP] = {
            let mut iter = fields.iter();
            array::from_fn(|_| iter.next().copied().unwrap_or_default())
        };

        let cache = RefCell::new(UnityPointerCache {
            base_address: Address::default(),
            offsets: [u64::default(); CAP],
            resolved_offsets: usize::default(),
            starting_class: None,
        });

        Self {
            cache,
            class_name,
            nr_of_parents,
            fields: this_fields,
            depth: fields.len().min(CAP),
        }
    }

    /// Tries to resolve the pointer path for the `IL2CPP` class specified
    fn find_offsets(&self, process: &Process, module: &Module, image: &Image) -> Result<(), Error> {
        let mut cache = self.cache.borrow_mut();

        // If the pointer path has already been found, there's no need to continue
        if cache.resolved_offsets == self.depth {
            return Ok(());
        }

        // Logic: the starting class can be recovered with the get_class() function,
        // and parent class can be recovered if needed. However, this is a VERY
        // intensive process because it involves looping through all the main classes
        // in the game. For this reason, once the class is found, we want to store it
        // into the cache, where it can be recovered if this function need to be run again
        // (for example if a previous attempt at pointer path resolution failed)
        let starting_class = match cache.starting_class {
            Some(starting_class) => starting_class,
            _ => {
                let mut current_class = image
                    .get_class(process, module, self.class_name)
                    .ok_or(Error {})?;

                for _ in 0..self.nr_of_parents {
                    current_class = current_class.get_parent(process, module).ok_or(Error {})?;
                }

                cache.starting_class = Some(current_class);
                current_class
            }
        };

        // Recovering the address of the static table is not very CPU intensive,
        // but it might be worth caching it as well
        if cache.base_address.is_null() {
            cache.base_address = starting_class
                .get_static_table(process, module)
                .ok_or(Error {})?;
        };

        // If we already resolved some offsets, we need to traverse them again starting from the base address
        // of the static table in order to recalculate the address of the farthest object we can reach.
        // If no offsets have been resolved yet, we just need to read the base address instead.
        let mut current_object = {
            let mut addr = cache.base_address;
            for &i in &cache.offsets[..cache.resolved_offsets] {
                addr = process.read_pointer(addr + i, module.pointer_size)?;
            }
            addr
        };

        // We keep track of the already resolved offsets in order to skip resolving them again
        for i in cache.resolved_offsets..self.depth {
            let offset_from_string = match self.fields[i].strip_prefix("0x") {
                Some(rem) => u32::from_str_radix(rem, 16).ok(),
                _ => self.fields[i].parse().ok(),
            };

            let current_offset = match offset_from_string {
                Some(offset) => offset as u64,
                _ => {
                    let current_class = match i {
                        0 => starting_class,
                        _ => process
                            .read_pointer(current_object, module.pointer_size)
                            .ok()
                            .filter(|val| !val.is_null())
                            .map(|class| Class { class })
                            .ok_or(Error {})?,
                    };

                    let val = current_class
                        .fields(process, module)
                        .find(|field| {
                            field
                                .get_name::<CSTR>(process, module)
                                .is_ok_and(|name| name.matches(self.fields[i]))
                        })
                        .and_then(|val| val.get_offset(process, module))
                        .ok_or(Error {})? as u64;

                    // Explicitly allowing this clippy because of borrowing rules shenanigans
                    #[allow(clippy::let_and_return)]
                    val
                }
            };

            cache.offsets[i] = current_offset;
            cache.resolved_offsets += 1;

            current_object =
                process.read_pointer(current_object + current_offset, module.pointer_size)?;
        }

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
        let cache = self.cache.borrow();
        let mut address = cache.base_address;
        let (&last, path) = cache.offsets[..self.depth].split_last().ok_or(Error {})?;
        for &offset in path {
            address = process.read_pointer(address + offset, module.pointer_size)?;
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<T, Error> {
        process.read(self.deref_offsets(process, module, image)?)
    }
}

struct Offsets {
    monoassembly_image: u8,
    monoassembly_aname: u8,
    monoassemblyname_name: u8,
    monoimage_typecount: u8,
    monoimage_metadatahandle: u8,
    monoclass_name: u8,
    monoclass_name_space: u8,
    monoclass_fields: u8,
    monoclass_field_count: u16,
    monoclass_static_fields: u8,
    monoclass_parent: u8,
    monoclassfield_structsize: u8,
    monoclassfield_name: u8,
    monoclassfield_offset: u8,
}

impl Offsets {
    const fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => {
                Some(match version {
                    Version::Base => &Self {
                        monoassembly_image: 0x0,
                        monoassembly_aname: 0x18,
                        monoassemblyname_name: 0x0,
                        monoimage_typecount: 0x1C,
                        monoimage_metadatahandle: 0x18, // MonoImage.typeStart
                        monoclass_name: 0x10,
                        monoclass_name_space: 0x18,
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
                        monoclass_name_space: 0x18,
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
                        monoclass_name_space: 0x18,
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
            _ => None,
        }
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

    if SIG_202X.scan(process, unity_module).is_some() {
        let il2cpp_version = {
            const SIG: Signature<14> = Signature::new("48 2B ?? 48 2B ?? ?? ?? ?? ?? 48 F7 ?? 48");
            let address = process.get_module_address("GameAssembly.dll").ok()?;
            let size = pe::read_size_of_image(process, address)? as u64;

            let ptr = {
                let addr = SIG.scan(process, (address, size))? + 6;
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
    } else if SIG_2019.scan(process, unity_module).is_some() {
        Some(Version::V2019)
    } else {
        Some(Version::Base)
    }
}

//! Support for attaching to Unity games that are using the standard Mono
//! backend.

use crate::{
    file_format::pe, future::retry, signature::Signature, string::ArrayCString, Address, Address32,
    Address64, Error, PointerSize, Process,
};
use core::{
    array,
    cell::RefCell,
    iter::{self, FusedIterator},
};

#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;
use bytemuck::CheckedBitPattern;

const CSTR: usize = 128;

/// Represents access to a Unity game that is using the standard Mono backend.
pub struct Module {
    pointer_size: PointerSize,
    version: Version,
    offsets: &'static Offsets,
    assemblies: Address,
}

impl Module {
    /// Tries attaching to a Unity game that is using the standard Mono backend.
    /// This function automatically detects the [Mono version](Version). If you
    /// know the version in advance or it fails detecting it, use
    /// [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let version = detect_version(process)?;
        Self::attach(process, version)
    }

    /// Tries attaching to a Unity game that is using the standard Mono backend
    /// with the [Mono version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let module = ["mono.dll", "mono-2.0-bdwgc.dll"]
            .iter()
            .find_map(|&name| process.get_module_address(name).ok())?;

        let pointer_size = match pe::MachineType::read(process, module)? {
            pe::MachineType::X86_64 => PointerSize::Bit64,
            _ => PointerSize::Bit32,
        };

        let offsets = Offsets::new(version, pointer_size)?;

        let root_domain_function_address = pe::symbols(process, module)
            .find(|symbol| {
                symbol
                    .get_name::<25>(process)
                    .is_ok_and(|name| name.matches("mono_assembly_foreach"))
            })?
            .address;

        let assemblies: Address = match pointer_size {
            PointerSize::Bit64 => {
                const SIG_MONO_64: Signature<3> = Signature::new("48 8B 0D");
                let scan_address: Address = SIG_MONO_64
                    .scan_process_range(process, (root_domain_function_address, 0x100))?
                    + 3;
                scan_address + 0x4 + process.read::<i32>(scan_address).ok()?
            }
            PointerSize::Bit32 => {
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
            pointer_size,
            version,
            offsets,
            assemblies,
        })
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
                self.assembly + module.offsets.monoassembly_aname,
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
            .filter(|val| !val.is_null())
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
    ) -> impl FusedIterator<Item = Class> + 'a {
        let class_cache_size = process
            .read::<i32>(
                self.image
                    + module.offsets.monoimage_class_cache
                    + module.offsets.monointernalhashtable_size,
            )
            .ok()
            .filter(|&val| val != 0);

        let table_addr = class_cache_size.and_then(|_| {
            process
                .read_pointer(
                    self.image
                        + module.offsets.monoimage_class_cache
                        + module.offsets.monointernalhashtable_table,
                    module.pointer_size,
                )
                .ok()
        });

        (0..class_cache_size.unwrap_or_default()).flat_map(move |i| {
            let mut table = table_addr.and_then(|addr| {
                process
                    .read_pointer(
                        addr + (i as u64).wrapping_mul(module.size_of_ptr()),
                        module.pointer_size,
                    )
                    .ok()
            });

            iter::from_fn(move || {
                let class = process.read_pointer(table?, module.pointer_size).ok()?;

                table = process
                    .read_pointer(
                        table? + module.offsets.monoclassdef_next_class_cache,
                        module.pointer_size,
                    )
                    .ok()
                    .filter(|val| !val.is_null());

                Some(Class { class })
            })
            .fuse()
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
                self.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_name,
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
                self.class
                    + module.offsets.monoclassdef_klass
                    + module.offsets.monoclass_name_space,
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
                .ok()?
                .matches("Object")
                || this_class?
                    .get_name_space::<CSTR>(process, module)
                    .ok()?
                    .matches("UnityEngine")
            {
                None
            } else {
                let field_count = process
                    .read::<u32>(this_class?.class + module.offsets.monoclassdef_field_count)
                    .ok()
                    .filter(|val| !val.eq(&0));

                let fields = field_count.and_then(|_| {
                    process
                        .read_pointer(
                            this_class?.class
                                + module.offsets.monoclassdef_klass
                                + module.offsets.monoclass_fields,
                            module.pointer_size,
                        )
                        .ok()
                });

                this_class = this_class?.get_parent(process, module);

                Some(
                    (0..field_count.unwrap_or_default() as u64).filter_map(move |i| {
                        fields.map(|fields| Field {
                            field: fields
                                + i.wrapping_mul(module.offsets.monoclassfieldalignment as u64),
                        })
                    }),
                )
            }
        })
        .fuse()
        .flatten()
    }

    /// Tries to find the offset for a field with the specified name in the class.
    /// If it's a static field, the offset will be from the start of the static
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
            let addr = process
                .read_pointer(singleton_location, module.pointer_size)
                .ok()?;

            if addr.is_null() {
                None
            } else {
                Some(addr)
            }
        })
        .await
    }

    fn get_static_table_pointer(&self, process: &Process, module: &Module) -> Option<Address> {
        let runtime_info = process
            .read_pointer(
                self.class
                    + module.offsets.monoclassdef_klass
                    + module.offsets.monoclass_runtime_info,
                module.pointer_size,
            )
            .ok()?;

        let mut vtables = process
            .read_pointer(
                runtime_info + module.offsets.monoclassruntimeinfo_domain_vtables,
                module.pointer_size,
            )
            .ok()?;

        // Mono V1 behaves differently when it comes to recover the static table
        match module.version {
            Version::V1 => Some(vtables + module.offsets.monoclass_vtable_size),
            _ => {
                vtables = vtables + module.offsets.monovtable_vtable;

                let vtable_size = process
                    .read::<u32>(
                        self.class
                            + module.offsets.monoclassdef_klass
                            + module.offsets.monoclass_vtable_size,
                    )
                    .ok()?;

                Some(vtables + (vtable_size as u64).wrapping_mul(module.size_of_ptr()))
            }
        }
    }

    /// Returns the address of the static table of the class. This contains the
    /// values of all the static fields.
    pub fn get_static_table(&self, process: &Process, module: &Module) -> Option<Address> {
        process
            .read_pointer(
                self.get_static_table_pointer(process, module)?,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        process
            .read_pointer(
                self.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_parent,
                module.pointer_size,
            )
            .ok()
            .filter(|val| !val.is_null())
            .and_then(|parent_addr| process.read_pointer(parent_addr, module.pointer_size).ok())
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
        process.read_pointer_path(
            self.field,
            module.pointer_size,
            &[module.offsets.monoclassfield_name.into(), 0x0],
        )
    }

    fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process
            .read(self.field + module.offsets.monoclassfield_offset)
            .ok()
    }
}

/// A Mono-specific implementation for automatic pointer path resolution
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

    /// Tries to resolve the pointer path for the `Mono` class specified
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
                            .and_then(|addr| process.read_pointer(addr, module.pointer_size).ok())
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

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
        image: &Image,
    ) -> Result<T, Error> {
        self.find_offsets(process, module, image)?;
        let cache = self.cache.borrow();
        process.read_pointer_path(
            cache.base_address,
            module.pointer_size,
            &cache.offsets[..self.depth],
        )
    }
}

struct Offsets {
    monoassembly_aname: u8,
    monoassembly_image: u8,
    monoimage_class_cache: u16,
    monointernalhashtable_table: u8,
    monointernalhashtable_size: u8,
    monoclassdef_next_class_cache: u16,
    monoclassdef_klass: u8,
    monoclass_name: u8,
    monoclass_name_space: u8,
    monoclass_fields: u8,
    monoclassdef_field_count: u16,
    monoclass_runtime_info: u8,
    monoclass_vtable_size: u8,
    monoclass_parent: u8,
    monoclassfield_name: u8,
    monoclassfield_offset: u8,
    monoclassruntimeinfo_domain_vtables: u8,
    monovtable_vtable: u8,
    monoclassfieldalignment: u8,
}

impl Offsets {
    const fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => match version {
                Version::V1 => Some(&Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x58,
                    monoimage_class_cache: 0x3D0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x100,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
                    monoclass_name_space: 0x50,
                    monoclass_fields: 0xA8,
                    monoclassdef_field_count: 0x94,
                    monoclass_runtime_info: 0xF8,
                    monoclass_vtable_size: 0x18, // MonoVtable.data
                    monoclass_parent: 0x30,
                    monoclassfield_name: 0x8,
                    monoclassfield_offset: 0x18,
                    monoclassruntimeinfo_domain_vtables: 0x8,
                    monovtable_vtable: 0x48,
                    monoclassfieldalignment: 0x20,
                }),
                Version::V2 => Some(&Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x60,
                    monoimage_class_cache: 0x4C0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x108,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
                    monoclass_name_space: 0x50,
                    monoclass_fields: 0x98,
                    monoclassdef_field_count: 0x100,
                    monoclass_runtime_info: 0xD0,
                    monoclass_vtable_size: 0x5C,
                    monoclass_parent: 0x30,
                    monoclassfield_name: 0x8,
                    monoclassfield_offset: 0x18,
                    monoclassruntimeinfo_domain_vtables: 0x8,
                    monovtable_vtable: 0x40,
                    monoclassfieldalignment: 0x20,
                }),
                Version::V3 => Some(&Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x60,
                    monoimage_class_cache: 0x4D0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x108,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
                    monoclass_name_space: 0x50,
                    monoclass_fields: 0x98,
                    monoclassdef_field_count: 0x100,
                    monoclass_runtime_info: 0xD0,
                    monoclass_vtable_size: 0x5C,
                    monoclass_parent: 0x30,
                    monoclassfield_name: 0x8,
                    monoclassfield_offset: 0x18,
                    monoclassruntimeinfo_domain_vtables: 0x8,
                    monovtable_vtable: 0x48,
                    monoclassfieldalignment: 0x20,
                }),
            },
            PointerSize::Bit32 => match version {
                Version::V1 => Some(&Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x40,
                    monoimage_class_cache: 0x2A0,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA8,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x30,
                    monoclass_name_space: 0x34,
                    monoclass_fields: 0x74,
                    monoclassdef_field_count: 0x64,
                    monoclass_runtime_info: 0xA4,
                    monoclass_vtable_size: 0xC, // MonoVtable.data
                    monoclass_parent: 0x24,
                    monoclassfield_name: 0x4,
                    monoclassfield_offset: 0xC,
                    monoclassruntimeinfo_domain_vtables: 0x4,
                    monovtable_vtable: 0x28,
                    monoclassfieldalignment: 0x10,
                }),
                Version::V2 => Some(&Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x44,
                    monoimage_class_cache: 0x354,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA8,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x2C,
                    monoclass_name_space: 0x30,
                    monoclass_fields: 0x60,
                    monoclassdef_field_count: 0xA4,
                    monoclass_runtime_info: 0x84,
                    monoclass_vtable_size: 0x38,
                    monoclass_parent: 0x20,
                    monoclassfield_name: 0x4,
                    monoclassfield_offset: 0xC,
                    monoclassruntimeinfo_domain_vtables: 0x4,
                    monovtable_vtable: 0x28,
                    monoclassfieldalignment: 0x10,
                }),
                Version::V3 => Some(&Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x48,
                    monoimage_class_cache: 0x35C,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA0,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x2C,
                    monoclass_name_space: 0x30,
                    monoclass_fields: 0x60,
                    monoclassdef_field_count: 0x9C,
                    monoclass_runtime_info: 0x7C,
                    monoclass_vtable_size: 0x38,
                    monoclass_parent: 0x20,
                    monoclassfield_name: 0x4,
                    monoclassfield_offset: 0xC,
                    monoclassruntimeinfo_domain_vtables: 0x4,
                    monovtable_vtable: 0x2C,
                    monoclassfieldalignment: 0x10,
                }),
            },
            _ => None,
        }
    }
}

/// The version of Mono that was used for the game. These don't correlate to the
/// Mono version numbers.
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    /// Version 1
    V1,
    /// Version 2
    V2,
    /// Version 3
    V3,
}

fn detect_version(process: &Process) -> Option<Version> {
    if process.get_module_address("mono.dll").is_ok() {
        return Some(Version::V1);
    }

    let unity_module = {
        let address = process.get_module_address("UnityPlayer.dll").ok()?;
        let range = pe::read_size_of_image(process, address)? as u64;
        (address, range)
    };

    const SIG_202X: Signature<6> = Signature::new("00 32 30 32 ?? 2E");

    let Some(addr) = SIG_202X.scan_process_range(process, unity_module) else {
        return Some(Version::V2);
    };

    const ZERO: u8 = b'0';
    const NINE: u8 = b'9';

    let version_string = process.read::<[u8; 6]>(addr + 1).ok()?;

    let (before, after) = version_string.split_at(version_string.iter().position(|&x| x == b'.')?);

    let mut unity: u32 = 0;
    for &val in before {
        match val {
            ZERO..=NINE => unity = unity * 10 + (val - ZERO) as u32,
            _ => break,
        }
    }

    let mut unity_minor: u32 = 0;
    for &val in &after[1..] {
        match val {
            ZERO..=NINE => unity_minor = unity_minor * 10 + (val - ZERO) as u32,
            _ => break,
        }
    }

    Some(if (unity == 2021 && unity_minor >= 2) || (unity > 2021) {
        Version::V3
    } else {
        Version::V2
    })
}

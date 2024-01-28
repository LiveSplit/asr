//! Support for attaching to Unity games that are using the standard Mono
//! backend.

use crate::{
    deep_pointer::{DeepPointer, DerefType},
    file_format::pe,
    future::retry,
    signature::Signature,
    string::ArrayCString,
    Address, Address32, Address64, Error, Process,
};
use core::{cell::OnceCell, iter};

use arrayvec::{ArrayString, ArrayVec};
#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;
use bytemuck::CheckedBitPattern;

const CSTR: usize = 128;

/// Represents access to a Unity game that is using the standard Mono backend.
pub struct Module {
    is_64_bit: bool,
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

        let is_64_bit = pe::MachineType::read(process, module)? == pe::MachineType::X86_64;
        let offsets = Offsets::new(version, is_64_bit);

        let root_domain_function_address = pe::symbols(process, module)
            .find(|symbol| {
                symbol
                    .get_name::<25>(process)
                    .is_ok_and(|name| name.matches("mono_assembly_foreach"))
            })?
            .address;

        let assemblies_pointer: Address = match is_64_bit {
            true => {
                const SIG_MONO_64: Signature<3> = Signature::new("48 8B 0D");
                let scan_address: Address = SIG_MONO_64
                    .scan_process_range(process, (root_domain_function_address, 0x100))?
                    + 3;
                scan_address + 0x4 + process.read::<i32>(scan_address).ok()?
            }
            false => {
                const SIG_32_1: Signature<2> = Signature::new("FF 35");
                const SIG_32_2: Signature<2> = Signature::new("8B 0D");

                let ptr = [SIG_32_1, SIG_32_2].iter().find_map(|sig| {
                    sig.scan_process_range(process, (root_domain_function_address, 0x100))
                })? + 2;

                process.read::<Address32>(ptr).ok()?.into()
            }
        };

        let assemblies: Address = match is_64_bit {
            true => process.read::<Address64>(assemblies_pointer).ok()?.into(),
            false => process.read::<Address32>(assemblies_pointer).ok()?.into(),
        };

        if assemblies.is_null() {
            None
        } else {
            Some(Self {
                is_64_bit,
                version,
                offsets,
                assemblies,
            })
        }
    }

    fn assemblies<'a>(&'a self, process: &'a Process) -> impl Iterator<Item = Assembly> + 'a {
        let mut assembly = self.assemblies;
        let mut iter_break = assembly.is_null();
        iter::from_fn(move || {
            if iter_break {
                None
            } else {
                let [data, next_assembly]: [Address; 2] = match self.is_64_bit {
                    true => process
                        .read::<[Address64; 2]>(assembly)
                        .ok()?
                        .map(|item| item.into()),
                    false => process
                        .read::<[Address32; 2]>(assembly)
                        .ok()?
                        .map(|item| item.into()),
                };

                if next_assembly.is_null() {
                    iter_break = true;
                } else {
                    assembly = next_assembly;
                }

                Some(Assembly { assembly: data })
            }
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
        process
            .read(module.read_pointer(process, self.assembly + module.offsets.monoassembly_aname)?)
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
    ) -> impl Iterator<Item = Class> + 'a {
        let class_cache_size = process.read::<i32>(
            self.image
                + module.offsets.monoimage_class_cache
                + module.offsets.monointernalhashtable_size,
        );

        let table_addr = match class_cache_size {
            Ok(_) => module.read_pointer(
                process,
                self.image
                    + module.offsets.monoimage_class_cache
                    + module.offsets.monointernalhashtable_table,
            ),
            _ => Err(Error {}),
        };

        (0..class_cache_size.unwrap_or_default()).flat_map(move |i| {
            let mut table = if let Ok(table_addr) = table_addr {
                module
                    .read_pointer(
                        process,
                        table_addr + (i as u64).wrapping_mul(module.size_of_ptr()),
                    )
                    .ok()
            } else {
                None
            };

            iter::from_fn(move || {
                let class = module.read_pointer(process, table?).ok()?;

                table = module
                    .read_pointer(
                        process,
                        table? + module.offsets.monoclassdef_next_class_cache,
                    )
                    .ok();

                Some(Class { class })
            })
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
        process.read(module.read_pointer(
            process,
            self.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_name,
        )?)
    }

    fn fields(&self, process: &Process, module: &Module) -> impl DoubleEndedIterator<Item = Field> {
        let field_count = process
            .read::<u32>(self.class + module.offsets.monoclassdef_field_count)
            .ok();

        let fields = match field_count {
            Some(_) => module
                .read_pointer(
                    process,
                    self.class
                        + module.offsets.monoclassdef_klass
                        + module.offsets.monoclass_fields,
                )
                .ok(),
            _ => None,
        };

        let monoclassfieldalignment = module.offsets.monoclassfieldalignment as u64;
        (0..field_count.unwrap_or_default()).filter_map(move |i| {
            Some(Field {
                field: fields? + (i as u64).wrapping_mul(monoclassfieldalignment),
            })
        })
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
        let runtime_info = module
            .read_pointer(
                process,
                self.class
                    + module.offsets.monoclassdef_klass
                    + module.offsets.monoclass_runtime_info,
            )
            .ok()?;

        let mut vtables = module
            .read_pointer(
                process,
                runtime_info + module.offsets.monoclassruntimeinfo_domain_vtables,
            )
            .ok()?;

        // Mono V1 behaves differently when it comes to recover the static table
        if module.version == Version::V1 {
            module
                .read_pointer(process, vtables + module.offsets.monoclass_vtable_size)
                .ok()
        } else {
            vtables = vtables + module.offsets.monovtable_vtable;

            let vtable_size = process
                .read::<u32>(
                    self.class
                        + module.offsets.monoclassdef_klass
                        + module.offsets.monoclass_vtable_size,
                )
                .ok()?;

            module
                .read_pointer(
                    process,
                    vtables + (vtable_size as u64).wrapping_mul(module.size_of_ptr()),
                )
                .ok()
        }
    }

    /// Tries to find the parent class.
    pub fn get_parent(&self, process: &Process, module: &Module) -> Option<Class> {
        let parent_addr = module
            .read_pointer(
                process,
                self.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_parent,
            )
            .ok()?;

        Some(Class {
            class: module.read_pointer(process, parent_addr).ok()?,
        })
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
        let name_addr =
            module.read_pointer(process, self.field + module.offsets.monoclassfield_name)?;
        process.read(name_addr)
    }

    fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process
            .read(self.field + module.offsets.monoclassfield_offset)
            .ok()
    }
}

/// A Mono-specific implementation useful for automatic pointer path resolution
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

    /// Tries to resolve the pointer path for the `Mono` class specified
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

            offsets.push(if let Some(val) = offset_from_string {
                val
            } else {
                target_field.get_offset(process, module).ok_or(Error {})?
            } as u64);

            // In every iteration of the loop, except the last one, we then need to find the Class address for the next offset
            if i != self.fields.len() - 1 {
                let vtable = module.read_pointer(process, target_field.field)?;

                current_class = Class {
                    class: module.read_pointer(process, vtable)?,
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
    monoassembly_aname: u8,
    monoassembly_image: u8,
    monoimage_class_cache: u16,
    monointernalhashtable_table: u8,
    monointernalhashtable_size: u8,
    monoclassdef_next_class_cache: u16,
    monoclassdef_klass: u8,
    monoclass_name: u8,
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
    const fn new(version: Version, is_64_bit: bool) -> &'static Self {
        match is_64_bit {
            true => match version {
                Version::V1 => &Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x58,
                    monoimage_class_cache: 0x3D0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x100,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
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
                },
                Version::V2 => &Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x60,
                    monoimage_class_cache: 0x4C0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x108,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
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
                },
                Version::V3 => &Self {
                    monoassembly_aname: 0x10,
                    monoassembly_image: 0x60,
                    monoimage_class_cache: 0x4D0,
                    monointernalhashtable_table: 0x20,
                    monointernalhashtable_size: 0x18,
                    monoclassdef_next_class_cache: 0x108,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x48,
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
                },
            },
            false => match version {
                Version::V1 => &Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x40,
                    monoimage_class_cache: 0x2A0,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA8,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x30,
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
                },
                Version::V2 => &Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x44,
                    monoimage_class_cache: 0x354,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA8,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x2C,
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
                },
                Version::V3 => &Self {
                    monoassembly_aname: 0x8,
                    monoassembly_image: 0x48,
                    monoimage_class_cache: 0x35C,
                    monointernalhashtable_table: 0x14,
                    monointernalhashtable_size: 0xC,
                    monoclassdef_next_class_cache: 0xA0,
                    monoclassdef_klass: 0x0,
                    monoclass_name: 0x2C,
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
                },
            },
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

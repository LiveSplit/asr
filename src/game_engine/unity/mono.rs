//! Support for attaching to Unity games that are using the standard Mono
//! backend.

use crate::{
    file_format::pe, future::retry, signature::Signature, string::ArrayCString, Address, Address32,
    Address64, Error, Process,
};
use core::iter;

#[cfg(feature = "derive")]
pub use asr_derive::MonoClass as Class;
use bytemuck::CheckedBitPattern;

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
        let pe_offsets = PEOffsets::new(is_64_bit);
        let offsets = Offsets::new(version, is_64_bit);

        // Get root domain address: code essentially taken from UnitySpy -
        // See https://github.com/hackf5/unityspy/blob/master/src/HackF5.UnitySpy/AssemblyImageFactory.cs#L123
        let start_index = process.read::<u32>(module + pe_offsets.signature).ok()?;

        let export_directory = process
            .read::<u32>(module + start_index + pe_offsets.export_directory_index_pe)
            .ok()?;

        let number_of_functions = process
            .read::<u32>(module + export_directory + pe_offsets.number_of_functions)
            .ok()?;
        let function_address_array_index = process
            .read::<u32>(module + export_directory + pe_offsets.function_address_array_index)
            .ok()?;
        let function_name_array_index = process
            .read::<u32>(module + export_directory + pe_offsets.function_name_array_index)
            .ok()?;

        let mut root_domain_function_address = Address::NULL;

        for val in 0..number_of_functions {
            let function_name_index = process
                .read::<u32>(module + function_name_array_index + (val as u64).wrapping_mul(4))
                .ok()?;

            if process
                .read::<[u8; 22]>(module + function_name_index)
                .is_ok_and(|function_name| &function_name == b"mono_assembly_foreach\0")
            {
                root_domain_function_address = module
                    + process
                        .read::<u32>(
                            module + function_address_array_index + (val as u64).wrapping_mul(4),
                        )
                        .ok()?;
                break;
            }
        }

        if root_domain_function_address.is_null() {
            return None;
        }

        let assemblies: Address = match is_64_bit {
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

                if let Some(addr) =
                    SIG_32_1.scan_process_range(process, (root_domain_function_address, 0x100))
                {
                    process.read::<Address32>(addr + 2).ok()?.into()
                } else if let Some(addr) =
                    SIG_32_2.scan_process_range(process, (root_domain_function_address, 0x100))
                {
                    process.read::<Address32>(addr + 2).ok()?.into()
                } else {
                    return None;
                }
            }
        };

        Some(Self {
            is_64_bit,
            version,
            offsets,
            assemblies,
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
            let data = self.read_pointer(process, assemblies).ok()?;

            if data.is_null() {
                return None;
            }

            let name_addr = self
                .read_pointer(
                    process,
                    data + self.offsets.monoassembly_aname + self.offsets.monoassemblyname_name,
                )
                .ok()?;

            let name = process.read::<ArrayCString<128>>(name_addr).ok()?;

            if name.matches(assembly_name) {
                break self
                    .read_pointer(process, data + self.offsets.monoassembly_image)
                    .ok()?;
            }

            assemblies = self
                .read_pointer(process, assemblies + self.offsets.glist_next)
                .ok()?;
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
    ) -> Result<impl Iterator<Item = Class> + 'a, Error> {
        let Ok(class_cache_size) = process.read::<i32>(
            self.image
                + module.offsets.monoimage_class_cache
                + module.offsets.monointernalhashtable_size,
        ) else {
            return Err(Error {});
        };

        let table_addr = module.read_pointer(
            process,
            self.image
                + module.offsets.monoimage_class_cache
                + module.offsets.monointernalhashtable_table,
        )?;

        Ok((0..class_cache_size).flat_map(move |i| {
            let mut table = module
                .read_pointer(
                    process,
                    table_addr + (i as u64).wrapping_mul(module.size_of_ptr()),
                )
                .unwrap_or_default();

            iter::from_fn(move || {
                if !table.is_null() {
                    let class = module.read_pointer(process, table).ok()?;
                    table = module
                        .read_pointer(
                            process,
                            table + module.offsets.monoclassdef_next_class_cache,
                        )
                        .unwrap_or_default();
                    Some(Class { class })
                } else {
                    None
                }
            })
        }))
    }

    /// Tries to find the specified [.NET class](struct@Class) in the image.
    pub fn get_class(&self, process: &Process, module: &Module, class_name: &str) -> Option<Class> {
        let mut classes = self.classes(process, module).ok()?;
        classes.find(|c| {
            let Ok(name_addr) = module.read_pointer(
                process,
                c.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_name,
            ) else {
                return false;
            };

            let Ok(name) = process.read::<ArrayCString<128>>(name_addr) else {
                return false;
            };
            if !name.matches(class_name) {
                return false;
            }

            module
                .read_pointer(
                    process,
                    c.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_fields,
                )
                .is_ok_and(|fields| !fields.is_null())
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
    fn fields(&self, process: &Process, module: &Module) -> impl Iterator<Item = Address> {
        let field_count = process
            .read::<u32>(self.class + module.offsets.monoclassdef_field_count)
            .unwrap_or_default();

        let fields = module
            .read_pointer(
                process,
                self.class + module.offsets.monoclassdef_klass + module.offsets.monoclass_fields,
            )
            .unwrap_or_default();

        let monoclassfieldalignment = module.offsets.monoclassfieldalignment as u64;
        (0..field_count).map(move |i| fields + (i as u64).wrapping_mul(monoclassfieldalignment))
    }

    /// Tries to find a field with the specified name in the class. This returns
    /// the offset of the field from the start of an instance of the class. If
    /// it's a static field, the offset will be from the start of the static
    /// table.
    pub fn get_field(&self, process: &Process, module: &Module, field_name: &str) -> Option<u32> {
        let field = self.fields(process, module).find(|&field| {
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
            .read(field + module.offsets.monoclassfield_offset)
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

        let parent = module.read_pointer(process, parent_addr).ok()?;

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

/// A Mono-specific implementation useful for automatic pointer path resolution
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

    /// Tries to resolve the pointer path for the `Mono` class specified, even if a pointer path has been already found.
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
        // If this function runs, for whatever reason, the address of the static table must be invalidated
        self.static_table = None;

        // Finding the first class in thie hierarchy from which we will build our pointer path
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
                let Ok(vtable) = module.read_pointer(process, target_field) else {
                    return false;
                };
                let Ok(new_class) = module.read_pointer(process, vtable) else {
                    return false;
                };

                current_class = Class { class: new_class };
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
    monoassembly_aname: u8,
    monoassembly_image: u8,
    monoassemblyname_name: u8,
    glist_next: u8,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x8,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x8,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x8,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x4,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x4,
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
                    monoassemblyname_name: 0x0,
                    glist_next: 0x4,
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

struct PEOffsets {
    signature: u8,
    export_directory_index_pe: u8,
    number_of_functions: u8,
    function_address_array_index: u8,
    function_name_array_index: u8,
    //function_entry_size: u32,
}

impl PEOffsets {
    const fn new(is_64_bit: bool) -> Self {
        PEOffsets {
            signature: 0x3C,
            export_directory_index_pe: if is_64_bit { 0x88 } else { 0x78 },
            number_of_functions: 0x14,
            function_address_array_index: 0x1C,
            function_name_array_index: 0x20,
            //function_entry_size: 0x4,
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

    const SIG: Signature<25> = Signature::new(
        "55 00 6E 00 69 00 74 00 79 00 20 00 56 00 65 00 72 00 73 00 69 00 6F 00 6E",
    );
    const ZERO: u16 = b'0' as u16;
    const NINE: u16 = b'9' as u16;

    let unity_module = process.get_module_range("UnityPlayer.dll").ok()?;

    let addr = SIG.scan_process_range(process, unity_module)? + 0x1E;
    let version_string = process.read::<[u16; 6]>(addr).ok()?;
    let (before, after) =
        version_string.split_at(version_string.iter().position(|&x| x == b'.' as u16)?);

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

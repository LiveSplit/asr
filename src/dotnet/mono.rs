use crate::{
    file_format::pe, future::retry, signature::Signature, Address, Address32, Address64, Error,
    Process,
};
use core::iter;

/// The main Mono class we use to access the target process' data structure
pub struct MonoModule<'a> {
    process: &'a Process,
    is_64_bit: bool,
    mono_version: MonoVersion,
    mono_offsets: MonoOffsets,
    size_of_ptr: u64,
    assemblies: Address,
}

impl<'a> MonoModule<'a> {
    /// Attaches to the target Mono process and tries to auto-identify
    /// the correct mono struct to use.
    /// 
    /// If this fails, try specifying the version manually 
    /// via the `attach()` function. 
    pub fn try_attach(process: &'a Process) -> Option<Self> {       
        let mono_version = detect_mono_version(process)?;
        Self::attach(process, mono_version)
    }
    
    /// Attaches to the target Mono process.
    ///
    /// This function will return `None` if either:
    /// - The process is not identified
    /// - The wrong Mono version was provided in the call
    /// - The mono assemblies are not found
    pub fn attach(process: &'a Process, version: MonoVersion) -> Option<Self> {
        let mono_version = version;
        const MONO_MODULE_NAMES: [&str; 2] = ["mono.dll", "mono-2.0-bdwgc.dll"];
        let mono_module = MONO_MODULE_NAMES
            .iter()
            .find_map(|&name| process.get_module_address(name).ok())?;

        let is_64_bit =
            pe::MachineType::read(process, mono_module) == Some(pe::MachineType::X86_64);
        let pe_offsets = MonoPEOffsets::new(is_64_bit);
        let mono_offsets = MonoOffsets::new(mono_version, is_64_bit);

        // Get root domain address: code essentially taken from UnitySpy -
        // See https://github.com/hackf5/unityspy/blob/master/src/HackF5.UnitySpy/AssemblyImageFactory.cs#L123
        let start_index = process
            .read::<u32>(mono_module + pe_offsets.signature)
            .ok()?;
        let export_directory_index = start_index + pe_offsets.export_directory_index_pe;

        let export_directory = process
            .read::<u32>(mono_module + export_directory_index)
            .ok()?;

        let number_of_functions = process
            .read::<u32>(mono_module + export_directory + pe_offsets.number_of_functions)
            .ok()?;
        let function_address_array_index = process
            .read::<u32>(mono_module + export_directory + pe_offsets.function_address_array_index)
            .ok()?;
        let function_name_array_index = process
            .read::<u32>(mono_module + export_directory + pe_offsets.function_name_array_index)
            .ok()?;

        let mut root_domain_function_address = Address::NULL;

        for val in 0..number_of_functions {
            let function_name_index = process
                .read::<u32>(mono_module + function_name_array_index + val * 4)
                .ok()?;
            let function_name = process
                .read::<[u8; 21]>(mono_module + function_name_index)
                .ok()?;

            if &function_name == b"mono_assembly_foreach" {
                root_domain_function_address = mono_module
                    + process
                        .read::<u32>(mono_module + function_address_array_index + val * 4)
                        .ok()?;
                break;
            }
        }

        if root_domain_function_address == Address::NULL {
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
            process,
            is_64_bit,
            mono_version: version,
            mono_offsets,
            size_of_ptr: if is_64_bit { 0x8 } else { 0x4 },
            assemblies,
        })
    }

    fn read_pointer(&self, address: Address) -> Result<Address, Error> {
        match self.is_64_bit {
            true => Ok(self.process.read::<Address64>(address)?.into()),
            false => Ok(self.process.read::<Address32>(address)?.into()),
        }
    }

    /// Looks for the specified binary image inside the target process.
    pub fn get_image(&self, assembly_name: &str) -> Option<MonoImage<'_>> {
        let mut assemblies = self.read_pointer(self.assemblies).ok()?;

        let image = loop {
            let data = self.read_pointer(assemblies).ok()?;

            if data.is_null() {
                return None;
            }

            let name_addr = self
                .read_pointer(
                    data + self.mono_offsets.monoassembly_aname
                        + self.mono_offsets.monoassemblyname_name,
                )
                .ok()?;
            let name = self.process.read::<[u8; 128]>(name_addr).ok()?;
            let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];

            if name == assembly_name.as_bytes() {
                break self
                    .read_pointer(data + self.mono_offsets.monoassembly_image)
                    .ok()?;
            }

            assemblies = self
                .read_pointer(assemblies + self.mono_offsets.glist_next)
                .ok()?;
        };

        Some(MonoImage {
            mono_module: self,
            image,
        })
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    pub fn get_default_image(&self) -> Option<MonoImage<'_>> {
        self.get_image("Assembly-CSharp")
    }

    /// Attaches to the target Mono process and internally gets the associated Mono assembly images.
    ///
    /// This function will return `None` is either:
    /// - The process is not identified as a valid IL2CPP game
    /// - The process is 32bit (64bit IL2CPP is not supported by this class)
    /// - The mono assemblies are not found
    ///
    /// This is the `await`able version of the `attach()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_attach(process: &'a Process, version: MonoVersion) -> MonoModule<'_> {
        retry(|| Self::attach(process, version)).await
    }

    /// Looks for the specified binary image inside the target process.
    ///
    /// This is the `await`able version of the `find_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_get_image(&self, assembly_name: &str) -> MonoImage<'_> {
        retry(|| self.get_image(assembly_name)).await
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    ///
    /// This is the `await`able version of the `find_default_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_get_default_image(&self) -> MonoImage<'_> {
        retry(|| self.get_default_image()).await
    }
}

/// A `MonoImage` represents a binary image, like a module, loaded by the target process.
/// When coding autosplitters for Unity games, you want, almost universally, look for the `Assembly-CSharp` image
pub struct MonoImage<'a> {
    mono_module: &'a MonoModule<'a>,
    image: Address,
}

impl MonoImage<'_> {
    fn classes(&self) -> Result<impl Iterator<Item = MonoClass<'_>> + '_, Error> {
        let Ok(class_cache_size) = self.mono_module.process.read::<i32>(self.image
            + self.mono_module.mono_offsets.monoimage_class_cache
            + self.mono_module.mono_offsets.monointernalhashtable_size) else { return Err(Error{}) };

        let table_addr = self
            .mono_module
            .read_pointer(
                self.image
                    + self.mono_module.mono_offsets.monoimage_class_cache
                    + self.mono_module.mono_offsets.monointernalhashtable_table,
            )?;

        let ptr = (0..class_cache_size).flat_map(move |i| {
            let mut table = self
            .mono_module
            .read_pointer(table_addr + i as u64 * self.mono_module.size_of_ptr)
            .unwrap_or_default();

            iter::from_fn(move || {
                if !table.is_null() {
                    let class = self.mono_module.read_pointer(table).ok()?;
                    table = self
                        .mono_module
                        .read_pointer(
                            table
                                + self.mono_module.mono_offsets.monoclassdef_next_class_cache,
                        )
                        .unwrap_or_default();
                    Some(MonoClass {
                        mono_module: self.mono_module,
                        class,
                    })
                } else {
                    None
                }
            })
        });
        Ok(ptr)
    }

    /// Search in memory for the specified `MonoClass`.
    ///
    /// Returns `Option<T>` if successful, `None` otherwise.
    pub fn get_class(&self, class_name: &str) -> Option<MonoClass<'_>> {
        let mut classes = self.classes().ok()?;
        classes.find(|c| {
            if let Ok(name_addr) = self.mono_module.read_pointer(
                c.class
                    + self.mono_module.mono_offsets.monoclassdef_klass
                    + self.mono_module.mono_offsets.monoclass_name,
            ) {
                if let Ok(name) = self.mono_module.process.read::<[u8; 128]>(name_addr) {
                    let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
                    let fields = self
                        .mono_module
                        .read_pointer(
                            c.class
                                + self.mono_module.mono_offsets.monoclassdef_klass
                                + self.mono_module.mono_offsets.monoclass_fields,
                        )
                        .unwrap_or_default();
                    name == class_name.as_bytes() && !fields.is_null()
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    /// Search in memory for the specified `MonoClass`.
    pub async fn wait_get_class(&self, class_name: &str) -> MonoClass<'_> {
        retry(|| self.get_class(class_name)).await
    }
}

/// A generic implementation for any class instantiated by Mono
pub struct MonoClass<'a> {
    mono_module: &'a MonoModule<'a>,
    class: Address,
}

impl MonoClass<'_> {
    fn fields(&self) -> impl Iterator<Item = Address> + '_ {
        let field_count = self
            .mono_module
            .process
            .read::<u32>(self.class + self.mono_module.mono_offsets.monoclassdef_field_count)
            .unwrap_or_default();

        let fields = self
            .mono_module
            .read_pointer(
                self.class
                    + self.mono_module.mono_offsets.monoclassdef_klass
                    + self.mono_module.mono_offsets.monoclass_fields,
            )
            .unwrap_or_default();

        (0..field_count).map(move |i| {
            fields + i as u64 * self.mono_module.mono_offsets.monoclassfieldalignment as u64
        })
    }

    /// Finds the offset of a given field by its name
    pub fn get_field(&self, name: &str) -> Option<u64> {
        let field = self.fields().find(|&field| {
            if let Ok(name_addr) = self
                .mono_module
                .read_pointer(field + self.mono_module.mono_offsets.monoclassfield_name)
            {
                if let Ok(this_name) = self.mono_module.process.read::<[u8; 128]>(name_addr) {
                    let this_name = &this_name[..this_name
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(this_name.len())];
                    this_name == name.as_bytes()
                } else {
                    false
                }
            } else {
                false
            }
        })?;

        if let Ok(offset) = self
            .mono_module
            .process
            .read::<u32>(field + self.mono_module.mono_offsets.monoclassfield_offset)
        {
            Some(offset as _)
        } else {
            None
        }
    }

    /// Returns the address of the static table for the current `MonoClass`
    pub fn get_static_table(&self) -> Option<Address> {
        let runtime_info = self
            .mono_module
            .read_pointer(
                self.class
                    + self.mono_module.mono_offsets.monoclassdef_klass
                    + self.mono_module.mono_offsets.monoclass_runtime_info,
            )
            .ok()?;

        let mut vtables = self
            .mono_module
            .read_pointer(
                runtime_info
                    + self
                        .mono_module
                        .mono_offsets
                        .monoclassruntimeinfo_domain_vtables,
            )
            .ok()?;

        // Mono V1 behaves differently when it comes to recover the static table
        if self.mono_module.mono_version == MonoVersion::MonoV1 {
            self.mono_module
                .read_pointer(vtables + self.mono_module.mono_offsets.monoclass_vtable_size)
                .ok()
        } else {
            vtables = vtables + self.mono_module.mono_offsets.monovtable_vtable;

            let vtable_size = self
                .mono_module
                .process
                .read::<i32>(
                    self.class
                        + self.mono_module.mono_offsets.monoclassdef_klass
                        + self.mono_module.mono_offsets.monoclass_vtable_size,
                )
                .ok()?;

            self.mono_module
                .read_pointer(vtables + vtable_size as u64 * self.mono_module.size_of_ptr)
                .ok()
        }
    }

    /// Finds the parent `MonoClass` of the current class
    pub fn get_parent(&self) -> Option<MonoClass<'_>> {
        let parent_addr = self
            .mono_module
            .read_pointer(
                self.class
                    + self.mono_module.mono_offsets.monoclassdef_klass
                    + self.mono_module.mono_offsets.monoclass_parent,
            )
            .ok()?;
        let parent = self.mono_module.read_pointer(parent_addr).ok()?;
        Some(MonoClass {
            mono_module: self.mono_module,
            class: parent,
        })
    }

    /// Finds the offset of a given field by its name
    pub async fn wait_get_field(&self, name: &str) -> u64 {
        retry(|| self.get_field(name)).await
    }

    /// Returns the address of the static table for the current `MonoClass`
    pub async fn wait_get_static_table(&self) -> Address {
        retry(|| self.get_static_table()).await
    }

    /// Finds the parent `MonoClass` of the current class
    pub async fn wait_get_parent(&self) -> MonoClass<'_> {
        retry(|| self.get_parent()).await
    }
}

struct MonoOffsets {
    monoassembly_aname: u32,
    monoassembly_image: u32,
    monoassemblyname_name: u32,
    glist_next: u32,
    monoimage_class_cache: u32,
    monointernalhashtable_table: u32,
    monointernalhashtable_size: u32,
    monoclassdef_next_class_cache: u32,
    monoclassdef_klass: u32,
    monoclass_name: u32,
    monoclass_fields: u32,
    monoclassdef_field_count: u32,
    monoclass_runtime_info: u32,
    monoclass_vtable_size: u32,
    monoclass_parent: u32,
    monoclassfield_name: u32,
    monoclassfield_offset: u32,
    monoclassruntimeinfo_domain_vtables: u32,
    monovtable_vtable: u32,
    monoclassfieldalignment: u32,
}

impl MonoOffsets {
    const fn new(version: MonoVersion, is_64_bit: bool) -> Self {
        match is_64_bit {
            true => match version {
                MonoVersion::MonoV1 => Self {
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
                MonoVersion::MonoV2 => Self {
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
                MonoVersion::MonoV3 => Self {
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
                MonoVersion::MonoV1 => Self {
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
                MonoVersion::MonoV2 => Self {
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
                MonoVersion::MonoV3 => Self {
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

struct MonoPEOffsets {
    signature: u32,
    export_directory_index_pe: u32,
    number_of_functions: u32,
    function_address_array_index: u32,
    function_name_array_index: u32,
    //function_entry_size: u32,
}

impl MonoPEOffsets {
    const fn new(is_64_bit: bool) -> Self {
        MonoPEOffsets {
            signature: 0x3C,
            export_directory_index_pe: if is_64_bit { 0x88 } else { 0x78 },
            number_of_functions: 0x14,
            function_address_array_index: 0x1C,
            function_name_array_index: 0x20,
            //function_entry_size: 0x4,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Hash, Debug)]
#[allow(missing_docs)]
pub enum MonoVersion {
    MonoV1,
    MonoV2,
    MonoV3,
}


fn detect_mono_version(process: &Process) -> Option<MonoVersion> {
    if process.get_module_address("mono.dll").is_ok() {
        return Some(MonoVersion::MonoV1)
    }

    const SIG: Signature<25> = Signature::new("55 00 6E 00 69 00 74 00 79 00 20 00 56 00 65 00 72 00 73 00 69 00 6F 00 6E");
    const ZERO: u8 = b'0';
    const ONE: u8 = b'1';
    const TWO: u8 = b'2';
    const THREE: u8 = b'3';
    const FOUR: u8 = b'4';
    const FIVE: u8 = b'5';
    const SIX: u8 = b'6';
    const SEVEN: u8 = b'7';
    const EIGHT: u8 = b'8';
    const NINE: u8 = b'9';

    let unity_module = process.get_module_range("UnityPlayer.dll").ok()?;

    let addr = SIG.scan_process_range(process, unity_module)? + 0x1E;
    let version_string = process.read::<[u16; 6]>(addr).ok()?;
    let version_string = version_string.map(|m| m as u8);
    let mut ver = version_string.split(|&b| b == b'.');

    let version = ver.next()?;
    let mut unity: u32 = 0;
    for &val in version {
        match val {
            ZERO => unity *= 10,
            ONE => unity = unity * 10 + 1,
            TWO => unity = unity * 10 + 2,
            THREE => unity = unity * 10 + 3,
            FOUR => unity = unity * 10 + 4,
            FIVE => unity = unity * 10 + 5,
            SIX => unity = unity * 10 + 6,
            SEVEN => unity = unity * 10 + 7,
            EIGHT => unity = unity * 10 + 8,
            NINE => unity = unity * 10 + 9,
            _ => break,
        }
    }

    let version = ver.next()?;
    let mut unity_minor: u32 = 0;
    for &val in version {
        match val {
            ZERO => unity_minor *= 10,
            ONE => unity_minor = unity_minor * 10 + 1,
            TWO => unity_minor = unity_minor * 10 + 2,
            THREE => unity_minor = unity_minor * 10 + 3,
            FOUR => unity_minor = unity_minor * 10 + 4,
            FIVE => unity_minor = unity_minor * 10 + 5,
            SIX => unity_minor = unity_minor * 10 + 6,
            SEVEN => unity_minor = unity_minor * 10 + 7,
            EIGHT => unity_minor = unity_minor * 10 + 8,
            NINE => unity_minor = unity_minor * 10 + 9,
            _ => break,
        }
    }

    if (unity == 2021 && unity_minor >= 2) || (unity > 2021) {
        Some(MonoVersion::MonoV3)
    } else {
        Some(MonoVersion::MonoV2)
    }
}
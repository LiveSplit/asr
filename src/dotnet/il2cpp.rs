use crate::{
    file_format::pe, future::retry, signature::Signature, Address, Address32, Address64, Error,
    Process
};

/// The main class we use to access the target process' data structure
pub struct Il2CppModule<'a> {
    process: &'a Process,
    is_64_bit: bool,
    il2cpp_version: Il2CppVersion,
    il2cpp_offsets: Il2CppOffsets,
    size_of_ptr: u64,
    assemblies: Address,
    type_info_definition_table: Address,
}

impl<'a> Il2CppModule<'a> {
    /// Attaches to the target Mono process.
    ///
    /// This function will return `None` if either:
    /// - The process is not identified
    /// - The wrong Il2Cpp version was provided in the call
    /// - The `GameAssembly.dll` module (common to all Il2Cpp games) is not found
    pub fn attach(process: &'a Process, version: Il2CppVersion) -> Option<Self> {
        let mono_module = process.get_module_range("GameAssembly.dll").ok()?;
        let is_64_bit = pe::MachineType::read(process, mono_module.0)? == pe::MachineType::X86_64;

        let assemblies_trg_addr = match is_64_bit {
            true => {
                const ASSEMBLIES_TRG_SIG: Signature<12> = Signature::new("48 FF C5 80 3C ?? 00 75 ?? 48 8B 1D");            
                let addr = ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 12;
                addr + 0x4 + process.read::<i32>(addr).ok()?
            },
            false => {
                const ASSEMBLIES_TRG_SIG: Signature<9> = Signature::new("8A 07 47 84 C0 75 ?? 8B 35");

                let addr = ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 9;
                process.read::<Address32>(addr).ok()?.into()
            },
        };

        let type_info_definition_table_trg_addr = match is_64_bit {
            true => {
                const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> = Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");
                let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG
                    .scan_process_range(process, mono_module)?
                    .add_signed(-4);
                addr + 0x4 + process.read::<i32>(addr).ok()?    
            },
            false => {
                const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> = Signature::new("C3 A1 ?? ?? ?? ?? 83 3C ?? 00");
                let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan_process_range(process, mono_module)? + 2;
                process.read::<Address32>(addr).ok()?.into()
            },
        };

        Some(Self {
            process,
            is_64_bit,
            il2cpp_version: version,
            il2cpp_offsets: Il2CppOffsets::new(version, is_64_bit)?,
            size_of_ptr: match is_64_bit { true => 0x8, false => 0x4 },
            assemblies: assemblies_trg_addr,
            type_info_definition_table: type_info_definition_table_trg_addr,
        })
    }

    /// Looks for the specified binary image inside the target process.
    pub fn get_image(&self, assembly_name: &str) -> Option<Il2CppImage<'_>> {
        let mut assemblies = self.read_pointer(self.assemblies).ok()?;

        let image = loop {
            let mono_assembly = self.read_pointer(assemblies).ok()?;
            if mono_assembly.is_null() {
                return None;
            }

            let name_addr = self.read_pointer(mono_assembly
                + self.il2cpp_offsets.monoassembly_aname
                + self.il2cpp_offsets.monoassemblyname_name).ok()?;

            let name = self.process.read::<[u8; 128]>(name_addr).ok()?;
            let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
            
            if name == assembly_name.as_bytes() {
                break self.read_pointer(mono_assembly + self.il2cpp_offsets.monoassembly_image).ok()?
            }
            assemblies = assemblies + self.size_of_ptr;
        };

        Some(Il2CppImage {
            mono_module: self,
            image,
        })
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    pub fn get_default_image(&self) -> Option<Il2CppImage<'_>> {
        self.get_image("Assembly-CSharp")
    }

    /// Attaches to the target Mono process.
    ///
    /// This function will return `None` if either:
    /// - The process is not identified
    /// - The wrong Il2Cpp version was provided in the call
    /// - The `GameAssembly.dll` module (common to all Il2Cpp games) is not found
    ///
    /// This is the `await`able version of the `attach()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_attach(process: &'a Process, version: Il2CppVersion) -> Il2CppModule<'_> {
        retry(|| Self::attach(process, version)).await
    }

    /// Looks for the specified binary image inside the target process.
    ///
    /// This is the `await`able version of the `find_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_get_image(&self, assembly_name: &str) -> Il2CppImage<'_> {
        retry(|| self.get_image(assembly_name)).await
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    ///
    /// This is the `await`able version of the `find_default_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_get_default_image(&self) -> Il2CppImage<'_> {
        retry(|| self.get_default_image()).await
    }

    /// Helper function to read a pointer according to the pointer size
    fn read_pointer(&self, address: Address) -> Result<Address, Error> {
        match self.is_64_bit {
            true => Ok(self.process.read::<Address64>(address)?.into()),
            false => Ok(self.process.read::<Address32>(address)?.into()),
        }
    }
}

/// A `MonoImage` represents a binary image, like a module, loaded by the target process.
/// When coding autosplitters for Unity games, you want, almost universally, look for the `Assembly-CSharp` image
pub struct Il2CppImage<'a> {
    mono_module: &'a Il2CppModule<'a>,
    image: Address,
}

impl Il2CppImage<'_> {
    fn classes(&self) -> Result<impl Iterator<Item = Il2CppClass<'_>> + '_, Error> {
        let type_count = self.mono_module.process.read::<u32>(self.image + self.mono_module.il2cpp_offsets.monoimage_typecount)?;

        let metadata_handle = match self.mono_module.il2cpp_version {
            Il2CppVersion::Il2Cpp_2020 => {
                let metadata_handle_addr = self.mono_module.read_pointer(self.image + self.mono_module.il2cpp_offsets.monoimage_metadatahandle)?;
                self.mono_module.process.read::<i32>(metadata_handle_addr)? as u64
            },
            _ => {
                self.mono_module.process.read::<i32>(self.image + self.mono_module.il2cpp_offsets.monoimage_metadatahandle)? as u64
            },
        };


        let ptr = self.mono_module.read_pointer(self.mono_module.type_info_definition_table)?
            + metadata_handle * self.mono_module.size_of_ptr;
        
        Ok( 
            (0..type_count).filter_map(move |i| {
                let class_ptr = self.mono_module.read_pointer(ptr + i as u64 * self.mono_module.size_of_ptr).ok()?;
                if !class_ptr.is_null() {
                    Some(Il2CppClass {
                        mono_module: self.mono_module,
                        class: class_ptr,
                    })
                } else {
                    None
                }
            })
        )
    }

    /// Search in memory for the specified `MonoClass`.
    ///
    /// Returns `Option<T>` if successful, `None` otherwise.
    pub fn get_class(&self, class_name: &str) -> Option<Il2CppClass<'_>> {
        self.classes().ok()?
            .find(|c| {
                let Ok(name_addr) = self.mono_module.read_pointer(c.class + self.mono_module.il2cpp_offsets.monoclass_name) else { return false };
                let Ok(name) = self.mono_module.process.read::<[u8; 128]>(name_addr) else { return false };
                let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
                name == class_name.as_bytes()
            })
    }

    /// Search in memory for the specified `MonoClass`.
    pub async fn wait_get_class(&self, class_name: &str) -> Il2CppClass<'_> {
        retry(|| self.get_class(class_name)).await
    }
}

/// A generic implementation for any class instantiated by Mono
pub struct Il2CppClass<'a> {
    mono_module: &'a Il2CppModule<'a>,
    class: Address,
}

impl Il2CppClass<'_> {
    fn fields(&self) -> impl Iterator<Item = Address> + '_ {
        let field_count = self.mono_module.process.read::<u16>(self.class + self.mono_module.il2cpp_offsets.monoclass_field_count).unwrap_or_default() as u64;
        let fields = self.mono_module.read_pointer(self.class + self.mono_module.il2cpp_offsets.monoclass_fields).unwrap_or_default();

        (0..field_count).map(move |i| fields + i * self.mono_module.il2cpp_offsets.monoclassfield_structsize as u64)
    }

    /// Finds the offset of a given field by its name
    pub fn get_field(&self, field_name: &str) -> Option<u64> {
        let found_field = self.fields()
        .find(|&field| {
            let Ok(name_addr) = self.mono_module.read_pointer(field + self.mono_module.il2cpp_offsets.monoclassfield_name) else { return false };
            let Ok(name) = self.mono_module.process.read::<[u8; 128]>(name_addr) else { return false };
            let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
            name == field_name.as_bytes()
        })?;

        Some(self.mono_module.process.read::<u32>(found_field + self.mono_module.il2cpp_offsets.monoclassfield_offset).ok()? as u64)
    }

    /// Returns the address of the static table for the current `MonoClass`
    pub fn get_static_table(&self) -> Option<Address> {
        let addr = self.mono_module.read_pointer(self.class + self.mono_module.il2cpp_offsets.monoclass_static_fields).ok()?;

        match addr.is_null() {
            true => None,
            false => Some(addr)
        }
    }

    /// Finds the parent `MonoClass` of the current class
    pub fn get_parent(&self) -> Option<Il2CppClass<'_>> {
        let parent = self.mono_module.read_pointer(self.class + self.mono_module.il2cpp_offsets.monoclass_parent).ok()?;

        Some(Il2CppClass {
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
    pub async fn wait_get_parent(&self) -> Il2CppClass<'_> {
        retry(|| self.get_parent()).await
    }
    
}

struct Il2CppOffsets {
    monoassembly_image: u32,
    monoassembly_aname: u32,
    monoassemblyname_name: u32,
    monoimage_typecount: u32,
    monoimage_metadatahandle: u32,
    monoclass_name: u32,
    monoclass_fields: u32,
    monoclass_field_count: u32,
    monoclass_static_fields: u32,
    monoclass_parent: u32,
    monoclassfield_structsize: u32,
    monoclassfield_name: u32,
    monoclassfield_offset: u32,
}

impl Il2CppOffsets {
    const fn new(version: Il2CppVersion, is_64_bit: bool) -> Option<Self> {
        match is_64_bit {
            true => match version {
                Il2CppVersion::Il2Cpp_base => Some(Self {
                    monoassembly_image: 0x0,
                    monoassembly_aname: 0x18,
                    monoassemblyname_name: 0x0,
                    monoimage_typecount: 0x1C,
                    monoimage_metadatahandle: 0x18,  // MonoImage.typeStart
                    monoclass_name: 0x10,
                    monoclass_fields: 0x80,
                    monoclass_field_count: 0x114,
                    monoclass_static_fields: 0xB8,
                    monoclass_parent: 0x58,
                    monoclassfield_structsize: 0x20,
                    monoclassfield_name: 0x0,
                    monoclassfield_offset: 0x18,
                }),
                Il2CppVersion::Il2Cpp_2019 => Some(Self {
                    monoassembly_image: 0x0,
                    monoassembly_aname: 0x18,
                    monoassemblyname_name: 0x0,
                    monoimage_typecount: 0x1C,
                    monoimage_metadatahandle: 0x18,  // MonoImage.typeStart
                    monoclass_name: 0x10,
                    monoclass_fields: 0x80,
                    monoclass_field_count: 0x11C,
                    monoclass_static_fields: 0xB8,
                    monoclass_parent: 0x58,
                    monoclassfield_structsize: 0x20,
                    monoclassfield_name: 0x0,
                    monoclassfield_offset: 0x18,
                }),
                Il2CppVersion::Il2Cpp_2020 => Some(Self {
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
                }),
            },
            false => match version {
                // Il2Cpp on 32-bit is unsupported. Although there are some games
                // using Il2Cpp_base, there are known issues with its offsets.
                Il2CppVersion::Il2Cpp_base => None,
                Il2CppVersion::Il2Cpp_2019 => None,
                Il2CppVersion::Il2Cpp_2020 => None,
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq, Hash, Debug)]
#[allow(missing_docs)]
#[allow(non_camel_case_types)]
pub enum Il2CppVersion {
    Il2Cpp_base,
    Il2Cpp_2019,
    Il2Cpp_2020,
}

use core::{
    fmt,
    marker::PhantomData,
    mem,
};

use crate::{Address, Process, Address64, Error, string::ArrayCString, file_format::pe, future::retry};
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
struct CStr;

#[derive(Copy, Clone)]
#[repr(transparent)]
/// A pointer to any Mono structure
pub struct MonoPtr<T = ()>(Address64, PhantomData<T>);

// SAFETY:
unsafe impl<T: 'static + Copy> Pod for MonoPtr<T> {}
// SAFETY:
unsafe impl<T> Zeroable for MonoPtr<T> {}

impl<T> MonoPtr<T> {
    /// Recovers the memory address the current MonoPtr points to
    pub fn addr(&self) -> Address {
        self.0.into()
    }

    fn is_null(&self) -> bool {
        self.addr().is_null()
    }

    fn offset(&self, count: u64) -> Self {
        Self(self.0 + count * mem::size_of::<T>() as u64, PhantomData)
    }
}

impl<T: Pod> MonoPtr<T> {
    fn read(&self, process: &Process) -> Result<T, Error> {
        process.read(self.addr())
    }

    fn index(&self, process: &Process, idx: usize) -> Result<T, Error> {
        process.read(self.addr() + (idx * mem::size_of::<T>()) as u64)
    }
}

impl MonoPtr<CStr> {
    fn read_str(&self, process: &Process) -> Result<ArrayCString<128>, Error> {
        let addr = self.addr();
        process.read::<ArrayCString<128>>(addr)
    }
}

impl<T> fmt::Debug for MonoPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            f.write_str("NULL")
        } else {
            write!(f, "{:X}", self.0.value())
        }
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct MonoAssembly {
    pub image: MonoPtr<MonoImage>,
    pub token: u32,
    pub referenced_assembly_start: i32,
    pub referenced_assembly_count: i32,
    _padding: [u8; 4],
    pub aname: MonoAssemblyName,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct MonoAssemblyName {
    name: MonoPtr<CStr>,
    culture: MonoPtr<CStr>,
    public_key: MonoPtr,
    hash_alg: u32,
    hash_len: i32,
    flags: u32,
    major: i32,
    minor: i32,
    build: i32,
    revision: i32,
    public_key_token: [u8; 8],
    _padding: [u8; 4],
}

pub struct MonoImageContainer<'a> {
    mono_module: &'a MonoModule<'a>,
    mono_image: MonoImage,
}

impl MonoImageContainer<'_> {
    /// Enumerates the classes inside the current MonoImage
    fn classes(&self) -> Result<impl Iterator<Item = MonoClass> + '_, Error> {
        let ptr = self.mono_module
            .type_info_definition_table
            .offset(self.mono_image.type_start as _);
        Ok((0..self.mono_image.type_count as usize).filter_map(move |i| {
            let class_ptr = ptr.index(self.mono_module.process, i).ok()?;
            if class_ptr.is_null() {
                None
            } else {
                class_ptr.read(self.mono_module.process).ok()
            }
        }))
    }

    /// This function will search in memory for the specified `MonoClass`.
    /// 
    /// Returns `Option<T>` if successful, `None` otherwise.
    pub fn get_class(&self, class_name: &str) -> Option<MonoClassContainer<'_>> {
        let mut classes = self.classes().ok()?;
        classes.find(|c| {
            if let Ok(success) = c.name.read_str(self.mono_module.process) {
                success.as_bytes() == class_name.as_bytes() && !c.fields.is_null()
            } else {
                false
            }
        })
        .map(|m| MonoClassContainer {
            mono_module: self.mono_module,
            mono_class: m
        })
    }

    /// Search in memory for the specified `MonoClass`.
    pub async fn wait_get_class(&self, class_name: &str) -> MonoClassContainer<'_> {
        retry(|| self.get_class(class_name)).await
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
/// A `MonoImage` represents a binary image, like a module, loaded by the target process.
/// When coding autosplitters for Unity games, you want, almost universally, look for the `Assembly-CSharp` image
pub struct MonoImage {
    name: MonoPtr<CStr>,
    name_no_ext: MonoPtr<CStr>,
    assembly: MonoPtr<MonoAssembly>,
    type_start: i32,    // new
    type_count: u32,
    exported_type_start: i32, // new
    exported_type_count: u32,
    custom_attribute_start: i32, // new
    custom_attribute_count: u32,
    entry_point_index: i32, // new
    _padding1: [u8; 4],
    name_to_class_hash_table: MonoPtr,
    token: u32,
    dynamic: u8,
    _padding2: [u8; 3],
}

pub struct MonoClassContainer<'a> {
    mono_module: &'a MonoModule<'a>,
    mono_class: MonoClass,
}

impl MonoClassContainer<'_> {
    /// Returns an iterator for the fields included in the current MonoClass
    fn fields(&self) -> impl Iterator<Item = MonoClassField> + '_ {
        (0..self.mono_class.field_count as usize).flat_map(|i| self.mono_class.fields.index(self.mono_module.process, i))
    }

    /// Returns the name of the current `MonoClass`
    pub fn get_name(&self) -> Result<ArrayCString<128>, Error> {
        self.mono_class.name.read_str(self.mono_module.process)
    }

    /// Finds the offset of a given field by its name
    pub fn get_field(&self, name: &str) -> Option<u64> {
        Some(
            self.fields()
                .find(|field| field.name
                    .read_str(self.mono_module.process)
                    .unwrap_or_default()
                    .as_bytes() == name.as_bytes())?
                .offset as _,
        )
    }

    /// Returns the address of the static table for the current `MonoClass`
    pub fn get_static_table(&self) -> Option<Address> {
        let addr = self.mono_class.static_fields.addr();
        if addr.is_null() {
            None
        } else {
            Some(addr)
        }
    }

    /// Finds the parent `MonoClass` of the current class
    pub fn get_parent(&self) -> Option<MonoClassContainer<'_>> {
        let parent = self.mono_class.parent.read(self.mono_module.process).ok()?;
        Some(
            MonoClassContainer {
                mono_module: self.mono_module,
                mono_class: parent
            }
        )
    }

    /// Returns the name of the current `MonoClass`
    pub async fn wait_get_name(&self) -> ArrayCString<128> {
        retry(|| self.get_name()).await
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
    pub async fn wait_get_parent(&self) -> MonoClassContainer<'_> {
        retry(|| self.get_parent()).await
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
/// A generic implementation for any class instantiated by Mono
pub struct MonoClass {
    image: MonoPtr<MonoImage>,
    gc_desc: MonoPtr,
    name: MonoPtr<CStr>,
    name_space: MonoPtr<CStr>,
    byval_arg: MonoType,
    this_arg: MonoType,
    element_class: MonoPtr<MonoClass>,
    cast_class: MonoPtr<MonoClass>,
    declaring_type: MonoPtr<MonoClass>,
    parent: MonoPtr<MonoClass>,
    generic_class: MonoPtr, // <MonoGenericClass>,
    type_definition: MonoPtr,
    interop_data: MonoPtr,
    klass: MonoPtr<MonoClass>,
    fields: MonoPtr<MonoClassField>,
    events: MonoPtr,       // <EventInfo>
    properties: MonoPtr,   // <PropertyInfo>
    methods: MonoPtr<MonoPtr>, // <MethodInfo>
    nested_types: MonoPtr<MonoPtr<MonoClass>>,
    implemented_interfaces: MonoPtr<MonoPtr<MonoClass>>,
    interface_offsets: MonoPtr,
    static_fields: MonoPtr,
    rgctx_data: MonoPtr,
    type_hierarchy: MonoPtr<MonoPtr<MonoClass>>,
    unity_user_data: MonoPtr,
    initialization_exception_gc_handle: u32,
    cctor_started: u32,
    cctor_finished: u32,
    _padding1: [u8; 4],
    cctor_thread: u64,
    generic_container_index: i32,
    instance_size: u32,
    actual_size: u32,
    element_size: u32,
    native_size: i32,
    static_fields_size: u32,
    thread_static_fields_size: u32,
    thread_static_fields_offset: i32,
    flags: u32,
    token: u32,
    method_count: u16,
    property_count: u16,
    field_count: u16,
    event_count: u16,
    nested_type_count: u16,
    vtable_count: u16,
    interfaces_count: u16,
    interface_offsets_count: u16,
    type_hierarchy_depth: u8,
    generic_recursion_depth: u8,
    rank: u8,
    minimum_alignment: u8,
    natural_aligment: u8,
    packing_size: u8,
    more_flags: [u8; 2],
    // initialized_and_no_error: u8:1,
    // valuetype: u8:1,
    // initialized: u8:1,
    // enumtype: u8:1,
    // is_generic: u8:1,
    // has_references: u8:1,
    // init_pending: u8:1,
    // size_inited: u8:1,

    // has_finalize: u8:1,
    // has_cctor: u8:1,
    // is_blittable: u8:1,
    // is_import_or_windows_runtime: u8:1,
    // is_vtable_initialized: u8:1,
    // has_initialization_error: u8:1,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MonoType {
    data: MonoPtr,
    attrs: u16,
    r#type: u8,
    flags: u8,
    _padding: [u8; 4],
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MonoClassField {
    name: MonoPtr<CStr>,
    r#type: MonoPtr<MonoType>,
    parent: MonoPtr<MonoClass>,
    offset: i32,
    token: u32,
}

/// The main Mono class we use to access the target process' data structure
pub struct MonoModule<'a> {
    process: &'a Process,
    assemblies: MonoPtr<MonoPtr<MonoAssembly>>,
    type_info_definition_table: MonoPtr<MonoPtr<MonoClass>>,
}

impl<'a> MonoModule<'a> {
    /// Attaches to the target Mono process and internally gets the associated Mono assembly images.
    /// 
    /// This function will return `None` is either:
    /// - The process is not identified as a valid IL2CPP game
    /// - The process is 32bit (64bit IL2CPP is not supported by this class)
    /// - The mono assemblies are not found
    pub fn attach(process: &'a Process) -> Option<Self> {
        let mono_module = process.get_module_range("GameAssembly.dll").ok()?;

        let ptr_size = pe::MachineType::read(process, mono_module.0)?;
        if ptr_size != pe::MachineType::X86_64 {
            crate::print_message("Class manager is supported only on 64-bit IL2CPP.");
            return None
        }

        let addr = super::ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 12;
        let assemblies_trg_addr = addr + 0x4 + process.read::<i32>(addr).ok()?;
        let assemblies: MonoPtr<MonoPtr<MonoAssembly>> = process.read(assemblies_trg_addr).ok()?;


        let addr = super::TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan_process_range(process, mono_module)?.add_signed(-4);
        let type_info_definition_table_trg_addr = addr + 0x4 + process.read::<i32>(addr).ok()?;
        let type_info_definition_table: MonoPtr<MonoPtr<MonoClass>> = process.read(type_info_definition_table_trg_addr).ok()?;

        Some(Self {
            process,
            assemblies,
            type_info_definition_table,
        })
    }

    /// Looks for the specified binary image inside the target process.
    pub fn find_image(&self, assembly_name: &str) -> Result<MonoImageContainer<'_>, Error> {
        let mut assemblies = self.assemblies;

        let image = loop {
            let ptr = assemblies.read(self.process)?;
            if ptr.is_null() {
                return Err(Error {})
            }

            let mono_assembly = ptr.read(self.process)?;

            if mono_assembly
                .aname
                .name
                .read_str(self.process)?
                .as_bytes() == assembly_name.as_bytes()
            {
                break mono_assembly.image.read(self.process)?;
            }
            assemblies = assemblies.offset(1);
        };
        Ok(MonoImageContainer { mono_module: self, mono_image: image })
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    pub fn find_default_image(&self) -> Result<MonoImageContainer<'_>, Error> {
        self.find_image("Assembly-CSharp")
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
    pub async fn wait_attach(process: &'a Process) -> MonoModule<'_> {
        retry(|| Self::attach(process)).await
    }

    /// Looks for the specified binary image inside the target process.
    /// 
    /// This is the `await`able version of the `find_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_find_image(&self, assembly_name: &str) -> MonoImageContainer<'_> {
        retry(|| self.find_image(assembly_name)).await
    }

    /// Looks for the `Assembly-CSharp` binary image inside the target process
    /// 
    /// This is the `await`able version of the `find_default_image()` function,
    /// yielding back to the runtime between each try.
    pub async fn wait_find_default_image(&self) -> MonoImageContainer<'_> {
        retry(|| self.find_default_image()).await
    }
}

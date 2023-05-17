use core::{
    fmt,
    marker::PhantomData,
    mem,
};

use crate::{signature::Signature, Address, Process, Address64, Error, string::ArrayCString, file_format::pe};
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
struct CStr;

#[derive(Copy, Clone)]
#[repr(transparent)]
/// A pointer to any Mono structure
pub struct MonoPtr<T = ()>(Address64, PhantomData<T>);

unsafe impl<T: 'static + Copy> Pod for MonoPtr<T> {}
unsafe impl<T> Zeroable for MonoPtr<T> {}

impl<T> MonoPtr<T> {
    /// Recovers the memory address the current MonoPtr points to
    pub fn addr(&self) -> Address {
        self.0.into()
    }

    fn is_null(&self) -> bool {
        self.addr().is_null()
    }

    const fn cast<U>(&self) -> MonoPtr<U> {
        MonoPtr(self.0, PhantomData)
    }

    fn byte_offset(&self, bytes: u64) -> Self {
        Self(self.0 + bytes, PhantomData)
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

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
/// A `MonoImage` represents a binary image, like a module, loaded by the target process.
/// When coding autosplitters for Unity games, you want, almost universally, look for the `Assembly-CSharp` image
pub struct MonoImage {
    name: MonoPtr<CStr>,
    name_no_ext: MonoPtr<CStr>,
    assembly: MonoPtr<MonoAssembly>,
    type_count: u32,
    exported_type_count: u32,
    custom_attribute_count: u32,
    _padding1: [u8; 4],
    metadata_handle: MonoPtr<i32>,
    name_to_class_hash_table: MonoPtr,
    code_gen_module: MonoPtr,
    token: u32,
    dynamic: u8,
    _padding2: [u8; 3],
}

impl MonoImage {
    /// Enumerates the classes inside the current MonoImage
    fn classes<'a>(
        &'a self,
        process: &'a Process,
        mono_module: &MonoModule,
    ) -> Result<impl Iterator<Item = MonoClass> + 'a, Error> {
        let ptr = mono_module
            .type_info_definition_table
            .offset(self.metadata_handle.read(process)? as _);
        Ok((0..self.type_count as usize).filter_map(move |i| {
            let class_ptr = ptr.index(process, i).ok()?;
            if class_ptr.is_null() {
                None
            } else {
                class_ptr.read(process).ok()
            }
        }))
    }

    /// This function will search in memory for the specified `MonoClass`.
    /// 
    /// Returns `Option<MonoClass>` if successful, `None` otherwise.
    pub fn get_class(&self, process: &Process, mono_module: &MonoModule, class_name: &str) -> Option<MonoClass> {
        let mut classes = self.classes(process, mono_module).ok()?;
        classes.find(|c| {
            if let Ok(success) = c.get_name(process) {
                success.as_bytes() == class_name.as_bytes() && !c.fields.is_null()
            } else {
                false
            }
        })
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
    type_metadata_handle: MonoPtr,
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
    generic_container_handle: MonoPtr,
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
    _padding2: [u8; 4],
}

impl MonoClass {
    /// Returns an iterator for the fields included in the current MonoClass
    fn fields<'a>(&'a self, process: &'a Process) -> impl Iterator<Item = MonoClassField> + 'a {
        (0..self.field_count as usize).flat_map(|i| self.fields.index(process, i))
    }

    /// Finds the offset of a given field by its name
    pub fn get_field(&self, process: &Process, name: &str) -> Option<i32> {
        Some(
            self.fields(process)
                .find(|field| field.get_name(process).unwrap_or_default().as_bytes() == name.as_bytes())?
                .offset,
        )
    }

    /// Returns the address of the static table for the current `MonoClass`
    pub fn get_static_table(&self) -> Option<Address> {
        let addr = self.static_fields.addr();
        if addr.is_null() {
            None
        } else {
            Some(addr)
        }
    }

    /// Finds the parent `MonoClass` of the current class
    pub fn get_parent(&self, process: &Process) -> Option<MonoClass> {
        self.parent.read(process).ok()
    }

    /// Returns the name of the current `MonoClass`
    pub fn get_name(&self, process: &Process) -> Result<ArrayCString<128>, Error> {
        self.name.read_str(process)
    }
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

impl MonoClassField {
    /// Returns a string containing the name of the current Mono field
    fn get_name(&self, process: &Process) -> Result<ArrayCString<128>, Error> {
        self.name.read_str(process)
    }
}

pub struct MonoModule {
    assemblies: MonoPtr<MonoPtr<MonoAssembly>>,
    type_info_definition_table: MonoPtr<MonoPtr<MonoClass>>,
}


impl MonoModule {
    pub fn attach(process: &Process) -> Option<Self> {
        const ASSEMBLIES_TRG_SIG: Signature<12> = Signature::new("48 FF C5 80 3C ?? 00 75 ?? 48 8B 1D");
        const TYPE_INFO_DEFINITION_TABLE_TRG_SIG: Signature<10> = Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");

        let mono_module = process.get_module_range("GameAssembly.dll").ok()?;

        let ptr_size = pe::MachineType::read(process, mono_module.0)?;
        if ptr_size != pe::MachineType::X86_64 {
            crate::print_message("Class manager is supported only on 64-bit IL2CPP.");
            return None
        }

        let addr = ASSEMBLIES_TRG_SIG.scan_process_range(process, mono_module)? + 12;
        let assemblies_trg_addr = addr + 0x4 + process.read::<i32>(addr).ok()?;
        let assemblies: MonoPtr<MonoPtr<MonoAssembly>> = process.read(assemblies_trg_addr).ok()?;


        let addr = TYPE_INFO_DEFINITION_TABLE_TRG_SIG.scan_process_range(process, mono_module)?.add_signed(-4);
        let type_info_definition_table_trg_addr = addr + 0x4 + process.read::<i32>(addr).ok()?;
        let type_info_definition_table: MonoPtr<MonoPtr<MonoClass>> = process.read(type_info_definition_table_trg_addr).ok()?;

        Some(Self {
            assemblies,
            type_info_definition_table,
        })
    }

    /// Looks for the specified binary image inside the target process.
    pub fn find_image(&self, process: &Process, assembly_name: &str) -> Result<MonoImage, Error> {
        let mut assemblies = self.assemblies;

        let image = loop {
            let ptr = assemblies.read(process)?;
            if ptr.is_null() {
                return Err(Error {})
            }

            let mono_assembly = ptr.read(process)?;

            if mono_assembly
                .aname
                .name
                .read_str(process)?
                .as_bytes() == assembly_name.as_bytes()
            {
                break mono_assembly.image.read(process)?;
            }
            assemblies = assemblies.offset(1);
        };
        Ok(image)
    }

    /// Looks for the Assembly-CSharp binary image inside the target process
    pub fn find_default_image(&self, process: &Process) -> Result<MonoImage, Error> {
        let mut assemblies = self.assemblies;

        let image = loop {
            let ptr = assemblies.read(process)?;
            if ptr.is_null() {
                return Err(Error {})
            }

            let mono_assembly = ptr.read(process)?;

            if mono_assembly
                .aname
                .name
                .read_str(process)?
                .as_bytes() == "Assembly-CSharp".as_bytes()
            {
                break mono_assembly.image.read(process)?;
            }
            assemblies = assemblies.offset(1);
        };
        Ok(image)
    }
}

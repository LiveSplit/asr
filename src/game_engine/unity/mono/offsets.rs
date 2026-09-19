use super::Library;
use crate::PointerSize;

/// A complete measured Mono runtime layout.
///
/// Profiles are deliberately exhaustive. If a future ASR release needs more
/// layout information, custom profiles must provide it rather than silently
/// inheriting an offset that may be wrong. Built-in measured profiles are
/// available in [`profiles`](super::profiles).
///
/// A custom profile lists the complete layout. Every field is written out
/// intentionally: using struct-update syntax would opt the custom profile in
/// to silently inheriting fields added to the source profile in the future.
///
/// ```
/// use asr::{
///     game_engine::unity::mono::{
///         AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets,
///         HashTableOffsets, ImageOffsets, Library, MonoVTableOffsets, Profile,
///         TypeOffsets,
///     },
///     PointerSize,
/// };
///
/// const CUSTOM_PROFILE: Profile = Profile {
///     pointer_size: PointerSize::Bit64,
///     library: Library::MonoBdwgc,
///     assembly: AssemblyOffsets {
///         aname: None,
///         image: 0x60,
///     },
///     image: ImageOffsets {
///         assembly_name: Some(0x30),
///         class_cache: 0x4d0,
///     },
///     hash_table: HashTableOffsets {
///         size: 0x18,
///         table: 0x20,
///     },
///     class: ClassOffsets {
///         class_kind: Some(0x1b),
///         instance_size: Some(0x1c),
///         parent: 0x30,
///         nested_in: Some(0x38),
///         name: 0x48,
///         namespace: 0x50,
///         vtable_size: 0x5c,
///         fields: 0x98,
///         runtime_info: 0xd0,
///         field_count: 0x100,
///         next_class_cache: 0x108,
///     },
///     generic: GenericOffsets {
///         generic_class: Some(0xf0),
///         container_class: Some(0x0),
///     },
///     type_words: TypeOffsets {
///         data: Some(0x0),
///         kind: Some(0xa),
///     },
///     field: FieldInfoOffsets {
///         type_: Some(0x0),
///         name: 0x8,
///         offset: 0x18,
///         alignment: 0x20,
///     },
///     v_table: MonoVTableOffsets { vtable: 0x48 },
/// };
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The pointer width this profile was measured for.
    pub pointer_size: PointerSize,
    /// The runtime library this profile was measured for.
    pub library: Library,
    /// Offsets within `MonoAssembly`.
    pub assembly: AssemblyOffsets,
    /// Offsets within `MonoImage`.
    pub image: ImageOffsets,
    /// Offsets within `MonoInternalHashTable`.
    pub hash_table: HashTableOffsets,
    /// Offsets within `MonoClass` and `MonoClassDef`.
    pub class: ClassOffsets,
    /// Offsets within `MonoClassGenericInst` and `MonoGenericClass`.
    pub generic: GenericOffsets,
    /// Offsets within `MonoType`.
    pub type_words: TypeOffsets,
    /// Offsets and stride of `MonoClassField`.
    pub field: FieldInfoOffsets,
    /// Offsets within `MonoVTable`.
    pub v_table: MonoVTableOffsets,
}

/// Offsets within `MonoAssembly`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssemblyOffsets {
    /// The assembly-name pointer, when the name is stored on the assembly.
    /// Otherwise [`ImageOffsets::assembly_name`] locates it.
    pub aname: Option<u8>,
    /// The image pointer.
    pub image: u8,
}

/// Offsets within `MonoImage`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ImageOffsets {
    /// The assembly-name pointer, when the name is stored on the image.
    /// Otherwise [`AssemblyOffsets::aname`] locates it.
    pub assembly_name: Option<u8>,
    /// The class cache, a `MonoInternalHashTable` embedded in the image.
    pub class_cache: u16,
}

/// Offsets within `MonoInternalHashTable`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HashTableOffsets {
    /// The number of buckets.
    pub size: u8,
    /// The pointer to the bucket array.
    pub table: u8,
}

/// Offsets within `MonoClass` and `MonoClassDef`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ClassOffsets {
    /// The byte whose low bits say what kind of class it is. The old
    /// runtime has none.
    pub class_kind: Option<u16>,
    /// What one instance occupies, boxed header included.
    pub instance_size: Option<u16>,
    /// The parent class pointer.
    pub parent: u8,
    /// Where a class keeps the one it is nested in.
    pub nested_in: Option<u16>,
    /// The name pointer.
    pub name: u8,
    /// The namespace pointer.
    pub namespace: u8,
    /// The number of method pointers in the vtable. On the old runtime,
    /// [`Library::Mono`], this is `MonoVTable::data` instead, where that
    /// runtime keeps the statics.
    pub vtable_size: u8,
    /// The pointer to the field array.
    pub fields: u8,
    /// The runtime info pointer, which leads to the vtable.
    pub runtime_info: u16,
    /// The field count, in `MonoClassDef`.
    pub field_count: u16,
    /// The next class in the same bucket of the class cache.
    pub next_class_cache: u16,
}

/// Offsets within `MonoType`: the data pointer and the element kind byte.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeOffsets {
    /// The data pointer, which leads to the class for a class type.
    pub data: Option<u16>,
    /// The element kind byte.
    pub kind: Option<u16>,
}

/// Offsets within `MonoClassGenericInst` and `MonoGenericClass`. A generic
/// instance keeps a descriptor whose container is the generic definition
/// the instance was made from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenericOffsets {
    /// The descriptor pointer in `MonoClassGenericInst`.
    pub generic_class: Option<u16>,
    /// The container class pointer in `MonoGenericClass`.
    pub container_class: Option<u16>,
}

/// Offsets and stride of `MonoClassField`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FieldInfoOffsets {
    /// Where a field keeps its `MonoType`.
    pub type_: Option<u16>,
    /// The name pointer.
    pub name: u8,
    /// The field offset.
    pub offset: u8,
    /// The stride of the field array.
    pub alignment: u8,
}

/// Offsets within `MonoVTable`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MonoVTableOffsets {
    /// Where the method pointers start. 0 on the old runtime, which keeps
    /// its statics in the data slot instead.
    pub vtable: u8,
}

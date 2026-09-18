use crate::PointerSize;

/// A complete measured IL2CPP runtime layout.
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
///     game_engine::unity::il2cpp::{
///         AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets,
///         ImageOffsets, Profile, TypeOffsets, TypeStart,
///     },
///     PointerSize,
/// };
///
/// const CUSTOM_PROFILE: Profile = Profile {
///     pointer_size: PointerSize::Bit64,
///     assembly: AssemblyOffsets {
///         image: 0x0,
///         name: None,
///     },
///     image: ImageOffsets {
///         assembly_name: Some(0x8),
///         type_count: 0x18,
///         type_start: TypeStart::Handle(0x28),
///     },
///     class: ClassOffsets {
///         name: 0x10,
///         namespace: 0x18,
///         declaring_type: Some(0x50),
///         parent: 0x58,
///         fields: 0x80,
///         static_fields: 0xb8,
///         instance_size: Some(0xf8),
///         field_count: 0x124,
///     },
///     generic: GenericOffsets {
///         cached_class: Some(0x18),
///     },
///     type_: TypeOffsets {
///         data: Some(0x0),
///         kind: Some(0xa),
///     },
///     field: FieldInfoOffsets {
///         name: 0x0,
///         type_: Some(0x8),
///         offset: 0x18,
///         size: 0x20,
///     },
/// };
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The pointer width this profile was measured for.
    pub pointer_size: PointerSize,
    /// Offsets within `Il2CppAssembly`.
    pub assembly: AssemblyOffsets,
    /// Offsets within `Il2CppImage`.
    pub image: ImageOffsets,
    /// Offsets within `Il2CppClass`.
    pub class: ClassOffsets,
    /// Offsets within `Il2CppGenericClass`.
    pub generic: GenericOffsets,
    /// Offsets within `Il2CppType`.
    pub type_: TypeOffsets,
    /// Offsets and size of `FieldInfo`.
    pub field: FieldInfoOffsets,
}

/// Offsets within `Il2CppAssembly`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssemblyOffsets {
    /// The image pointer.
    pub image: u8,
    /// The assembly-name pointer, when the name is stored on the assembly.
    /// Otherwise [`ImageOffsets::assembly_name`] locates it.
    pub name: Option<u8>,
}

/// Offsets within `Il2CppImage`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ImageOffsets {
    /// The assembly-name pointer, when the name is stored on the image.
    /// Otherwise [`AssemblyOffsets::name`] locates it.
    pub assembly_name: Option<u8>,
    /// The number of types in the image.
    pub type_count: u8,
    /// Where the image stores the index of its first type.
    pub type_start: TypeStart,
}

/// Where an image keeps the index of its first type in the type table.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TypeStart {
    /// The index sits directly in the image at this offset.
    Inline(u8),
    /// A pointer sits in the image at this offset; the index sits where it
    /// points.
    Handle(u8),
}

/// Offsets within `Il2CppClass`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ClassOffsets {
    /// The class-name pointer.
    pub name: u8,
    /// The namespace-name pointer.
    pub namespace: u8,
    /// The declaring class, when measured.
    pub declaring_type: Option<u16>,
    /// The parent-class pointer.
    pub parent: u8,
    /// The field table pointer.
    pub fields: u8,
    /// The static-field storage pointer.
    pub static_fields: u8,
    /// The boxed instance size, when measured.
    pub instance_size: Option<u16>,
    /// The number of fields declared by the class.
    pub field_count: u16,
}

/// Offsets within `Il2CppGenericClass`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenericOffsets {
    /// The class cached for a resolved generic instantiation, when measured.
    pub cached_class: Option<u16>,
}

/// Offsets within `Il2CppType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeOffsets {
    /// The type-data pointer, when measured.
    pub data: Option<u16>,
    /// The byte describing the element kind, when measured.
    pub kind: Option<u16>,
}

/// Offsets and size of `FieldInfo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FieldInfoOffsets {
    /// The field-name pointer.
    pub name: u8,
    /// The field's `Il2CppType` pointer, when measured.
    pub type_: Option<u16>,
    /// The field offset within its object or static storage.
    pub offset: u8,
    /// The size of one `FieldInfo` entry.
    pub size: u8,
}

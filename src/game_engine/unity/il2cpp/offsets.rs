pub(super) struct IL2CPPOffsets {
    pub(super) assembly: AssemblyOffsets,
    pub(super) image: ImageOffsets,
    pub(super) class: ClassOffsets,
    pub(super) generic: GenericOffsets,
    pub(super) type_words: TypeOffsets,
    pub(super) field: FieldInfoOffsets,
}

pub(super) struct AssemblyOffsets {
    pub(super) image: u8,
    pub(super) aname: Option<u8>, // Either this or ImageOffsets::assembly_name locates the name
}

pub(super) struct ImageOffsets {
    pub(super) assembly_name: Option<u8>, // Either this or AssemblyOffsets::aname locates the name
    pub(super) type_count: u8,
    pub(super) type_start: TypeStart,
}

/// Where an image keeps the index of its first type in the type table.
#[derive(Copy, Clone)]
pub(super) enum TypeStart {
    /// The index sits in the image, at this offset.
    Inline(u8),
    /// A pointer sits in the image at this offset. The index sits where it
    /// points.
    Handle(u8),
}

pub(super) struct ClassOffsets {
    pub(super) name: u8,
    pub(super) namespace: u8,
    pub(super) declaring_type: Option<u16>, // Where a class keeps the one declaring it
    pub(super) parent: u8,
    pub(super) fields: u8,
    pub(super) static_fields: u8,
    pub(super) instance_size: Option<u16>, // What one instance occupies, boxed header included
    pub(super) field_count: u16,
}

// Il2CppGenericClass keeps the class an instantiation resolved to.
pub(super) struct GenericOffsets {
    pub(super) cached_class: Option<u16>,
}

// Il2CppType's own words: the data pointer and the element kind byte.
pub(super) struct TypeOffsets {
    pub(super) data: Option<u16>,
    pub(super) kind: Option<u16>,
}

pub(super) struct FieldInfoOffsets {
    pub(super) name: u8,
    pub(super) type_: Option<u16>, // Where a field keeps its Il2CppType

    pub(super) offset: u8,
    pub(super) struct_size: u8,
}

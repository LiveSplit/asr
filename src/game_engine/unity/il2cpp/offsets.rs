use crate::{game_engine::unity::il2cpp::Version, PointerSize};

pub(super) struct IL2CPPOffsets {
    pub(super) assembly: AssemblyOffsets,
    pub(super) image: ImageOffsets,
    pub(super) class: ClassOffsets,
    pub(super) generic: GenericOffsets,
    pub(super) type_words: TypeOffsets,
    pub(super) field: FieldInfoOffsets,
}

impl IL2CPPOffsets {
    pub(super) fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => Some(match version {
                Version::V2022 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: Some(0x18),
                    },
                    image: ImageOffsets {
                        assembly_name: None,
                        type_count: 0x18,
                        metadata_handle: 0x28,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        declaring_type: Some(0x50), // 2023.1 through 6000.7
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        instance_size: None,
                        field_count: 0x124,
                    },
                    generic: GenericOffsets { cached_class: None },
                    type_words: TypeOffsets {
                        data: Some(0x0), // 2023.1 through 6000.7
                        kind: Some(0xA), // 2023.1 through 6000.7
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        type_: Some(0x8), // 2023.1 through 6000.7
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::V2020 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: Some(0x18),
                    },
                    image: ImageOffsets {
                        assembly_name: None,
                        type_count: 0x18,
                        metadata_handle: 0x28,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        declaring_type: None,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        instance_size: None,
                        field_count: 0x120,
                    },
                    generic: GenericOffsets { cached_class: None },
                    type_words: TypeOffsets {
                        data: None,
                        kind: None,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        type_: None,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::V2019 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: Some(0x18),
                    },
                    image: ImageOffsets {
                        assembly_name: None,
                        type_count: 0x1C,
                        metadata_handle: 0x18,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        declaring_type: Some(0x50), // 2019.4, 2020.1
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        instance_size: Some(0xF4), // 2019.4, 2020.1
                        field_count: 0x11C,
                    },
                    generic: GenericOffsets {
                        cached_class: Some(0x18), // 2019.4, 2020.1
                    },
                    type_words: TypeOffsets {
                        data: Some(0x0), // 2019.4, 2020.1
                        kind: Some(0xA), // 2019.4, 2020.1
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        type_: Some(0x8), // 2019.4, 2020.1
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::Base => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: Some(0x18),
                    },
                    image: ImageOffsets {
                        assembly_name: None,
                        type_count: 0x1C,
                        metadata_handle: 0x18,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        declaring_type: None,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        instance_size: None,
                        field_count: 0x114,
                    },
                    generic: GenericOffsets { cached_class: None },
                    type_words: TypeOffsets {
                        data: None,
                        kind: None,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        type_: None,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
            }),
            _ => None,
        }
    }
}

pub(super) struct AssemblyOffsets {
    pub(super) image: u8,
    pub(super) aname: Option<u8>, // Either this or ImageOffsets::assembly_name locates the name
}

pub(super) struct ImageOffsets {
    pub(super) assembly_name: Option<u8>, // Either this or AssemblyOffsets::aname locates the name
    pub(super) type_count: u8,
    pub(super) metadata_handle: u8,
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

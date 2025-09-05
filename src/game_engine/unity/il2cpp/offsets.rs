use crate::{game_engine::unity::il2cpp::Version, PointerSize};

pub(crate) struct IL2CPPOffsets {
    pub(crate) assembly: AssemblyOffsets,
    pub(crate) image: ImageOffsets,
    pub(crate) class: ClassOffsets,
    pub(crate) field: FieldInfoOffsets,
}

impl IL2CPPOffsets {
    pub fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => Some(match version {
                Version::V2022 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: 0x18,
                    },
                    image: ImageOffsets {
                        type_count: 0x18,
                        matadata_handle: 0x28,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        field_count: 0x124,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::V2020 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: 0x18,
                    },
                    image: ImageOffsets {
                        type_count: 0x18,
                        matadata_handle: 0x28,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        field_count: 0x120,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::V2019 => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: 0x18,
                    },
                    image: ImageOffsets {
                        type_count: 0x1C,
                        matadata_handle: 0x18,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        field_count: 0x11C,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
                Version::Base => &Self {
                    assembly: AssemblyOffsets {
                        image: 0x0,
                        aname: 0x18,
                    },
                    image: ImageOffsets {
                        type_count: 0x1C,
                        matadata_handle: 0x18,
                    },
                    class: ClassOffsets {
                        name: 0x10,
                        namespace: 0x18,
                        parent: 0x58,
                        fields: 0x80,
                        static_fields: 0xB8,
                        field_count: 0x114,
                    },
                    field: FieldInfoOffsets {
                        name: 0x0,
                        offset: 0x18,
                        struct_size: 0x20,
                    },
                },
            }),
            _ => None,
        }
    }
}

pub(crate) struct AssemblyOffsets {
    pub(crate) image: u8,
    pub(crate) aname: u8,
}

pub(crate) struct ImageOffsets {
    pub(crate) type_count: u8,
    pub(crate) matadata_handle: u8,
}

pub(crate) struct ClassOffsets {
    pub(crate) name: u8,
    pub(crate) namespace: u8,
    pub(crate) parent: u8,
    pub(crate) fields: u8,
    pub(crate) static_fields: u8,
    pub(crate) field_count: u16,
}

pub(crate) struct FieldInfoOffsets {
    pub(crate) name: u8,
    pub(crate) offset: u8,
    pub(crate) struct_size: u8,
}

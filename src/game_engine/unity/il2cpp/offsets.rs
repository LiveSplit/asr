use crate::{game_engine::unity::il2cpp::Version, PointerSize};

pub(super) struct IL2CPPOffsets {
    pub(super) assembly: AssemblyOffsets,
    pub(super) image: ImageOffsets,
    pub(super) class: ClassOffsets,
    pub(super) field: FieldInfoOffsets,
}

impl IL2CPPOffsets {
    pub(super) fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
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

pub(super) struct AssemblyOffsets {
    pub(super) image: u8,
    pub(super) aname: u8,
}

pub(super) struct ImageOffsets {
    pub(super) type_count: u8,
    pub(super) matadata_handle: u8,
}

pub(super) struct ClassOffsets {
    pub(super) name: u8,
    pub(super) namespace: u8,
    pub(super) parent: u8,
    pub(super) fields: u8,
    pub(super) static_fields: u8,
    pub(super) field_count: u16,
}

pub(super) struct FieldInfoOffsets {
    pub(super) name: u8,
    pub(super) offset: u8,
    pub(super) struct_size: u8,
}

use super::{BinaryFormat, Version};
use crate::PointerSize;

pub(super) struct MonoOffsets {
    pub(super) assembly: AssemblyOffsets,
    pub(super) image: ImageOffsets,
    pub(super) hash_table: HashTableOffsets,
    pub(super) class: ClassOffsets,
    pub(super) field: FieldInfoOffsets,
    pub(super) v_table: MonoVTableOffsets,
}

impl MonoOffsets {
    pub(super) fn new(
        version: Version,
        pointer_size: PointerSize,
        format: BinaryFormat,
    ) -> Option<&'static Self> {
        match (format, version, pointer_size) {
            (BinaryFormat::PE, Version::V3, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x60,
                },
                image: ImageOffsets { class_cache: 0x4D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x30,
                    image: 0x40,
                    name: 0x48,
                    namespace: 0x50,
                    vtable_size: 0x5C,
                    fields: 0x98,
                    runtime_info: 0xD0,
                    field_count: 0x100,
                    next_class_cache: 0x108,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            (BinaryFormat::PE, Version::V3, PointerSize::Bit32) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x8,
                    image: 0x48,
                },
                image: ImageOffsets { class_cache: 0x35C },
                hash_table: HashTableOffsets {
                    size: 0x0C,
                    table: 0x14,
                },
                class: ClassOffsets {
                    parent: 0x20,
                    image: 0x28,
                    name: 0x2C,
                    namespace: 0x30,
                    vtable_size: 0x38,
                    fields: 0x60,
                    runtime_info: 0x7C,
                    field_count: 0x9C,
                    next_class_cache: 0xA0,
                },
                field: FieldInfoOffsets {
                    name: 0x4,
                    offset: 0xC,
                    alignment: 0x10,
                },
                v_table: MonoVTableOffsets { vtable: 0x2C },
            }),
            (BinaryFormat::PE, Version::V2, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x60,
                },
                image: ImageOffsets { class_cache: 0x4C0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x30,
                    image: 0x40,
                    name: 0x48,
                    namespace: 0x50,
                    vtable_size: 0x5C,
                    fields: 0x98,
                    runtime_info: 0xD0,
                    field_count: 0x100,
                    next_class_cache: 0x108,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x40 },
            }),
            (BinaryFormat::PE, Version::V2, PointerSize::Bit32) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x8,
                    image: 0x44,
                },
                image: ImageOffsets { class_cache: 0x354 },
                hash_table: HashTableOffsets {
                    size: 0x0C,
                    table: 0x14,
                },
                class: ClassOffsets {
                    parent: 0x20,
                    image: 0x28,
                    name: 0x2C,
                    namespace: 0x30,
                    vtable_size: 0x38,
                    fields: 0x60,
                    runtime_info: 0x84,
                    field_count: 0xA4,
                    next_class_cache: 0xA8,
                },
                field: FieldInfoOffsets {
                    name: 0x4,
                    offset: 0xC,
                    alignment: 0x10,
                },
                v_table: MonoVTableOffsets { vtable: 0x28 },
            }),
            (BinaryFormat::PE, Version::V1Cattrs, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x58,
                },
                image: ImageOffsets { class_cache: 0x3D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x30,
                    image: 0x48,
                    name: 0x50,
                    namespace: 0x58,
                    vtable_size: 0x18,
                    fields: 0xB0,
                    runtime_info: 0x100,
                    field_count: 0x9C,
                    next_class_cache: 0x108,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            (BinaryFormat::PE, Version::V1Cattrs, PointerSize::Bit32) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x8,
                    image: 0x40,
                },
                image: ImageOffsets { class_cache: 0x2A0 },
                hash_table: HashTableOffsets {
                    size: 0xC,
                    table: 0x14,
                },
                class: ClassOffsets {
                    parent: 0x24,
                    image: 0x30,
                    name: 0x34,
                    namespace: 0x38,
                    vtable_size: 0xC,
                    fields: 0x78,
                    runtime_info: 0xA8,
                    field_count: 0x68,
                    next_class_cache: 0xAC,
                },
                field: FieldInfoOffsets {
                    name: 0x4,
                    offset: 0xC,
                    alignment: 0x10,
                },
                v_table: MonoVTableOffsets { vtable: 0x28 },
            }),
            (BinaryFormat::PE, Version::V1, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x58,
                },
                image: ImageOffsets { class_cache: 0x3D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x30,
                    image: 0x40,
                    name: 0x48,
                    namespace: 0x50,
                    vtable_size: 0x18,
                    fields: 0xA8,
                    runtime_info: 0xF8,
                    field_count: 0x94,
                    next_class_cache: 0x100,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            (BinaryFormat::PE, Version::V1, PointerSize::Bit32) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x8,
                    image: 0x40,
                },
                image: ImageOffsets { class_cache: 0x2A0 },
                hash_table: HashTableOffsets {
                    size: 0xC,
                    table: 0x14,
                },
                class: ClassOffsets {
                    parent: 0x24,
                    image: 0x2C,
                    name: 0x30,
                    namespace: 0x34,
                    vtable_size: 0xC,
                    fields: 0x74,
                    runtime_info: 0xA4,
                    field_count: 0x64,
                    next_class_cache: 0xA8,
                },
                field: FieldInfoOffsets {
                    name: 0x4,
                    offset: 0xC,
                    alignment: 0x10,
                },
                v_table: MonoVTableOffsets { vtable: 0x28 },
            }),
            (BinaryFormat::ELF | BinaryFormat::MachO, Version::V3, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x60,
                },
                image: ImageOffsets { class_cache: 0x4D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x28,
                    image: 0x38,
                    name: 0x40,
                    namespace: 0x48,
                    vtable_size: 0x54,
                    fields: 0x90,
                    runtime_info: 0xC8,
                    field_count: 0xF8,
                    next_class_cache: 0x100,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            (BinaryFormat::ELF | BinaryFormat::MachO, Version::V2, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x60,
                },
                image: ImageOffsets { class_cache: 0x4C0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x28,
                    image: 0x38,
                    name: 0x40,
                    namespace: 0x48,
                    vtable_size: 0x54,
                    fields: 0x90,
                    runtime_info: 0xC8,
                    field_count: 0xF8,
                    next_class_cache: 0x100,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x40 },
            }),
            (BinaryFormat::ELF | BinaryFormat::MachO, Version::V1Cattrs, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x58,
                },
                image: ImageOffsets { class_cache: 0x3D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x28,
                    image: 0x40,
                    name: 0x48,
                    namespace: 0x50,
                    vtable_size: 0x18,
                    fields: 0xA8,
                    runtime_info: 0xF8,
                    field_count: 0x94,
                    next_class_cache: 0x100,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            (BinaryFormat::ELF | BinaryFormat::MachO, Version::V1, PointerSize::Bit64) => Some(&Self {
                assembly: AssemblyOffsets {
                    aname: 0x10,
                    image: 0x58,
                },
                image: ImageOffsets { class_cache: 0x3D0 },
                hash_table: HashTableOffsets {
                    size: 0x18,
                    table: 0x20,
                },
                class: ClassOffsets {
                    parent: 0x28,
                    image: 0x38,
                    name: 0x40,
                    namespace: 0x48,
                    vtable_size: 0x18,
                    fields: 0xA0,
                    runtime_info: 0xF0,
                    field_count: 0x8C,
                    next_class_cache: 0xF8,
                },
                field: FieldInfoOffsets {
                    name: 0x8,
                    offset: 0x18,
                    alignment: 0x20,
                },
                v_table: MonoVTableOffsets { vtable: 0x48 },
            }),
            _ => None,
        }
    }
}

pub(super) struct AssemblyOffsets {
    pub(super) aname: u8,
    pub(super) image: u8,
}

pub(super) struct ImageOffsets {
    pub(super) class_cache: u16,
}

pub(super) struct HashTableOffsets {
    pub(super) size: u8,
    pub(super) table: u8,
}

pub(super) struct ClassOffsets {
    pub(super) parent: u8,
    #[allow(unused)]
    pub(super) image: u8, // Unused for now, kept in the struct for future use
    pub(super) name: u8,
    pub(super) namespace: u8,
    pub(super) vtable_size: u8, // On mono V1 and V1_cattrs, this offset represents MonoVTable.data
    pub(super) fields: u8,
    pub(super) runtime_info: u16,
    pub(super) field_count: u16,
    pub(super) next_class_cache: u16,
}

pub(super) struct FieldInfoOffsets {
    pub(super) name: u8,
    pub(super) offset: u8,
    pub(super) alignment: u8,
}

pub(super) struct MonoVTableOffsets {
    pub(super) vtable: u8,
}

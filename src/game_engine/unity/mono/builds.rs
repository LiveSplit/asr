//! Known mono builds. Each entry is one exact mono binary. The GUID of its
//! PDB says which binary. The offsets come from that PDB.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, HashTableOffsets, ImageOffsets, MonoOffsets,
    MonoVTableOffsets,
};
use super::Version;
use crate::{file_format::pe::DebugId, PointerSize};

/// One exact mono binary and the offsets from its PDB.
pub(super) struct Build {
    pub(super) guid: [u8; 16],
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: MonoOffsets,
}

/// Finds the build with this GUID.
pub(super) fn find(debug_id: &DebugId) -> Option<&'static Build> {
    BUILDS
        .binary_search_by(|build| build.guid.cmp(&debug_id.guid))
        .ok()
        .map(|index| &BUILDS[index])
}

/// Parses a canonical GUID into the byte order the debug directory stores.
/// The first three fields are little-endian there.
const fn guid(canonical: &str) -> [u8; 16] {
    const fn hex(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("The GUID is not lowercase hex."),
        }
    }

    let canonical = canonical.as_bytes();
    assert!(
        canonical.len() == 36
            && canonical[8] == b'-'
            && canonical[13] == b'-'
            && canonical[18] == b'-'
            && canonical[23] == b'-',
        "The GUID is not in its canonical form.",
    );

    let mut parsed = [0; 16];
    let mut index = 0;
    let mut at = 0;
    while index < 16 {
        if canonical[at] == b'-' {
            at += 1;
            continue;
        }
        parsed[index] = (hex(canonical[at]) << 4) | hex(canonical[at + 1]);
        index += 1;
        at += 2;
    }

    [
        parsed[3], parsed[2], parsed[1], parsed[0], parsed[5], parsed[4], parsed[7], parsed[6],
        parsed[8], parsed[9], parsed[10], parsed[11], parsed[12], parsed[13], parsed[14],
        parsed[15],
    ]
}

// The table is sorted by GUID. The mono.dll builds set `v_table.vtable` to
// 0. Their statics path reads `MonoVTable.data` through `vtable_size` and
// never reads `v_table.vtable`.
static BUILDS: &[Build] = &[
    // Unity 2017.4.40f1, mono-2.0-bdwgc.dll (net_4_6), x86.
    // No x86 PDB exists for this binary. The offsets are the x64 layout with
    // 32-bit field sizes, worked out by hand.
    Build {
        guid: guid("54fe0c31-c851-4749-baa5-7699d1279165"),
        pointer_size: PointerSize::Bit32,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x44,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x354,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 6000.5.8, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("eb6b6239-5624-487c-a84e-d7f0a7335670"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2017.4.40, mono-2.0-bdwgc.dll (net_4_6), x64.
    // No PDB exists for this exact binary. The offsets come from another
    // build's mono-2.0-bdwgc.pdb.
    Build {
        guid: guid("2f7a3442-3c29-424d-8a46-8cc59237ed89"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x4c0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2021.3.11, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("1d994642-9a41-4a6a-84be-f55f9cff8f57"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2018.4.36, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("f469c84e-5b81-4c42-8c3f-72ad629f99cb"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x4c0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2018.4.36, mono.dll, x64.
    Build {
        guid: guid("487fa150-59b5-4a18-8fed-964001db1b82"),
        pointer_size: PointerSize::Bit64,
        version: Version::V1Cattrs,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x3d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x50,
                namespace: 0x58,
                vtable_size: 0x18,
                fields: 0xb0,
                runtime_info: 0x100,
                field_count: 0x9c,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.5.8, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("4f356e63-5da8-496c-8bb8-aaf2a0b1f364"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.2.12, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("018d6f65-a658-4607-93eb-2518f5018226"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.7.0, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("49c1826a-d1b9-442e-8388-4509b7c91395"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.3.21, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("1ac99f6b-fd3a-4dc0-93e7-782ca1b4be7d"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2018.4.36, mono.dll, x86.
    Build {
        guid: guid("c3c97c70-f490-4462-a27d-b4103d2aca1f"),
        pointer_size: PointerSize::Bit32,
        version: Version::V1Cattrs,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x40,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x2a0,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x24,
                name: 0x34,
                namespace: 0x38,
                vtable_size: 0xc,
                fields: 0x78,
                runtime_info: 0xa8,
                field_count: 0x68,
                next_class_cache: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 5.6.7, mono.dll, x64.
    // No PDB exists for this exact binary. The offsets come from another
    // build's mono.pdb.
    Build {
        guid: guid("924a8172-8d25-496f-b684-20c9f04d4f92"),
        pointer_size: PointerSize::Bit64,
        version: Version::V1,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x3d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x18,
                fields: 0xa8,
                runtime_info: 0xf8,
                field_count: 0x94,
                next_class_cache: 0x100,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2020.1.18, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("984e5687-3dd9-4d72-8e88-552c6810430d"),
        pointer_size: PointerSize::Bit32,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x44,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x354,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 2020.1.18, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("0b5f7f89-7937-4300-9c3b-a1ec2c75e06e"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x4c0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2017.4.40, mono.dll, x64.
    Build {
        guid: guid("c1c35e9c-fd72-4ebf-af5e-e7c932e2865d"),
        pointer_size: PointerSize::Bit64,
        version: Version::V1Cattrs,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x3d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x50,
                namespace: 0x58,
                vtable_size: 0x18,
                fields: 0xb0,
                runtime_info: 0x100,
                field_count: 0x9c,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.3.21, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("44e461a2-1832-413d-afb1-3fe613634de3"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2017.4.40, mono.dll, x86.
    Build {
        guid: guid("d45555b8-4783-4fba-9eeb-f830cb655d89"),
        pointer_size: PointerSize::Bit32,
        version: Version::V1Cattrs,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x40,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x2a0,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x24,
                name: 0x34,
                namespace: 0x38,
                vtable_size: 0xc,
                fields: 0x78,
                runtime_info: 0xa8,
                field_count: 0x68,
                next_class_cache: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.7.0, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("8e2fbcbc-d64d-4993-a733-a489d7a90b2b"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2023.1.22, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("4aac62be-dfea-4610-91fc-8a1b6c768935"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x30),
                class_cache: 0x4d0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2019.4.41, mono-2.0-bdwgc.dll, x64.
    Build {
        guid: guid("7710aac7-315a-4d30-a77a-0807296966f6"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: Some(0x28),
                class_cache: 0x4c0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                parent: 0x30,
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            field: FieldInfoOffsets {
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2019.4.41, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("998210ce-aee9-4d0b-a225-9c529815fc78"),
        pointer_size: PointerSize::Bit32,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x44,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x354,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 5.6.7, mono.dll, x86.
    // No x86 PDB exists for this binary. The offsets are the x64 layout with
    // 32-bit field sizes, worked out by hand.
    Build {
        guid: guid("064ccfd8-ab0c-4a5b-b33d-7a59b8eafbab"),
        pointer_size: PointerSize::Bit32,
        version: Version::V1,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x40,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x2a0,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x24,
                name: 0x30,
                namespace: 0x34,
                vtable_size: 0xc,
                fields: 0x74,
                runtime_info: 0xa4,
                field_count: 0x64,
                next_class_cache: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2018.4.36, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("7059c7da-c870-4870-951d-758ba588a378"),
        pointer_size: PointerSize::Bit32,
        version: Version::V2,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x44,
            },
            image: ImageOffsets {
                assembly_name: Some(0x18),
                class_cache: 0x354,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 2021.3.11, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("51a376db-5854-4c34-925f-acb714c49e65"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 6000.2.12, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("9fd463e5-f21d-49da-8e5d-67d03349843a"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2023.1.22, mono-2.0-bdwgc.dll, x86.
    Build {
        guid: guid("347d7ee9-ca67-435d-be75-237735403a3d"),
        pointer_size: PointerSize::Bit32,
        version: Version::V3,
        offsets: MonoOffsets {
            assembly: AssemblyOffsets {
                aname: None,
                image: 0x48,
            },
            image: ImageOffsets {
                assembly_name: Some(0x1c),
                class_cache: 0x35c,
            },
            hash_table: HashTableOffsets {
                size: 0xc,
                table: 0x14,
            },
            class: ClassOffsets {
                parent: 0x20,
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            field: FieldInfoOffsets {
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::super::{BinaryFormat, Version};
    use super::{find, guid, MonoOffsets, BUILDS};
    use crate::file_format::pe::DebugId;
    use crate::PointerSize;

    // The GUID of the 2019.4 mono runtime, in the byte order the debug
    // directory stores. The pe tests read the same GUID from a mapped image.
    const STORED: [u8; 16] = [
        0xC7, 0xAA, 0x10, 0x77, 0x5A, 0x31, 0x30, 0x4D, 0xA7, 0x7A, 0x08, 0x07, 0x29, 0x69, 0x66,
        0xF6,
    ];

    #[test]
    fn parses_canonical_guids_into_storage_order() {
        assert_eq!(guid("7710aac7-315a-4d30-a77a-0807296966f6"), STORED);
    }

    #[test]
    fn table_is_sorted_and_unique() {
        assert!(BUILDS.windows(2).all(|pair| pair[0].guid < pair[1].guid));
    }

    #[test]
    fn finds_known_builds() {
        let build = find(&DebugId {
            guid: STORED,
            age: 1,
        })
        .unwrap();
        assert_eq!(build.pointer_size, PointerSize::Bit64);
        assert!(matches!(build.version, Version::V2));

        assert!(find(&DebugId {
            guid: [0; 16],
            age: 1,
        })
        .is_none());
    }

    // Every build has the layout its version table describes. The one
    // difference is where the assembly name lives, and mono.dll builds never
    // read the vtable slot.
    #[test]
    fn every_build_matches_its_version_table() {
        for build in BUILDS {
            let table =
                MonoOffsets::new(build.version, build.pointer_size, BinaryFormat::PE).unwrap();
            let offsets = &build.offsets;

            assert_eq!(offsets.assembly.image, table.assembly.image);
            assert_eq!(offsets.image.class_cache, table.image.class_cache);
            assert_eq!(offsets.hash_table.size, table.hash_table.size);
            assert_eq!(offsets.hash_table.table, table.hash_table.table);
            assert_eq!(offsets.class.parent, table.class.parent);
            assert_eq!(offsets.class.name, table.class.name);
            assert_eq!(offsets.class.namespace, table.class.namespace);
            assert_eq!(offsets.class.vtable_size, table.class.vtable_size);
            assert_eq!(offsets.class.fields, table.class.fields);
            assert_eq!(offsets.class.runtime_info, table.class.runtime_info);
            assert_eq!(offsets.class.field_count, table.class.field_count);
            assert_eq!(offsets.class.next_class_cache, table.class.next_class_cache);
            assert_eq!(offsets.field.name, table.field.name);
            assert_eq!(offsets.field.offset, table.field.offset);
            assert_eq!(offsets.field.alignment, table.field.alignment);
            if !matches!(build.version, Version::V1 | Version::V1Cattrs) {
                assert_eq!(offsets.v_table.vtable, table.v_table.vtable);
            }
        }
    }
}

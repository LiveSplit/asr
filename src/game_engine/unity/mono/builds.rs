//! Known mono builds. Each entry is one exact mono binary. The GUID of its
//! PDB says which binary. The offsets come from that PDB.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoOffsets, MonoVTableOffsets, TypeOffsets,
};
use super::{Library, Version};
use crate::{file_format::pe::DebugId, PointerSize};

/// One exact mono binary and the offsets from its PDB. The Unity version is
/// the four parts of the file version of the player it shipped with.
pub(super) struct Build {
    pub(super) debug_id: DebugId,
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) library: Library,
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: MonoOffsets,
}

/// Finds the build for a player that is no known build: the newest build
/// of its library at its pointer size whose major.minor is at or below the
/// player's major.minor, or the oldest such build when no build is below.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    nearest_by_version(
        BUILDS,
        unity,
        |build| (build.unity, build.library, build.pointer_size),
        library,
        pointer_size,
    )
}

/// The rule every table shares. The tables are not sorted by version, so
/// the newest build at or below the player is the greatest version among
/// those, and the oldest build is the least version of all. A Linux or Mac
/// player names only three parts of its version, so the exact arm serves
/// Windows; the identity match is the exact hit elsewhere.
pub(super) fn nearest_by_version<B>(
    builds: &'static [B],
    unity: (u16, u16, u16, u16),
    key: impl Fn(&B) -> ((u16, u16, u16, u16), Library, PointerSize),
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static B> {
    let same = || {
        builds.iter().filter(|build| {
            let (_, built_for, width) = key(build);
            built_for == library && width == pointer_size
        })
    };
    let version = |build: &B| key(build).0;

    same()
        .find(|build| version(build) == unity)
        .or_else(|| {
            same()
                .filter(|build| {
                    let built = version(build);
                    (built.0, built.1) <= (unity.0, unity.1)
                })
                .max_by_key(|build| version(build))
        })
        .or_else(|| same().min_by_key(|build| version(build)))
}

/// Finds the build with this GUID and age. A relink keeps the GUID and
/// raises the age, and the relinked binary can have a different layout.
pub(super) fn find(debug_id: &DebugId) -> Option<&'static Build> {
    BUILDS
        .binary_search_by(|build| {
            (build.debug_id.guid, build.debug_id.age).cmp(&(debug_id.guid, debug_id.age))
        })
        .ok()
        .map(|index| &BUILDS[index])
}

/// Makes a debug id from a GUID, written the way a PDB prints it, and an age.
const fn debug_id(canonical: &str, age: u32) -> DebugId {
    DebugId {
        guid: guid(canonical),
        age,
    }
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

// The table is sorted by GUID, then age. The mono.dll builds set
// `v_table.vtable` to 0. Their statics path reads `MonoVTable.data` through
// `vtable_size` and never reads `v_table.vtable`.
pub(super) static BUILDS: &[Build] = &[
    // Unity 2017.4.40f1, mono-2.0-bdwgc.dll (net_4_6), x86.
    // No x86 PDB exists for this binary. The offsets are the x64 layout with
    // 32-bit field sizes, worked out by hand.
    Build {
        debug_id: debug_id("54fe0c31-c851-4749-baa5-7699d1279165", 1),
        unity: (2017, 4, 40, 5126),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1e),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            generic: GenericOffsets {
                generic_class: Some(0x94),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 6000.5.8, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("eb6b6239-5624-487c-a84e-d7f0a7335670", 1),
        unity: (6000, 5, 8, 47071),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
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
        debug_id: debug_id("2f7a3442-3c29-424d-8a46-8cc59237ed89", 1),
        unity: (2017, 4, 40, 5126),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x2a),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2021.3.11, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("1d994642-9a41-4a6a-84be-f55f9cff8f57", 1),
        unity: (2021, 3, 11, 23713),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2018.4.36, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("f469c84e-5b81-4c42-8c3f-72ad629f99cb", 1),
        unity: (2018, 4, 36, 54151),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x2a),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2018.4.36, mono.dll, x64.
    Build {
        debug_id: debug_id("487fa150-59b5-4a18-8fed-964001db1b82", 1),
        unity: (2018, 4, 36, 54151),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x50,
                namespace: 0x58,
                vtable_size: 0x18,
                fields: 0xb0,
                runtime_info: 0x100,
                field_count: 0x9c,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.5.8, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("4f356e63-5da8-496c-8bb8-aaf2a0b1f364", 1),
        unity: (6000, 5, 8, 47071),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.2.12, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("018d6f65-a658-4607-93eb-2518f5018226", 1),
        unity: (6000, 2, 12, 40285),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.7.0, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("49c1826a-d1b9-442e-8388-4509b7c91395", 1),
        unity: (6000, 7, 0, 5476),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 6000.3.21, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("1ac99f6b-fd3a-4dc0-93e7-782ca1b4be7d", 1),
        unity: (6000, 3, 21, 9777),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2018.4.36, mono.dll, x86.
    Build {
        debug_id: debug_id("c3c97c70-f490-4462-a27d-b4103d2aca1f", 1),
        unity: (2018, 4, 36, 54151),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x10),
                parent: 0x24,
                nested_in: Some(0x28),
                name: 0x34,
                namespace: 0x38,
                vtable_size: 0xc,
                fields: 0x78,
                runtime_info: 0xa8,
                field_count: 0x68,
                next_class_cache: 0xac,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
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
        debug_id: debug_id("924a8172-8d25-496f-b684-20c9f04d4f92", 1),
        unity: (5, 6, 7, 3267),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x18,
                fields: 0xa8,
                runtime_info: 0xf8,
                field_count: 0x94,
                next_class_cache: 0x100,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2020.1.18, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("984e5687-3dd9-4d72-8e88-552c6810430d", 1),
        unity: (2020, 1, 18, 38512),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1e),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            generic: GenericOffsets {
                generic_class: Some(0x94),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 2020.1.18, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("0b5f7f89-7937-4300-9c3b-a1ec2c75e06e", 1),
        unity: (2020, 1, 18, 38512),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x2a),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2017.4.40, mono.dll, x64.
    Build {
        debug_id: debug_id("c1c35e9c-fd72-4ebf-af5e-e7c932e2865d", 1),
        unity: (2017, 4, 40, 5126),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x50,
                namespace: 0x58,
                vtable_size: 0x18,
                fields: 0xb0,
                runtime_info: 0x100,
                field_count: 0x9c,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.3.21, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("44e461a2-1832-413d-afb1-3fe613634de3", 1),
        unity: (6000, 3, 21, 9777),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2021.2.20, mono-2.0-bdwgc.dll, x64. The same binary ships with
    // 2021.3.0.
    Build {
        debug_id: debug_id("f188d9a9-144f-44e2-a0c8-2a5272ab4119", 1),
        unity: (2021, 2, 20, 62729),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2017.4.40, mono.dll, x86.
    Build {
        debug_id: debug_id("d45555b8-4783-4fba-9eeb-f830cb655d89", 1),
        unity: (2017, 4, 40, 5126),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x10),
                parent: 0x24,
                nested_in: Some(0x28),
                name: 0x34,
                namespace: 0x38,
                vtable_size: 0xc,
                fields: 0x78,
                runtime_info: 0xa8,
                field_count: 0x68,
                next_class_cache: 0xac,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 6000.7.0, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("8e2fbcbc-d64d-4993-a733-a489d7a90b2b", 1),
        unity: (6000, 7, 0, 5476),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2023.1.22, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("4aac62be-dfea-4610-91fc-8a1b6c768935", 1),
        unity: (2023, 1, 22, 16744),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1b),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2019.4.41, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("7710aac7-315a-4d30-a77a-0807296966f6", 1),
        unity: (2019, 4, 41, 9172),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x2a),
                instance_size: Some(0x1c),
                parent: 0x30,
                nested_in: Some(0x38),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x5c,
                fields: 0x98,
                runtime_info: 0xd0,
                field_count: 0x100,
                next_class_cache: 0x108,
            },
            generic: GenericOffsets {
                generic_class: Some(0xf0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                alignment: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2019.4.41, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("998210ce-aee9-4d0b-a225-9c529815fc78", 1),
        unity: (2019, 4, 41, 9172),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1e),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            generic: GenericOffsets {
                generic_class: Some(0x94),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
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
        debug_id: debug_id("064ccfd8-ab0c-4a5b-b33d-7a59b8eafbab", 1),
        unity: (5, 6, 7, 3267),
        library: Library::Mono,
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
                class_kind: None,
                instance_size: Some(0x10),
                parent: 0x24,
                nested_in: Some(0x28),
                name: 0x30,
                namespace: 0x34,
                vtable_size: 0xc,
                fields: 0x74,
                runtime_info: 0xa4,
                field_count: 0x64,
                next_class_cache: 0xa8,
            },
            generic: GenericOffsets {
                generic_class: None,
                container_class: None,
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2021.2.20, mono-2.0-bdwgc.dll, x86. The same binary ships with
    // 2021.3.0.
    Build {
        debug_id: debug_id("e955f8d8-44c0-48cf-a868-8c52d305a00e", 1),
        unity: (2021, 2, 20, 62729),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2018.4.36, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("7059c7da-c870-4870-951d-758ba588a378", 1),
        unity: (2018, 4, 36, 54151),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0x1e),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x84,
                field_count: 0xa4,
                next_class_cache: 0xa8,
            },
            generic: GenericOffsets {
                generic_class: Some(0x94),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 2021.3.11, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("51a376db-5854-4c34-925f-acb714c49e65", 1),
        unity: (2021, 3, 11, 23713),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 6000.2.12, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("9fd463e5-f21d-49da-8e5d-67d03349843a", 1),
        unity: (6000, 2, 12, 40285),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x4,
                offset: 0xc,
                alignment: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
    // Unity 2023.1.22, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("347d7ee9-ca67-435d-be75-237735403a3d", 1),
        unity: (2023, 1, 22, 16744),
        library: Library::MonoBdwgc,
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
                class_kind: Some(0xf),
                instance_size: Some(0x10),
                parent: 0x20,
                nested_in: Some(0x24),
                name: 0x2c,
                namespace: 0x30,
                vtable_size: 0x38,
                fields: 0x60,
                runtime_info: 0x7c,
                field_count: 0x9c,
                next_class_cache: 0xa0,
            },
            generic: GenericOffsets {
                generic_class: Some(0x8c),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
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
        assert!(BUILDS.windows(2).all(|pair| {
            (pair[0].debug_id.guid, pair[0].debug_id.age)
                < (pair[1].debug_id.guid, pair[1].debug_id.age)
        }));
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

    // Unity 2021.2.20f1 is the oldest measured player with the layout that
    // every later build shares. Two members tell that layout apart from the
    // one before it: the class kind byte moved to 0x1B on x64 and 0xF on
    // x86, and the vtable's method pointers to 0x48 and 0x2C.
    #[test]
    fn the_2021_2_20_players_are_known_builds() {
        let x64 = find(&super::debug_id("f188d9a9-144f-44e2-a0c8-2a5272ab4119", 1)).unwrap();
        assert_eq!(x64.pointer_size, PointerSize::Bit64);
        assert_eq!(x64.offsets.class.class_kind, Some(0x1B));
        assert_eq!(x64.offsets.v_table.vtable, 0x48);

        let x86 = find(&super::debug_id("e955f8d8-44c0-48cf-a868-8c52d305a00e", 1)).unwrap();
        assert_eq!(x86.pointer_size, PointerSize::Bit32);
        assert_eq!(x86.offsets.class.class_kind, Some(0xF));
        assert_eq!(x86.offsets.v_table.vtable, 0x2C);
    }

    #[test]
    fn builds_with_another_age_are_not_found() {
        assert!(find(&DebugId {
            guid: STORED,
            age: 2,
        })
        .is_none());
    }

    // A version table's value for any of the grown members must match every
    // measured build it stands in for, or say nothing.
    #[test]
    fn version_tables_never_contradict_a_measured_build() {
        fn agrees(table: Option<u16>, measured: Option<u16>) -> bool {
            table.is_none() || table == measured
        }

        for build in BUILDS {
            let Some(table) = MonoOffsets::new(build.version, build.pointer_size, BinaryFormat::PE)
            else {
                continue;
            };
            assert!(agrees(table.class.nested_in, build.offsets.class.nested_in));
            assert!(agrees(
                table.class.class_kind,
                build.offsets.class.class_kind
            ));
            assert!(agrees(
                table.generic.generic_class,
                build.offsets.generic.generic_class
            ));
            assert!(agrees(
                table.generic.container_class,
                build.offsets.generic.container_class
            ));
            assert!(agrees(
                table.class.instance_size,
                build.offsets.class.instance_size
            ));
            assert!(agrees(table.type_words.data, build.offsets.type_words.data));
            assert!(agrees(table.type_words.kind, build.offsets.type_words.kind));
            assert!(agrees(table.field.type_, build.offsets.field.type_));
        }
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

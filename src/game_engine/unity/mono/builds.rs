//! The Windows Mono builds where the layout changes. Each entry is one
//! exact mono binary, told by the GUID of its PDB, and the offsets come
//! from that PDB. A game on any other build takes the newest entry at or
//! below its version.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoVTableOffsets, Profile, TypeOffsets,
};
use super::Library;
use crate::{file_format::pe::DebugId, PointerSize};

/// One exact mono binary and the offsets from its PDB. The Unity version is
/// the four parts of the file version of the player it shipped with.
pub(super) struct Build {
    pub(super) debug_id: DebugId,
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) profile: Profile,
}

/// Finds the newest Windows build at or below the player's version, for the
/// player's library and pointer size. Returns `None` for a player older than
/// every build.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    nearest_by_version(
        BUILDS,
        unity,
        |build| {
            (
                build.unity,
                build.profile.library,
                build.profile.pointer_size,
            )
        },
        library,
        pointer_size,
    )
}

/// Finds the newest build at or below the player's version, for the
/// player's library and pointer size. Only major.minor.patch is compared,
/// because Linux and Mac players don't carry a build number.
pub(super) fn nearest_by_version<B>(
    builds: &'static [B],
    unity: (u16, u16, u16, u16),
    key: impl Fn(&B) -> ((u16, u16, u16, u16), Library, PointerSize),
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static B> {
    builds
        .iter()
        .filter(|build| {
            let (built, built_for, width) = key(build);
            built_for == library
                && width == pointer_size
                && (built.0, built.1, built.2) <= (unity.0, unity.1, unity.2)
        })
        .max_by_key(|build| key(build).0)
}

/// Like [`nearest_by_version`], but a player older than every build gets the
/// oldest build.
pub(super) fn nearest_or_oldest<B>(
    builds: &'static [B],
    unity: (u16, u16, u16, u16),
    key: impl Fn(&B) -> ((u16, u16, u16, u16), Library, PointerSize) + Copy,
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static B> {
    nearest_by_version(builds, unity, key, library, pointer_size).or_else(|| {
        builds
            .iter()
            .filter(|build| {
                let (_, built_for, width) = key(build);
                built_for == library && width == pointer_size
            })
            .min_by_key(|build| key(build).0)
    })
}

/// Finds the build with this GUID and age. A relink keeps the GUID and
/// raises the age, and the relinked binary can have a different layout.
pub(super) fn find(debug_id: &DebugId) -> Option<&'static Build> {
    BUILDS.iter().find(|build| &build.debug_id == debug_id)
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

// The table reads from the oldest player to the newest. The mono.dll builds
// set `v_table.vtable` to 0. Their statics path reads `MonoVTable.data`
// through `vtable_size` and never reads `v_table.vtable`.
pub(super) const BUILDS: &[Build] = &[
    // Unity 5.0.0, mono.dll, x64.
    Build {
        debug_id: debug_id("5136738d-3ea6-4d1e-9648-6b32e5c9bd88", 1),
        unity: (5, 0, 0, 39095),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
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
                stride: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 5.0.0, mono.dll, x86.
    Build {
        debug_id: debug_id("eba34511-cb2c-49b7-a5cb-2887ed792a46", 1),
        unity: (5, 0, 0, 39095),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            library: Library::Mono,
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
                stride: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2017.2.0, mono-2.0-bdwgc.dll, x64. The offsets come from the PDB
    // of the 2017.3.0 runtime, which has the same layout.
    Build {
        debug_id: debug_id("e3d757e3-23e3-44a7-bb3a-c6735ca84523", 1),
        unity: (2017, 2, 0, 58714),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::MonoBdwgc,
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
                stride: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x40 },
        },
    },
    // Unity 2017.2.0, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("01871050-7e1d-491b-9083-0c5d4bf2c3ae", 1),
        unity: (2017, 2, 0, 58714),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            library: Library::MonoBdwgc,
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
                stride: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x28 },
        },
    },
    // Unity 2017.4.6, mono.dll, x64. The offsets come from the PDB of the
    // 2017.4.40 runtime, which has the same layout.
    Build {
        debug_id: debug_id("2b555578-2909-4668-a7a9-65356291192a", 1),
        unity: (2017, 4, 6, 20272),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
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
                stride: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2017.4.6, mono.dll, x86.
    Build {
        debug_id: debug_id("60283a3e-8c5c-4de0-bcb2-eb794ba9ea7c", 1),
        unity: (2017, 4, 6, 20272),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            library: Library::Mono,
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
                stride: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x0 },
        },
    },
    // Unity 2021.2.0, mono-2.0-bdwgc.dll, x64.
    Build {
        debug_id: debug_id("1d844e97-52aa-4f02-8bca-3078b5a96371", 1),
        unity: (2021, 2, 0, 61932),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::MonoBdwgc,
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
                stride: 0x20,
            },
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // Unity 2021.2.0, mono-2.0-bdwgc.dll, x86.
    Build {
        debug_id: debug_id("6d0f69e4-7a71-449a-b952-35cf3caac221", 1),
        unity: (2021, 2, 0, 61932),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            library: Library::MonoBdwgc,
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
                stride: 0x10,
            },
            v_table: MonoVTableOffsets { vtable: 0x2c },
        },
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{find, guid, BUILDS};
    use crate::file_format::pe::DebugId;
    use crate::PointerSize;

    // The GUID of the 2021.2.0 x64 mono runtime, in the byte order the
    // debug directory stores.
    const STORED: [u8; 16] = [
        0x97, 0x4E, 0x84, 0x1D, 0xAA, 0x52, 0x02, 0x4F, 0x8B, 0xCA, 0x30, 0x78, 0xB5, 0xA9, 0x63,
        0x71,
    ];

    #[test]
    fn parses_canonical_guids_into_storage_order() {
        assert_eq!(guid("1d844e97-52aa-4f02-8bca-3078b5a96371"), STORED);
    }

    #[test]
    fn table_reads_oldest_to_newest() {
        assert!(BUILDS.windows(2).all(|pair| pair[0].unity <= pair[1].unity));
    }

    #[test]
    fn table_has_each_build_once() {
        for (index, build) in BUILDS.iter().enumerate() {
            assert!(!BUILDS[..index]
                .iter()
                .any(|earlier| earlier.debug_id == build.debug_id));
        }
    }

    #[test]
    fn finds_known_builds() {
        let build = find(&DebugId {
            guid: STORED,
            age: 1,
        })
        .unwrap();
        assert_eq!(build.profile.pointer_size, PointerSize::Bit64);

        assert!(find(&DebugId {
            guid: [0; 16],
            age: 1,
        })
        .is_none());
    }

    // Unity 2021.2.0f1 is the first player with the most recent layout. The
    // class kind byte moved to 0x1B on x64 and 0xF on x86, and the vtable's
    // method pointers to 0x48 and 0x2C.
    #[test]
    fn the_2021_2_players_are_known_builds() {
        let x64 = find(&super::debug_id("1d844e97-52aa-4f02-8bca-3078b5a96371", 1)).unwrap();
        assert_eq!(x64.unity, (2021, 2, 0, 61932));
        assert_eq!(x64.profile.pointer_size, PointerSize::Bit64);
        assert_eq!(x64.profile.class.class_kind, Some(0x1B));
        assert_eq!(x64.profile.v_table.vtable, 0x48);

        let x86 = find(&super::debug_id("6d0f69e4-7a71-449a-b952-35cf3caac221", 1)).unwrap();
        assert_eq!(x86.unity, (2021, 2, 0, 61932));
        assert_eq!(x86.profile.pointer_size, PointerSize::Bit32);
        assert_eq!(x86.profile.class.class_kind, Some(0xF));
        assert_eq!(x86.profile.v_table.vtable, 0x2C);
    }

    #[test]
    fn builds_with_another_age_are_not_found() {
        assert!(find(&DebugId {
            guid: STORED,
            age: 2,
        })
        .is_none());
    }
}

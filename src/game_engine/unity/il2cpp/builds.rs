//! Known IL2CPP builds: exact metadata layouts, named by the version of the
//! game's `global-metadata.dat` and the Unity version that shipped it, paired
//! with the offsets measured from their symbols.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, IL2CPPOffsets, ImageOffsets,
};
use super::Version;
use crate::PointerSize;

/// One exact IL2CPP layout and the offsets measured from it.
pub(super) struct Build {
    pub(super) metadata: u32,
    pub(super) unity: (u16, u16),
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: IL2CPPOffsets,
}

/// Looks up the newest known build at or below the given identity. Unlike a
/// mono runtime, `GameAssembly.dll` is compiled per game, so no identity names
/// one binary: a build declares the version it applies from, and an identity
/// below the oldest known build answers nothing.
pub(super) fn find(
    metadata: u32,
    unity: (u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    BUILDS
        .iter()
        .rev()
        .filter(|build| build.pointer_size == pointer_size)
        .find(|build| (build.metadata, build.unity) <= (metadata, unity))
}

// The table reads from the oldest metadata to the newest.
static BUILDS: &[Build] = &[
    // Unity 2018.4.36f1, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 24,
        unity: (2018, 4),
        pointer_size: PointerSize::Bit64,
        version: Version::Base,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x114,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2019.4.41f2, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 24,
        unity: (2019, 4),
        pointer_size: PointerSize::Bit64,
        version: Version::V2019,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x11c,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 24,
        unity: (2020, 1),
        pointer_size: PointerSize::Bit64,
        version: Version::V2019,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x11c,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 24,
        unity: (2020, 1),
        pointer_size: PointerSize::Bit32,
        version: Version::V2019,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0x10,
                metadata_handle: 0xc,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                field_count: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 29,
        unity: (2021, 3),
        pointer_size: PointerSize::Bit64,
        version: Version::V2020,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x120,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 29,
        unity: (2021, 3),
        pointer_size: PointerSize::Bit32,
        version: Version::V2020,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                field_count: 0xa8,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 29,
        unity: (2023, 1),
        pointer_size: PointerSize::Bit64,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x124,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 29,
        unity: (2023, 1),
        pointer_size: PointerSize::Bit32,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                field_count: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 31,
        unity: (6000, 2),
        pointer_size: PointerSize::Bit64,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x124,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 31,
        unity: (6000, 2),
        pointer_size: PointerSize::Bit32,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                field_count: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 39,
        unity: (6000, 3),
        pointer_size: PointerSize::Bit64,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                field_count: 0x124,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 39,
        unity: (6000, 3),
        pointer_size: PointerSize::Bit32,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                field_count: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.5.8f1, metadata version 107, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 107,
        unity: (6000, 5),
        pointer_size: PointerSize::Bit64,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xa0,
                field_count: 0x124,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.5.8f1, metadata version 107, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 107,
        unity: (6000, 5),
        pointer_size: PointerSize::Bit32,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x50,
                field_count: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 110,
        unity: (6000, 7),
        pointer_size: PointerSize::Bit64,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                metadata_handle: 0x28,
            },
            class: ClassOffsets {
                name: 0x10,
                namespace: 0x18,
                parent: 0x58,
                fields: 0x80,
                static_fields: 0x98,
                field_count: 0x11c,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        metadata: 110,
        unity: (6000, 7),
        pointer_size: PointerSize::Bit32,
        version: Version::V2022,
        offsets: IL2CPPOffsets {
            assembly: AssemblyOffsets {
                image: 0x0,
                aname: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                metadata_handle: 0x18,
            },
            class: ClassOffsets {
                name: 0x8,
                namespace: 0xc,
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x4c,
                field_count: 0xac,
            },
            field: FieldInfoOffsets {
                name: 0x0,
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{find, IL2CPPOffsets, BUILDS};
    use crate::PointerSize;

    #[test]
    fn table_reads_oldest_to_newest() {
        assert!(BUILDS
            .windows(2)
            .all(|pair| (pair[0].metadata, pair[0].unity) <= (pair[1].metadata, pair[1].unity)));
    }

    #[test]
    fn finds_exact_builds() {
        let build = find(39, (6000, 3), PointerSize::Bit64).unwrap();
        assert_eq!(build.metadata, 39);
        assert_eq!(build.unity, (6000, 3));

        let narrow = find(39, (6000, 3), PointerSize::Bit32).unwrap();
        assert_eq!(narrow.metadata, 39);
        assert_eq!(narrow.pointer_size, PointerSize::Bit32);
    }

    #[test]
    fn unmeasured_identities_answer_the_newest_build_below() {
        let build = find(29, (2022, 1), PointerSize::Bit64).unwrap();
        assert_eq!((build.metadata, build.unity), (29, (2021, 3)));

        let build = find(35, (6000, 0), PointerSize::Bit64).unwrap();
        assert_eq!((build.metadata, build.unity), (31, (6000, 2)));

        let build = find(200, (7000, 0), PointerSize::Bit64).unwrap();
        assert_eq!((build.metadata, build.unity), (110, (6000, 7)));
    }

    // The version table for 6000.5 and 6000.7 puts static_fields where 2022.3
    // had it. Both measured players put it lower, and 6000.7 moves field_count
    // too.
    #[test]
    fn identities_below_the_oldest_build_answer_nothing() {
        assert!(find(16, (5, 6), PointerSize::Bit64).is_none());
        assert!(find(24, (2018, 4), PointerSize::Bit32).is_none());
    }

    // The version table for 6000.5 and 6000.7 puts static_fields where 2022.3
    // had it. Both measured players put it lower, and 6000.7 moves field_count
    // too.
    #[test]
    fn unity_6000_5_builds_diverge_from_their_version_table_on_statics() {
        for (metadata, unity, static_fields) in [(107, (6000, 5), 0xA0), (110, (6000, 7), 0x98)] {
            let build = find(metadata, unity, PointerSize::Bit64).unwrap();
            let table = IL2CPPOffsets::new(build.version, build.pointer_size).unwrap();
            assert_eq!(build.offsets.class.static_fields, static_fields);
            assert_ne!(build.offsets.class.static_fields, table.class.static_fields);
        }

        let build = find(110, (6000, 7), PointerSize::Bit64).unwrap();
        let table = IL2CPPOffsets::new(build.version, build.pointer_size).unwrap();
        assert_ne!(build.offsets.class.field_count, table.class.field_count);
    }
}

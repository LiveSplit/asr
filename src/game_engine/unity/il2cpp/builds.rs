//! Known IL2CPP builds. Each entry is one measured player. The full Unity
//! version says which player. The offsets come from that player's
//! `GameAssembly.pdb`.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, IL2CPPOffsets, ImageOffsets,
    TypeOffsets,
};
use super::Version;
use crate::PointerSize;

/// One measured IL2CPP player and the offsets from its PDB.
pub(super) struct Build {
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: IL2CPPOffsets,
}

/// Finds the build for a player at its width. The Unity version is the four
/// parts of `UnityPlayer.dll`'s file version, and the last part tells `f1`
/// from `f2` and an alpha from its release. A measured player gets its own
/// build. Any other player gets the newest build whose major.minor is at or
/// below its own, since patch releases of one major.minor share their
/// layouts, or the oldest build when nothing is below. The table reads
/// oldest to newest, so the newest match is the last one.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    let at_width = || {
        BUILDS
            .iter()
            .filter(move |build| build.pointer_size == pointer_size)
    };

    at_width()
        .find(|build| build.unity == unity)
        .or_else(|| at_width().rfind(|build| (build.unity.0, build.unity.1) <= (unity.0, unity.1)))
        .or_else(|| at_width().next())
}

// The table reads from the oldest player to the newest.
static BUILDS: &[Build] = &[
    // Unity 2018.4.36f1, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2018, 4, 36, 54151),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xec),
                field_count: 0x114,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2018.4.36f1, metadata version 24, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2018, 4, 36, 54151),
        pointer_size: PointerSize::Bit32,
        version: Version::Base,
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x84),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2019.4.41f2, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2019, 4, 41, 9172),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf4),
                field_count: 0x11c,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2019.4.41f2, metadata version 24, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2019, 4, 41, 9172),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xa8,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2020, 1, 18, 38512),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf4),
                field_count: 0x11c,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2020, 1, 18, 38512),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xa8,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2021, 3, 11, 23713),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x120,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2021, 3, 11, 23713),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xa8,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2022.3.0f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2022, 3, 0, 4507),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2022.3.0f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2022, 3, 0, 4507),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2023.1.0f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2023, 1, 0, 2298),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2023.1.0f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2023, 1, 0, 2298),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2023, 1, 22, 16744),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (2023, 1, 22, 16744),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 2, 12, 40285),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 2, 12, 40285),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 3, 21, 9777),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xb8,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 3, 21, 9777),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x5c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.5.10f1, metadata version 107, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 5, 10, 54518),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0xa0,
                instance_size: Some(0xf8),
                field_count: 0x124,
            },
            generic: GenericOffsets {
                cached_class: Some(0x18),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.5.10f1, metadata version 107, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 5, 10, 54518),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x50,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0xc),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x64.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 7, 0, 5476),
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
                declaring_type: Some(0x50),
                parent: 0x58,
                fields: 0x80,
                static_fields: 0x98,
                instance_size: Some(0xf0),
                field_count: 0x11c,
            },
            generic: GenericOffsets {
                cached_class: Some(0x10),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                struct_size: 0x20,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x86.
    // Offsets from the player's own GameAssembly.pdb.
    Build {
        unity: (6000, 7, 0, 5476),
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
                declaring_type: Some(0x28),
                parent: 0x2c,
                fields: 0x40,
                static_fields: 0x4c,
                instance_size: Some(0x80),
                field_count: 0xac,
            },
            generic: GenericOffsets {
                cached_class: Some(0x8),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                struct_size: 0x14,
            },
        },
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{nearest, IL2CPPOffsets, BUILDS};
    use crate::PointerSize;

    #[test]
    fn table_reads_oldest_to_newest() {
        assert!(BUILDS.windows(2).all(|pair| pair[0].unity <= pair[1].unity));
    }

    // Every player was measured at both widths, so each version in the
    // table has an x64 entry and an x86 entry.
    #[test]
    fn every_version_is_measured_at_both_widths() {
        for build in BUILDS {
            let other = match build.pointer_size {
                PointerSize::Bit64 => PointerSize::Bit32,
                _ => PointerSize::Bit64,
            };
            let twin = nearest(build.unity, other).unwrap();
            assert_eq!(
                twin.unity, build.unity,
                "{:?} at {:?} has no twin",
                build.unity, other
            );
        }
    }

    // A player nobody measured takes the newest build at or below its
    // major.minor, or the oldest build when nothing is below. A measured
    // player takes its own build even when a newer patch was measured.
    #[test]
    fn nearest_takes_the_newest_build_at_or_below_the_major_minor() {
        let x64 = PointerSize::Bit64;
        let unity = |player| nearest(player, x64).unwrap().unity;
        assert_eq!(unity((2021, 3, 5, 1)), (2021, 3, 11, 23713));
        assert_eq!(unity((2021, 3, 45, 1)), (2021, 3, 11, 23713));
        assert_eq!(unity((2023, 1, 10, 1)), (2023, 1, 22, 16744));
        assert_eq!(unity((2023, 1, 0, 2298)), (2023, 1, 0, 2298));
        assert_eq!(unity((2022, 1, 0, 1)), (2021, 3, 11, 23713));
        assert_eq!(unity((7000, 0, 0, 0)), (6000, 7, 0, 5476));
        assert_eq!(unity((5, 6, 7, 0)), (2018, 4, 36, 54151));
    }

    #[test]
    fn nearest_is_the_build_itself_on_a_measured_version() {
        for build in BUILDS {
            let found = nearest(build.unity, build.pointer_size).unwrap();
            assert_eq!(found.unity, build.unity);
            assert_eq!(found.pointer_size, build.pointer_size);
        }
    }

    #[test]
    fn nearest_keeps_the_width() {
        let build = nearest((2021, 3, 5, 1), PointerSize::Bit32).unwrap();
        assert_eq!(build.unity, (2021, 3, 11, 23713));
        assert_eq!(build.pointer_size, PointerSize::Bit32);
        assert!(nearest((2021, 3, 5, 1), PointerSize::Bit16).is_none());
    }

    // A version table's value for any of the grown members must match every
    // measured build it stands in for, or say nothing.
    #[test]
    fn version_tables_never_contradict_a_measured_build() {
        fn agrees(table: Option<u16>, measured: Option<u16>) -> bool {
            table.is_none() || table == measured
        }

        for build in BUILDS {
            let Some(table) = IL2CPPOffsets::new(build.version, build.pointer_size) else {
                continue;
            };
            assert!(agrees(
                table.class.declaring_type,
                build.offsets.class.declaring_type
            ));
            assert!(agrees(
                table.class.instance_size,
                build.offsets.class.instance_size
            ));
            assert!(agrees(
                table.generic.cached_class,
                build.offsets.generic.cached_class
            ));
            assert!(agrees(table.type_words.data, build.offsets.type_words.data));
            assert!(agrees(table.type_words.kind, build.offsets.type_words.kind));
            assert!(agrees(table.field.type_, build.offsets.field.type_));
        }
    }

    // The version table for 6000.5 and 6000.7 puts static_fields where 2022.3
    // had it. Both measured players put it lower, and 6000.7 moves field_count
    // too.
    #[test]
    fn unity_6000_5_builds_diverge_from_their_version_table_on_statics() {
        for (unity, static_fields) in [((6000, 5, 10, 54518), 0xA0), ((6000, 7, 0, 5476), 0x98)] {
            let build = nearest(unity, PointerSize::Bit64).unwrap();
            let table = IL2CPPOffsets::new(build.version, build.pointer_size).unwrap();
            assert_eq!(build.offsets.class.static_fields, static_fields);
            assert_ne!(build.offsets.class.static_fields, table.class.static_fields);
        }

        let build = nearest((6000, 7, 0, 5476), PointerSize::Bit64).unwrap();
        let table = IL2CPPOffsets::new(build.version, build.pointer_size).unwrap();
        assert_ne!(build.offsets.class.field_count, table.class.field_count);
    }
}

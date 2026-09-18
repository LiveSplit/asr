//! Known IL2CPP builds. Each entry is one measured player. The full Unity
//! version says which player. The offsets come from that player's
//! `GameAssembly.pdb`.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, ImageOffsets, Profile,
    TypeOffsets, TypeStart,
};
use crate::PointerSize;

/// One measured IL2CPP player and the offsets from its PDB.
#[derive(Copy, Clone)]
pub(super) struct Build {
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) profile: Profile,
}

/// Finds the build for a player at its width. The Unity version is the four
/// parts of the file version of `UnityPlayer.dll`. The last part tells `f1`
/// from `f2` and an alpha from its release. A measured player gets its own
/// build. Any other player gets the newest build whose major.minor is at or
/// below the player's major.minor, or the oldest build when no build is
/// below. This answer is right when the table holds the first player of
/// every layout, because a layout lasts until the next layout starts. When a
/// layout changed between two measured players, a player in that gap gets the
/// older layout. The table reads from the oldest player to the newest, so the
/// newest match is the last match.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    let at_width = || {
        BUILDS
            .iter()
            .filter(move |build| build.profile.pointer_size == pointer_size)
    };

    at_width()
        .find(|build| build.unity == unity)
        .or_else(|| at_width().rfind(|build| (build.unity.0, build.unity.1) <= (unity.0, unity.1)))
        .or_else(|| at_width().next())
}

// The table reads from the oldest player to the newest.
pub(super) const BUILDS: &[Build] = &[
    // Unity 2018.4.36f1, metadata version 24, x64.
    Build {
        unity: (2018, 4, 36, 54151),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                type_start: TypeStart::Inline(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2018.4.36f1, metadata version 24, x86.
    Build {
        unity: (2018, 4, 36, 54151),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0x10,
                type_start: TypeStart::Inline(0xc),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2019.1.0f2, metadata version 24, x64.
    Build {
        unity: (2019, 1, 0, 11155),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                type_start: TypeStart::Inline(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2019.1.0f2, metadata version 24, x86.
    Build {
        unity: (2019, 1, 0, 11155),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0x10,
                type_start: TypeStart::Inline(0xc),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2019.4.41f2, metadata version 24, x64.
    Build {
        unity: (2019, 4, 41, 9172),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                type_start: TypeStart::Inline(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2019.4.41f2, metadata version 24, x86.
    Build {
        unity: (2019, 4, 41, 9172),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0x10,
                type_start: TypeStart::Inline(0xc),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x64.
    Build {
        unity: (2020, 1, 18, 38512),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x1c,
                type_start: TypeStart::Inline(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2020.1.18f1, metadata version 24, x86.
    Build {
        unity: (2020, 1, 18, 38512),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0x10,
                type_start: TypeStart::Inline(0xc),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2020.2.0f1, metadata version 27, x64.
    Build {
        unity: (2020, 2, 0, 8671),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2020.2.0f1, metadata version 27, x86.
    Build {
        unity: (2020, 2, 0, 8671),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x64.
    Build {
        unity: (2021, 3, 11, 23713),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2021.3.11f1, metadata version 29, x86.
    Build {
        unity: (2021, 3, 11, 23713),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2022.3.0f1, metadata version 29, x64.
    Build {
        unity: (2022, 3, 0, 4507),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2022.3.0f1, metadata version 29, x86.
    Build {
        unity: (2022, 3, 0, 4507),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2023.1.0f1, metadata version 29, x64.
    Build {
        unity: (2023, 1, 0, 2298),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2023.1.0f1, metadata version 29, x86.
    Build {
        unity: (2023, 1, 0, 2298),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x64.
    Build {
        unity: (2023, 1, 22, 16744),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 2023.1.22f1, metadata version 29, x86.
    Build {
        unity: (2023, 1, 22, 16744),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x64.
    Build {
        unity: (6000, 2, 12, 40285),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 6000.2.12f1, metadata version 31, x86.
    Build {
        unity: (6000, 2, 12, 40285),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x64.
    Build {
        unity: (6000, 3, 21, 9777),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 6000.3.21f1, metadata version 39, x86.
    Build {
        unity: (6000, 3, 21, 9777),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 6000.5.10f1, metadata version 107, x64.
    Build {
        unity: (6000, 5, 10, 54518),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 6000.5.10f1, metadata version 107, x86.
    Build {
        unity: (6000, 5, 10, 54518),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 6000.6.0f1, metadata version 108, x64.
    Build {
        unity: (6000, 6, 0, 63725),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 6000.6.0f1, metadata version 108, x86.
    Build {
        unity: (6000, 6, 0, 63725),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x64.
    Build {
        unity: (6000, 7, 0, 5476),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x8),
                type_count: 0x18,
                type_start: TypeStart::Handle(0x28),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xa),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x8),
                offset: 0x18,
                size: 0x20,
            },
        },
    },
    // Unity 6000.7.0a3, metadata version 110, x86.
    Build {
        unity: (6000, 7, 0, 5476),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            assembly: AssemblyOffsets {
                image: 0x0,
                name: None,
            },
            image: ImageOffsets {
                assembly_name: Some(0x4),
                type_count: 0xc,
                type_start: TypeStart::Handle(0x18),
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
            type_: TypeOffsets {
                data: Some(0x0),
                kind: Some(0x6),
            },
            field: FieldInfoOffsets {
                name: 0x0,
                type_: Some(0x4),
                offset: 0xc,
                size: 0x14,
            },
        },
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{nearest, BUILDS};
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
            let other = match build.profile.pointer_size {
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
        assert_eq!(unity((2019, 2, 0, 1)), (2019, 1, 0, 11155));
        assert_eq!(unity((2020, 3, 48, 1)), (2020, 2, 0, 8671));
        assert_eq!(unity((2021, 1, 0, 1)), (2020, 2, 0, 8671));
        assert_eq!(unity((6000, 6, 5, 1)), (6000, 6, 0, 63725));
        assert_eq!(unity((7000, 0, 0, 0)), (6000, 7, 0, 5476));
        assert_eq!(unity((5, 6, 7, 0)), (2018, 4, 36, 54151));
    }

    // Players before Unity 2020.2 keep the index of the first type inside the
    // image, in the member the PDB calls `typeStart`. From Unity 2020.2 on,
    // the image keeps a pointer in the member the PDB calls `metadataHandle`,
    // and the index sits where that pointer points.
    #[test]
    fn type_start_moves_behind_a_handle_from_2020_2() {
        use super::TypeStart;
        use crate::PointerSize::Bit64;

        for build in BUILDS {
            let (inline, handle) = match build.profile.pointer_size {
                Bit64 => (0x18, 0x28),
                _ => (0xC, 0x18),
            };
            let before_2020_2 = (build.unity.0, build.unity.1) < (2020, 2);
            match build.profile.image.type_start {
                TypeStart::Inline(at) => {
                    assert!(before_2020_2 && at == inline, "{:?}", build.unity)
                }
                TypeStart::Handle(at) => {
                    assert!(!before_2020_2 && at == handle, "{:?}", build.unity)
                }
            }
        }
    }

    #[test]
    fn nearest_is_the_build_itself_on_a_measured_version() {
        for build in BUILDS {
            let found = nearest(build.unity, build.profile.pointer_size).unwrap();
            assert_eq!(found.unity, build.unity);
            assert_eq!(found.profile.pointer_size, build.profile.pointer_size);
        }
    }

    #[test]
    fn nearest_keeps_the_width() {
        let build = nearest((2021, 3, 5, 1), PointerSize::Bit32).unwrap();
        assert_eq!(build.unity, (2021, 3, 11, 23713));
        assert_eq!(build.profile.pointer_size, PointerSize::Bit32);
        assert!(nearest((2021, 3, 5, 1), PointerSize::Bit16).is_none());
    }
}

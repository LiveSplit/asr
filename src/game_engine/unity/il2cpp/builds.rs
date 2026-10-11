//! The IL2CPP builds where the layout changes. Each entry is one measured
//! player, identified by its full Unity version, and the offsets come from
//! that player's `GameAssembly.pdb`. A game on any other player takes the
//! newest entry at or below its patch.

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
/// parts of the file version of `UnityPlayer.dll`, and the first three of
/// them are the patch. A player takes the newest build whose patch is at or
/// below its own patch. A player below every build takes none.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    // The comparison drops the build number, the fourth part, because a later
    // build of one patch can carry a lower number than an earlier build.
    let patch = |version: (u16, u16, u16, u16)| (version.0, version.1, version.2);

    // The table reads from the oldest player to the newest, so the last match
    // is the newest build at or below the player's patch.
    BUILDS
        .iter()
        .filter(|build| build.profile.pointer_size == pointer_size)
        .rfind(|build| patch(build.unity) <= patch(unity))
}

// The table reads from the oldest player to the newest.
pub(super) const BUILDS: &[Build] = &[
    // Unity 2018.1.0f1, metadata version 24, x64.
    Build {
        unity: (2018, 1, 0, 30795),
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
                instance_size: Some(0xe8),
                field_count: 0x110,
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
                size: 0x28,
            },
        },
    },
    // Unity 2018.1.0f1, metadata version 24, x86.
    Build {
        unity: (2018, 1, 0, 30795),
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
                size: 0x18,
            },
        },
    },
    // Unity 2018.2.0f1, metadata version 24, x64.
    Build {
        unity: (2018, 2, 0, 44229),
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
                instance_size: Some(0xf0),
                field_count: 0x118,
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
                size: 0x28,
            },
        },
    },
    // Unity 2018.2.0f1, metadata version 24, x86.
    Build {
        unity: (2018, 2, 0, 44229),
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
                instance_size: Some(0x88),
                field_count: 0xb0,
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
                size: 0x18,
            },
        },
    },
    // Unity 2018.3.0f1, metadata version 24, x64.
    Build {
        unity: (2018, 3, 0, 9156),
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
    // Unity 2018.3.0f1, metadata version 24, x86.
    Build {
        unity: (2018, 3, 0, 9156),
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
    // Unity 2019.1.0f1, metadata version 24, x64.
    Build {
        unity: (2019, 1, 0, 21026),
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
    // Unity 2019.1.0f1, metadata version 24, x86.
    Build {
        unity: (2019, 1, 0, 21026),
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
    // Unity 2022.2.0f1, metadata version 29, x64.
    Build {
        unity: (2022, 2, 0, 56532),
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
    // Unity 2022.2.0f1, metadata version 29, x86.
    Build {
        unity: (2022, 2, 0, 56532),
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
    // Unity 6000.5.0f1, metadata version 106, x64.
    Build {
        unity: (6000, 5, 0, 46204),
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
    // Unity 6000.5.0f1, metadata version 106, x86.
    Build {
        unity: (6000, 5, 0, 46204),
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
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{nearest, BUILDS};
    use crate::PointerSize;
    use std::vec::Vec;

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

    // Checks the rule against every entry of the table, so an entry added or
    // moved needs no change here.
    #[test]
    fn a_player_takes_the_newest_build_at_or_below_its_patch() {
        let patch = |v: (u16, u16, u16, u16)| (v.0, v.1, v.2);
        for pointer_size in [PointerSize::Bit64, PointerSize::Bit32] {
            let picked = |player| nearest(player, pointer_size).unwrap().unity;
            let table: Vec<_> = BUILDS
                .iter()
                .filter(|build| build.profile.pointer_size == pointer_size)
                .map(|build| build.unity)
                .collect();

            assert!(nearest((0, 0, 0, 0), pointer_size).is_none());
            assert_eq!(picked((u16::MAX, 0, 0, 0)), *table.last().unwrap());

            for (index, &entry) in table.iter().enumerate() {
                for build in [0, u16::MAX] {
                    let player = (entry.0, entry.1, entry.2, build);
                    assert_eq!(patch(picked(player)), patch(entry), "{player:?}");
                }
                // The version right below an entry reads the entry before it,
                // across a major or minor too, and nothing below the first entry.
                let below = match entry {
                    (major, minor, number, _) if number > 0 => (major, minor, number - 1, u16::MAX),
                    (major, minor, _, _) if minor > 0 => (major, minor - 1, u16::MAX, u16::MAX),
                    (major, _, _, _) => (major - 1, u16::MAX, u16::MAX, u16::MAX),
                };
                if index == 0 {
                    assert!(nearest(below, pointer_size).is_none(), "{below:?}");
                } else {
                    assert_eq!(picked(below), table[index - 1], "{below:?}");
                }
            }
        }
    }

    // Every entry starts a layout: its offsets differ from the entry
    // before it at its pointer size.
    #[test]
    fn each_build_starts_a_layout() {
        for pointer_size in [PointerSize::Bit64, PointerSize::Bit32] {
            let mut at_size = BUILDS
                .iter()
                .filter(|build| build.profile.pointer_size == pointer_size);
            let mut before = at_size.next().unwrap();
            for build in at_size {
                assert_ne!(build.profile, before.profile, "{:?}", build.unity);
                before = build;
            }
        }
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
        assert_eq!(build.unity, (2020, 2, 0, 8671));
        assert_eq!(build.profile.pointer_size, PointerSize::Bit32);
        assert!(nearest((2021, 3, 5, 1), PointerSize::Bit16).is_none());
    }
}

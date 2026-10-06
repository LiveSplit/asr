//! The Linux Mono builds where the layout changes, each paired with the
//! offsets measured from it. A game on any other build takes the newest
//! entry at or below its version.
//!
//! A build is told by its build ID, which a linker computes from the binary
//! it writes. Mono's library has one through 2019.4 and none after, so the
//! ID of `UnityPlayer.so` stands in for the later builds. Each entry says
//! which file its ID was read from.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoVTableOffsets, Profile, TypeOffsets,
};
use super::{builds::nearest_or_oldest, Library};
use crate::PointerSize;

/// One exact Mono library and the offsets measured from it.
pub(super) struct Build {
    pub(super) build_id: &'static [u8],
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) profile: Profile,
}

/// Finds the build for a player that is no known build, by the rule of
/// [`nearest_or_oldest`].
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    library: Library,
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    nearest_or_oldest(
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

/// Looks a build up by the ID read from the module that names it.
pub(super) fn find(build_id: &[u8]) -> Option<&'static Build> {
    BUILDS.iter().find(|build| build.build_id == build_id)
}

/// Parses a build ID from the hex it is normally written as, into the bytes
/// the binary stores.
const fn id<const N: usize>(written: &str) -> [u8; N] {
    const fn hex(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("The build ID is not lowercase hex."),
        }
    }

    let written = written.as_bytes();
    assert!(written.len() == 2 * N, "The build ID is the wrong length.");

    let mut parsed = [0; N];
    let mut index = 0;
    while index < N {
        parsed[index] = (hex(written[2 * index]) << 4) | hex(written[2 * index + 1]);
        index += 1;
    }

    parsed
}

pub(super) const BUILDS: &[Build] = &[
    // 5.6.0f1, libmono.so
    Build {
        build_id: &id::<20>("83bc621e6ccf123cafbc71c0894207071968f29e"),
        unity: (5, 6, 0, 23754),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x3D0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: None,
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x40,
                namespace: 0x48,
                vtable_size: 0x18,
                fields: 0xA0,
                runtime_info: 0xF0,
                field_count: 0x8C,
                next_class_cache: 0xF8,
            },
            generic: GenericOffsets {
                generic_class: Some(0xD0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                stride: 0x20,
            },
            // Nothing reads this: these builds keep their statics in the slot
            // `vtable_size` points at.
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // 2017.1.0f1, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("488594493d54ab4951119e260d8d039d0fb94b53"),
        unity: (2017, 1, 0, 32737),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::MonoBdwgc,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x4A8,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: None,
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x40,
                namespace: 0x48,
                vtable_size: 0x54,
                fields: 0xA8,
                runtime_info: 0xF0,
                field_count: 0x8C,
                next_class_cache: 0xF8,
            },
            generic: GenericOffsets {
                generic_class: Some(0xD8),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
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
    // 2017.2.0f1, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("a276fd2ef4fd4fe54270bf3a1d8cc294f32c8684"),
        unity: (2017, 2, 0, 58714),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::MonoBdwgc,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x4C0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: Some(0x24),
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x40,
                namespace: 0x48,
                vtable_size: 0x54,
                fields: 0x90,
                runtime_info: 0xC8,
                field_count: 0xF8,
                next_class_cache: 0x100,
            },
            generic: GenericOffsets {
                generic_class: Some(0xE8),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
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
    // 2017.4.6f1, libmono.so
    Build {
        build_id: &id::<20>("806bb9c8b6bbb0b9aa964d423694bdd5d29aee4c"),
        unity: (2017, 4, 6, 20272),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x3D0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: None,
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x18,
                fields: 0xA8,
                runtime_info: 0xF8,
                field_count: 0x94,
                next_class_cache: 0x100,
            },
            generic: GenericOffsets {
                generic_class: Some(0xD8),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                stride: 0x20,
            },
            // Nothing reads this: these builds keep their statics in the slot
            // `vtable_size` points at.
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // 2018.1.0f1, libmono.so
    Build {
        build_id: &id::<20>("8297a83ffb74d772baf82ebd05fa663e0bf9feda"),
        unity: (2018, 1, 0, 30795),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x3D0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: None,
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x40,
                namespace: 0x48,
                vtable_size: 0x18,
                fields: 0xA0,
                runtime_info: 0xF0,
                field_count: 0x8C,
                next_class_cache: 0xF8,
            },
            generic: GenericOffsets {
                generic_class: Some(0xD0),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                stride: 0x20,
            },
            // Nothing reads this: these builds keep their statics in the slot
            // `vtable_size` points at.
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // 2018.2.0f2, libmono.so
    Build {
        build_id: &id::<20>("f14b1787abe509c65d8b9b1bf32de20818303c6e"),
        unity: (2018, 2, 0, 30296),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::Mono,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x58,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x3D0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: None,
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x48,
                namespace: 0x50,
                vtable_size: 0x18,
                fields: 0xA8,
                runtime_info: 0xF8,
                field_count: 0x94,
                next_class_cache: 0x100,
            },
            generic: GenericOffsets {
                generic_class: Some(0xD8),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
            },
            field: FieldInfoOffsets {
                type_: Some(0x0),
                name: 0x8,
                offset: 0x18,
                stride: 0x20,
            },
            // Nothing reads this: these builds keep their statics in the slot
            // `vtable_size` points at.
            v_table: MonoVTableOffsets { vtable: 0x48 },
        },
    },
    // 2021.2.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("18a01e8f0aa6d238"),
        unity: (2021, 2, 0, 61932),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            library: Library::MonoBdwgc,
            assembly: AssemblyOffsets {
                aname: Some(0x10),
                image: 0x60,
            },
            image: ImageOffsets {
                assembly_name: None,
                class_cache: 0x4D0,
            },
            hash_table: HashTableOffsets {
                size: 0x18,
                table: 0x20,
            },
            class: ClassOffsets {
                class_kind: Some(0x1B),
                instance_size: Some(0x1C),
                parent: 0x28,
                nested_in: Some(0x30),
                name: 0x40,
                namespace: 0x48,
                vtable_size: 0x54,
                fields: 0x90,
                runtime_info: 0xC8,
                field_count: 0xF8,
                next_class_cache: 0x100,
            },
            generic: GenericOffsets {
                generic_class: Some(0xE8),
                container_class: Some(0x0),
            },
            type_words: TypeOffsets {
                data: Some(0x0),
                kind: Some(0xA),
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
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{find, id, BUILDS};
    use crate::PointerSize;

    // The build ID of 2017.2's Mono library, as it is stored.
    const STORED: [u8; 20] = [
        0xA2, 0x76, 0xFD, 0x2E, 0xF4, 0xFD, 0x4F, 0xE5, 0x42, 0x70, 0xBF, 0x3A, 0x1D, 0x8C, 0xC2,
        0x94, 0xF3, 0x2C, 0x86, 0x84,
    ];

    #[test]
    fn parses_written_build_ids_into_stored_bytes() {
        assert_eq!(id::<20>("a276fd2ef4fd4fe54270bf3a1d8cc294f32c8684"), STORED);
    }

    #[test]
    fn table_names_each_build_once() {
        for (index, build) in BUILDS.iter().enumerate() {
            assert!(!BUILDS[..index]
                .iter()
                .any(|earlier| earlier.build_id == build.build_id));
        }
    }

    #[test]
    fn finds_known_builds() {
        let build = find(&STORED).unwrap();
        assert_eq!(build.profile.pointer_size, PointerSize::Bit64);

        assert!(find(&[0; 20]).is_none());
        assert!(find(&[]).is_none());
    }
}

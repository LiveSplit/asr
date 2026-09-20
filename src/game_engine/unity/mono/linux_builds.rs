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
use super::{builds::nearest_by_version, Library};
use crate::PointerSize;

/// One exact Mono library and the offsets measured from it.
pub(super) struct Build {
    pub(super) build_id: &'static [u8],
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) profile: Profile,
}

/// Finds the build for a player that is no known build, by the rule of
/// [`nearest_by_version`].
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

// 5.6 through 2017.3
const UNITY_5_6: Profile = Profile {
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
        alignment: 0x20,
    },
    // Nothing reads this: these builds keep their statics in the slot
    // `vtable_size` points at.
    v_table: MonoVTableOffsets { vtable: 0x48 },
};

// 2017.4 on
const UNITY_2017_4: Profile = Profile {
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
        alignment: 0x20,
    },
    // Nothing reads this: these builds keep their statics in the slot
    // `vtable_size` points at.
    v_table: MonoVTableOffsets { vtable: 0x48 },
};

// 2018.4 through 2021.1
const UNITY_2018_4: Profile = Profile {
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
        alignment: 0x20,
    },
    v_table: MonoVTableOffsets { vtable: 0x40 },
};

// 2021.2 on
const UNITY_2021_2: Profile = Profile {
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
        alignment: 0x20,
    },
    v_table: MonoVTableOffsets { vtable: 0x48 },
};

pub(super) const BUILDS: &[Build] = &[
    // 5.6.7f1, libmono.so
    Build {
        build_id: &id::<20>("c1a53ea7109a2da58220ab30f4cab7c8ce8f3813"),
        unity: (5, 6, 7, 3267),
        profile: UNITY_5_6,
    },
    // 2017.4.40f1, libmono.so
    Build {
        build_id: &id::<20>("93b5b95d7a6112b3f7d53b40e79a663cbcd62b14"),
        unity: (2017, 4, 40, 5126),
        profile: UNITY_2017_4,
    },
    // 2018.4.36f1, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("dd788d1860d9a782468cb7304637dcdb7fbfd289"),
        unity: (2018, 4, 36, 54151),
        profile: UNITY_2018_4,
    },
    // 2021.2.20f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("8efc4cbe377f5467"),
        unity: (2021, 2, 20, 62729),
        profile: UNITY_2021_2,
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{find, id, BUILDS};
    use crate::PointerSize;

    // The build ID of 2018.4's Mono library, as it is stored.
    const STORED: [u8; 20] = [
        0xDD, 0x78, 0x8D, 0x18, 0x60, 0xD9, 0xA7, 0x82, 0x46, 0x8C, 0xB7, 0x30, 0x46, 0x37, 0xDC,
        0xDB, 0x7F, 0xBF, 0xD2, 0x89,
    ];

    #[test]
    fn parses_written_build_ids_into_stored_bytes() {
        assert_eq!(id::<20>("dd788d1860d9a782468cb7304637dcdb7fbfd289"), STORED);
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

//! Known Linux Mono builds: exact runtime binaries paired with the offsets
//! measured from them.
//!
//! A build is named by its build ID, which a linker computes from the binary
//! it writes, so it names that one build and nothing else. Mono's library
//! has one through 2019.4 and none after, so `UnityPlayer.so`'s ID names the
//! later builds instead. Each entry names the file its ID was read from.

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

// 5.6
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

// 2017.4
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

// 2018.4, 2019.4
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

// 2021.3 - 6000.7
const UNITY_2021_3: Profile = Profile {
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
    // 2019.4.41f2, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("4e690f264a2a90120347f94ae43b0b83d578f686"),
        unity: (2019, 4, 41, 9172),
        profile: UNITY_2018_4,
    },
    // 2021.3.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("c19105ba5aabaf80"),
        unity: (2021, 3, 0, 44232),
        profile: UNITY_2021_3,
    },
    // 2021.3.11f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("1ecf45334b2b3190"),
        unity: (2021, 3, 11, 23713),
        profile: UNITY_2021_3,
    },
    // 2022.3.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("03da1f34b6af4765"),
        unity: (2022, 3, 0, 4507),
        profile: UNITY_2021_3,
    },
    // 2023.1.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("63be5bab8a2fbae2"),
        unity: (2023, 1, 0, 2298),
        profile: UNITY_2021_3,
    },
    // 2023.1.22f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("3bc89a83403a19ca"),
        unity: (2023, 1, 22, 16744),
        profile: UNITY_2021_3,
    },
    // 6000.2.12f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("bde00f619381ee2e39d0919cc82f1bb7fd314f21"),
        unity: (6000, 2, 12, 40285),
        profile: UNITY_2021_3,
    },
    // 6000.3.21f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("ad56ac1afbcf42b846610f49f2b6b78d9f24035f"),
        unity: (6000, 3, 21, 9777),
        profile: UNITY_2021_3,
    },
    // 6000.5.8f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("ac33e63fb791d385766540ef0d21b4a6677edf71"),
        unity: (6000, 5, 8, 47071),
        profile: UNITY_2021_3,
    },
    // 6000.7.0a3, UnityPlayer.so
    Build {
        build_id: &id::<20>("c2e66208668984ba644c54c277f63e537a57f00e"),
        unity: (6000, 7, 0, 5476),
        profile: UNITY_2021_3,
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::{find, id, BUILDS};
    use crate::PointerSize;

    // The build ID of 2019.4's Mono library, as it is stored.
    const STORED: [u8; 20] = [
        0x4E, 0x69, 0x0F, 0x26, 0x4A, 0x2A, 0x90, 0x12, 0x03, 0x47, 0xF9, 0x4A, 0xE4, 0x3B, 0x0B,
        0x83, 0xD5, 0x78, 0xF6, 0x86,
    ];

    #[test]
    fn parses_written_build_ids_into_stored_bytes() {
        assert_eq!(id::<20>("4e690f264a2a90120347f94ae43b0b83d578f686"), STORED);
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

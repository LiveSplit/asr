//! Known Linux Mono builds: exact runtime binaries paired with the offsets
//! measured from them.
//!
//! A build is named by its build ID, which a linker computes from the binary
//! it writes, so it names that one build and nothing else. Mono's library
//! has one through 2019.4 and none after, so `UnityPlayer.so`'s ID names the
//! later builds instead. Each entry names the file its ID was read from.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoOffsets, MonoVTableOffsets, TypeOffsets,
};
use super::Version;
use crate::PointerSize;

/// One exact Mono library and the offsets measured from it.
pub(super) struct Build {
    pub(super) build_id: &'static [u8],
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: &'static MonoOffsets,
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
static UNITY_5_6: MonoOffsets = MonoOffsets {
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
static UNITY_2017_4: MonoOffsets = MonoOffsets {
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
static UNITY_2018_4: MonoOffsets = MonoOffsets {
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
static UNITY_2021_3: MonoOffsets = MonoOffsets {
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

static BUILDS: &[Build] = &[
    // 5.6.7f1, libmono.so
    Build {
        build_id: &id::<20>("c1a53ea7109a2da58220ab30f4cab7c8ce8f3813"),
        pointer_size: PointerSize::Bit64,
        version: Version::V1,
        offsets: &UNITY_5_6,
    },
    // 2017.4.40f1, libmono.so
    Build {
        build_id: &id::<20>("93b5b95d7a6112b3f7d53b40e79a663cbcd62b14"),
        pointer_size: PointerSize::Bit64,
        version: Version::V1Cattrs,
        offsets: &UNITY_2017_4,
    },
    // 2018.4.36f1, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("dd788d1860d9a782468cb7304637dcdb7fbfd289"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: &UNITY_2018_4,
    },
    // 2019.4.41f2, libmonobdwgc-2.0.so
    Build {
        build_id: &id::<20>("4e690f264a2a90120347f94ae43b0b83d578f686"),
        pointer_size: PointerSize::Bit64,
        version: Version::V2,
        offsets: &UNITY_2018_4,
    },
    // 2021.3.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("c19105ba5aabaf80"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 2021.3.11f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("1ecf45334b2b3190"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 2022.3.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("03da1f34b6af4765"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 2023.1.0f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("63be5bab8a2fbae2"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 2023.1.22f1, UnityPlayer.so
    Build {
        build_id: &id::<8>("3bc89a83403a19ca"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 6000.2.12f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("bde00f619381ee2e39d0919cc82f1bb7fd314f21"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 6000.3.21f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("ad56ac1afbcf42b846610f49f2b6b78d9f24035f"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 6000.5.8f1, UnityPlayer.so
    Build {
        build_id: &id::<20>("ac33e63fb791d385766540ef0d21b4a6677edf71"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
    // 6000.7.0a3, UnityPlayer.so
    Build {
        build_id: &id::<20>("c2e66208668984ba644c54c277f63e537a57f00e"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_2021_3,
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::super::{BinaryFormat, Version};
    use super::{find, id, MonoOffsets, BUILDS};
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
        assert_eq!(build.pointer_size, PointerSize::Bit64);
        assert!(matches!(build.version, Version::V2));

        assert!(find(&[0; 20]).is_none());
        assert!(find(&[]).is_none());
    }

    // Pairs each member a measurement can supply with its name, so both tests
    // compare the same set and a failure says which member it was.
    fn grown(offsets: &MonoOffsets) -> [(&'static str, Option<u16>); 8] {
        [
            ("class_kind", offsets.class.class_kind),
            ("instance_size", offsets.class.instance_size),
            ("nested_in", offsets.class.nested_in),
            ("generic_class", offsets.generic.generic_class),
            ("container_class", offsets.generic.container_class),
            ("type data", offsets.type_words.data),
            ("type kind", offsets.type_words.kind),
            ("field type", offsets.field.type_),
        ]
    }

    // A version table stands in for the builds nobody measured. Two builds
    // agreeing on a member is what justifies that, so the table has to carry
    // what they agree on. One build on its own proves nothing.
    #[test]
    fn version_tables_carry_what_two_builds_agree_on() {
        for build in BUILDS {
            let agreeing = BUILDS.iter().filter(|other| {
                core::mem::discriminant(&other.version) == core::mem::discriminant(&build.version)
                    && other.pointer_size == build.pointer_size
            });
            let mut agreed = grown(build.offsets).map(|(name, measured)| (name, Some(measured)));
            let mut count = 0;
            for other in agreeing {
                count += 1;
                for (slot, (_, measured)) in grown(other.offsets).into_iter().enumerate() {
                    if agreed[slot].1 != Some(measured) {
                        agreed[slot].1 = None;
                    }
                }
            }
            if count < 2 {
                continue;
            }

            let Some(table) =
                MonoOffsets::new(build.version, build.pointer_size, BinaryFormat::ELF)
            else {
                continue;
            };
            for (slot, (name, carried)) in grown(table).into_iter().enumerate() {
                let (_, agreed) = agreed[slot];
                if let Some(agreed) = agreed {
                    assert_eq!(carried, agreed, "the table says nothing about {name}");
                }
            }
        }
    }

    // A version table's value for any member must match every measured build
    // it stands in for, or say nothing.
    #[test]
    fn version_tables_never_contradict_a_measured_build() {
        fn agrees(table: Option<u16>, measured: Option<u16>) -> bool {
            table.is_none() || table == measured
        }

        for build in BUILDS {
            let Some(table) =
                MonoOffsets::new(build.version, build.pointer_size, BinaryFormat::ELF)
            else {
                continue;
            };
            let measured = build.offsets;

            assert_eq!(table.assembly.image, measured.assembly.image);
            assert_eq!(table.image.class_cache, measured.image.class_cache);
            assert_eq!(table.hash_table.size, measured.hash_table.size);
            assert_eq!(table.hash_table.table, measured.hash_table.table);
            assert_eq!(table.class.parent, measured.class.parent);
            assert_eq!(table.class.name, measured.class.name);
            assert_eq!(table.class.namespace, measured.class.namespace);
            assert_eq!(table.class.vtable_size, measured.class.vtable_size);
            assert_eq!(table.class.fields, measured.class.fields);
            assert_eq!(table.class.runtime_info, measured.class.runtime_info);
            assert_eq!(table.class.field_count, measured.class.field_count);
            assert_eq!(
                table.class.next_class_cache,
                measured.class.next_class_cache,
            );
            assert_eq!(table.field.name, measured.field.name);
            assert_eq!(table.field.offset, measured.field.offset);
            assert_eq!(table.field.alignment, measured.field.alignment);

            assert!(agrees(table.class.class_kind, measured.class.class_kind));
            assert!(agrees(
                table.class.instance_size,
                measured.class.instance_size
            ));
            assert!(agrees(table.class.nested_in, measured.class.nested_in));
            assert!(agrees(
                table.generic.generic_class,
                measured.generic.generic_class
            ));
            assert!(agrees(
                table.generic.container_class,
                measured.generic.container_class
            ));
            assert!(agrees(table.type_words.data, measured.type_words.data));
            assert!(agrees(table.type_words.kind, measured.type_words.kind));
            assert!(agrees(table.field.type_, measured.field.type_));
        }
    }
}

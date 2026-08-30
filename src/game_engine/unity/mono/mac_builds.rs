//! Known Mac Mono builds: exact runtime binaries paired with the offsets
//! measured from them.
//!
//! A build is named by its UUID, which a linker derives from the binary it
//! writes, so it names that one build and nothing else. A Mac library holds a
//! slice per architecture, each linked separately with its own UUID, so one
//! build carries as many UUIDs as it has slices.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoOffsets, MonoVTableOffsets, TypeOffsets,
};
use super::Version;
use crate::PointerSize;

/// One exact Mono library and the offsets measured from it.
pub(super) struct Build {
    pub(super) uuid: [u8; 16],
    pub(super) pointer_size: PointerSize,
    pub(super) version: Version,
    pub(super) offsets: &'static MonoOffsets,
}

/// Looks a build up by the UUID read from the library.
pub(super) fn find(uuid: &[u8; 16]) -> Option<&'static Build> {
    BUILDS.iter().find(|build| &build.uuid == uuid)
}

/// Parses a UUID from the form it is normally written in, into the bytes the
/// load command stores.
const fn id(written: &str) -> [u8; 16] {
    const fn hex(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("The UUID is not uppercase hex."),
        }
    }

    let written = written.as_bytes();
    assert!(written.len() == 36, "The UUID is the wrong length.");

    let mut parsed = [0; 16];
    let mut index = 0;
    let mut at = 0;
    while index < 16 {
        if written[at] == b'-' {
            at += 1;
            continue;
        }
        parsed[index] = (hex(written[at]) << 4) | hex(written[at + 1]);
        index += 1;
        at += 2;
    }

    parsed
}

// 6000.5
static UNITY_6000_5: MonoOffsets = MonoOffsets {
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
    // 6000.5.10f1, x86_64
    Build {
        uuid: id("7FB2E193-873C-3C2F-B045-4F074BD06996"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_6000_5,
    },
    // 6000.5.10f1, arm64
    Build {
        uuid: id("E9C5E06A-62E5-3A9A-9D5D-54F0D16FFFA8"),
        pointer_size: PointerSize::Bit64,
        version: Version::V3,
        offsets: &UNITY_6000_5,
    },
];

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::super::{BinaryFormat, Version};
    use super::{find, id, MonoOffsets, BUILDS};
    use crate::PointerSize;

    // The arm64 slice of 6000.5.10f1, as the load command stores it.
    const STORED: [u8; 16] = [
        0xE9, 0xC5, 0xE0, 0x6A, 0x62, 0xE5, 0x3A, 0x9A, 0x9D, 0x5D, 0x54, 0xF0, 0xD1, 0x6F, 0xFF,
        0xA8,
    ];

    #[test]
    fn parses_written_uuids_into_stored_bytes() {
        assert_eq!(id("E9C5E06A-62E5-3A9A-9D5D-54F0D16FFFA8"), STORED);
    }

    #[test]
    fn table_names_each_slice_once() {
        for (index, build) in BUILDS.iter().enumerate() {
            assert!(!BUILDS[..index]
                .iter()
                .any(|earlier| earlier.uuid == build.uuid));
        }
    }

    #[test]
    fn finds_known_builds() {
        let build = find(&STORED).unwrap();
        assert_eq!(build.pointer_size, PointerSize::Bit64);
        assert!(matches!(build.version, Version::V3));

        assert!(find(&[0; 16]).is_none());
    }

    // One build says nothing about the versions a table stands in for, so the
    // Mach-O tables stay silent about every member a measurement could fill.
    #[test]
    fn version_tables_say_nothing_a_single_build_would_have_to_carry() {
        for build in BUILDS {
            let Some(table) =
                MonoOffsets::new(build.version, build.pointer_size, BinaryFormat::MachO)
            else {
                continue;
            };

            assert_eq!(table.class.class_kind, None);
            assert_eq!(table.class.instance_size, None);
            assert_eq!(table.class.nested_in, None);
            assert_eq!(table.generic.generic_class, None);
            assert_eq!(table.generic.container_class, None);
            assert_eq!(table.type_words.data, None);
            assert_eq!(table.type_words.kind, None);
            assert_eq!(table.field.type_, None);

            let measured = build.offsets;
            assert_eq!(table.assembly.image, measured.assembly.image);
            assert_eq!(table.image.class_cache, measured.image.class_cache);
            assert_eq!(table.class.parent, measured.class.parent);
            assert_eq!(table.class.name, measured.class.name);
            assert_eq!(table.class.namespace, measured.class.namespace);
            assert_eq!(table.class.fields, measured.class.fields);
            assert_eq!(table.class.field_count, measured.class.field_count);
            assert_eq!(table.v_table.vtable, measured.v_table.vtable);
        }
    }
}

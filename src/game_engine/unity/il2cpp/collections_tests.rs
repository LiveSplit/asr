//! Tests pinning dictionary resolution over hand-laid class metadata at the
//! literal offsets of the Unity 2019.4 IL2CPP runtime. The entry class is a
//! generic instance reached through the instantiation's cached class, which
//! is the route every corlib dictionary takes.

use super::super::managed::DictionaryShape;
use super::Module;
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x60_0000;
const MEASURED_2019: (u16, u16, u16, u16) = (2019, 4, 41, 9172);

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

// One field entry: the name heads it, the Il2CppType pointer sits at 0x8,
// the offset at 0x18.
fn field(image: &mut [u8], at: u64, name: u64, type_: u64, offset: i32) {
    ptr(image, at, name);
    if type_ != 0 {
        ptr(image, at + 0x8, type_);
    }
    put(image, at + 0x18, &offset.to_le_bytes());
}

// The image is laid out by hand from the GameAssembly.pdb of the Unity 2019.4
// player, not from the player's entry in the table, so a wrong entry fails
// against this image. The 2019.4 player counts fields at 0x11C.
fn image(unity: (u16, u16, u16, u16)) -> Vec<u8> {
    let field_count_at = match unity {
        MEASURED_2019 => 0x11C,
        other => panic!("no hand-laid image for {other:?}"),
    };
    let mut i = vec![0; 0x3000];

    let strings = [
        (0x1800, "_buckets"),
        (0x1840, "_entries"),
        (0x1880, "_count"),
        (0x18C0, "_freeCount"),
        (0x1900, "hashCode"),
        (0x1940, "next"),
        (0x1980, "key"),
        (0x19C0, "value"),
        (0x1A00, "Dictionary`2"),
        (0x1A40, "System.Collections.Generic"),
        (0x1A80, "DerivedDictionary"),
        (0x1AC0, "DictionaryLookalike"),
        (0x1B00, "_slots"),
        (0x1B40, "_lastIndex"),
        (0x1B80, "HashSet`1"),
        (0x1BC0, "DerivedHashSet"),
        (0x1C00, "HashSetLookalike"),
    ];
    for (at, text) in strings {
        put(&mut i, at, text.as_bytes());
    }

    // The slot, the dictionary object heading with its class, and the class's
    // four fields: field_count at +0x11C, fields behind +0x80.
    ptr(&mut i, 0x0, BASE + 0x100);
    ptr(&mut i, 0x8, BASE + 0x2200);
    ptr(&mut i, 0x10, BASE + 0x2600);
    ptr(&mut i, 0x18, BASE + 0x2700);
    ptr(&mut i, 0x100, BASE + 0x200);
    ptr(&mut i, 0x200 + 0x10, BASE + 0x1A00);
    ptr(&mut i, 0x200 + 0x18, BASE + 0x1A40);
    put(&mut i, 0x200 + field_count_at, &4_u16.to_le_bytes());
    ptr(&mut i, 0x200 + 0x80, BASE + 0x340);
    field(&mut i, 0x340, BASE + 0x1800, 0, 0x10);
    field(&mut i, 0x360, BASE + 0x1840, BASE + 0x500, 0x18);
    field(&mut i, 0x380, BASE + 0x1880, 0, 0x20);
    field(&mut i, 0x3A0, BASE + 0x18C0, 0, 0x24);

    // The entries field's type: a SzArray whose data is the element's own
    // type, a generic instance whose descriptor caches the entry class.
    ptr(&mut i, 0x500, BASE + 0x550);
    put(&mut i, 0x50A, &[0x1D]); // SzArray
    ptr(&mut i, 0x550, BASE + 0x580);
    put(&mut i, 0x55A, &[0x15]); // GenericInst
    ptr(&mut i, 0x580 + 0x18, BASE + 0x600); // descriptor: cached_class

    // The entry class: instance_size at +0xF4, field_count at +0x11C, four
    // members at their boxed-frame offsets.
    put(&mut i, 0x600 + 0xF4, &0x20_i32.to_le_bytes());
    put(&mut i, 0x600 + field_count_at, &4_u16.to_le_bytes());
    ptr(&mut i, 0x600 + 0x80, BASE + 0x740);
    field(&mut i, 0x740, BASE + 0x1900, 0, 0x10);
    field(&mut i, 0x760, BASE + 0x1940, 0, 0x14);
    field(&mut i, 0x780, BASE + 0x1980, 0, 0x18);
    field(&mut i, 0x7A0, BASE + 0x19C0, 0, 0x1C);

    // The dictionary's live state: three counted entries over a four-slot
    // backing, the middle one freed, so two pairs are live.
    ptr(&mut i, 0x118, BASE + 0x800);
    put(&mut i, 0x120, &3_i32.to_le_bytes());
    put(&mut i, 0x124, &1_i32.to_le_bytes());
    put(&mut i, 0x818, &4_u32.to_le_bytes());
    for (index, entry) in [(1111, -1, 10, 100), (-1, -1, 99, 999), (2222, -1, 20, 200)]
        .into_iter()
        .enumerate()
    {
        let at = 0x820 + 0x10 * index as u64;
        let (hash, next, key, value): (i32, i32, i32, i32) = entry;
        for (word, value) in [hash, next, key, value].into_iter().enumerate() {
            put(&mut i, at + 4 * word as u64, &value.to_le_bytes());
        }
    }

    // A hash set, its slot class the entry class without the key claim: two
    // live values around a freed slot, the high-water mark past the count.
    ptr(&mut i, 0x20, BASE + 0x900);
    ptr(&mut i, 0x900, BASE + 0xA00);
    ptr(&mut i, 0xA00 + 0x10, BASE + 0x1B80);
    ptr(&mut i, 0xA00 + 0x18, BASE + 0x1A40);
    put(&mut i, 0xA00 + field_count_at, &4_u16.to_le_bytes());
    ptr(&mut i, 0xA00 + 0x80, BASE + 0xB40);
    field(&mut i, 0xB40, BASE + 0x1800, 0, 0x10);
    field(&mut i, 0xB60, BASE + 0x1B00, BASE + 0x500, 0x18);
    field(&mut i, 0xB80, BASE + 0x1880, 0, 0x20);
    field(&mut i, 0xBA0, BASE + 0x1B40, 0, 0x24);
    ptr(&mut i, 0x918, BASE + 0xC00);
    put(&mut i, 0x920, &2_i32.to_le_bytes());
    put(&mut i, 0x924, &3_i32.to_le_bytes());
    put(&mut i, 0xC18, &4_u32.to_le_bytes());
    for (index, entry) in [(111, -1, 0, 7), (-1, -1, 0, 0), (222, -1, 0, 9)]
        .into_iter()
        .enumerate()
    {
        let at = 0xC20 + 0x10 * index as u64;
        let (hash, next, key, value): (i32, i32, i32, i32) = entry;
        for (word, value) in [hash, next, key, value].into_iter().enumerate() {
            put(&mut i, at + 4 * word as u64, &value.to_le_bytes());
        }
    }

    // A derived hash set inherits the collection fields from the corlib
    // HashSet class rather than declaring them itself.
    ptr(&mut i, 0xD00 + 0x10, BASE + 0x1BC0);
    ptr(&mut i, 0xD00 + 0x18, BASE + 0x1A40);
    ptr(&mut i, 0xD00 + 0x58, BASE + 0xA00);
    ptr(&mut i, 0xE00, BASE + 0xD00);
    ptr(&mut i, 0xE00 + 0x18, BASE + 0xC00);
    put(&mut i, 0xE00 + 0x20, &2_i32.to_le_bytes());
    put(&mut i, 0xE00 + 0x24, &3_i32.to_le_bytes());
    ptr(&mut i, 0x28, BASE + 0xE00);

    // Matching private field names do not make an unrelated class a hash
    // set.
    ptr(&mut i, 0xF00 + 0x10, BASE + 0x1C00);
    ptr(&mut i, 0xF00 + 0x18, BASE + 0x1A40);
    put(&mut i, 0xF00 + field_count_at, &4_u16.to_le_bytes());
    ptr(&mut i, 0xF00 + 0x80, BASE + 0x1040);
    field(&mut i, 0x1040, BASE + 0x1800, 0, 0x10);
    field(&mut i, 0x1060, BASE + 0x1B00, BASE + 0x500, 0x18);
    field(&mut i, 0x1080, BASE + 0x1880, 0, 0x20);
    field(&mut i, 0x10A0, BASE + 0x1B40, 0, 0x24);
    ptr(&mut i, 0x1100, BASE + 0xF00);
    ptr(&mut i, 0x30, BASE + 0x1100);

    // A newly constructed hash set has zero counts and no backing slots
    // array until its first insertion.
    ptr(&mut i, 0x1200, BASE + 0xA00);
    ptr(&mut i, 0x38, BASE + 0x1200);

    // A derived dictionary inherits the collection fields from the corlib
    // Dictionary class rather than declaring them itself.
    ptr(&mut i, 0x2000 + 0x10, BASE + 0x1A80);
    ptr(&mut i, 0x2000 + 0x18, BASE + 0x1A40);
    ptr(&mut i, 0x2000 + 0x58, BASE + 0x200);
    ptr(&mut i, 0x2200, BASE + 0x2000);
    ptr(&mut i, 0x2200 + 0x18, BASE + 0x800);
    put(&mut i, 0x2200 + 0x20, &3_i32.to_le_bytes());
    put(&mut i, 0x2200 + 0x24, &1_i32.to_le_bytes());

    // Matching private field names do not make an unrelated class a
    // dictionary.
    ptr(&mut i, 0x2300 + 0x10, BASE + 0x1AC0);
    ptr(&mut i, 0x2300 + 0x18, BASE + 0x1A40);
    put(&mut i, 0x2300 + field_count_at, &4_u16.to_le_bytes());
    ptr(&mut i, 0x2300 + 0x80, BASE + 0x2480);
    field(&mut i, 0x2480, BASE + 0x1800, 0, 0x10);
    field(&mut i, 0x24A0, BASE + 0x1840, BASE + 0x500, 0x18);
    field(&mut i, 0x24C0, BASE + 0x1880, 0, 0x20);
    field(&mut i, 0x24E0, BASE + 0x18C0, 0, 0x24);
    ptr(&mut i, 0x2600, BASE + 0x2300);

    // A newly constructed dictionary has zero counts and no backing entries
    // array until its first insertion.
    ptr(&mut i, 0x2700, BASE + 0x200);

    i
}

fn module(unity: (u16, u16, u16, u16)) -> Module {
    Module {
        assemblies: Address::new(BASE),
        type_info_definition_table: Address::new(BASE + 0x40),
        profile: super::builds::nearest(unity, PointerSize::Bit64)
            .unwrap()
            .profile,
        pointer_size: PointerSize::Bit64,
    }
}

fn on_fixture(unity: (u16, u16, u16, u16), test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image(unity))], |process| {
        test(process, &module(unity));
    });
}

#[test]
fn dictionaries_resolve_through_the_cached_class() {
    on_fixture(MEASURED_2019, |process, module| {
        let offsets = module
            .get_dictionary_offsets(process, Address::new(BASE))
            .unwrap();
        let DictionaryShape::Entries {
            entries,
            count,
            free_count,
            layout,
        } = offsets.shape
        else {
            panic!("the entries shape resolves");
        };
        assert_eq!(entries, 0x18);
        assert_eq!(count, 0x20);
        assert_eq!(free_count, 0x24);
        assert_eq!(layout.stride, 0x10);
        assert_eq!(layout.hash, 0x0);
        assert_eq!(layout.next, 0x4);
        assert_eq!(layout.key, 0x8);
        assert_eq!(layout.value, 0xC);
    });
}

#[test]
fn dictionaries_read_their_live_pairs() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(pairs.as_slice(), [(10, 100), (20, 200)]);
    });
}

#[test]
fn dictionaries_derived_from_corlibs_dictionary_resolve() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE + 0x8);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(pairs.as_slice(), [(10, 100), (20, 200)]);
    });
}

#[test]
fn dictionary_shaped_objects_are_not_dictionaries() {
    on_fixture(MEASURED_2019, |process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE + 0x10))
            .is_none());
    });
}

#[test]
fn dictionaries_without_allocated_backing_read_empty() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE + 0x18);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 0>(process, offsets, slot)
            .unwrap();
        assert!(pairs.is_empty());
    });
}

#[test]
fn hash_sets_read_their_live_values() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE + 0x20);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(values.as_slice(), [7, 9]);
    });
}

#[test]
fn hash_sets_derived_from_corlibs_hash_set_resolve() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE + 0x28);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(values.as_slice(), [7, 9]);
    });
}

#[test]
fn hash_set_shaped_objects_are_not_hash_sets() {
    on_fixture(MEASURED_2019, |process, module| {
        assert!(module
            .get_hash_set_offsets(process, Address::new(BASE + 0x30))
            .is_none());
    });
}

#[test]
fn hash_sets_without_allocated_backing_read_empty() {
    on_fixture(MEASURED_2019, |process, module| {
        let slot = Address::new(BASE + 0x38);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 0>(process, offsets, slot)
            .unwrap();
        assert!(values.is_empty());
    });
}

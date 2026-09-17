//! Tests pinning dictionary resolution over hand-laid class metadata at the
//! literal offsets of the Unity 2019.4 x64 runtime. Resolution starts at a
//! live object and walks classes and fields only, so the fixtures stay their
//! own small blobs.

use super::super::managed::{DictionaryShape, SetShape};
use super::{BinaryFormat, Module, MonoOffsets, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x50_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

// One field entry: the MonoType pointer heads it, the name sits at 0x8, the
// offset at 0x18.
fn field(image: &mut [u8], at: u64, type_: u64, name: u64, offset: i32) {
    if type_ != 0 {
        ptr(image, at, type_);
    }
    ptr(image, at + 0x8, name);
    put(image, at + 0x18, &offset.to_le_bytes());
}

// A dictionary class: field_count at +0x100, fields behind +0x98. The names
// arrive in the caller's order with their instance offsets.
fn dictionary_class(image: &mut [u8], class: u64, fields: u64, names: [u64; 4], entries_type: u64) {
    collection_class(image, class, fields, BASE + 0x2400, names, entries_type);
}

// A corlib collection class with the given name, in the collections
// namespace, holding four fields.
fn collection_class(
    image: &mut [u8],
    class: u64,
    fields: u64,
    name: u64,
    names: [u64; 4],
    entries_type: u64,
) {
    ptr(image, class + 0x48, name);
    ptr(image, class + 0x50, BASE + 0x2440);
    put(image, class + 0x100, &4_i32.to_le_bytes());
    ptr(image, class + 0x98, BASE + fields);
    field(image, fields, 0, names[0], 0x10); // the buckets array
    field(image, fields + 0x20, entries_type, names[1], 0x18);
    field(image, fields + 0x40, 0, names[2], 0x20);
    field(image, fields + 0x60, 0, names[3], 0x24);
}

// An entry class: instance_size at +0x1C, four members at their boxed-frame
// offsets.
fn entry_class(image: &mut [u8], class: u64, fields: u64, size: i32, strings: [u64; 4]) {
    put(image, class + 0x1C, &size.to_le_bytes());
    put(image, class + 0x100, &4_i32.to_le_bytes());
    ptr(image, class + 0x98, BASE + fields);
    field(image, fields, 0, strings[0], 0x10);
    field(image, fields + 0x20, 0, strings[1], 0x14);
    field(image, fields + 0x40, 0, strings[2], 0x18);
    field(image, fields + 0x60, 0, strings[3], 0x1C);
}

// An object heading with its vtable, whose own head is the class.
fn object(image: &mut [u8], at: u64, vtable: u64, class: u64) {
    ptr(image, at, BASE + vtable);
    ptr(image, vtable, BASE + class);
}

fn image() -> Vec<u8> {
    let mut i = vec![0; 0x5000];

    let strings = [
        (0x1F00, "Next"),
        (0x1F40, "links"),
        (0x2000, "_buckets"),
        (0x2040, "_entries"),
        (0x2080, "_count"),
        (0x20C0, "_freeCount"),
        (0x2100, "hashCode"),
        (0x2140, "next"),
        (0x2180, "key"),
        (0x21C0, "value"),
        (0x2200, "buckets"),
        (0x2240, "entries"),
        (0x2280, "count"),
        (0x22C0, "freeCount"),
        (0x2300, "table"),
        (0x2340, "linkSlots"),
        (0x2380, "keySlots"),
        (0x23C0, "valueSlots"),
        (0x2400, "Dictionary`2"),
        (0x2440, "System.Collections.Generic"),
        (0x2480, "DerivedDictionary"),
        (0x24C0, "DictionaryLookalike"),
        (0x2500, "_slots"),
        (0x2540, "_lastIndex"),
        (0x2580, "m_slots"),
        (0x25C0, "m_buckets"),
        (0x2600, "m_count"),
        (0x2640, "m_lastIndex"),
        (0x2680, "HashSet`1"),
        (0x26C0, "DerivedHashSet"),
        (0x2700, "HashSetLookalike"),
        (0x4C00, "touchedSlots"),
        (0x4C40, "HashCode"),
        (0x4C80, "slots"),
        (0x4CC0, "touched"),
    ];
    for (at, text) in strings {
        put(&mut i, at, text.as_bytes());
    }
    let modern = [BASE + 0x2000, BASE + 0x2040, BASE + 0x2080, BASE + 0x20C0];
    let framework = [BASE + 0x2200, BASE + 0x2240, BASE + 0x2280, BASE + 0x22C0];
    let members = [BASE + 0x2100, BASE + 0x2140, BASE + 0x2180, BASE + 0x21C0];

    // The slots holding the references.
    ptr(&mut i, 0x0, BASE + 0x100);
    ptr(&mut i, 0x8, BASE + 0x180);
    ptr(&mut i, 0x10, BASE + 0xB00);
    ptr(&mut i, 0x18, BASE + 0xB80);
    ptr(&mut i, 0x20, BASE + 0x1A00);
    ptr(&mut i, 0x28, BASE + 0x2980);
    ptr(&mut i, 0x30, BASE + 0x2D40);
    ptr(&mut i, 0x38, BASE + 0x2E00);
    ptr(&mut i, 0x40, BASE + 0x2F00);
    ptr(&mut i, 0x48, BASE + 0x3200);
    ptr(&mut i, 0x50, BASE + 0x3500);
    ptr(&mut i, 0x58, BASE + 0x3800);
    ptr(&mut i, 0x60, BASE + 0x3C00);
    ptr(&mut i, 0x68, BASE + 0x3D00);
    ptr(&mut i, 0x70, BASE + 0x4000);
    ptr(&mut i, 0x78, BASE + 0x4800);
    ptr(&mut i, 0x80, BASE + 0x4B00);

    // The healthy dictionary in the modern naming generation: its entries
    // field's type is a SzArray whose data names the entry class directly.
    object(&mut i, 0x100, 0x140, 0x200);
    dictionary_class(&mut i, 0x200, 0x340, modern, BASE + 0x500);
    ptr(&mut i, 0x500, BASE + 0x600); // type data: the entry class
    put(&mut i, 0x50A, &[0x1D]); // SzArray
    entry_class(&mut i, 0x600, 0x740, 0x20, members);

    // The framework naming generation shares the entry class.
    object(&mut i, 0x180, 0x1C0, 0x900);
    dictionary_class(&mut i, 0x900, 0xA40, framework, BASE + 0x500);

    // A dictionary whose entry class has no instance size yet: not ready,
    // answers nothing.
    object(&mut i, 0xB00, 0xB40, 0xC00);
    dictionary_class(&mut i, 0xC00, 0xD40, modern, BASE + 0xF00);
    ptr(&mut i, 0xF00, BASE + 0x1600);
    put(&mut i, 0xF0A, &[0x1D]);
    entry_class(&mut i, 0x1600, 0x1740, 0, members);

    // A dictionary whose entry layout cannot hold its members: the stride is
    // eight bytes and the key sits at its end.
    object(&mut i, 0xB80, 0xBC0, 0x1000);
    dictionary_class(&mut i, 0x1000, 0x1140, modern, BASE + 0x1300);
    ptr(&mut i, 0x1300, BASE + 0x1800);
    put(&mut i, 0x130A, &[0x1D]);
    entry_class(&mut i, 0x1800, 0x1940, 0x18, members);

    // The old corlib's parallel-arrays shape: its names answer, ours do not.
    object(&mut i, 0x1A00, 0x1A40, 0x1B00);
    let parallel = [BASE + 0x2300, BASE + 0x2340, BASE + 0x2380, BASE + 0x23C0];
    dictionary_class(&mut i, 0x1B00, 0x1C40, parallel, 0);

    // The healthy dictionary's live state: three counted entries over a
    // four-slot backing, the middle one freed, so two pairs are live.
    ptr(&mut i, 0x118, BASE + 0x1D00);
    put(&mut i, 0x120, &3_i32.to_le_bytes());
    put(&mut i, 0x124, &1_i32.to_le_bytes());
    put(&mut i, 0x1D18, &4_u32.to_le_bytes());
    entry(&mut i, 0x1D20, 1111, -1, 10, 100);
    entry(&mut i, 0x1D30, -1, -1, 99, 999); // freed: the hash carries the mark
    entry(&mut i, 0x1D40, 2222, -1, 20, 200);
    entry(&mut i, 0x1D50, 0, 0, 0, 0);

    // The framework dictionary claims three live entries but holds two: its
    // tally cannot balance.
    ptr(&mut i, 0x198, BASE + 0x1E00);
    put(&mut i, 0x1A0, &3_i32.to_le_bytes());
    put(&mut i, 0x1A4, &0_i32.to_le_bytes());
    put(&mut i, 0x1E18, &4_u32.to_le_bytes());
    entry(&mut i, 0x1E20, 1111, -1, 1, 2);
    entry(&mut i, 0x1E30, -1, -1, 0, 0);
    entry(&mut i, 0x1E40, 2222, -1, 3, 4);
    entry(&mut i, 0x1E50, 0, 0, 0, 0);

    // A derived dictionary inherits the collection fields from the corlib
    // Dictionary class rather than declaring them itself.
    ptr(&mut i, 0x2800 + 0x30, BASE + 0x200);
    ptr(&mut i, 0x2800 + 0x48, BASE + 0x2480);
    ptr(&mut i, 0x2800 + 0x50, BASE + 0x2440);
    object(&mut i, 0x2980, 0x2940, 0x2800);
    ptr(&mut i, 0x2980 + 0x18, BASE + 0x1D00);
    put(&mut i, 0x2980 + 0x20, &3_i32.to_le_bytes());
    put(&mut i, 0x2980 + 0x24, &1_i32.to_le_bytes());

    // Matching private field names do not make an unrelated class a
    // dictionary.
    dictionary_class(&mut i, 0x2B00, 0x2C40, modern, BASE + 0x500);
    ptr(&mut i, 0x2B00 + 0x48, BASE + 0x24C0);
    object(&mut i, 0x2D40, 0x2D00, 0x2B00);

    // A newly constructed dictionary has zero counts and no backing entries
    // array until its first insertion.
    object(&mut i, 0x2E00, 0x140, 0x200);

    // A hash set in each naming generation, sharing the entry class as its
    // slot class (a slot is an entry without the key claim) and one slots
    // array: two live values around a freed slot, the high-water mark past
    // the live count.
    let set_names = [BASE + 0x2000, BASE + 0x2500, BASE + 0x2080, BASE + 0x2540];
    object(&mut i, 0x2F00, 0x2F40, 0x3000);
    collection_class(
        &mut i,
        0x3000,
        0x3140,
        BASE + 0x2680,
        set_names,
        BASE + 0x500,
    );
    ptr(&mut i, 0x2F18, BASE + 0x3E00);
    put(&mut i, 0x2F20, &2_i32.to_le_bytes());
    put(&mut i, 0x2F24, &3_i32.to_le_bytes());
    put(&mut i, 0x3E18, &4_u32.to_le_bytes());
    entry(&mut i, 0x3E20, 111, -1, 0, 7);
    entry(&mut i, 0x3E30, -1, -1, 0, 0);
    entry(&mut i, 0x3E40, 222, -1, 0, 9);
    entry(&mut i, 0x3E50, 0, 0, 0, 0);

    let m_names = [BASE + 0x25C0, BASE + 0x2580, BASE + 0x2600, BASE + 0x2640];
    object(&mut i, 0x3200, 0x3240, 0x3300);
    collection_class(&mut i, 0x3300, 0x3440, BASE + 0x2680, m_names, BASE + 0x500);
    ptr(&mut i, 0x3218, BASE + 0x3E00);
    put(&mut i, 0x3220, &2_i32.to_le_bytes());
    put(&mut i, 0x3224, &3_i32.to_le_bytes());

    // A hash set claiming three live values over the same two: its tally
    // cannot balance.
    object(&mut i, 0x3500, 0x3540, 0x3000);
    ptr(&mut i, 0x3518, BASE + 0x3E00);
    put(&mut i, 0x3520, &3_i32.to_le_bytes());
    put(&mut i, 0x3524, &3_i32.to_le_bytes());

    // A derived hash set inherits the collection fields from the corlib
    // HashSet class rather than declaring them itself.
    ptr(&mut i, 0x3700 + 0x30, BASE + 0x3000);
    ptr(&mut i, 0x3700 + 0x48, BASE + 0x26C0);
    ptr(&mut i, 0x3700 + 0x50, BASE + 0x2440);
    object(&mut i, 0x3800, 0x3840, 0x3700);
    ptr(&mut i, 0x3818, BASE + 0x3E00);
    put(&mut i, 0x3820, &2_i32.to_le_bytes());
    put(&mut i, 0x3824, &3_i32.to_le_bytes());

    // Matching private field names do not make an unrelated class a hash
    // set.
    collection_class(
        &mut i,
        0x3A00,
        0x3B40,
        BASE + 0x2700,
        set_names,
        BASE + 0x500,
    );
    object(&mut i, 0x3C00, 0x3C40, 0x3A00);

    // A newly constructed hash set has zero counts and no backing slots
    // array until its first insertion.
    object(&mut i, 0x3D00, 0x3D40, 0x3000);

    // The oldest corlib's parallel-arrays dictionary: four arrays side by
    // side, links carrying the live flag in their stored hashes, and a Link
    // class saying which of its two ints is which.
    object(&mut i, 0x4000, 0x4040, 0x4100);
    ptr(&mut i, 0x4100 + 0x48, BASE + 0x2400);
    ptr(&mut i, 0x4100 + 0x50, BASE + 0x2440);
    put(&mut i, 0x4100 + 0x100, &6_i32.to_le_bytes());
    ptr(&mut i, 0x4100 + 0x98, BASE + 0x4240);
    field(&mut i, 0x4240, 0, BASE + 0x2300, 0x10); // table
    field(&mut i, 0x4260, BASE + 0x4300, BASE + 0x2340, 0x18); // linkSlots
    field(&mut i, 0x4280, 0, BASE + 0x2380, 0x20); // keySlots
    field(&mut i, 0x42A0, 0, BASE + 0x23C0, 0x28); // valueSlots
    field(&mut i, 0x42C0, 0, BASE + 0x4C00, 0x30); // touchedSlots
    field(&mut i, 0x42E0, 0, BASE + 0x2280, 0x34); // count
    ptr(&mut i, 0x4300, BASE + 0x4400); // the links' type: SzArray of Link
    put(&mut i, 0x430A, &[0x1D]);
    put(&mut i, 0x4400 + 0x1C, &0x18_i32.to_le_bytes());
    put(&mut i, 0x4400 + 0x100, &2_i32.to_le_bytes());
    ptr(&mut i, 0x4400 + 0x98, BASE + 0x4540);
    field(&mut i, 0x4540, 0, BASE + 0x4C40, 0x10); // HashCode
    field(&mut i, 0x4560, 0, BASE + 0x1F00, 0x14); // Next

    ptr(&mut i, 0x4018, BASE + 0x4600);
    ptr(&mut i, 0x4020, BASE + 0x4680);
    ptr(&mut i, 0x4028, BASE + 0x4700);
    put(&mut i, 0x4030, &3_i32.to_le_bytes());
    put(&mut i, 0x4034, &2_i32.to_le_bytes());
    put(&mut i, 0x4618, &4_u32.to_le_bytes());
    for (index, hash) in [0x8000_0001_u32, 0x0000_0002, 0x8000_0003]
        .into_iter()
        .enumerate()
    {
        put(&mut i, 0x4620 + 8 * index as u64, &hash.to_le_bytes());
        put(&mut i, 0x4624 + 8 * index as u64, &(-1_i32).to_le_bytes());
    }
    put(&mut i, 0x4698, &4_u32.to_le_bytes());
    for (index, key) in [10_i32, 99, 20].into_iter().enumerate() {
        put(&mut i, 0x46A0 + 4 * index as u64, &key.to_le_bytes());
    }
    put(&mut i, 0x4718, &4_u32.to_le_bytes());
    for (index, value) in [100_i32, 999, 200].into_iter().enumerate() {
        put(&mut i, 0x4720 + 4 * index as u64, &value.to_le_bytes());
    }

    // The parallel hash set, sharing the links and reusing the key array as
    // its values.
    object(&mut i, 0x4800, 0x4840, 0x4900);
    ptr(&mut i, 0x4900 + 0x48, BASE + 0x2680);
    ptr(&mut i, 0x4900 + 0x50, BASE + 0x2440);
    put(&mut i, 0x4900 + 0x100, &5_i32.to_le_bytes());
    ptr(&mut i, 0x4900 + 0x98, BASE + 0x4A40);
    field(&mut i, 0x4A40, 0, BASE + 0x2300, 0x10); // table
    field(&mut i, 0x4A60, BASE + 0x4300, BASE + 0x1F40, 0x18); // links
    field(&mut i, 0x4A80, 0, BASE + 0x4C80, 0x20); // slots
    field(&mut i, 0x4AA0, 0, BASE + 0x4CC0, 0x28); // touched
    field(&mut i, 0x4AC0, 0, BASE + 0x2280, 0x2C); // count
    ptr(&mut i, 0x4818, BASE + 0x4600);
    ptr(&mut i, 0x4820, BASE + 0x4680);
    put(&mut i, 0x4828, &3_i32.to_le_bytes());
    put(&mut i, 0x482C, &2_i32.to_le_bytes());

    // A parallel set claiming three live values over the same two.
    object(&mut i, 0x4B00, 0x4B40, 0x4900);
    ptr(&mut i, 0x4B18, BASE + 0x4600);
    ptr(&mut i, 0x4B20, BASE + 0x4680);
    put(&mut i, 0x4B28, &3_i32.to_le_bytes());
    put(&mut i, 0x4B2C, &3_i32.to_le_bytes());

    i
}

// One live or freed entry at the fixture's 16-byte stride: the stored hash,
// the chain link, and an i32 key and value.
fn entry(image: &mut [u8], at: u64, hash: i32, next: i32, key: i32, value: i32) {
    for (index, word) in [hash, next, key, value].into_iter().enumerate() {
        put(image, at + 4 * index as u64, &word.to_le_bytes());
    }
}

fn module() -> Module {
    Module {
        assemblies: Address::new(BASE),
        version: Version::V2,
        offsets: MonoOffsets::new(Version::V2, PointerSize::Bit64, BinaryFormat::PE).unwrap(),
        pointer_size: PointerSize::Bit64,
    }
}

fn on_fixture(test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image())], |process| {
        test(process, &module());
    });
}

#[test]
fn dictionaries_resolve_in_both_naming_generations() {
    on_fixture(|process, module| {
        for at in [BASE, BASE + 0x8] {
            let slot = Address::new(at);
            let offsets = module.get_dictionary_offsets(process, slot).unwrap();
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
        }
    });
}

// An entry class with no instance size yet is a target still starting up:
// resolution answers nothing rather than baking a zero stride.
#[test]
fn half_initialized_entry_classes_answer_nothing() {
    on_fixture(|process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE + 0x10))
            .is_none());
    });
}

// A layout whose members cannot sit inside its stride is refused whole.
#[test]
fn scrambled_entry_layouts_answer_nothing() {
    on_fixture(|process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE + 0x18))
            .is_none());
    });
}

// The old corlib's parallel-arrays dictionary is not this shape and misses
// cleanly rather than resolving to wrong offsets.
#[test]
fn parallel_shape_names_answer_nothing() {
    on_fixture(|process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE + 0x20))
            .is_none());
    });
}

// The read returns exactly the live pairs: the counted entries minus the
// freed one, in entry order.
#[test]
fn dictionaries_read_their_live_pairs() {
    on_fixture(|process, module| {
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
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x28);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(pairs.as_slice(), [(10, 100), (20, 200)]);
    });
}

#[test]
fn dictionary_shaped_objects_are_not_dictionaries() {
    on_fixture(|process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE + 0x30))
            .is_none());
    });
}

#[test]
fn dictionaries_without_allocated_backing_read_empty() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x38);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 0>(process, offsets, slot)
            .unwrap();
        assert!(pairs.is_empty());
    });
}

// The buffer judges the live pairs, never the counted entries or the
// backing capacity.
#[test]
fn read_buffers_judge_live_pairs() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        assert!(module
            .read_dictionary::<i32, i32, 2>(process, offsets, slot)
            .is_ok());
        assert!(module
            .read_dictionary::<i32, i32, 1>(process, offsets, slot)
            .is_err());
    });
}

// A claimed element size past its member's room would read a sibling's
// bytes; the read refuses instead.
#[test]
fn oversized_element_claims_refuse() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        assert!(module
            .read_dictionary::<u64, u64, 8>(process, offsets, slot)
            .is_err());
    });
}

// A live tally that cannot balance against the counts is a torn or lying
// dictionary, and fails rather than answering wrong pairs.
#[test]
fn unbalanced_tallies_refuse() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x8);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        assert!(module
            .read_dictionary::<i32, i32, 8>(process, offsets, slot)
            .is_err());
    });
}

#[test]
fn hash_sets_resolve_in_both_naming_generations() {
    on_fixture(|process, module| {
        for at in [BASE + 0x40, BASE + 0x48] {
            let slot = Address::new(at);
            let offsets = module.get_hash_set_offsets(process, slot).unwrap();
            let SetShape::Slots {
                slots,
                count,
                last_index,
                layout,
            } = offsets.shape
            else {
                panic!("the slots shape resolves");
            };
            assert_eq!(slots, 0x18);
            assert_eq!(count, 0x20);
            assert_eq!(last_index, 0x24);
            assert_eq!(layout.stride, 0x10);
            assert_eq!(layout.hash, 0x0);
            assert_eq!(layout.next, 0x4);
            assert_eq!(layout.value, 0xC);
        }
    });
}

// The walk runs to the high-water mark, not the live count, skipping the
// freed slot inside it.
#[test]
fn hash_sets_read_their_live_values() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x40);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(values.as_slice(), [7, 9]);
    });
}

#[test]
fn unbalanced_set_tallies_refuse() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x50);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        assert!(module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .is_err());
    });
}

// The oldest corlib's dictionary keeps four arrays side by side; the same
// call resolves it and the same call reads it.
#[test]
fn parallel_dictionaries_resolve_and_read() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x70);
        let offsets = module.get_dictionary_offsets(process, slot).unwrap();
        let pairs = module
            .read_dictionary::<i32, i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(pairs.as_slice(), [(10, 100), (20, 200)]);
    });
}

#[test]
fn parallel_hash_sets_resolve_and_read() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x78);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(values.as_slice(), [10, 20]);
    });
}

#[test]
fn hash_sets_derived_from_corlibs_hash_set_resolve() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x58);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .unwrap();
        assert_eq!(values.as_slice(), [7, 9]);
    });
}

#[test]
fn unbalanced_parallel_tallies_refuse() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x80);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        assert!(module
            .read_hash_set::<i32, 8>(process, offsets, slot)
            .is_err());
    });
}

#[test]
fn hash_set_shaped_objects_are_not_hash_sets() {
    on_fixture(|process, module| {
        assert!(module
            .get_hash_set_offsets(process, Address::new(BASE + 0x60))
            .is_none());
    });
}

#[test]
fn hash_sets_without_allocated_backing_read_empty() {
    on_fixture(|process, module| {
        let slot = Address::new(BASE + 0x68);
        let offsets = module.get_hash_set_offsets(process, slot).unwrap();
        let values = module
            .read_hash_set::<i32, 0>(process, offsets, slot)
            .unwrap();
        assert!(values.is_empty());
    });
}

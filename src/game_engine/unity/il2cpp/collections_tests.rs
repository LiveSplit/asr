//! Tests pinning dictionary resolution over hand-laid class metadata at the
//! literal offsets of the Unity 2019.4 IL2CPP runtime. The entry class is a
//! generic instance reached through the instantiation's cached class, which
//! is the route every corlib dictionary takes.

use super::{IL2CPPOffsets, Module, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x60_0000;

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

fn image(version: Version) -> Vec<u8> {
    let field_count_at = match version {
        Version::V2019 => 0x11C,
        _ => 0x124,
    };
    let mut i = vec![0; 0x2000];

    let strings = [
        (0x1800, "_buckets"),
        (0x1840, "_entries"),
        (0x1880, "_count"),
        (0x18C0, "_freeCount"),
        (0x1900, "hashCode"),
        (0x1940, "next"),
        (0x1980, "key"),
        (0x19C0, "value"),
    ];
    for (at, text) in strings {
        put(&mut i, at, text.as_bytes());
    }

    // The slot, the dictionary object heading with its class, and the class's
    // four fields: field_count at +0x11C, fields behind +0x80.
    ptr(&mut i, 0x0, BASE + 0x100);
    ptr(&mut i, 0x100, BASE + 0x200);
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

    i
}

fn module(version: Version) -> Module {
    Module {
        assemblies: Address::new(BASE),
        type_info_definition_table: Address::new(BASE + 0x40),
        version,
        offsets: IL2CPPOffsets::new(version, PointerSize::Bit64).unwrap(),
        pointer_size: PointerSize::Bit64,
    }
}

fn on_fixture(version: Version, test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image(version))], |process| {
        test(process, &module(version));
    });
}

#[test]
fn dictionaries_resolve_through_the_cached_class() {
    on_fixture(Version::V2019, |process, module| {
        let offsets = module
            .get_dictionary_offsets(process, Address::new(BASE))
            .unwrap();
        assert_eq!(offsets.entries, 0x18);
        assert_eq!(offsets.count, 0x20);
        assert_eq!(offsets.free_count, 0x24);
        assert_eq!(offsets.layout.stride, 0x10);
        assert_eq!(offsets.layout.hash, 0x0);
        assert_eq!(offsets.layout.next, 0x4);
        assert_eq!(offsets.layout.key, 0x8);
        assert_eq!(offsets.layout.value, 0xC);
    });
}

// The 2022.2-and-later fallback table carries no cached class, because the
// measured builds inside that stretch disagree. A fallback attach misses
// cleanly; known builds carry their own value.
#[test]
fn fallback_tables_without_a_cached_class_answer_nothing() {
    on_fixture(Version::V2022, |process, module| {
        assert!(module
            .get_dictionary_offsets(process, Address::new(BASE))
            .is_none());
    });
}

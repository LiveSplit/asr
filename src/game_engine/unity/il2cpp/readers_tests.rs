//! Parity tests for the managed readers through the IL2CPP module. The
//! implementation is shared, so this pins the mirror surface and the one
//! behavior whose rationale is IL2CPP's: the full-width length judgment.

use super::{IL2CPPOffsets, Module, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x40_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

fn image() -> Vec<u8> {
    let mut i = vec![0; 0x1000];

    ptr(&mut i, 0x0, BASE + 0x100);
    put(&mut i, 0x100 + 0x10, &4_i32.to_le_bytes());
    for (index, unit) in "Loop".encode_utf16().enumerate() {
        put(&mut i, 0x100 + 0x14 + 2 * index as u64, &unit.to_le_bytes());
    }

    // An array whose length slot carries garbage above the low u32. IL2CPP's
    // length really is pointer-sized, so the whole slot judges the claim.
    ptr(&mut i, 0x8, BASE + 0x200);
    put(&mut i, 0x200 + 0x18, &0x1_0000_0003_u64.to_le_bytes());

    ptr(&mut i, 0x10, BASE + 0x300);
    put(&mut i, 0x300 + 0x18, &2_u64.to_le_bytes());
    put(&mut i, 0x300 + 0x20, &11_u32.to_le_bytes());
    put(&mut i, 0x300 + 0x24, &22_u32.to_le_bytes());

    // A string array holding the string object and a null element.
    ptr(&mut i, 0x18, BASE + 0x400);
    put(&mut i, 0x400 + 0x18, &2_u64.to_le_bytes());
    ptr(&mut i, 0x400 + 0x20, BASE + 0x100);

    i
}

fn on_fixture(test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image())], |process| {
        let module = Module {
            assemblies: Address::new(BASE),
            type_info_definition_table: Address::new(BASE + 0x10),
            version: Version::V2022,
            offsets: IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
            pointer_size: PointerSize::Bit64,
        };
        test(process, &module);
    });
}

#[test]
fn strings_resolve_through_their_reference() {
    on_fixture(|process, module| {
        let read = module
            .read_string::<8>(process, Address::new(BASE))
            .unwrap();
        assert!(read.matches_str("Loop"));
    });
}

#[test]
fn arrays_resolve_through_their_reference() {
    on_fixture(|process, module| {
        let read = module
            .read_array::<u32, 4>(process, Address::new(BASE + 0x10))
            .unwrap();
        assert_eq!(read.as_slice(), [11, 22]);
    });
}

// A length whose low u32 reads small but whose full width does not is
// garbage, not a small array. An i32 read here would wrongly succeed.
#[test]
fn array_lengths_judge_at_full_width() {
    on_fixture(|process, module| {
        assert!(module
            .read_array::<i32, 8>(process, Address::new(BASE + 0x8))
            .is_err());
    });
}

#[test]
fn reference_arrays_resolve_through_their_reference() {
    on_fixture(|process, module| {
        let objects = module
            .read_reference_array::<4>(process, Address::new(BASE + 0x18))
            .unwrap();
        assert_eq!(
            objects.as_slice(),
            [Address::new(BASE + 0x100), Address::NULL]
        );

        let read = module.read_string_object::<8>(process, objects[0]).unwrap();
        assert!(read.matches_str("Loop"));
        assert!(module.read_string_object::<8>(process, objects[1]).is_err());
    });
}

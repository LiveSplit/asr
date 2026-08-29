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

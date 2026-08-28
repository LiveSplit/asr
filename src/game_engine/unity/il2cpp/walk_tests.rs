//! Tests over a hand-laid image of IL2CPP's structures.

use super::{IL2CPPOffsets, Module, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize};

use std::vec;

const BASE: u64 = 0x20_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

// A 32 bit target lays the assemblies vector and its pointers at four bytes.
#[test]
fn images_resolve_on_32_bit_targets() {
    let mut i = vec![0; 0x1000];
    let ptr = |i: &mut [u8], at: u64, target: u64| {
        put(i, at, &(target as u32).to_le_bytes());
    };

    put(&mut i, 0x800, b"Assembly-CSharp");
    ptr(&mut i, 0x0, BASE + 0x40); // the vector's begin
    ptr(&mut i, 0x4, BASE + 0x44); // and end, one assembly along
    ptr(&mut i, 0x40, BASE + 0x80);
    ptr(&mut i, 0x80, BASE + 0x100); // Il2CppAssembly.image
    ptr(&mut i, 0x80 + 0x18, BASE + 0x800); // Il2CppAssembly.aname

    with_process(&[(BASE, &i)], |process| {
        let module = Module {
            assemblies: Address::new(BASE),
            type_info_definition_table: Address::new(BASE + 0x10),
            version: Version::V2022,
            offsets: IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
            pointer_size: PointerSize::Bit32,
        };
        assert!(module.get_default_image(process).is_some());
    });
}

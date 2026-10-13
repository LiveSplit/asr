//! Tests reading a module's format and size from its header, for auto
//! splitters that hand in the module themselves.

use super::{module_at, BinaryFormat};
use crate::{runtime::mock::with_process, Address};

use std::vec;

const BASE: u64 = 0x7FF6_1000_0000;

fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

#[test]
fn pe_module_spans_its_size_of_image() {
    // The DOS header points at the COFF header, which is followed by a PE32+
    // optional header that says the image is 0x5000 bytes.
    let mut image = vec![0; 0x400];
    put(&mut image, 0, b"MZ");
    put(&mut image, 0x3C, &0x80_u32.to_le_bytes());
    put(&mut image, 0x80, b"PE\0\0");
    put(&mut image, 0x84, &0x8664_u16.to_le_bytes());
    put(&mut image, 0x94, &0xF0_u16.to_le_bytes());
    put(&mut image, 0x98, &0x20B_u16.to_le_bytes());
    put(&mut image, 0x98 + 0x38, &0x5000_u32.to_le_bytes());
    with_process(&[(BASE, &image)], |process| {
        assert_eq!(
            module_at(process, Address::new(BASE)),
            Some(((Address::new(BASE), 0x5000), BinaryFormat::PE))
        );
    });
}

#[test]
fn elf_module_spans_to_its_last_load_segment() {
    // An ELF64 header with one program header, a load segment that runs to
    // 0x3000 in memory.
    let mut image = vec![0; 0x400];
    put(&mut image, 0, b"\x7fELF\x02\x01\x01");
    put(&mut image, 0x10, &3_u16.to_le_bytes());
    put(&mut image, 0x20, &0x40_u64.to_le_bytes());
    put(&mut image, 0x36, &56_u16.to_le_bytes());
    put(&mut image, 0x38, &1_u16.to_le_bytes());
    put(&mut image, 0x40, &1_u32.to_le_bytes());
    put(&mut image, 0x68, &0x3000_u64.to_le_bytes());
    with_process(&[(BASE, &image)], |process| {
        assert_eq!(
            module_at(process, Address::new(BASE)),
            Some(((Address::new(BASE), 0x3000), BinaryFormat::ELF))
        );
    });
}

#[test]
fn unknown_header_gives_no_module() {
    let image = vec![0; 0x400];
    with_process(&[(BASE, &image)], |process| {
        assert_eq!(module_at(process, Address::new(BASE)), None);
    });
}

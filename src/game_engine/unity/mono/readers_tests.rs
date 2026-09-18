//! Tests pinning the managed readers over hand-laid string and array
//! objects. The readers are pointer-size ABI, not walk work, so the fixtures
//! are tiny blobs rather than the walk's class fixtures.

use super::{BinaryFormat, Module, MonoOffsets, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x30_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

fn utf16(image: &mut [u8], at: u64, text: &str) {
    for (index, unit) in text.encode_utf16().enumerate() {
        put(image, at + 2 * index as u64, &unit.to_le_bytes());
    }
}

// String and array objects at the 64-bit layout: two header words, then the
// string's i32 character count and inline UTF-16 characters, or the array's
// bounds word, length, and inline elements. Each slot at the front holds one
// reference the readers dereference.
fn image() -> Vec<u8> {
    let mut i = vec![0; 0x1000];

    ptr(&mut i, 0x0, BASE + 0x100); // a healthy string
    ptr(&mut i, 0x8, BASE + 0x200); // one claiming more than a buffer holds
    ptr(&mut i, 0x10, BASE + 0x300); // one claiming a negative count
    ptr(&mut i, 0x18, 0); // a null reference
    ptr(&mut i, 0x20, BASE + 0x400); // a string with a nul inside
    ptr(&mut i, 0x28, BASE + 0x500); // a healthy i32 array
    ptr(&mut i, 0x30, BASE + 0x600); // an array claiming more than a buffer holds
    ptr(&mut i, 0x38, BASE + 0x700); // a u16 array
    ptr(&mut i, 0x40, BASE + 0x800); // a string array with a null element

    put(&mut i, 0x100 + 0x10, &9_i32.to_le_bytes());
    utf16(&mut i, 0x100 + 0x14, "Chapter 3");

    put(&mut i, 0x200 + 0x10, &64_i32.to_le_bytes());
    put(&mut i, 0x300 + 0x10, &(-1_i32).to_le_bytes());

    // A managed string may hold a nul character; its count says where it
    // ends.
    put(&mut i, 0x400 + 0x10, &3_i32.to_le_bytes());
    for (index, unit) in [b'a' as u16, 0, b'b' as u16].into_iter().enumerate() {
        put(&mut i, 0x400 + 0x14 + 2 * index as u64, &unit.to_le_bytes());
    }

    // Mono stores the length as a u32 the allocator's zeroing pads to the
    // pointer-wide slot the reader judges, which is what these bytes lay.
    put(&mut i, 0x500 + 0x18, &3_u32.to_le_bytes());
    for (index, value) in [7_i32, 8, 9].into_iter().enumerate() {
        put(
            &mut i,
            0x500 + 0x20 + 4 * index as u64,
            &value.to_le_bytes(),
        );
    }

    put(&mut i, 0x600 + 0x18, &64_u32.to_le_bytes());

    put(&mut i, 0x700 + 0x18, &5_u32.to_le_bytes());
    utf16(&mut i, 0x700 + 0x20, "melon");

    put(&mut i, 0x900 + 0x10, &5_i32.to_le_bytes());
    utf16(&mut i, 0x900 + 0x14, "Docks");

    // The reference array's slots hold the two string objects around a null
    // element.
    put(&mut i, 0x800 + 0x18, &3_u32.to_le_bytes());
    ptr(&mut i, 0x800 + 0x20, BASE + 0x100);
    ptr(&mut i, 0x800 + 0x30, BASE + 0x900);

    i
}

fn module(pointer_size: PointerSize) -> Module {
    Module {
        assemblies: Address::new(BASE),
        version: Version::V2,
        offsets: MonoOffsets::new(Version::V2, pointer_size, BinaryFormat::PE).unwrap(),
        pointer_size,
    }
}

fn on_fixture(test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image())], |process| {
        test(process, &module(PointerSize::Bit64));
    });
}

#[test]
fn strings_resolve_through_their_reference() {
    on_fixture(|process, module| {
        let read = module
            .read_string::<16>(process, Address::new(BASE))
            .unwrap();
        assert!(read.matches_str("Chapter 3"));
    });
}

// The buffer size is the bound past which a claimed count is nonsense: a
// torn read claims billions, and refusing beats truncating.
// A nul character inside a managed string is a character like any other.
// The read keeps the runtime's count, so every unit stays reachable, and the
// string is not "a" just because a nul follows the a.
#[test]
fn strings_keep_a_nul_inside() {
    on_fixture(|process, module| {
        let read = module
            .read_string::<16>(process, Address::new(BASE + 0x20))
            .unwrap();
        assert_eq!(read.len(), 3);
        assert_eq!(read.as_slice(), [b'a' as u16, 0, b'b' as u16]);
        assert!(read.matches_str("a\0b"));
        assert!(read.matches([b'a' as u16, 0, b'b' as u16]));
        assert!(!read.matches_str("a"));
        assert!(!read.matches([b'a' as u16]));
        assert_ne!(
            read,
            module
                .read_string::<16>(process, Address::new(BASE))
                .unwrap()
        );
    });
}

#[test]
fn string_counts_past_the_buffer_refuse() {
    on_fixture(|process, module| {
        assert!(module
            .read_string::<16>(process, Address::new(BASE + 0x8))
            .is_err());
        assert!(module
            .read_string::<16>(process, Address::new(BASE + 0x10))
            .is_err());
        assert!(module
            .read_string::<16>(process, Address::new(BASE + 0x18))
            .is_err());
    });
}

#[test]
fn arrays_resolve_through_their_reference() {
    on_fixture(|process, module| {
        let read = module
            .read_array::<i32, 8>(process, Address::new(BASE + 0x28))
            .unwrap();
        assert_eq!(read.as_slice(), [7, 8, 9]);
        assert_eq!(read.len(), 3);
        assert_eq!(read[1], 8);
        assert_eq!(read.as_slice(), [7, 8, 9]);
        assert_ne!(read.as_slice(), [7, 8]);
    });
}

// The element type is the caller's claim; a u16 claim strides a managed
// char array correctly.
#[test]
fn array_elements_stride_by_their_claimed_type() {
    on_fixture(|process, module| {
        let read = module
            .read_array::<u16, 8>(process, Address::new(BASE + 0x38))
            .unwrap();
        let melon: Vec<u16> = "melon".encode_utf16().collect();
        assert_eq!(read.as_slice(), melon);
    });
}

#[test]
fn array_lengths_past_the_buffer_refuse() {
    on_fixture(|process, module| {
        assert!(module
            .read_array::<i32, 8>(process, Address::new(BASE + 0x30))
            .is_err());
        assert!(module
            .read_array::<i32, 8>(process, Address::new(BASE + 0x18))
            .is_err());
    });
}

// The 32-bit layout halves the header, the reference width, and the length
// slot. The reference array's slots are 4 bytes wide with garbage right
// behind them, so a wide stride reads loudly wrong addresses.
#[test]
fn readers_resolve_on_32_bit_targets() {
    let mut i = vec![0; 0x1000];
    put(&mut i, 0x0, &((BASE + 0x100) as u32).to_le_bytes());
    put(&mut i, 0x100 + 0x8, &5_i32.to_le_bytes());
    utf16(&mut i, 0x100 + 0xC, "Ridge");
    put(&mut i, 0x8, &((BASE + 0x200) as u32).to_le_bytes());
    put(&mut i, 0x200 + 0xC, &2_u32.to_le_bytes());
    put(&mut i, 0x200 + 0x10, &21_i32.to_le_bytes());
    put(&mut i, 0x200 + 0x14, &22_i32.to_le_bytes());
    put(&mut i, 0x10, &((BASE + 0x300) as u32).to_le_bytes());
    put(&mut i, 0x300 + 0xC, &2_u32.to_le_bytes());
    put(&mut i, 0x300 + 0x10, &((BASE + 0x100) as u32).to_le_bytes());
    put(&mut i, 0x300 + 0x18, &u32::MAX.to_le_bytes());

    with_process(&[(BASE, &i)], |process| {
        let module = module(PointerSize::Bit32);
        let read = module
            .read_string::<8>(process, Address::new(BASE))
            .unwrap();
        assert!(read.matches_str("Ridge"));

        let read = module
            .read_array::<i32, 4>(process, Address::new(BASE + 0x8))
            .unwrap();
        assert_eq!(read.as_slice(), [21, 22]);

        let read = module
            .read_reference_array::<4>(process, Address::new(BASE + 0x10))
            .unwrap();
        assert_eq!(read.as_slice(), [Address::new(BASE + 0x100), Address::NULL]);

        let read = module.read_string_object::<8>(process, read[0]).unwrap();
        assert!(read.matches_str("Ridge"));
    });
}

// Null elements are data: they come back as null addresses at their
// positions, since positions carry meaning and filtering is the caller's
// one-liner. Only a null array reference refuses.
#[test]
fn reference_arrays_preserve_null_elements_in_place() {
    on_fixture(|process, module| {
        let read = module
            .read_reference_array::<8>(process, Address::new(BASE + 0x40))
            .unwrap();
        assert_eq!(
            read.as_slice(),
            [
                Address::new(BASE + 0x100),
                Address::NULL,
                Address::new(BASE + 0x900),
            ]
        );
    });
}

#[test]
fn reference_array_lengths_past_the_buffer_refuse() {
    on_fixture(|process, module| {
        assert!(module
            .read_reference_array::<2>(process, Address::new(BASE + 0x40))
            .is_err());
        assert!(module
            .read_reference_array::<8>(process, Address::new(BASE + 0x18))
            .is_err());
    });
}

// The object-address form composes over a reference array's elements; a
// null address refuses at the element, with the index in hand.
#[test]
fn string_objects_read_at_their_addresses() {
    on_fixture(|process, module| {
        let objects = module
            .read_reference_array::<8>(process, Address::new(BASE + 0x40))
            .unwrap();

        let read = module
            .read_string_object::<16>(process, objects[0])
            .unwrap();
        assert!(read.matches_str("Chapter 3"));
        assert!(module
            .read_string_object::<16>(process, objects[1])
            .is_err());
        let read = module
            .read_string_object::<16>(process, objects[2])
            .unwrap();
        assert!(read.matches_str("Docks"));
    });
}

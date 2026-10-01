//! Tests finding the two globals through the functions IL2CPP exports. Each
//! test builds a small `GameAssembly.dll` with an export table and the code
//! shape one kind of build has.

use super::globals;
use crate::{
    runtime::mock::{put_exports, with_process},
    Address, PointerSize,
};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x1000_0000;
const SIZE: usize = 0x4000;

/// Builds a PE image with the given exports and code. Each piece of code is
/// an offset and its bytes.
fn image(pointer_size: PointerSize, exports: &[(&str, u32)], code: &[(u32, &[u8])]) -> Vec<u8> {
    let mut image = vec![0; SIZE];
    put_exports(&mut image, pointer_size, exports);
    for (at, bytes) in code {
        let at = *at as usize;
        image[at..at + bytes.len()].copy_from_slice(bytes);
    }
    image
}

/// The four bytes of a displacement from `next` to `target`.
const fn displacement(next: u32, target: u32) -> [u8; 4] {
    (target.wrapping_sub(next) as i32).to_le_bytes()
}

/// The four bytes of the absolute address of `target`.
const fn absolute(target: u32) -> [u8; 4] {
    (BASE as u32 + target).to_le_bytes()
}

fn assemblies(pointer_size: PointerSize, code: &[(u32, &[u8])]) -> Option<Address> {
    let image = image(
        pointer_size,
        &[("il2cpp_domain_get_assemblies", 0x1000)],
        code,
    );
    with_process(&[(BASE, &image)], |process| {
        globals::assemblies(process, (Address::new(BASE), SIZE as u64), pointer_size)
    })
}

#[test]
fn finds_s_assemblies_through_the_x64_release_getter() {
    // sub rsp, 0x28; call getter. The getter is lea rax, [s_Assemblies]; ret.
    let call = [
        &[0x48, 0x83, 0xEC, 0x28, 0xE8][..],
        &displacement(0x1009, 0x1100),
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit64, &[(0x1000, &call), (0x1100, &getter)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_in_the_x64_master_export() {
    // mov rax, [s_Assemblies + 8]; sub rax, [s_Assemblies]; sar rax, 3;
    // mov [rdx], rax; mov rax, [s_Assemblies]; ret
    let code = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3008),
        &[0x48, 0x2B, 0x05],
        &displacement(0x100E, 0x3000),
        &[0x48, 0xC1, 0xF8, 0x03, 0x48, 0x89, 0x02, 0x48, 0x8B, 0x05],
        &displacement(0x101C, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit64, &[(0x1000, &code)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_through_the_x86_release_getter() {
    // push ebp; mov ebp, esp; call getter. The getter is mov eax, s_Assemblies; ret.
    let call = [&[0x55, 0x8B, 0xEC, 0xE8][..], &displacement(0x1008, 0x1100)].concat();
    let getter = [&[0xB8][..], &absolute(0x3000), &[0xC3]].concat();
    assert_eq!(
        assemblies(PointerSize::Bit32, &[(0x1000, &call), (0x1100, &getter)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_in_the_x86_master_export() {
    // push ebp; mov ebp, esp; mov ecx, [s_Assemblies + 4]; mov eax, [ebp + 0xC];
    // sub ecx, [s_Assemblies]; sar ecx, 2; mov [eax], ecx;
    // mov eax, [s_Assemblies]; pop ebp; ret
    let code = [
        &[0x55, 0x8B, 0xEC, 0x8B, 0x0D][..],
        &absolute(0x3004),
        &[0x8B, 0x45, 0x0C, 0x2B, 0x0D],
        &absolute(0x3000),
        &[0xC1, 0xF9, 0x02, 0x89, 0x08, 0xA1],
        &absolute(0x3000),
        &[0x5D, 0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit32, &[(0x1000, &code)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn takes_the_getter_over_a_load_in_the_next_function() {
    // sub rsp, 0x28; call getter; add rsp, 0x28; ret. The next function
    // starts at 0x1010 and ends with mov rax, [other]; ret.
    let call = [
        &[0x48, 0x83, 0xEC, 0x28, 0xE8][..],
        &displacement(0x1009, 0x1100),
        &[0x48, 0x83, 0xC4, 0x28, 0xC3],
    ]
    .concat();
    let next = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1017, 0x3800),
        &[0xC3],
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit64,
            &[(0x1000, &call), (0x1010, &next), (0x1100, &getter)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn skips_a_call_that_does_not_go_to_the_getter() {
    // call other; call getter. The other function starts with push rbx.
    let calls = [
        &[0xE8][..],
        &displacement(0x1005, 0x1200),
        &[0xE8],
        &displacement(0x100A, 0x1100),
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit64,
            &[(0x1000, &calls), (0x1100, &getter), (0x1200, &[0x40, 0x53])]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_no_s_assemblies_without_the_export() {
    let image = image(PointerSize::Bit64, &[], &[]);
    let found = with_process(&[(BASE, &image)], |process| {
        globals::assemblies(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, None);
}

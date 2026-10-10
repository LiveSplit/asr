//! Attachment to an older Linux player whose engine is in the executable,
//! rather than a separate `UnityPlayer.so` module.

use super::{profiles, Module};
use crate::runtime::mock::{poll_once, with_modules};
use std::{task::Poll, vec, vec::Vec};

const RUNTIME: u64 = 0x7F00_0000_0000;
#[cfg(feature = "alloc")]
const EXECUTABLE: u64 = 0x7F00_0010_0000;

// The real Unity 5.6.7f1 Linux legacy-Mono fixture ships this libmono.so ID
// and no UnityPlayer.so. The process memory below is synthetic: it provides
// a GNU note, the assembly export and its getter, but does not run Unity.
const BUILD_ID: [u8; 20] = [
    0xC1, 0xA5, 0x3E, 0xA7, 0x10, 0x9A, 0x2D, 0xA5, 0x82, 0x20, 0xAB, 0x30, 0xF4, 0xCA, 0xB7, 0xC8,
    0xCE, 0x8F, 0x38, 0x13,
];

fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn runtime() -> Vec<u8> {
    let mut image = vec![0; 0x2000];
    put(&mut image, 0, b"\x7fELF");
    image[4..7].copy_from_slice(&[2, 1, 1]);
    put(&mut image, 0x10, &3_u16.to_le_bytes());
    put(&mut image, 0x12, &0x3E_u16.to_le_bytes());
    put(&mut image, 0x20, &0x40_u64.to_le_bytes());
    put(&mut image, 0x36, &56_u16.to_le_bytes());
    put(&mut image, 0x38, &3_u16.to_le_bytes());
    put(&mut image, 0x40, &1_u32.to_le_bytes()); // PT_LOAD
    put(&mut image, 0x60, &0x2000_u64.to_le_bytes());
    put(&mut image, 0x68, &0x2000_u64.to_le_bytes());
    put(&mut image, 0x78, &4_u32.to_le_bytes()); // PT_NOTE
    put(&mut image, 0x88, &0x200_u64.to_le_bytes());
    put(&mut image, 0x98, &0x24_u64.to_le_bytes());
    put(&mut image, 0xA0, &0x24_u64.to_le_bytes());
    put(&mut image, 0xB0, &2_u32.to_le_bytes()); // PT_DYNAMIC
    put(&mut image, 0xC0, &0x300_u64.to_le_bytes());
    put(&mut image, 0xD0, &0x30_u64.to_le_bytes());
    put(&mut image, 0xD8, &0x30_u64.to_le_bytes());
    put(&mut image, 0x200, &4_u32.to_le_bytes());
    put(&mut image, 0x204, &20_u32.to_le_bytes());
    put(&mut image, 0x208, &3_u32.to_le_bytes());
    put(&mut image, 0x20C, b"GNU\0");
    put(&mut image, 0x210, &BUILD_ID);
    for (slot, (tag, value)) in [(6_u64, RUNTIME + 0x400), (5, RUNTIME + 0x500), (10, 0x80)]
        .into_iter()
        .enumerate()
    {
        put(&mut image, 0x300 + slot * 16, &tag.to_le_bytes());
        put(&mut image, 0x308 + slot * 16, &value.to_le_bytes());
    }
    put(&mut image, 0x418, &1_u32.to_le_bytes()); // second Elf64_Sym name
    put(&mut image, 0x420, &0x800_u64.to_le_bytes()); // symbol value
    put(&mut image, 0x428, &0x100_u64.to_le_bytes()); // symbol size
    put(&mut image, 0x430, &0x80_u32.to_le_bytes()); // terminate symbol walk
    put(&mut image, 0x501, b"mono_assembly_foreach\0");
    put(&mut image, 0x800, &[0x48, 0x8B, 0x3D]);
    put(&mut image, 0x803, &0x1F9_i32.to_le_bytes()); // global at 0xA00
    image
}

#[test]
fn an_explicit_profile_needs_neither_a_player_nor_alloc() {
    let runtime = runtime();
    with_modules(
        &[(RUNTIME, &runtime)],
        &[("libmono.so", RUNTIME, runtime.len() as u64)],
        |process| {
            let profile = profiles::UNITY_5_6_0F1_LINUX_MONO_X86_64;
            let attached = Module::attach(process, profile).unwrap();
            assert_eq!(attached.profile, profile);
            let Poll::Ready(attached) = poll_once(Module::wait_attach(process, profile)) else {
                panic!("the explicit profile should attach immediately");
            };
            assert_eq!(attached.profile, profile);
        },
    );
}

#[cfg(feature = "alloc")]
#[test]
fn an_unknown_legacy_runtime_uses_the_executables_version() {
    let runtime = runtime();
    let mut executable = vec![0; 0x2000];
    let version = b"\x005.6.7f1\0";
    put(&mut executable, 0x100, version);
    with_modules(
        &[(EXECUTABLE, &executable), (RUNTIME, &runtime)],
        &[
            ("fixture", EXECUTABLE, executable.len() as u64),
            ("libmono.so", RUNTIME, runtime.len() as u64),
        ],
        |process| {
            // Ensure this goes through version detection, not a known-ID match.
            assert!(super::linux_builds::find(&BUILD_ID).is_none());
            assert_eq!(
                Module::unity_version(process, super::BinaryFormat::ELF),
                Some((5, 6, 7, 0))
            );
            let profile = profiles::UNITY_5_6_0F1_LINUX_MONO_X86_64;
            let attached = Module::attach_auto_detect(process).unwrap();
            assert_eq!(attached.profile, profile);
            let Poll::Ready(attached) = poll_once(Module::wait_attach_auto_detect(process)) else {
                panic!("the executable's version should allow automatic attachment");
            };
            assert_eq!(attached.profile, profile);
        },
    );
}

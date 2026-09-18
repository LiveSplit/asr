//! Tests pinning which binary names a Linux build. The runtime binary's own
//! build ID answers whenever it has one; `UnityPlayer.so`'s answers only when
//! the runtime carries none at all.

use super::{BinaryFormat, Identity};
use crate::runtime::mock::with_process;
use crate::{Address, Process};

use std::{vec, vec::Vec};

const RUNTIME: u64 = 0x7F00_0000_0000;
const PLAYER: u64 = 0x7F00_0010_0000;

const RUNTIME_ID: [u8; 20] = [
    0x4E, 0x69, 0x0F, 0x26, 0x4A, 0x2A, 0x90, 0x12, 0x03, 0x47, 0xF9, 0x4A, 0xE4, 0x3B, 0x0B, 0x83,
    0xD5, 0x78, 0xF6, 0x86,
];
const PLAYER_ID: [u8; 8] = [0xC1, 0x91, 0x05, 0xBA, 0x5A, 0xAB, 0xAF, 0x80];

fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

// A mapped 64-bit ELF holding one note segment, carrying the given build ID
// when there is one to carry.
fn image(build_id: Option<&[u8]>) -> Vec<u8> {
    let mut image = vec![0; 0x400];
    put(&mut image, 0x00, b"\x7fELF");
    image[0x04] = 2;
    image[0x05] = 1;
    image[0x06] = 1;
    put(&mut image, 0x10, &3_u16.to_le_bytes());
    put(&mut image, 0x20, &0x40_u64.to_le_bytes());
    put(&mut image, 0x36, &56_u16.to_le_bytes());
    put(&mut image, 0x38, &2_u16.to_le_bytes());
    put(&mut image, 0x40, &1_u32.to_le_bytes());

    let Some(build_id) = build_id else {
        return image;
    };

    put(&mut image, 0x78, &4_u32.to_le_bytes());
    put(&mut image, 0x88, &0x200_u64.to_le_bytes());
    put(
        &mut image,
        0x98,
        &(0x10 + build_id.len() as u64).to_le_bytes(),
    );
    put(&mut image, 0x200, &4_u32.to_le_bytes());
    put(&mut image, 0x204, &(build_id.len() as u32).to_le_bytes());
    put(&mut image, 0x208, &3_u32.to_le_bytes());
    put(&mut image, 0x20C, b"GNU\0");
    put(&mut image, 0x210, build_id);
    image
}

fn on_modules(runtime: Option<&[u8]>, player: Option<&[u8]>, test: impl FnOnce(&Process)) {
    let runtime = image(runtime);
    let player = image(player);
    with_process(&[(RUNTIME, &runtime), (PLAYER, &player)], |process| {
        test(process)
    });
}

fn read(process: &Process, player: bool) -> Option<Identity> {
    Identity::read(
        process,
        ((Address::new(RUNTIME), 0x400), "libmonobdwgc-2.0.so"),
        player.then(|| (Address::new(PLAYER), 0x400)),
        BinaryFormat::ELF,
    )
}

fn named(identity: Option<Identity>) -> (Vec<u8>, &'static str) {
    match identity {
        Some(Identity::Build(build_id, module)) => (build_id.as_bytes().to_vec(), module),
        _ => panic!("the identity is a build ID"),
    }
}

#[test]
fn runtimes_are_named_by_their_own_build_id() {
    on_modules(Some(&RUNTIME_ID), None, |process| {
        let (build_id, module) = named(read(process, false));
        assert_eq!(build_id, RUNTIME_ID);
        assert_eq!(module, "libmonobdwgc-2.0.so");
    });
}

// A runtime binary carrying no build ID is the later Linux shape, where the
// player is the only thing that names the build.
#[test]
fn runtimes_without_one_are_named_by_the_player() {
    on_modules(None, Some(&PLAYER_ID), |process| {
        let (build_id, module) = named(read(process, true));
        assert_eq!(build_id, PLAYER_ID);
        assert_eq!(module, "UnityPlayer.so");
    });
}

// A runtime binary that names itself is the one that answers, known to the
// table or not. Reaching past it to the player would pair a runtime nobody
// measured with another binary's offsets, and attach with them.
#[test]
fn runtimes_that_name_themselves_are_never_passed_over() {
    on_modules(Some(&RUNTIME_ID), Some(&PLAYER_ID), |process| {
        let (build_id, module) = named(read(process, true));
        assert_eq!(build_id, RUNTIME_ID);
        assert_eq!(module, "libmonobdwgc-2.0.so");
    });
}

#[test]
fn binaries_without_any_build_id_name_nothing() {
    on_modules(None, None, |process| {
        assert!(read(process, true).is_none());
        assert!(read(process, false).is_none());
    });
}

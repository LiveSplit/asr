//! Tests pinning how a scene's path is read. The path field is 32 bytes.
//! Players before 2021 keep a pointer to the characters there. From 2021
//! on, an x64 player keeps a path of up to 31 characters inline, with a NUL
//! after it, and a longer path behind a pointer followed by its length and
//! capacity. From 2023.1 on, the last byte of the field holds 31 minus the
//! length while the path is inline.

use super::{builds, offsets::PathShape, Scene, SceneManager};
use crate::{runtime::mock::with_process, string::ArrayCString, Address, PointerSize, Process};
use std::vec;

const BASE: u64 = 0x7FF6_1000_0000;
const SCENE: u64 = 0x100;
const TEXT: u64 = 0x800;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn manager(unity: (u16, u16, u16, u16), pointer_size: PointerSize) -> SceneManager {
    SceneManager {
        pointer_size,
        is_il2cpp: false,
        address: Address::new(BASE),
        profile: &builds::nearest(unity, pointer_size).unwrap().profile,
    }
}

fn read_path(image: &[u8], manager: &SceneManager) -> Option<ArrayCString<128>> {
    with_process(&[(BASE, image)], |process: &Process| {
        let scene = Scene {
            address: Address::new(BASE + SCENE),
        };
        scene.path(process, manager).ok()
    })
}

// The path field of the scene sits at 0x10 on every x64 build and at 0xC on
// the x86 builds before Unity 6000.5.
const fn field(manager: &SceneManager) -> u64 {
    SCENE + manager.profile.scene.path as u64
}

// A path kept behind a pointer, with the length and the capacity after it.
// Only x64 players keep a path inline, so only they spill one.
fn spilled(image: &mut [u8], at: u64, text: &[u8]) {
    put(image, TEXT, text);
    put(image, at, &(BASE + TEXT).to_le_bytes());
    put(image, at + 8, &(text.len() as u64).to_le_bytes());
    put(image, at + 16, &(text.len() as u64).to_le_bytes());
}

#[test]
fn x64_builds_change_the_path_shape_on_2021_and_2023_1() {
    for build in builds::BUILDS {
        if build.profile.pointer_size != PointerSize::Bit64 {
            continue;
        }
        let major_minor = (build.unity.0, build.unity.1);
        let expected = if major_minor < (2021, 1) {
            PathShape::Pointer
        } else if major_minor < (2023, 1) {
            PathShape::InlineNul
        } else {
            PathShape::InlineSpare
        };
        assert_eq!(build.profile.path, expected, "{:?}", build.unity);
    }
}

#[test]
fn x86_builds_keep_the_path_behind_a_pointer() {
    for build in builds::BUILDS {
        if build.profile.pointer_size == PointerSize::Bit32 {
            assert_eq!(build.profile.path, PathShape::Pointer, "{:?}", build.unity);
        }
    }
}

// The inline path starts with Unity 2021.1, so a 2021.1 or 2021.2 player
// takes the 2021.1 build and its shape, and a 2020.x player the pointer.
#[test]
fn the_first_inline_path_build_is_2021_1() {
    let shape = |player| {
        let build = builds::nearest(player, PointerSize::Bit64).unwrap();
        (build.unity, build.profile.path)
    };
    let first = (2021, 1, 29, 10531);
    assert_eq!(shape((2021, 1, 0, 1)), (first, PathShape::InlineNul));
    assert_eq!(shape((2021, 2, 20, 62729)), (first, PathShape::InlineNul));
    assert_eq!(shape((2021, 3, 0, 44232)), (first, PathShape::InlineNul));
    assert_eq!(
        shape((2020, 3, 48, 1)),
        ((2018, 4, 36, 54151), PathShape::Pointer)
    );
}

#[test]
fn a_pointer_path_reads_through_the_pointer() {
    let manager = manager((2018, 4, 36, 54151), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    put(&mut image, TEXT, b"Assets/Scenes/Boot.unity");
    put(&mut image, field(&manager), &(BASE + TEXT).to_le_bytes());

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Boot.unity");
}

#[test]
fn a_short_path_reads_inline_with_the_spare_byte() {
    let manager = manager((2023, 1, 22, 16744), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    let at = field(&manager);
    put(&mut image, at, b"Assets/Scenes/Boot.unity");
    put(&mut image, at + 31, &[31 - 24]);

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Boot.unity");
}

#[test]
fn a_path_of_31_characters_fills_the_field() {
    let manager = manager((2023, 1, 22, 16744), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    let at = field(&manager);
    put(&mut image, at, b"Assets/Scenes/Deep/Deeper.unity");
    put(&mut image, at + 31, &[0]);

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Deep/Deeper.unity");
}

#[test]
fn an_empty_path_reads_inline() {
    let manager = manager((2023, 1, 22, 16744), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    put(&mut image, field(&manager) + 31, &[31]);

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"");
}

// The spilled form is tried first, so the last byte of the field, which is
// unrelated data while the path is spilled, never fakes an inline path.
#[test]
fn a_long_path_reads_through_the_pointer_beside_the_spare_byte() {
    let manager = manager((2023, 1, 22, 16744), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    let at = field(&manager);
    spilled(&mut image, at, b"Assets/Scenes/Deeply/Nested/Second.unity");
    put(&mut image, at + 24, &[0x41; 8]);
    put(&mut image, at + 31, &[31 - 6]);

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Deeply/Nested/Second.unity");
}

// Before the spare byte existed, the bytes past the NUL of an inline path
// are whatever was there before.
#[test]
fn a_short_path_reads_inline_up_to_the_nul() {
    let manager = manager((2021, 1, 29, 10531), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    let at = field(&manager);
    put(&mut image, at, b"Assets/Scenes/Boot.unity\0");
    put(&mut image, at + 25, &[0xCC; 7]);

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Boot.unity");
}

#[test]
fn a_long_path_reads_through_the_pointer_before_the_spare_byte_existed() {
    let manager = manager((2021, 1, 29, 10531), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    spilled(
        &mut image,
        field(&manager),
        b"Assets/Scenes/Deeply/Nested/Second.unity",
    );

    let path = read_path(&image, &manager).unwrap();
    assert_eq!(path.as_bytes(), b"Assets/Scenes/Deeply/Nested/Second.unity");
}

// A spilled path whose pointer leads to text of another length is not
// trusted, and the field is not an inline path either.
#[test]
fn a_spilled_path_of_the_wrong_length_is_refused() {
    let manager = manager((2023, 1, 22, 16744), PointerSize::Bit64);
    let mut image = vec![0; 0x1000];
    let at = field(&manager);
    spilled(&mut image, at, b"Assets/Scenes/Deeply/Nested/Second.unity");
    put(&mut image, at + 8, &39_u64.to_le_bytes());

    assert!(read_path(&image, &manager).is_none());
}

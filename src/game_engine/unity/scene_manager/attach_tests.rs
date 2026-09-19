//! Tests pinning how the scene manager is found through a measured build:
//! the anchor of the build is scanned in the engine module, every match
//! must name one global, and the global holds the manager.

use super::{builds, Scene, SceneManager};
use crate::{runtime::mock::with_modules, Address, PointerSize, Process};
use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x7FF6_1000_0000;

// An x86 player lives below 4 GiB, and its anchor holds the absolute address
// of the global.
const BASE32: u64 = 0x1000_0000;

// The module is the first 0x1000 bytes of the image. The scanner reads a
// little past the end of the module, so the image holds bytes there too.
const MODULE: u64 = 0x1000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

// The Unity 6 x64 anchor is the body of the scene count getter. The load's
// displacement is relative to the instruction after it, which sits 7 bytes
// into the body.
fn anchor_x64(image: &mut [u8], at: u64, global: u64) {
    let next = BASE + at + 7;
    let displacement = (global as i64 - next as i64) as i32;
    put(image, at, &[0x48, 0x8B, 0x05]);
    put(image, at + 3, &displacement.to_le_bytes());
    put(image, at + 7, &[0x8B, 0x40, 0x18, 0xC3]);
}

// The Unity 6 x86 anchor loads the global through its absolute address.
fn anchor_x86(image: &mut [u8], at: u64, global: u64) {
    put(image, at, &[0xA1]);
    put(image, at + 1, &(global as u32).to_le_bytes());
    put(image, at + 5, &[0x3B, 0x50, 0x10]);
}

fn attach(image: &[u8], pointer_size: PointerSize) -> Option<SceneManager> {
    let profile = &builds::nearest((6000, 3, 21, 9777), pointer_size)
        .unwrap()
        .profile;
    let base = match pointer_size {
        PointerSize::Bit64 => BASE,
        _ => BASE32,
    };
    with_modules(
        &[(base, image)],
        &[("UnityPlayer.dll", base, MODULE)],
        |process| SceneManager::attach_with(process, (Address::new(base), MODULE), profile),
    )
}

#[test]
fn nearest_is_the_build_itself_on_a_measured_player() {
    for build in builds::BUILDS {
        let found = builds::nearest(build.unity, build.profile.pointer_size).unwrap();
        assert_eq!(found.unity, build.unity);
        assert_eq!(found.profile.pointer_size, build.profile.pointer_size);
    }
}

#[test]
fn nearest_takes_the_newest_build_at_or_below_the_major_minor() {
    let unity = |player| builds::nearest(player, PointerSize::Bit64).unwrap().unity;
    assert_eq!(unity((6000, 3, 5, 1)), (6000, 3, 21, 9777));
    assert_eq!(unity((6000, 4, 0, 62614)), (6000, 3, 21, 9777));
    assert_eq!(unity((6000, 0, 58, 1)), (6000, 0, 84, 43887));
    assert_eq!(unity((6000, 1, 17, 47571)), (6000, 0, 84, 43887));
    assert_eq!(unity((6000, 2, 12, 40285)), (6000, 0, 84, 43887));
    assert_eq!(unity((2019, 4, 41, 9172)), (2018, 4, 36, 54151));
    assert_eq!(unity((7000, 0, 0, 0)), (6000, 5, 10, 54518));
    assert_eq!(unity((5, 6, 7, 0)), (5, 6, 7, 3267));
    assert_eq!(unity((5, 5, 0, 0)), (5, 6, 7, 3267));
}

// Every layout starts with an entry at the pointer size it holds for. The
// x86 players of Unity 6000.1 keep the root list of a scene 4 bytes earlier
// than the players of 6000.0 and 6000.2, so x86 has entries at all three
// where x64 has one.
#[test]
fn x86_entries_follow_the_root_list_move_of_6000_1() {
    let roots = |player| {
        let build = builds::nearest(player, PointerSize::Bit32).unwrap();
        (build.unity, build.profile.scene.roots)
    };
    assert_eq!(roots((6000, 0, 58, 1)), ((6000, 0, 84, 43887), 0x98));
    assert_eq!(roots((6000, 1, 17, 47571)), ((6000, 1, 17, 47571), 0x94));
    assert_eq!(roots((6000, 2, 12, 40285)), ((6000, 2, 12, 40285), 0x98));
    assert_eq!(roots((6000, 3, 21, 9777)), ((6000, 3, 21, 9777), 0x98));
}

#[test]
fn table_reads_oldest_to_newest() {
    for pointer_size in [PointerSize::Bit64, PointerSize::Bit32] {
        let mut last = (0, 0, 0, 0);
        for build in builds::BUILDS {
            if build.profile.pointer_size != pointer_size {
                continue;
            }
            assert!(build.unity > last, "{:?} after {:?}", build.unity, last);
            last = build.unity;
        }
    }
}

#[test]
fn x64_anchor_reaches_the_manager_through_a_relative_load() {
    let mut image = vec![0; 0x2000];
    anchor_x64(&mut image, 0x100, BASE + 0x800);
    put(&mut image, 0x800, &(BASE + 0x900).to_le_bytes());

    let manager = attach(&image, PointerSize::Bit64).unwrap();
    assert_eq!(manager.address, Address::new(BASE + 0x900));
    assert_eq!(manager.pointer_size, PointerSize::Bit64);
}

#[test]
fn x86_anchor_reaches_the_manager_through_an_absolute_load() {
    let mut image = vec![0; 0x2000];
    anchor_x86(&mut image, 0x100, BASE32 + 0x800);
    put(&mut image, 0x800, &((BASE32 + 0x900) as u32).to_le_bytes());

    let manager = attach(&image, PointerSize::Bit32).unwrap();
    assert_eq!(manager.address, Address::new(BASE32 + 0x900));
    assert_eq!(manager.pointer_size, PointerSize::Bit32);
}

#[test]
fn matches_that_agree_on_the_global_attach() {
    let mut image = vec![0; 0x2000];
    anchor_x64(&mut image, 0x100, BASE + 0x800);
    anchor_x64(&mut image, 0x200, BASE + 0x800);
    put(&mut image, 0x800, &(BASE + 0x900).to_le_bytes());

    assert!(attach(&image, PointerSize::Bit64).is_some());
}

#[test]
fn matches_that_disagree_on_the_global_do_not_attach() {
    let mut image = vec![0; 0x2000];
    anchor_x64(&mut image, 0x100, BASE + 0x800);
    anchor_x64(&mut image, 0x200, BASE + 0x808);
    put(&mut image, 0x800, &(BASE + 0x900).to_le_bytes());
    put(&mut image, 0x808, &(BASE + 0x900).to_le_bytes());

    assert!(attach(&image, PointerSize::Bit64).is_none());
}

#[test]
fn a_null_manager_does_not_attach() {
    let mut image = vec![0; 0x2000];
    anchor_x64(&mut image, 0x100, BASE + 0x800);

    assert!(attach(&image, PointerSize::Bit64).is_none());
}

// The manager keeps its loaded scenes in a dynamic array at 0x8, with the
// pointer first and the size two pointers in. The next array starts at 0x28.
#[test]
fn scenes_come_from_the_loaded_scene_array() {
    let mut image = vec![0; 0x2000];
    anchor_x64(&mut image, 0x100, BASE + 0x800);
    put(&mut image, 0x800, &(BASE + 0x900).to_le_bytes());
    put(&mut image, 0x900 + 0x8, &(BASE + 0xA00).to_le_bytes());
    put(&mut image, 0x900 + 0x18, &2_u64.to_le_bytes());
    put(&mut image, 0x900 + 0x28, &(BASE + 0xB00).to_le_bytes());
    put(&mut image, 0xA00, &(BASE + 0xC00).to_le_bytes());
    put(&mut image, 0xA08, &(BASE + 0xD00).to_le_bytes());
    put(&mut image, 0xB00, &(BASE + 0xE00).to_le_bytes());
    put(&mut image, 0xB08, &(BASE + 0xE00).to_le_bytes());

    let scenes: Vec<Address> = with_modules(
        &[(BASE, &image)],
        &[("UnityPlayer.dll", BASE, MODULE)],
        |process: &Process| {
            let profile = &builds::nearest((6000, 3, 21, 9777), PointerSize::Bit64)
                .unwrap()
                .profile;
            let manager =
                SceneManager::attach_with(process, (Address::new(BASE), MODULE), profile).unwrap();
            assert_eq!(manager.get_scene_count(process).unwrap(), 2);
            manager
                .scenes(process)
                .map(|scene: Scene| scene.address())
                .collect()
        },
    );
    assert_eq!(
        scenes,
        [Address::new(BASE + 0xC00), Address::new(BASE + 0xD00)]
    );
}

// Unity 5.6 links the engine into the game's own executable. Its x86 anchor
// is the head of the function that tears the manager down: the load of the
// global, a null check, a virtual call, and the 0x58 bytes it frees, which
// tell it apart from the other teardowns sharing the same head.
fn anchor_5_6_x86(image: &mut [u8], at: u64, global: u64, freed: u8) {
    put(image, at, &[0x8B, 0x0D]);
    put(image, at + 2, &(global as u32).to_le_bytes());
    put(
        image,
        at + 6,
        &[
            0x56, 0x8B, 0xF1, 0x85, 0xC9, 0x74, 0x08, 0x8B, 0x01, 0x8B, 0x10, 0x6A, 0x00, 0xFF,
            0xD2, 0x6A, freed, 0x56,
        ],
    );
}

#[test]
fn the_5_6_x86_anchor_needs_the_whole_teardown_head() {
    let profile = &builds::nearest((5, 6, 7, 3267), PointerSize::Bit32)
        .unwrap()
        .profile;
    let mut image = vec![0; 0x2000];
    anchor_5_6_x86(&mut image, 0x100, BASE32 + 0x800, 0x58);
    anchor_5_6_x86(&mut image, 0x200, BASE32 + 0x808, 0x40);
    put(&mut image, 0x800, &((BASE32 + 0x900) as u32).to_le_bytes());
    put(&mut image, 0x808, &((BASE32 + 0xA00) as u32).to_le_bytes());

    let manager = with_modules(
        &[(BASE32, &image)],
        &[("game.exe", BASE32, MODULE)],
        |process| SceneManager::attach_with(process, (Address::new(BASE32), MODULE), profile),
    )
    .unwrap();
    assert_eq!(manager.address, Address::new(BASE32 + 0x900));
}

// The least of a PE header: the DOS header pointing at the COFF header, an
// x64 machine, and an optional header carrying the size of the image.
fn pe_header(image: &mut [u8]) {
    put(image, 0, b"MZ");
    put(image, 0x3C, &0x80_u32.to_le_bytes());
    put(image, 0x80, b"PE  ");
    put(image, 0x84, &0x8664_u16.to_le_bytes());
    put(image, 0x94, &0xF0_u16.to_le_bytes());
    put(image, 0x98, &0x20B_u16.to_le_bytes());
    put(image, 0x98 + 0x38, &(MODULE as u32).to_le_bytes());
}

#[test]
fn the_engine_module_is_the_player_when_there_is_one() {
    let mut player = vec![0; 0x200];
    pe_header(&mut player);
    with_modules(
        &[(BASE + 0x10000, &player)],
        &[
            ("game.exe", BASE, MODULE),
            ("UnityPlayer.dll", BASE + 0x10000, MODULE),
        ],
        |process| {
            let (range, format) = SceneManager::engine_module(process).unwrap();
            assert_eq!(range, (Address::new(BASE + 0x10000), MODULE));
            assert_eq!(format, super::BinaryFormat::PE);
        },
    );
}

// Without a player module the engine is linked into the game's executable,
// the first module of the mock process.
#[cfg(feature = "alloc")]
#[test]
fn the_engine_module_is_the_executable_when_there_is_no_player() {
    let mut executable = vec![0; 0x200];
    pe_header(&mut executable);
    with_modules(
        &[(BASE, &executable)],
        &[("game.exe", BASE, MODULE)],
        |process| {
            let (range, format) = SceneManager::engine_module(process).unwrap();
            assert_eq!(range, (Address::new(BASE), MODULE));
            assert_eq!(format, super::BinaryFormat::PE);
        },
    );
}

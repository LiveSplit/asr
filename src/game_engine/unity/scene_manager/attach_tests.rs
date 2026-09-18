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
    assert_eq!(unity((5, 6, 7, 0)), (2017, 4, 40, 5126));
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

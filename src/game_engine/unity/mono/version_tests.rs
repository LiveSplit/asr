//! Tests pinning how a Mono game that is no known build picks its offsets:
//! the newest build at or below its Unity version, for the library it runs,
//! at its pointer size, and where that version comes from.

#[cfg(feature = "alloc")]
use super::mac_builds;
use super::{builds, linux_builds, Library, Module};
use crate::runtime::mock::{unity_player_image, with_modules, with_process};
use crate::{Address, PointerSize, Process};
use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x1900_0000;

// Checks the rule against every entry of the Windows table, so adding or
// moving an entry needs no change here.
#[test]
fn a_player_takes_the_newest_build_at_or_below_its_patch() {
    let patch = |v: (u16, u16, u16, u16)| (v.0, v.1, v.2);
    for library in [Library::Mono, Library::MonoBdwgc] {
        for pointer_size in [PointerSize::Bit64, PointerSize::Bit32] {
            let picked = |player| {
                builds::nearest(player, library, pointer_size)
                    .unwrap()
                    .unity
            };
            let table: Vec<_> = builds::BUILDS
                .iter()
                .filter(|build| {
                    build.profile.library == library && build.profile.pointer_size == pointer_size
                })
                .map(|build| build.unity)
                .collect();

            assert!(builds::nearest((0, 0, 0, 0), library, pointer_size).is_none());
            assert_eq!(picked((u16::MAX, 0, 0, 0)), *table.last().unwrap());

            for (index, &entry) in table.iter().enumerate() {
                for build in [0, u16::MAX] {
                    let player = (entry.0, entry.1, entry.2, build);
                    assert_eq!(patch(picked(player)), patch(entry), "{player:?}");
                }
                // The version right below an entry reads the entry before it,
                // across a major or minor too, and nothing below the first entry.
                let below = match entry {
                    (major, minor, number, _) if number > 0 => (major, minor, number - 1, u16::MAX),
                    (major, minor, _, _) if minor > 0 => (major, minor - 1, u16::MAX, u16::MAX),
                    (major, _, _, _) => (major - 1, u16::MAX, u16::MAX, u16::MAX),
                };
                if index == 0 {
                    let found = builds::nearest(below, library, pointer_size);
                    assert!(found.is_none(), "{below:?}");
                } else {
                    assert_eq!(picked(below), table[index - 1], "{below:?}");
                }
            }
        }
    }
}

#[test]
fn nearest_is_the_build_itself_on_a_measured_player() {
    for build in builds::BUILDS {
        let found = builds::nearest(
            build.unity,
            build.profile.library,
            build.profile.pointer_size,
        )
        .unwrap();
        assert_eq!(found.unity, build.unity, "{:?}", build.unity);
        assert_eq!(found.profile.library, build.profile.library);
    }
}

#[test]
fn linux_builds_take_the_nearest_too() {
    let linux = |player, library| {
        linux_builds::nearest(player, library, PointerSize::Bit64)
            .unwrap()
            .unity
    };
    assert_eq!(
        linux((2020, 1, 0, 0), Library::MonoBdwgc),
        (2017, 3, 0, 63597)
    );
    assert_eq!(
        linux((6000, 0, 0, 0), Library::MonoBdwgc),
        (2021, 2, 20, 62729)
    );
    assert_eq!(linux((2017, 2, 0, 0), Library::Mono), (5, 6, 7, 3267));
}

#[cfg(feature = "alloc")]
#[test]
fn mac_builds_take_the_nearest_too() {
    let mac = mac_builds::nearest((2022, 3, 0, 0), Library::MonoBdwgc, PointerSize::Bit64).unwrap();
    assert_eq!(mac.unity, (6000, 5, 10, 54518));
}

// Linux and Mac players carry no file version. The Unity version sits in
// the player as a string like `2021.3.11f1`, and only a string of that
// shape counts, so a date or a build number nearby is skipped.
#[test]
fn the_version_string_in_the_player_names_major_minor_and_patch() {
    let version = |text: &[u8]| {
        let mut image = vec![0; 0x2000];
        image[0x100..0x100 + text.len()].copy_from_slice(text);
        with_process(&[(BASE, &image)], |process: &Process| {
            Module::version_string(process, (Address::new(BASE), 0x1000))
        })
    };
    assert_eq!(version(b"\x002021.3.11f1\0"), Some((2021, 3, 11, 0)));
    assert_eq!(version(b"\x006000.5.10f1\0"), Some((6000, 5, 10, 0)));
    assert_eq!(version(b"\x005.6.7f1\0"), Some((5, 6, 7, 0)));
    assert_eq!(version(b"\x002023.05.01\0"), None);
    assert_eq!(
        version(b"\x002023.05\0\x002024.1.0b3\0"),
        Some((2024, 1, 0, 0))
    );
}

// On Windows the file version of `UnityPlayer.dll` is the Unity version.
#[test]
fn the_player_file_version_is_the_unity_version() {
    let player = unity_player_image((2021, 3, 11, 23713));
    with_modules(
        &[(BASE, &player)],
        &[("UnityPlayer.dll", BASE, 0x1000)],
        |process| {
            assert_eq!(
                Module::unity_version(process, super::BinaryFormat::PE),
                Some((2021, 3, 11, 23713))
            );
        },
    );
}

// A library and pointer size nobody measured have no nearest build.
#[test]
fn nothing_is_nearest_where_nothing_was_measured() {
    assert!(linux_builds::nearest((2020, 1, 0, 0), Library::Mono, PointerSize::Bit32).is_none());
    assert!(
        builds::nearest((2020, 1, 18, 38512), Library::MonoBdwgc, PointerSize::Bit16).is_none()
    );
    assert!(builds::nearest((0, 0, 0, 0), Library::Mono, PointerSize::Bit32).is_none());
}

// A Windows game before Unity 2017.2 ships no player module. Its executable
// carries the file version, and finding the executable takes the `alloc`
// feature.
#[cfg(feature = "alloc")]
#[test]
fn the_executable_carries_the_unity_version_when_there_is_no_player() {
    let executable = unity_player_image((5, 6, 7, 3267));
    with_modules(
        &[(BASE, &executable)],
        &[
            ("game.exe", BASE, 0x1000),
            ("mono.dll", BASE + 0x10000, 0x1000),
        ],
        |process| {
            assert_eq!(
                Module::unity_version(process, super::BinaryFormat::PE),
                Some((5, 6, 7, 3267))
            );
        },
    );
}

#[cfg(not(feature = "alloc"))]
#[test]
fn without_alloc_a_game_with_no_player_has_no_version() {
    let executable = unity_player_image((5, 6, 7, 3267));
    with_modules(
        &[(BASE, &executable)],
        &[
            ("game.exe", BASE, 0x1000),
            ("mono.dll", BASE + 0x10000, 0x1000),
        ],
        |process| {
            assert_eq!(
                Module::unity_version(process, super::BinaryFormat::PE),
                None
            );
        },
    );
}

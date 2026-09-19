//! Tests pinning how a Mono game that is no known build picks its offsets:
//! the build of its Unity version, or else the newest build whose
//! major.minor is below it, for the library it runs, at its pointer size,
//! and where that version comes from.

#[cfg(feature = "alloc")]
use super::mac_builds;
use super::{builds, linux_builds, Library, Module};
use crate::runtime::mock::{unity_player_image, with_modules, with_process};
use crate::{Address, PointerSize, Process};
use std::vec;

const BASE: u64 = 0x1900_0000;

#[test]
fn nearest_takes_the_newest_build_at_or_below_the_major_minor() {
    let unity = |player, pointer_size| {
        builds::nearest(player, Library::MonoBdwgc, pointer_size)
            .unwrap()
            .unity
    };
    let x64 = PointerSize::Bit64;
    assert_eq!(unity((2020, 2, 0, 8671), x64), (2020, 1, 18, 38512));
    assert_eq!(unity((2021, 1, 29, 10531), x64), (2020, 1, 18, 38512));
    assert_eq!(unity((2021, 2, 0, 1), x64), (2021, 2, 20, 62729));
    assert_eq!(unity((2021, 2, 20, 62729), x64), (2021, 2, 20, 62729));
    assert_eq!(unity((2021, 3, 0, 44232), x64), (2021, 3, 11, 23713));
    assert_eq!(unity((2022, 2, 0, 56532), x64), (2021, 3, 11, 23713));
    assert_eq!(unity((6000, 0, 84, 43887), x64), (2023, 1, 22, 16744));
    assert_eq!(unity((7000, 0, 0, 0), x64), (6000, 7, 0, 5476));
    assert_eq!(
        unity((2019, 4, 41, 9172), PointerSize::Bit32),
        (2019, 4, 41, 9172)
    );
}

#[test]
fn libraries_do_not_mix() {
    let mono = |player| {
        builds::nearest(player, Library::Mono, PointerSize::Bit64)
            .unwrap()
            .unity
    };
    assert_eq!(mono((2019, 4, 41, 9172)), (2018, 4, 36, 54151));
    assert_eq!(mono((2017, 1, 5, 1)), (5, 6, 7, 3267));
    assert_eq!(mono((5, 4, 0, 0)), (5, 6, 7, 3267));
    assert_eq!(mono((2018, 2, 0, 0)), (2017, 4, 40, 5126));
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
        (2019, 4, 41, 9172)
    );
    assert_eq!(
        linux((6000, 0, 0, 0), Library::MonoBdwgc),
        (2023, 1, 22, 16744)
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
    assert_eq!(version(b"\02021.3.11f1\0"), Some((2021, 3, 11, 0)));
    assert_eq!(version(b"\06000.5.10f1\0"), Some((6000, 5, 10, 0)));
    assert_eq!(version(b"\05.6.7f1\0"), Some((5, 6, 7, 0)));
    assert_eq!(version(b"\02023.05.01\0"), None);
    assert_eq!(version(b"\02023.05\0\02024.1.0b3\0"), Some((2024, 1, 0, 0)));
}

// On Windows the file version of `UnityPlayer.dll` names the Unity version.
#[test]
fn the_player_file_version_names_the_unity_version() {
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
    assert_eq!(
        builds::nearest((0, 0, 0, 0), Library::Mono, PointerSize::Bit32)
            .unwrap()
            .unity,
        (5, 6, 7, 3267)
    );
}

// A game before Unity 2017.1 ships no player module. Its executable carries
// the file version, and naming the executable takes the `alloc` feature.
#[cfg(feature = "alloc")]
#[test]
fn the_executable_names_the_unity_version_when_there_is_no_player() {
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

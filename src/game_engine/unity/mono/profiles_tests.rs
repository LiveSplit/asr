//! Tests pinning the public profile constants: each names the measured
//! player it came from, and a module attaches with one directly.

use super::profiles::*;
use super::{builds, linux_builds, Library, Profile};
use crate::PointerSize;

// The Windows table is sorted by GUID, so an entry added between two others
// renumbers everything after it. Every constant is pinned to the build it
// names, so a renumbering fails here rather than in a splitter.
#[test]
fn windows_constants_name_the_expected_profiles() {
    let profiles: [((u16, u16, u16, u16), Library, PointerSize, Profile); 28] = [
        (
            (5, 6, 7, 3267),
            Library::Mono,
            PointerSize::Bit64,
            UNITY_5_6_7F1_WINDOWS_MONO_X86_64,
        ),
        (
            (5, 6, 7, 3267),
            Library::Mono,
            PointerSize::Bit32,
            UNITY_5_6_7F1_WINDOWS_MONO_X86,
        ),
        (
            (2017, 4, 40, 5126),
            Library::Mono,
            PointerSize::Bit64,
            UNITY_2017_4_40F1_WINDOWS_MONO_X86_64,
        ),
        (
            (2017, 4, 40, 5126),
            Library::Mono,
            PointerSize::Bit32,
            UNITY_2017_4_40F1_WINDOWS_MONO_X86,
        ),
        (
            (2017, 4, 40, 5126),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2017, 4, 40, 5126),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2018, 4, 36, 54151),
            Library::Mono,
            PointerSize::Bit64,
            UNITY_2018_4_36F1_WINDOWS_MONO_X86_64,
        ),
        (
            (2018, 4, 36, 54151),
            Library::Mono,
            PointerSize::Bit32,
            UNITY_2018_4_36F1_WINDOWS_MONO_X86,
        ),
        (
            (2018, 4, 36, 54151),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2018, 4, 36, 54151),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2019, 4, 41, 9172),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2019_4_41F2_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2019, 4, 41, 9172),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2019_4_41F2_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2020, 1, 18, 38512),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2020_1_18F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2020, 1, 18, 38512),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2020_1_18F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2021, 2, 20, 62729),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2021_2_20F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2021, 2, 20, 62729),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2021_2_20F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2021, 3, 11, 23713),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2021_3_11F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2021, 3, 11, 23713),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2021_3_11F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2023, 1, 22, 16744),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2023_1_22F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2023, 1, 22, 16744),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2023_1_22F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (6000, 2, 12, 40285),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_6000_2_12F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 2, 12, 40285),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_6000_2_12F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (6000, 3, 21, 9777),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_6000_3_21F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 3, 21, 9777),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_6000_3_21F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (6000, 5, 8, 47071),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_6000_5_8F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 5, 8, 47071),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_6000_5_8F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (6000, 7, 0, 5476),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_6000_7_0A3_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 7, 0, 5476),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_6000_7_0A3_WINDOWS_MONO_BDWGC_X86,
        ),
    ];
    for (unity, library, pointer_size, profile) in profiles {
        let build = builds::nearest(unity, library, pointer_size).unwrap();
        assert_eq!(build.unity, unity, "{unity:?} {library:?} {pointer_size:?}");
        assert_eq!(
            build.profile, profile,
            "{unity:?} {library:?} {pointer_size:?}"
        );
        assert_eq!(profile.pointer_size, pointer_size);
        assert_eq!(profile.library, library);
    }
}

#[test]
fn linux_constants_name_the_expected_profiles() {
    let profiles: [((u16, u16, u16, u16), Library, Profile); 13] = [
        (
            (5, 6, 7, 3267),
            Library::Mono,
            UNITY_5_6_7F1_LINUX_MONO_X86_64,
        ),
        (
            (2017, 4, 40, 5126),
            Library::Mono,
            UNITY_2017_4_40F1_LINUX_MONO_X86_64,
        ),
        (
            (2018, 4, 36, 54151),
            Library::MonoBdwgc,
            UNITY_2018_4_36F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2019, 4, 41, 9172),
            Library::MonoBdwgc,
            UNITY_2019_4_41F2_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2021, 3, 0, 44232),
            Library::MonoBdwgc,
            UNITY_2021_3_0F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2021, 3, 11, 23713),
            Library::MonoBdwgc,
            UNITY_2021_3_11F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2022, 3, 0, 4507),
            Library::MonoBdwgc,
            UNITY_2022_3_0F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2023, 1, 0, 2298),
            Library::MonoBdwgc,
            UNITY_2023_1_0F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2023, 1, 22, 16744),
            Library::MonoBdwgc,
            UNITY_2023_1_22F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 2, 12, 40285),
            Library::MonoBdwgc,
            UNITY_6000_2_12F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 3, 21, 9777),
            Library::MonoBdwgc,
            UNITY_6000_3_21F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 5, 8, 47071),
            Library::MonoBdwgc,
            UNITY_6000_5_8F1_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (6000, 7, 0, 5476),
            Library::MonoBdwgc,
            UNITY_6000_7_0A3_LINUX_MONO_BDWGC_X86_64,
        ),
    ];
    for (unity, library, profile) in profiles {
        let build = linux_builds::nearest(unity, library, PointerSize::Bit64).unwrap();
        assert_eq!(build.unity, unity, "{unity:?} {library:?}");
        assert_eq!(build.profile, profile, "{unity:?} {library:?}");
        assert_eq!(profile.library, library);
    }
}

#[cfg(feature = "alloc")]
#[test]
fn the_mac_constant_names_both_slices() {
    let mac =
        super::mac_builds::nearest((6000, 5, 10, 54518), Library::MonoBdwgc, PointerSize::Bit64)
            .unwrap();
    assert_eq!(mac.profile, UNITY_6000_5_10F1_MACOS_MONO_BDWGC);
    for build in super::mac_builds::BUILDS {
        assert_eq!(build.profile, UNITY_6000_5_10F1_MACOS_MONO_BDWGC);
    }
}

// The old runtime keeps a class's statics in the data slot of its vtable,
// and its profiles say so through the library.
#[test]
fn the_old_runtime_is_told_by_the_library() {
    assert_eq!(UNITY_2017_4_40F1_WINDOWS_MONO_X86_64.library, Library::Mono);
    assert_eq!(UNITY_2017_4_40F1_WINDOWS_MONO_X86_64.v_table.vtable, 0);
    assert_eq!(
        UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86_64.library,
        Library::MonoBdwgc
    );
    assert_eq!(
        UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86_64.v_table.vtable,
        0x40
    );
}

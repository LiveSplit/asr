//! Tests pinning the public profile constants: each is the layout of the
//! measured player it came from, and a module attaches with one directly.

use super::profiles::*;
use super::{builds, linux_builds, Library, Profile};
use crate::PointerSize;

// Every constant is pinned to its build, so an entry added between two
// others, which renumbers the ones after it, fails here rather than in a
// splitter.
#[test]
fn windows_constants_name_the_expected_profiles() {
    let profiles: [((u16, u16, u16, u16), Library, PointerSize, Profile); 8] = [
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
            (2017, 3, 0, 63597),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2017_3_0F3_WINDOWS_MONO_BDWGC_X86_64,
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
            (2018, 4, 36, 54151),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86,
        ),
        (
            (2021, 2, 0, 61932),
            Library::MonoBdwgc,
            PointerSize::Bit64,
            UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86_64,
        ),
        (
            (2021, 2, 0, 61932),
            Library::MonoBdwgc,
            PointerSize::Bit32,
            UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86,
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
    let profiles: [((u16, u16, u16, u16), Library, Profile); 4] = [
        (
            (5, 6, 7, 3267),
            Library::Mono,
            UNITY_5_6_7F1_LINUX_MONO_X86_64,
        ),
        (
            (2017, 3, 0, 63597),
            Library::MonoBdwgc,
            UNITY_2017_3_0F3_LINUX_MONO_BDWGC_X86_64,
        ),
        (
            (2017, 4, 40, 5126),
            Library::Mono,
            UNITY_2017_4_40F1_LINUX_MONO_X86_64,
        ),
        (
            (2021, 2, 20, 62729),
            Library::MonoBdwgc,
            UNITY_2021_2_20F1_LINUX_MONO_BDWGC_X86_64,
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
        UNITY_2017_3_0F3_WINDOWS_MONO_BDWGC_X86_64.library,
        Library::MonoBdwgc
    );
    assert_eq!(
        UNITY_2017_3_0F3_WINDOWS_MONO_BDWGC_X86_64.v_table.vtable,
        0x40
    );
}

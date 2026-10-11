//! Tests pinning the public profile constants: each is the layout of the
//! measured player it came from.

use super::profiles::*;
use super::{builds, linux_builds, Profile};
use crate::PointerSize;

use std::format;

// `Signature` has no `PartialEq`, so the profiles compare by their debug
// output.
fn same(a: &Profile, b: &Profile) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

// Every constant is pinned to its build, so an entry added between two
// others, which renumbers the ones after it, fails here rather than in a
// splitter.
#[test]
fn windows_constants_are_their_table_entries() {
    let profiles: [((u16, u16, u16, u16), PointerSize, Profile); 29] = [
        (
            (5, 6, 0, 23754),
            PointerSize::Bit64,
            UNITY_5_6_0F1_WINDOWS_X86_64,
        ),
        (
            (5, 6, 0, 23754),
            PointerSize::Bit32,
            UNITY_5_6_0F1_WINDOWS_X86,
        ),
        (
            (5, 6, 2, 37180),
            PointerSize::Bit64,
            UNITY_5_6_2F1_WINDOWS_X86_64,
        ),
        (
            (5, 6, 2, 37180),
            PointerSize::Bit32,
            UNITY_5_6_2F1_WINDOWS_X86,
        ),
        (
            (2017, 1, 0, 32737),
            PointerSize::Bit64,
            UNITY_2017_1_0F1_WINDOWS_X86_64,
        ),
        (
            (2017, 1, 0, 32737),
            PointerSize::Bit32,
            UNITY_2017_1_0F1_WINDOWS_X86,
        ),
        (
            (2017, 3, 0, 20311),
            PointerSize::Bit32,
            UNITY_2017_3_0F1_WINDOWS_X86,
        ),
        (
            (2018, 2, 0, 44229),
            PointerSize::Bit64,
            UNITY_2018_2_0F1_WINDOWS_X86_64,
        ),
        (
            (2018, 2, 0, 44229),
            PointerSize::Bit32,
            UNITY_2018_2_0F1_WINDOWS_X86,
        ),
        (
            (2018, 3, 0, 9156),
            PointerSize::Bit64,
            UNITY_2018_3_0F1_WINDOWS_X86_64,
        ),
        (
            (2018, 3, 0, 9156),
            PointerSize::Bit32,
            UNITY_2018_3_0F1_WINDOWS_X86,
        ),
        (
            (2021, 1, 0, 42313),
            PointerSize::Bit64,
            UNITY_2021_1_0F1_WINDOWS_X86_64,
        ),
        (
            (2022, 2, 0, 56532),
            PointerSize::Bit32,
            UNITY_2022_2_0F1_WINDOWS_X86,
        ),
        (
            (2022, 3, 5, 29734),
            PointerSize::Bit64,
            UNITY_2022_3_5F1_WINDOWS_X86_64,
        ),
        (
            (2022, 3, 5, 29734),
            PointerSize::Bit32,
            UNITY_2022_3_5F1_WINDOWS_X86,
        ),
        (
            (2023, 1, 0, 2298),
            PointerSize::Bit64,
            UNITY_2023_1_0F1_WINDOWS_X86_64,
        ),
        (
            (2023, 1, 0, 2298),
            PointerSize::Bit32,
            UNITY_2023_1_0F1_WINDOWS_X86,
        ),
        (
            (2023, 1, 4, 6702),
            PointerSize::Bit64,
            UNITY_2023_1_4F1_WINDOWS_X86_64,
        ),
        (
            (2023, 1, 4, 6702),
            PointerSize::Bit32,
            UNITY_2023_1_4F1_WINDOWS_X86,
        ),
        (
            (2023, 2, 0, 54845),
            PointerSize::Bit64,
            UNITY_2023_2_0F1_WINDOWS_X86_64,
        ),
        (
            (2023, 2, 0, 54845),
            PointerSize::Bit32,
            UNITY_2023_2_0F1_WINDOWS_X86,
        ),
        (
            (6000, 0, 59, 10268),
            PointerSize::Bit32,
            UNITY_6000_0_59F2_WINDOWS_X86,
        ),
        (
            (6000, 1, 0, 41298),
            PointerSize::Bit32,
            UNITY_6000_1_0F1_WINDOWS_X86,
        ),
        (
            (6000, 2, 0, 53701),
            PointerSize::Bit64,
            UNITY_6000_2_0F1_WINDOWS_X86_64,
        ),
        (
            (6000, 2, 2, 14734),
            PointerSize::Bit32,
            UNITY_6000_2_2F1_WINDOWS_X86,
        ),
        (
            (6000, 3, 0, 34572),
            PointerSize::Bit64,
            UNITY_6000_3_0F1_WINDOWS_X86_64,
        ),
        (
            (6000, 3, 0, 34572),
            PointerSize::Bit32,
            UNITY_6000_3_0F1_WINDOWS_X86,
        ),
        (
            (6000, 5, 0, 46204),
            PointerSize::Bit64,
            UNITY_6000_5_0F1_WINDOWS_X86_64,
        ),
        (
            (6000, 5, 0, 46204),
            PointerSize::Bit32,
            UNITY_6000_5_0F1_WINDOWS_X86,
        ),
    ];
    for (unity, pointer_size, profile) in profiles {
        let build = builds::nearest(unity, pointer_size).unwrap();
        assert_eq!(build.unity, unity, "{unity:?} {pointer_size:?}");
        assert!(same(&build.profile, &profile), "{unity:?} {pointer_size:?}");
        assert_eq!(profile.pointer_size, pointer_size);
    }
}

#[test]
fn linux_constants_are_their_table_entries() {
    let profiles: [((u16, u16, u16, u16), Profile); 17] = [
        ((5, 6, 0, 23754), UNITY_5_6_0F1_LINUX_X86_64),
        ((5, 6, 2, 37180), UNITY_5_6_2F1_LINUX_X86_64),
        ((2017, 1, 0, 32737), UNITY_2017_1_0F1_LINUX_X86_64),
        ((2017, 3, 0, 20311), UNITY_2017_3_0F1_LINUX_X86_64),
        ((2018, 2, 0, 44229), UNITY_2018_2_0F1_LINUX_X86_64),
        ((2018, 3, 0, 9156), UNITY_2018_3_0F1_LINUX_X86_64),
        ((2019, 1, 0, 21026), UNITY_2019_1_0F1_LINUX_X86_64),
        ((2019, 3, 0, 44266), UNITY_2019_3_0F1_LINUX_X86_64),
        ((2020, 2, 0, 8671), UNITY_2020_2_0F1_LINUX_X86_64),
        ((2021, 1, 0, 42313), UNITY_2021_1_0F1_LINUX_X86_64),
        ((2022, 3, 5, 29734), UNITY_2022_3_5F1_LINUX_X86_64),
        ((2023, 1, 0, 2298), UNITY_2023_1_0F1_LINUX_X86_64),
        ((2023, 1, 4, 6702), UNITY_2023_1_4F1_LINUX_X86_64),
        ((2023, 2, 0, 54845), UNITY_2023_2_0F1_LINUX_X86_64),
        ((6000, 3, 0, 34572), UNITY_6000_3_0F1_LINUX_X86_64),
        ((6000, 5, 0, 46204), UNITY_6000_5_0F1_LINUX_X86_64),
        ((6000, 6, 0, 63725), UNITY_6000_6_0F1_LINUX_X86_64),
    ];
    for (unity, profile) in profiles {
        let build = linux_builds::nearest(unity).unwrap();
        assert_eq!(build.unity, unity, "{unity:?}");
        assert!(same(&build.profile, &profile), "{unity:?}");
        assert_eq!(profile.pointer_size, PointerSize::Bit64);
    }
}

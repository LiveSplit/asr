//! Complete IL2CPP layouts measured from specific Unity players.
//!
//! Each constant describes one architecture of one measured player. Pass a
//! profile directly to [`Module::attach`](super::Module::attach) or
//! [`Module::wait_attach`](super::Module::wait_attach) when the target game is
//! known. This avoids linking the automatic lookup table and its other
//! profiles into the final auto splitter.

use super::{builds::BUILDS, Profile};

macro_rules! profiles {
    ($(#[$meta:meta] $name:ident = $index:literal;)+) => {
        $(
            #[$meta]
            pub const $name: Profile = BUILDS[$index].profile;
        )+
    };
}

profiles! {
    /// Unity 2018.4.36f1, file version `2018.4.36.54151`, x86-64.
    UNITY_2018_4_36F1_X86_64 = 0;
    /// Unity 2018.4.36f1, file version `2018.4.36.54151`, x86.
    UNITY_2018_4_36F1_X86 = 1;
    /// Unity 2019.1.0f2, file version `2019.1.0.11155`, x86-64.
    UNITY_2019_1_0F2_X86_64 = 2;
    /// Unity 2019.1.0f2, file version `2019.1.0.11155`, x86.
    UNITY_2019_1_0F2_X86 = 3;
    /// Unity 2019.4.41f2, file version `2019.4.41.9172`, x86-64.
    UNITY_2019_4_41F2_X86_64 = 4;
    /// Unity 2019.4.41f2, file version `2019.4.41.9172`, x86.
    UNITY_2019_4_41F2_X86 = 5;
    /// Unity 2020.1.18f1, file version `2020.1.18.38512`, x86-64.
    UNITY_2020_1_18F1_X86_64 = 6;
    /// Unity 2020.1.18f1, file version `2020.1.18.38512`, x86.
    UNITY_2020_1_18F1_X86 = 7;
    /// Unity 2020.2.0f1, file version `2020.2.0.8671`, x86-64.
    UNITY_2020_2_0F1_X86_64 = 8;
    /// Unity 2020.2.0f1, file version `2020.2.0.8671`, x86.
    UNITY_2020_2_0F1_X86 = 9;
    /// Unity 2021.3.11f1, file version `2021.3.11.23713`, x86-64.
    UNITY_2021_3_11F1_X86_64 = 10;
    /// Unity 2021.3.11f1, file version `2021.3.11.23713`, x86.
    UNITY_2021_3_11F1_X86 = 11;
    /// Unity 2022.3.0f1, file version `2022.3.0.4507`, x86-64.
    UNITY_2022_3_0F1_X86_64 = 12;
    /// Unity 2022.3.0f1, file version `2022.3.0.4507`, x86.
    UNITY_2022_3_0F1_X86 = 13;
    /// Unity 2023.1.0f1, file version `2023.1.0.2298`, x86-64.
    UNITY_2023_1_0F1_X86_64 = 14;
    /// Unity 2023.1.0f1, file version `2023.1.0.2298`, x86.
    UNITY_2023_1_0F1_X86 = 15;
    /// Unity 2023.1.22f1, file version `2023.1.22.16744`, x86-64.
    UNITY_2023_1_22F1_X86_64 = 16;
    /// Unity 2023.1.22f1, file version `2023.1.22.16744`, x86.
    UNITY_2023_1_22F1_X86 = 17;
    /// Unity 6000.2.12f1, file version `6000.2.12.40285`, x86-64.
    UNITY_6000_2_12F1_X86_64 = 18;
    /// Unity 6000.2.12f1, file version `6000.2.12.40285`, x86.
    UNITY_6000_2_12F1_X86 = 19;
    /// Unity 6000.3.21f1, file version `6000.3.21.9777`, x86-64.
    UNITY_6000_3_21F1_X86_64 = 20;
    /// Unity 6000.3.21f1, file version `6000.3.21.9777`, x86.
    UNITY_6000_3_21F1_X86 = 21;
    /// Unity 6000.5.10f1, file version `6000.5.10.54518`, x86-64.
    UNITY_6000_5_10F1_X86_64 = 22;
    /// Unity 6000.5.10f1, file version `6000.5.10.54518`, x86.
    UNITY_6000_5_10F1_X86 = 23;
    /// Unity 6000.6.0f1, file version `6000.6.0.63725`, x86-64.
    UNITY_6000_6_0F1_X86_64 = 24;
    /// Unity 6000.6.0f1, file version `6000.6.0.63725`, x86.
    UNITY_6000_6_0F1_X86 = 25;
    /// Unity 6000.7.0a3, file version `6000.7.0.5476`, x86-64.
    UNITY_6000_7_0A3_X86_64 = 26;
    /// Unity 6000.7.0a3, file version `6000.7.0.5476`, x86.
    UNITY_6000_7_0A3_X86 = 27;
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::PointerSize;

    #[test]
    fn constants_name_the_expected_profiles() {
        let profiles = [
            (
                (2018, 4, 36, 54151),
                UNITY_2018_4_36F1_X86_64,
                UNITY_2018_4_36F1_X86,
            ),
            (
                (2019, 1, 0, 11155),
                UNITY_2019_1_0F2_X86_64,
                UNITY_2019_1_0F2_X86,
            ),
            (
                (2019, 4, 41, 9172),
                UNITY_2019_4_41F2_X86_64,
                UNITY_2019_4_41F2_X86,
            ),
            (
                (2020, 1, 18, 38512),
                UNITY_2020_1_18F1_X86_64,
                UNITY_2020_1_18F1_X86,
            ),
            (
                (2020, 2, 0, 8671),
                UNITY_2020_2_0F1_X86_64,
                UNITY_2020_2_0F1_X86,
            ),
            (
                (2021, 3, 11, 23713),
                UNITY_2021_3_11F1_X86_64,
                UNITY_2021_3_11F1_X86,
            ),
            (
                (2022, 3, 0, 4507),
                UNITY_2022_3_0F1_X86_64,
                UNITY_2022_3_0F1_X86,
            ),
            (
                (2023, 1, 0, 2298),
                UNITY_2023_1_0F1_X86_64,
                UNITY_2023_1_0F1_X86,
            ),
            (
                (2023, 1, 22, 16744),
                UNITY_2023_1_22F1_X86_64,
                UNITY_2023_1_22F1_X86,
            ),
            (
                (6000, 2, 12, 40285),
                UNITY_6000_2_12F1_X86_64,
                UNITY_6000_2_12F1_X86,
            ),
            (
                (6000, 3, 21, 9777),
                UNITY_6000_3_21F1_X86_64,
                UNITY_6000_3_21F1_X86,
            ),
            (
                (6000, 5, 10, 54518),
                UNITY_6000_5_10F1_X86_64,
                UNITY_6000_5_10F1_X86,
            ),
            (
                (6000, 6, 0, 63725),
                UNITY_6000_6_0F1_X86_64,
                UNITY_6000_6_0F1_X86,
            ),
            (
                (6000, 7, 0, 5476),
                UNITY_6000_7_0A3_X86_64,
                UNITY_6000_7_0A3_X86,
            ),
        ];

        for (unity, x86_64, x86) in profiles {
            assert_eq!(
                super::super::builds::nearest(unity, PointerSize::Bit64)
                    .unwrap()
                    .profile,
                x86_64,
            );
            assert_eq!(
                super::super::builds::nearest(unity, PointerSize::Bit32)
                    .unwrap()
                    .profile,
                x86,
            );
        }
    }
}

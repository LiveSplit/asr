//! Complete IL2CPP layouts measured from specific Unity players.
//!
//! Each constant is the layout at one pointer size, measured on the first
//! player that has it. The layout holds until the version of the next
//! constant. Pass a profile directly to
//! [`Module::attach`](super::Module::attach) or
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
    /// Unity 2020.2.0f1, file version `2020.2.0.8671`, x86-64.
    UNITY_2020_2_0F1_X86_64 = 4;
    /// Unity 2020.2.0f1, file version `2020.2.0.8671`, x86.
    UNITY_2020_2_0F1_X86 = 5;
    /// Unity 2022.3.0f1, file version `2022.3.0.4507`, x86-64.
    UNITY_2022_3_0F1_X86_64 = 6;
    /// Unity 2022.3.0f1, file version `2022.3.0.4507`, x86.
    UNITY_2022_3_0F1_X86 = 7;
    /// Unity 6000.5.10f1, file version `6000.5.10.54518`, x86-64.
    UNITY_6000_5_10F1_X86_64 = 8;
    /// Unity 6000.5.10f1, file version `6000.5.10.54518`, x86.
    UNITY_6000_5_10F1_X86 = 9;
    /// Unity 6000.6.0f1, file version `6000.6.0.63725`, x86-64.
    UNITY_6000_6_0F1_X86_64 = 10;
    /// Unity 6000.6.0f1, file version `6000.6.0.63725`, x86.
    UNITY_6000_6_0F1_X86 = 11;
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::PointerSize;

    #[test]
    fn constants_are_the_expected_profiles() {
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
                (2020, 2, 0, 8671),
                UNITY_2020_2_0F1_X86_64,
                UNITY_2020_2_0F1_X86,
            ),
            (
                (2022, 3, 0, 4507),
                UNITY_2022_3_0F1_X86_64,
                UNITY_2022_3_0F1_X86,
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

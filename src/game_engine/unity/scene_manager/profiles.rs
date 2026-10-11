//! Complete scene manager layouts measured from specific Unity players.
//!
//! Each constant is the layout for the platform and pointer size in its
//! name, measured on the first player that has it. The layout holds until the
//! version of the next constant for the same platform and pointer size. Pass a
//! profile directly to [`SceneManager::attach`](super::SceneManager::attach)
//! or [`SceneManager::wait_attach`](super::SceneManager::wait_attach) when the
//! target game is known. This avoids linking the automatic lookup tables and
//! their other profiles into the final auto splitter.

use super::{builds, linux_builds, Profile};

macro_rules! profiles {
    ($table:path; $($(#[$meta:meta])* $name:ident = $index:literal;)+) => {
        $(
            $(#[$meta])*
            pub const $name: Profile = $table[$index].profile;
        )+
    };
}

profiles! {
    builds::BUILDS;
    /// Unity 5.6.0f1, Windows, x86-64, file version `5.6.0.23754`.
    UNITY_5_6_0F1_WINDOWS_X86_64 = 0;
    /// Unity 5.6.0f1, Windows, x86, file version `5.6.0.23754`.
    UNITY_5_6_0F1_WINDOWS_X86 = 1;
    /// Unity 5.6.2f1, Windows, x86-64, file version `5.6.2.37180`.
    UNITY_5_6_2F1_WINDOWS_X86_64 = 2;
    /// Unity 5.6.2f1, Windows, x86, file version `5.6.2.37180`.
    UNITY_5_6_2F1_WINDOWS_X86 = 3;
    /// Unity 2017.1.0f1, Windows, x86-64, file version `2017.1.0.32737`.
    UNITY_2017_1_0F1_WINDOWS_X86_64 = 4;
    /// Unity 2017.1.0f1, Windows, x86, file version `2017.1.0.32737`.
    UNITY_2017_1_0F1_WINDOWS_X86 = 5;
    /// Unity 2017.3.0f1, Windows, x86, file version `2017.3.0.20311`.
    UNITY_2017_3_0F1_WINDOWS_X86 = 6;
    /// Unity 2018.2.0f1, Windows, x86-64, file version `2018.2.0.44229`.
    UNITY_2018_2_0F1_WINDOWS_X86_64 = 7;
    /// Unity 2018.2.0f1, Windows, x86, file version `2018.2.0.44229`.
    UNITY_2018_2_0F1_WINDOWS_X86 = 8;
    /// Unity 2018.3.0f1, Windows, x86-64, file version `2018.3.0.9156`.
    UNITY_2018_3_0F1_WINDOWS_X86_64 = 9;
    /// Unity 2018.3.0f1, Windows, x86, file version `2018.3.0.9156`.
    UNITY_2018_3_0F1_WINDOWS_X86 = 10;
    /// Unity 2021.1.0f1, Windows, x86-64, file version `2021.1.0.42313`.
    UNITY_2021_1_0F1_WINDOWS_X86_64 = 11;
    /// Unity 2022.2.0f1, Windows, x86, file version `2022.2.0.56532`.
    UNITY_2022_2_0F1_WINDOWS_X86 = 12;
    /// Unity 2022.3.5f1, Windows, x86-64, file version `2022.3.5.29734`.
    UNITY_2022_3_5F1_WINDOWS_X86_64 = 13;
    /// Unity 2022.3.5f1, Windows, x86, file version `2022.3.5.29734`.
    UNITY_2022_3_5F1_WINDOWS_X86 = 14;
    /// Unity 2023.1.0f1, Windows, x86-64, file version `2023.1.0.2298`.
    UNITY_2023_1_0F1_WINDOWS_X86_64 = 15;
    /// Unity 2023.1.0f1, Windows, x86, file version `2023.1.0.2298`.
    UNITY_2023_1_0F1_WINDOWS_X86 = 16;
    /// Unity 2023.1.4f1, Windows, x86-64, file version `2023.1.4.6702`.
    UNITY_2023_1_4F1_WINDOWS_X86_64 = 17;
    /// Unity 2023.1.4f1, Windows, x86, file version `2023.1.4.6702`.
    UNITY_2023_1_4F1_WINDOWS_X86 = 18;
    /// Unity 2023.2.0f1, Windows, x86-64, file version `2023.2.0.54845`.
    UNITY_2023_2_0F1_WINDOWS_X86_64 = 19;
    /// Unity 2023.2.0f1, Windows, x86, file version `2023.2.0.54845`.
    UNITY_2023_2_0F1_WINDOWS_X86 = 20;
    /// Unity 6000.0.59f2, Windows, x86, file version `6000.0.59.10268`.
    UNITY_6000_0_59F2_WINDOWS_X86 = 21;
    /// Unity 6000.1.0f1, Windows, x86, file version `6000.1.0.41298`.
    UNITY_6000_1_0F1_WINDOWS_X86 = 22;
    /// Unity 6000.2.0f1, Windows, x86-64, file version `6000.2.0.53701`.
    UNITY_6000_2_0F1_WINDOWS_X86_64 = 23;
    /// Unity 6000.2.2f1, Windows, x86, file version `6000.2.2.14734`.
    UNITY_6000_2_2F1_WINDOWS_X86 = 24;
    /// Unity 6000.3.0f1, Windows, x86-64, file version `6000.3.0.34572`.
    UNITY_6000_3_0F1_WINDOWS_X86_64 = 25;
    /// Unity 6000.3.0f1, Windows, x86, file version `6000.3.0.34572`.
    UNITY_6000_3_0F1_WINDOWS_X86 = 26;
    /// Unity 6000.5.0f1, Windows, x86-64, file version `6000.5.0.46204`.
    UNITY_6000_5_0F1_WINDOWS_X86_64 = 27;
    /// Unity 6000.5.0f1, Windows, x86, file version `6000.5.0.46204`.
    UNITY_6000_5_0F1_WINDOWS_X86 = 28;
}

profiles! {
    linux_builds::BUILDS;
    /// Unity 5.6.0f1, Linux, x86-64.
    UNITY_5_6_0F1_LINUX_X86_64 = 0;
    /// Unity 5.6.2f1, Linux, x86-64.
    UNITY_5_6_2F1_LINUX_X86_64 = 1;
    /// Unity 2017.1.0f1, Linux, x86-64.
    UNITY_2017_1_0F1_LINUX_X86_64 = 2;
    /// Unity 2017.3.0f1, Linux, x86-64.
    UNITY_2017_3_0F1_LINUX_X86_64 = 3;
    /// Unity 2018.2.0f1, Linux, x86-64.
    UNITY_2018_2_0F1_LINUX_X86_64 = 4;
    /// Unity 2018.3.0f1, Linux, x86-64.
    UNITY_2018_3_0F1_LINUX_X86_64 = 5;
    /// Unity 2019.1.0f1, Linux, x86-64.
    UNITY_2019_1_0F1_LINUX_X86_64 = 6;
    /// Unity 2019.3.0f1, Linux, x86-64.
    UNITY_2019_3_0F1_LINUX_X86_64 = 7;
    /// Unity 2020.2.0f1, Linux, x86-64.
    UNITY_2020_2_0F1_LINUX_X86_64 = 8;
    /// Unity 2021.1.0f1, Linux, x86-64.
    UNITY_2021_1_0F1_LINUX_X86_64 = 9;
    /// Unity 2022.3.5f1, Linux, x86-64.
    UNITY_2022_3_5F1_LINUX_X86_64 = 10;
    /// Unity 2023.1.0f1, Linux, x86-64.
    UNITY_2023_1_0F1_LINUX_X86_64 = 11;
    /// Unity 2023.1.4f1, Linux, x86-64.
    UNITY_2023_1_4F1_LINUX_X86_64 = 12;
    /// Unity 2023.2.0f1, Linux, x86-64.
    UNITY_2023_2_0F1_LINUX_X86_64 = 13;
    /// Unity 6000.3.0f1, Linux, x86-64.
    UNITY_6000_3_0F1_LINUX_X86_64 = 14;
    /// Unity 6000.5.0f1, Linux, x86-64.
    UNITY_6000_5_0F1_LINUX_X86_64 = 15;
    /// Unity 6000.6.0f1, Linux, x86-64.
    UNITY_6000_6_0F1_LINUX_X86_64 = 16;
}

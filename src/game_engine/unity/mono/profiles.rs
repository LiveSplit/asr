//! Complete Mono layouts measured from specific Unity players.
//!
//! Each constant describes one runtime library of one architecture of one
//! measured player. Pass a profile directly to
//! [`Module::attach`](super::Module::attach) or
//! [`Module::wait_attach`](super::Module::wait_attach) when the target game
//! is known. This avoids linking the automatic lookup tables and their other
//! profiles into the final auto splitter.

#[cfg(feature = "alloc")]
use super::mac_builds;
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
    /// Unity 5.6.7f1, `mono.dll`, x86-64, file version `5.6.7.3267`.
    UNITY_5_6_7F1_WINDOWS_MONO_X86_64 = 11;
    /// Unity 5.6.7f1, `mono.dll`, x86, file version `5.6.7.3267`.
    UNITY_5_6_7F1_WINDOWS_MONO_X86 = 22;
    /// Unity 2017.4.40f1, `mono.dll`, x86-64, file version `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_X86_64 = 14;
    /// Unity 2017.4.40f1, `mono.dll`, x86, file version `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_X86 = 17;
    /// Unity 2017.4.40f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86_64 = 2;
    /// Unity 2017.4.40f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_BDWGC_X86 = 0;
    /// Unity 2018.4.36f1, `mono.dll`, x86-64, file version `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_X86_64 = 5;
    /// Unity 2018.4.36f1, `mono.dll`, x86, file version `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_X86 = 10;
    /// Unity 2018.4.36f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86_64 = 4;
    /// Unity 2018.4.36f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86 = 24;
    /// Unity 2019.4.41f2, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2019.4.41.9172`.
    UNITY_2019_4_41F2_WINDOWS_MONO_BDWGC_X86_64 = 20;
    /// Unity 2019.4.41f2, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2019.4.41.9172`.
    UNITY_2019_4_41F2_WINDOWS_MONO_BDWGC_X86 = 21;
    /// Unity 2020.1.18f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2020.1.18.38512`.
    UNITY_2020_1_18F1_WINDOWS_MONO_BDWGC_X86_64 = 13;
    /// Unity 2020.1.18f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2020.1.18.38512`.
    UNITY_2020_1_18F1_WINDOWS_MONO_BDWGC_X86 = 12;
    /// Unity 2021.2.20f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2021.2.20.62729`.
    UNITY_2021_2_20F1_WINDOWS_MONO_BDWGC_X86_64 = 16;
    /// Unity 2021.2.20f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2021.2.20.62729`.
    UNITY_2021_2_20F1_WINDOWS_MONO_BDWGC_X86 = 23;
    /// Unity 2021.3.11f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2021.3.11.23713`.
    UNITY_2021_3_11F1_WINDOWS_MONO_BDWGC_X86_64 = 3;
    /// Unity 2021.3.11f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2021.3.11.23713`.
    UNITY_2021_3_11F1_WINDOWS_MONO_BDWGC_X86 = 25;
    /// Unity 2023.1.22f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2023.1.22.16744`.
    UNITY_2023_1_22F1_WINDOWS_MONO_BDWGC_X86_64 = 19;
    /// Unity 2023.1.22f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2023.1.22.16744`.
    UNITY_2023_1_22F1_WINDOWS_MONO_BDWGC_X86 = 27;
    /// Unity 6000.2.12f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `6000.2.12.40285`.
    UNITY_6000_2_12F1_WINDOWS_MONO_BDWGC_X86_64 = 7;
    /// Unity 6000.2.12f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `6000.2.12.40285`.
    UNITY_6000_2_12F1_WINDOWS_MONO_BDWGC_X86 = 26;
    /// Unity 6000.3.21f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `6000.3.21.9777`.
    UNITY_6000_3_21F1_WINDOWS_MONO_BDWGC_X86_64 = 9;
    /// Unity 6000.3.21f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `6000.3.21.9777`.
    UNITY_6000_3_21F1_WINDOWS_MONO_BDWGC_X86 = 15;
    /// Unity 6000.5.8f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `6000.5.8.47071`.
    UNITY_6000_5_8F1_WINDOWS_MONO_BDWGC_X86_64 = 6;
    /// Unity 6000.5.8f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `6000.5.8.47071`.
    UNITY_6000_5_8F1_WINDOWS_MONO_BDWGC_X86 = 1;
    /// Unity 6000.7.0a3, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `6000.7.0.5476`.
    UNITY_6000_7_0A3_WINDOWS_MONO_BDWGC_X86_64 = 8;
    /// Unity 6000.7.0a3, `mono-2.0-bdwgc.dll`, x86, file version
    /// `6000.7.0.5476`.
    UNITY_6000_7_0A3_WINDOWS_MONO_BDWGC_X86 = 18;
}

profiles! {
    linux_builds::BUILDS;
    /// Unity 5.6.7f1, `libmono.so`, x86-64.
    UNITY_5_6_7F1_LINUX_MONO_X86_64 = 0;
    /// Unity 2017.4.40f1, `libmono.so`, x86-64.
    UNITY_2017_4_40F1_LINUX_MONO_X86_64 = 1;
    /// Unity 2018.4.36f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2018_4_36F1_LINUX_MONO_BDWGC_X86_64 = 2;
    /// Unity 2019.4.41f2, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2019_4_41F2_LINUX_MONO_BDWGC_X86_64 = 3;
    /// Unity 2021.3.0f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2021_3_0F1_LINUX_MONO_BDWGC_X86_64 = 4;
    /// Unity 2021.3.11f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2021_3_11F1_LINUX_MONO_BDWGC_X86_64 = 5;
    /// Unity 2022.3.0f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2022_3_0F1_LINUX_MONO_BDWGC_X86_64 = 6;
    /// Unity 2023.1.0f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2023_1_0F1_LINUX_MONO_BDWGC_X86_64 = 7;
    /// Unity 2023.1.22f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2023_1_22F1_LINUX_MONO_BDWGC_X86_64 = 8;
    /// Unity 6000.2.12f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_6000_2_12F1_LINUX_MONO_BDWGC_X86_64 = 9;
    /// Unity 6000.3.21f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_6000_3_21F1_LINUX_MONO_BDWGC_X86_64 = 10;
    /// Unity 6000.5.8f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_6000_5_8F1_LINUX_MONO_BDWGC_X86_64 = 11;
    /// Unity 6000.7.0a3, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_6000_7_0A3_LINUX_MONO_BDWGC_X86_64 = 12;
}

profiles! {
    mac_builds::BUILDS;
    /// Unity 6000.5.10f1, `libmonobdwgc-2.0.dylib`, x86-64 and arm64, which
    /// share one layout.
    #[cfg(feature = "alloc")]
    UNITY_6000_5_10F1_MACOS_MONO_BDWGC = 0;
}

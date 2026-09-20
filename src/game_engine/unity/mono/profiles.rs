//! Complete Mono layouts measured from specific Unity players.
//!
//! Each constant is the layout of one runtime library at one pointer size,
//! measured on the first player that has it. The layout holds until the
//! version of the next constant for that library. Pass a profile directly
//! to [`Module::attach`](super::Module::attach) or
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
    UNITY_5_6_7F1_WINDOWS_MONO_X86_64 = 0;
    /// Unity 5.6.7f1, `mono.dll`, x86, file version `5.6.7.3267`.
    UNITY_5_6_7F1_WINDOWS_MONO_X86 = 1;
    /// Unity 2017.4.40f1, `mono.dll`, x86-64, file version `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_X86_64 = 2;
    /// Unity 2017.4.40f1, `mono.dll`, x86, file version `2017.4.40.5126`.
    UNITY_2017_4_40F1_WINDOWS_MONO_X86 = 3;
    /// Unity 2018.4.36f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86_64 = 4;
    /// Unity 2018.4.36f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2018.4.36.54151`.
    UNITY_2018_4_36F1_WINDOWS_MONO_BDWGC_X86 = 5;
    /// Unity 2021.2.0f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2021.2.0.61932`.
    UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86_64 = 6;
    /// Unity 2021.2.0f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2021.2.0.61932`.
    UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86 = 7;
}

profiles! {
    linux_builds::BUILDS;
    /// Unity 5.6.7f1, `libmono.so`, x86-64.
    UNITY_5_6_7F1_LINUX_MONO_X86_64 = 0;
    /// Unity 2017.4.40f1, `libmono.so`, x86-64.
    UNITY_2017_4_40F1_LINUX_MONO_X86_64 = 1;
    /// Unity 2018.4.36f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2018_4_36F1_LINUX_MONO_BDWGC_X86_64 = 2;
    /// Unity 2021.2.20f1, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2021_2_20F1_LINUX_MONO_BDWGC_X86_64 = 3;
}

profiles! {
    mac_builds::BUILDS;
    /// Unity 6000.5.10f1, `libmonobdwgc-2.0.dylib`, x86-64 and arm64, which
    /// share one layout.
    #[cfg(feature = "alloc")]
    UNITY_6000_5_10F1_MACOS_MONO_BDWGC = 0;
}

//! Complete Mono layouts measured from specific Unity players.
//!
//! Each constant is the layout measured on one Unity player, for the
//! platform, runtime library and pointer size in its name. Matching a player
//! nobody measured by its version is a guess, not proof that the layout
//! changes exactly at the listed versions. Pass a profile directly
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
    /// Unity 5.0.0f4, `mono.dll`, x86-64, file version `5.0.0.39095`.
    UNITY_5_0_0F4_WINDOWS_MONO_X86_64 = 0;
    /// Unity 5.0.0f4, `mono.dll`, x86, file version `5.0.0.39095`.
    UNITY_5_0_0F4_WINDOWS_MONO_X86 = 1;
    /// Unity 2017.1.0f3, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2017.1.0.9747`.
    UNITY_2017_1_0F3_WINDOWS_MONO_BDWGC_X86_64 = 2;
    /// Unity 2017.1.0f3, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2017.1.0.9747`.
    UNITY_2017_1_0F3_WINDOWS_MONO_BDWGC_X86 = 3;
    /// Unity 2017.2.0f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2017.2.0.58714`.
    UNITY_2017_2_0F1_WINDOWS_MONO_BDWGC_X86_64 = 4;
    /// Unity 2017.2.0f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2017.2.0.58714`.
    UNITY_2017_2_0F1_WINDOWS_MONO_BDWGC_X86 = 5;
    /// Unity 2017.4.6f1, `mono.dll`, x86-64, file version `2017.4.6.20272`.
    UNITY_2017_4_6F1_WINDOWS_MONO_X86_64 = 6;
    /// Unity 2017.4.6f1, `mono.dll`, x86, file version `2017.4.6.20272`.
    UNITY_2017_4_6F1_WINDOWS_MONO_X86 = 7;
    /// Unity 2018.1.0f1, `mono.dll`, x86-64, file version `2018.1.0.30795`.
    UNITY_2018_1_0F1_WINDOWS_MONO_X86_64 = 8;
    /// Unity 2018.1.0f1, `mono.dll`, x86, file version `2018.1.0.30795`.
    UNITY_2018_1_0F1_WINDOWS_MONO_X86 = 9;
    /// Unity 2018.1.7f1, `mono.dll`, x86-64, file version `2018.1.7.46210`.
    UNITY_2018_1_7F1_WINDOWS_MONO_X86_64 = 10;
    /// Unity 2018.1.7f1, `mono.dll`, x86, file version `2018.1.7.46210`.
    UNITY_2018_1_7F1_WINDOWS_MONO_X86 = 11;
    /// Unity 2021.2.0f1, `mono-2.0-bdwgc.dll`, x86-64, file version
    /// `2021.2.0.61932`.
    UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86_64 = 12;
    /// Unity 2021.2.0f1, `mono-2.0-bdwgc.dll`, x86, file version
    /// `2021.2.0.61932`.
    UNITY_2021_2_0F1_WINDOWS_MONO_BDWGC_X86 = 13;
}

profiles! {
    linux_builds::BUILDS;
    /// Unity 5.6.7f1, `libmono.so`, x86-64.
    UNITY_5_6_7F1_LINUX_MONO_X86_64 = 0;
    /// Unity 2017.3.0f3, `libmonobdwgc-2.0.so`, x86-64.
    UNITY_2017_3_0F3_LINUX_MONO_BDWGC_X86_64 = 1;
    /// Unity 2017.4.40f1, `libmono.so`, x86-64.
    UNITY_2017_4_40F1_LINUX_MONO_X86_64 = 2;
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

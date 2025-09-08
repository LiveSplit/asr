use crate::{file_format::pe, PointerSize, Process};

/// The version of IL2CPP that was used for the game.
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    /// The base version.
    Base,
    /// The version used starting from Unity 2019.0
    V2019,
    /// The version used starting from Unity 2020.2
    V2020,
    /// The version used starting from Unity 2022.2
    V2022,
}

impl Version {
    pub(crate) fn detect(process: &Process) -> Option<Self> {
        let unity_module = process.get_module_address("UnityPlayer.dll").ok()?;

        if pe::MachineType::pointer_size(pe::MachineType::read(process, unity_module)?)?
            == PointerSize::Bit32
        {
            return Some(Self::Base);
        }

        let file_version = pe::file_version_info(process, unity_module)?;

        return Some(
            if file_version.major_version > 2023
                || (file_version.major_version == 2022 && file_version.minor_version >= 2)
            {
                Self::V2022
            } else if (file_version.major_version >= 2021 && file_version.major_version < 2023)
                || (file_version.major_version == 2020 && file_version.minor_version >= 2)
            {
                Self::V2020
            } else if (file_version.major_version == 2020 && file_version.minor_version < 2)
                || file_version.major_version == 2019
            {
                Self::V2019
            } else {
                Self::Base
            },
        );
    }
}

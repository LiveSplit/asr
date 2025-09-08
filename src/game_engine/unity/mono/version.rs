use super::{BinaryFormat, Module};
use crate::{file_format::pe, signature::Signature, Process};

/// The version of Mono that was used for the game. These don't correlate to the
/// Mono version numbers.
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    /// Version 1
    V1,
    /// Version 1 with cattrs
    V1Cattrs,
    /// Version 2
    V2,
    /// Version 3
    V3,
}

impl Version {
    pub(super) fn detect(process: &Process) -> Option<Version> {
        // First, check if Mono is being used (mono.dll on Windows or libmono.so on Linux).
        if process.get_module_address("mono.dll").is_ok()
            || process.get_module_address("libmono.so").is_ok()
        {
            // The mono.dll module is present -> could be either Version::V1 or Version::V1Cattrs.
            //
            // To tell them apart:
            // - Load the default Assembly-CSharp image.
            // - Get the first class in that image.
            // - Read the pointer to its name field.
            //
            // If the pointer equals the address of assembly image -> it's V1Cattrs.
            // Otherwise -> it's V1.
            //
            // Reference: https://github.com/Voxelse/Voxif/blob/main/Voxif.Helpers/Voxif.Helpers.UnityHelper/UnityHelper.cs#L343-L344
            let module = Module::attach(process, Version::V1)?;
            let image = module.get_default_image(process)?;
            let class = image.classes(process, &module).next()?;

            let pointer_to_image = process
                .read_pointer(class.class + module.offsets.class.name, module.pointer_size)
                .ok()?;

            return Some(if pointer_to_image.eq(&image.image) {
                Version::V1Cattrs
            } else {
                Version::V1
            });
        }

        // For more recent versions of Mono, we need the UnityPlayer module
        // - On Windows: UnityPlayer.dll
        // - On Linux/macOS: UnityPlayer.so.
        let (unity_module, binary_format) = [
            ("UnityPlayer.dll", BinaryFormat::PE),
            ("UnityPlayer.so", BinaryFormat::ELF),
        ]
        .into_iter()
        .find_map(|(name, format)| match format {
            BinaryFormat::PE => {
                let address = process.get_module_address(name).ok()?;
                let range = pe::read_size_of_image(process, address)? as u64;
                Some(((address, range), BinaryFormat::PE))
            }
            BinaryFormat::ELF => Some((process.get_module_range(name).ok()?, BinaryFormat::ELF)),
        })?;

        // For Windows (PE):
        //   We can read Unity’s FileVersionInfo directly from the PE header
        //   and infer the version from its major/minor numbers.
        if binary_format == BinaryFormat::PE {
            let file_version = pe::file_version_info(process, unity_module.0)?;
            return Some(
                if file_version.major_version > 2021
                    || (file_version.major_version == 2021 && file_version.minor_version >= 2)
                {
                    Version::V3
                } else {
                    Version::V2
                },
            );
        }

        // For ELF (Linux/macOS):
        //   No FileVersionInfo is available, so we fall back to scanning memory.
        //   Look for the ASCII signature "202?.", which appears in Unity’s version string.
        // TODO: find the unity version programmatically
        const SIG_202X: Signature<6> = Signature::new("00 32 30 32 ?? 2E");

        let Some(addr) = SIG_202X.scan_process_range(process, unity_module) else {
            return Some(Version::V2);
        };

        const ZERO: u8 = b'0';
        const NINE: u8 = b'9';

        let version_string = process.read::<[u8; 6]>(addr + 1).ok()?;

        let (before, after) =
            version_string.split_at(version_string.iter().position(|&x| x == b'.')?);

        let mut unity: u32 = 0;
        for &val in before {
            match val {
                ZERO..=NINE => unity = unity * 10 + (val - ZERO) as u32,
                _ => break,
            }
        }

        let mut unity_minor: u32 = 0;
        for &val in &after[1..] {
            match val {
                ZERO..=NINE => unity_minor = unity_minor * 10 + (val - ZERO) as u32,
                _ => break,
            }
        }

        Some(if (unity == 2021 && unity_minor >= 2) || (unity > 2021) {
            Version::V3
        } else {
            Version::V2
        })
    }
}
